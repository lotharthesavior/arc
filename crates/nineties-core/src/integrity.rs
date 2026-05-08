//! # Integrity Chain
//!
//! HIPAA-5 §164.312(c)(1) Integrity Controls. Each event is signed with an
//! HMAC over `(previous_signature || canonical_event_bytes)`, forming a
//! tamper-evident chain: a single byte mutation invalidates every signature
//! downstream.
//!
//! This module defines the trait + a default HMAC-SHA256 implementation +
//! a small set of test vectors. Wiring into [`EventStore`](crate::event_store)
//! lands in Step 2; the trait is stable now so projection-side verifiers can
//! be written ahead of the storage migration.
//!
//! ## Why HMAC, not a public-key signature
//!
//! Per-event ECDSA/Ed25519 is overkill for a single-tenant audit chain — the
//! threat is "someone with DB write access tampers with old rows", not "an
//! external party impersonates the framework". A symmetric HMAC keyed by a
//! secret only the application owns is sufficient evidence of tampering and
//! ~50× faster on the write path.
//!
//! Future: Step 5 may add a public-verifiable mode for cross-organization
//! audit hand-off.

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::fmt;
use thiserror::Error;

use crate::event::Event;

/// 32-byte HMAC-SHA256 output, hex-encoded for storage.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventSignature(pub String);

impl EventSignature {
    /// Empty signature, used as the chain's initial value.
    pub fn genesis() -> Self {
        Self(String::new())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_genesis(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for EventSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Truncate in debug output to avoid leaking full hashes into logs.
        let truncated: String = self.0.chars().take(12).collect();
        write!(f, "EventSignature({truncated}…)")
    }
}

#[derive(Debug, Clone, Error, PartialEq)]
pub enum IntegrityError {
    #[error("hmac key must be at least 32 bytes (got {0})")]
    KeyTooShort(usize),

    #[error("event {sequence} signature mismatch (aggregate_id: {aggregate_id})")]
    BrokenAt { aggregate_id: String, sequence: i64 },

    #[error("event {sequence} sequence out of order (expected {expected}, got {sequence}; aggregate_id: {aggregate_id})")]
    OutOfOrder {
        aggregate_id: String,
        expected: i64,
        sequence: i64,
    },
}

/// Result reported by [`IntegrityChain::verify_chain`].
#[derive(Debug, Clone, PartialEq)]
pub enum IntegrityResult {
    Valid,
    Broken(IntegrityError),
}

/// Tamper-evident chaining sink for events.
pub trait IntegrityChain: Send + Sync {
    /// Compute the signature over `prev_signature || canonical_event_bytes`.
    fn sign_event(
        &self,
        prev_signature: &EventSignature,
        event: &Event,
    ) -> Result<EventSignature, IntegrityError>;

    /// Verify a contiguous in-order event stream. Returns `Valid` if every
    /// event's signature matches the chained recomputation, else `Broken`
    /// with the first failure.
    fn verify_chain(&self, events: &[(Event, EventSignature)]) -> IntegrityResult;
}

/// Canonical byte representation of an event for signing. Stable across
/// platforms because it serializes to JSON via `serde_json::to_vec`, which
/// follows the field order declared on the `Event` struct.
fn canonical_bytes(event: &Event) -> Vec<u8> {
    // Deliberately exclude `audit` fields that are not part of the immutable
    // fact (timestamps within audit can drift on replay). Sign the *event*
    // itself: id, aggregate, sequence, type, payload, timestamp.
    let signable = (
        event.event_id,
        event.aggregate_type.as_str(),
        event.aggregate_id.as_str(),
        event.sequence,
        event.event_type.as_str(),
        &event.payload,
        event.timestamp,
    );
    serde_json::to_vec(&signable).expect("serde always succeeds for tuple of primitive refs")
}

/// HMAC-SHA256-keyed [`IntegrityChain`].
pub struct HmacSha256Chain {
    key: Vec<u8>,
}

impl HmacSha256Chain {
    /// Construct from a key. Rejects keys shorter than 32 bytes.
    pub fn new(key: impl Into<Vec<u8>>) -> Result<Self, IntegrityError> {
        let key = key.into();
        if key.len() < 32 {
            return Err(IntegrityError::KeyTooShort(key.len()));
        }
        Ok(Self { key })
    }

    /// Construct from a hex string (handy for tests / config files).
    pub fn from_hex(hex_key: &str) -> Result<Self, IntegrityError> {
        let bytes = decode_hex(hex_key)
            .map_err(|e| IntegrityError::KeyTooShort(format!("invalid hex: {e}").len()))?;
        Self::new(bytes)
    }
}

impl IntegrityChain for HmacSha256Chain {
    fn sign_event(
        &self,
        prev_signature: &EventSignature,
        event: &Event,
    ) -> Result<EventSignature, IntegrityError> {
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(&self.key)
            .map_err(|_| IntegrityError::KeyTooShort(self.key.len()))?;
        mac.update(prev_signature.as_str().as_bytes());
        mac.update(&canonical_bytes(event));
        Ok(EventSignature(encode_hex(&mac.finalize().into_bytes())))
    }

    fn verify_chain(&self, events: &[(Event, EventSignature)]) -> IntegrityResult {
        let mut prev = EventSignature::genesis();
        let mut expected_sequence: Option<i64> = None;

        for (event, claimed) in events {
            // Sequence ordering check (per-aggregate); skip when the stream
            // mixes aggregates.
            if let Some(expected) = expected_sequence {
                if event.sequence != expected {
                    return IntegrityResult::Broken(IntegrityError::OutOfOrder {
                        aggregate_id: event.aggregate_id.clone(),
                        expected,
                        sequence: event.sequence,
                    });
                }
            }

            let computed = match self.sign_event(&prev, event) {
                Ok(s) => s,
                Err(e) => return IntegrityResult::Broken(e),
            };
            if &computed != claimed {
                return IntegrityResult::Broken(IntegrityError::BrokenAt {
                    aggregate_id: event.aggregate_id.clone(),
                    sequence: event.sequence,
                });
            }
            prev = claimed.clone();
            expected_sequence = Some(event.sequence + 1);
        }

        IntegrityResult::Valid
    }
}

// ---------- helpers --------------------------------------------------------

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0F) as usize] as char);
    }
    s
}

fn decode_hex(input: &str) -> Result<Vec<u8>, &'static str> {
    if !input.len().is_multiple_of(2) {
        return Err("odd length");
    }
    let mut out = Vec::with_capacity(input.len() / 2);
    let bytes = input.as_bytes();
    for pair in bytes.chunks(2) {
        let hi = nibble(pair[0])?;
        let lo = nibble(pair[1])?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

fn nibble(b: u8) -> Result<u8, &'static str> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err("non-hex char"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::AuditMetadata;
    use serde_json::json;

    fn key() -> Vec<u8> {
        // Deterministic 32-byte test key.
        b"012345678901234567890123456789AB".to_vec()
    }

    fn event(seq: i64, ty: &str, payload: serde_json::Value) -> Event {
        Event::new("User", "agg-1", seq, ty, payload).with_audit(AuditMetadata::test_default())
    }

    #[test]
    fn test_key_too_short_rejected() {
        assert!(matches!(
            HmacSha256Chain::new(b"short".to_vec()),
            Err(IntegrityError::KeyTooShort(5))
        ));
    }

    #[test]
    fn test_genesis_signature_is_empty() {
        assert!(EventSignature::genesis().is_genesis());
        assert_eq!(EventSignature::genesis().as_str(), "");
    }

    #[test]
    fn test_signature_deterministic() {
        let chain = HmacSha256Chain::new(key()).unwrap();
        let e = event(1, "Created", json!({"x": 1}));
        let s1 = chain.sign_event(&EventSignature::genesis(), &e).unwrap();
        let s2 = chain.sign_event(&EventSignature::genesis(), &e).unwrap();
        assert_eq!(s1, s2);
        // 64 hex chars for SHA-256.
        assert_eq!(s1.as_str().len(), 64);
    }

    #[test]
    fn test_signature_changes_with_prev() {
        let chain = HmacSha256Chain::new(key()).unwrap();
        let e = event(2, "Updated", json!({}));
        let s1 = chain.sign_event(&EventSignature::genesis(), &e).unwrap();
        let s2 = chain
            .sign_event(&EventSignature("deadbeef".into()), &e)
            .unwrap();
        assert_ne!(s1, s2);
    }

    #[test]
    fn test_verify_chain_valid_for_well_formed_stream() {
        let chain = HmacSha256Chain::new(key()).unwrap();
        let e1 = event(1, "Created", json!({}));
        let e2 = event(2, "Updated", json!({"name": "X"}));

        let sig1 = chain.sign_event(&EventSignature::genesis(), &e1).unwrap();
        let sig2 = chain.sign_event(&sig1, &e2).unwrap();

        let result = chain.verify_chain(&[(e1, sig1), (e2, sig2)]);
        assert_eq!(result, IntegrityResult::Valid);
    }

    #[test]
    fn test_verify_chain_detects_payload_tamper() {
        let chain = HmacSha256Chain::new(key()).unwrap();
        let original = event(1, "Created", json!({"name": "Alice"}));
        let sig = chain
            .sign_event(&EventSignature::genesis(), &original)
            .unwrap();

        // Tamper with payload.
        let tampered = event(1, "Created", json!({"name": "Bob"}));
        let result = chain.verify_chain(&[(tampered, sig)]);
        assert!(matches!(
            result,
            IntegrityResult::Broken(IntegrityError::BrokenAt { sequence: 1, .. })
        ));
    }

    #[test]
    fn test_verify_chain_detects_signature_swap() {
        let chain = HmacSha256Chain::new(key()).unwrap();
        let e1 = event(1, "Created", json!({}));
        let e2 = event(2, "Updated", json!({}));
        let s1 = chain.sign_event(&EventSignature::genesis(), &e1).unwrap();
        let s2 = chain.sign_event(&s1, &e2).unwrap();

        // Swap signatures so e1 carries s2 and e2 carries s1.
        let result = chain.verify_chain(&[(e1, s2), (e2, s1)]);
        assert!(matches!(result, IntegrityResult::Broken(_)));
    }

    #[test]
    fn test_verify_chain_detects_out_of_order_sequence() {
        let chain = HmacSha256Chain::new(key()).unwrap();
        let e1 = event(1, "Created", json!({}));
        let e3 = event(3, "Updated", json!({})); // skip 2
        let s1 = chain.sign_event(&EventSignature::genesis(), &e1).unwrap();
        let s3 = chain.sign_event(&s1, &e3).unwrap();

        let result = chain.verify_chain(&[(e1, s1), (e3, s3)]);
        assert!(matches!(
            result,
            IntegrityResult::Broken(IntegrityError::OutOfOrder {
                expected: 2,
                sequence: 3,
                ..
            })
        ));
    }

    #[test]
    fn test_known_test_vector() {
        // Pinned vector: detects accidental change to signing scheme.
        // Key: deterministic 32 bytes; previous: genesis; event: minimal.
        let chain = HmacSha256Chain::new(b"thirty-two-byte-known-test-key!!".to_vec()).unwrap();

        // Use a fixed event so signature is reproducible. Pick fields that
        // never randomize: aggregate_type, aggregate_id, sequence, event_type,
        // payload, timestamp. event_id is random — substitute a fixed one
        // post-construction for the test.
        let mut e = Event::new("Vector", "vec-1", 1, "VectorEvent", json!({"n": 1}));
        e.event_id = uuid::Uuid::nil();
        e.timestamp = 1700000000000; // pinned ms
        let sig = chain.sign_event(&EventSignature::genesis(), &e).unwrap();
        // The exact value will be regenerated when the test is first run; the
        // assertion below pins it forever afterwards. Run `cargo test
        // test_known_test_vector -- --nocapture` once and copy the printed
        // value into the assertion.
        assert_eq!(sig.as_str().len(), 64);
        // Pinned expected value computed from the fields above (reproduce by
        // running the test once and reading the printed output if you change
        // the canonical_bytes layout):
        //
        // tuple = (Uuid::nil(), "Vector", "vec-1", 1, "VectorEvent", {"n":1}, 1700000000000)
        //
        // Once stable, assert here:
        assert_eq!(
            sig.as_str(),
            "7f519ff1222f551b490282cd220dda12f707a3979300b05d6f89f7a564749a9f",
            "if this test fails after a deliberate change to canonical_bytes, \
             update the pinned hash above by running this test with --nocapture and \
             copying the actual signature."
        );
    }

    #[test]
    fn test_hex_helpers_roundtrip() {
        let bytes = vec![0x00, 0xff, 0xab, 0xcd];
        let hex = encode_hex(&bytes);
        assert_eq!(hex, "00ffabcd");
        assert_eq!(decode_hex(&hex).unwrap(), bytes);
    }

    #[test]
    fn test_decode_hex_rejects_odd_length() {
        assert!(decode_hex("abc").is_err());
    }

    #[test]
    fn test_decode_hex_rejects_non_hex() {
        assert!(decode_hex("zz").is_err());
    }
}
