# Integrity Chain (HIPAA-5)

Each event in the store can carry an HMAC-SHA256 signature over its content
plus the previous event's signature. Tampering with any historical row
invalidates every downstream signature — the chain is tamper-evident.

§164.312(c)(1) Integrity Controls.

## Algorithm

![Architecture Diagram - HMAC chain takes previous signature plus canonical event bytes, hashes with SHA-256 keyed by application secret, hex-encodes the 32 bytes; verify_chain re-runs the computation and compares against claimed signatures](../diagrams/architecture-23-integrity-chain.svg)

```
sig(0)   = ""                                                  -- genesis
sig(n)   = hex( HMAC-SHA256(key, sig(n-1) || canonical(event_n)) )
```

`canonical(event)` is JSON of the tuple
`(event_id, aggregate_type, aggregate_id, sequence, event_type, payload, timestamp)`
— the immutable parts of the event. `audit` fields are deliberately
**excluded** from the signature so audit metadata can be projected to
analytic stores in a different shape without invalidating the chain.

## API

```rust
trait IntegrityChain {
    fn sign_event(&self, prev: &EventSignature, event: &Event)
        -> Result<EventSignature, IntegrityError>;

    fn verify_chain(&self, events: &[(Event, EventSignature)])
        -> IntegrityResult;
}

let chain = HmacSha256Chain::new(thirty_two_byte_key)?;
let sig1 = chain.sign_event(&EventSignature::genesis(), &event1)?;
let sig2 = chain.sign_event(&sig1, &event2)?;

assert_eq!(
    chain.verify_chain(&[(event1, sig1), (event2, sig2)]),
    IntegrityResult::Valid
);
```

`verify_chain` reports the first failure as `IntegrityResult::Broken`:

- `BrokenAt { sequence, aggregate_id }` — signature mismatch (tamper)
- `OutOfOrder { expected, sequence, aggregate_id }` — gap or reordering

## Why HMAC, not a public-key signature

Per-event ECDSA/Ed25519 is overkill for a single-tenant audit chain. The
threat model is *internal* tampering by someone with DB write access, not
*external* impersonation of the framework. A symmetric HMAC keyed by a
secret only the application owns is enough evidence of tampering — and
~50× faster on the write path.

Step 5 may add a public-verifiable mode for cross-organization audit
hand-off; the trait surface accommodates it without breaking changes.

## Status

The trait + reference impl + 12 unit tests (including pinned test vectors)
land in HIPAA-5. **Wiring into `EventStore` is not yet done** — that's
Step 2. Until then projections can verify externally:

```rust
// load events for an aggregate (audit field excluded by canonical_bytes)
let events = store.load(&aggregate_id).await?;

// expect signatures alongside (Step 2 will return them in the same load)
let pairs: Vec<(Event, EventSignature)> = pair_events_with_signatures(events);

let chain = HmacSha256Chain::from_hex(&env::var("INTEGRITY_KEY")?)?;
match chain.verify_chain(&pairs) {
    IntegrityResult::Valid => log::info!("audit chain intact"),
    IntegrityResult::Broken(e) => alert(&e),
}
```

## Key management

Out of scope here. Real deployments:

- Load `INTEGRITY_KEY` from a KMS / Vault / sealed-secret at startup.
- Rotate by versioning: store `key_id` alongside the signature; `verify_chain`
  picks the right key per event.
- Never log the key. The `Debug` impl on `EventSignature` already truncates
  output to avoid leaking full hashes into telemetry.

## Pinned test vector

```
key:    "thirty-two-byte-known-test-key!!"
prev:   "" (genesis)
event:  Event {
    event_id: 00000000-0000-0000-0000-000000000000,
    aggregate_type: "Vector",
    aggregate_id: "vec-1",
    sequence: 1,
    event_type: "VectorEvent",
    payload: { "n": 1 },
    timestamp: 1700000000000,
}
sig:    7f519ff1222f551b490282cd220dda12f707a3979300b05d6f89f7a564749a9f
```

`canonical_bytes` changes will break this test vector — that is intentional.
Updating the canonical layout is a deliberate breaking change to the chain
format.
