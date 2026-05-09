//! # Audit Module
//!
//! HIPAA §164.312(b) audit metadata for every persisted event.
//!
//! Every event in the store carries an `AuditMetadata` value documenting *who*
//! caused the change, *when* it happened, *from where*, and *why* (causation).
//! The `EventStore::append` implementation rejects events whose audit fails
//! [`AuditMetadata::validate`].
//!
//! ## Lifecycle
//!
//! 1. An aggregate's `handle()` returns `Vec<Event>` with `Event::audit ==
//!    AuditMetadata::pending()`. Aggregates do not deal with audit data.
//! 2. The `CommandBus` builds one [`AuditMetadata`] from the request-scoped
//!    `CommandContext` (with `timestamp_utc_us = now`) and stamps it on every
//!    event before calling `append`.
//! 3. Every store implementation calls `audit.validate()?` at the top of its
//!    `append` method as a defense-in-depth assertion.
//!
//! ## Why typed, not free-form JSON
//!
//! Field names cannot drift; required fields cannot be silently null; an
//! auditor's `SELECT WHERE actor_id = ?` query is reliable across every event
//! ever written.
//!
//! ## Reserved actor identifiers
//!
//! - `"system"`  — internal jobs, seeders, migrations
//! - `"anonymous"` — unauthenticated requests (e.g. self-registration)
//! - `"legacy-pre-hipaa"` — backfill sentinel for events written before this
//!   module existed (see migration `add_hipaa_audit`)

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use uuid::Uuid;

/// Sentinel `actor_id` for events written before HIPAA-1 landed.
pub const LEGACY_ACTOR: &str = "legacy-pre-hipaa";

/// Sentinel `actor_id` for system-internal commands (seeders, cron, migrations).
pub const SYSTEM_ACTOR: &str = "system";

/// Sentinel `actor_id` for unauthenticated requests.
pub const ANONYMOUS_ACTOR: &str = "anonymous";

/// Audit fields stamped on every persisted event.
///
/// See module docs for the lifecycle and reserved actor identifiers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditMetadata {
    /// Required. Aggregate UUID, `"system"`, `"anonymous"`, or `"legacy-pre-hipaa"`.
    /// Must be non-empty after trim.
    pub actor_id: String,

    /// Optional. The session this command was dispatched in. Pair with HIPAA-4
    /// server-side session store for revocation.
    pub actor_session_id: Option<String>,

    /// Optional. Source IP. `None` for system-internal commands.
    pub source_ip: Option<String>,

    /// Optional. `User-Agent` header.
    pub user_agent: Option<String>,

    /// Required. Microseconds since UNIX epoch. Must be `> 0`.
    pub timestamp_utc_us: i64,

    /// Optional. Event ID that triggered the command that produced this event
    /// (saga / projection-driven follow-up).
    pub causation_id: Option<Uuid>,

    /// Required. Groups every event from one logical request together.
    /// `CommandContext::system` synthesizes one for internal jobs.
    pub correlation_id: Uuid,
}

/// Validation errors for [`AuditMetadata`].
#[derive(Debug, Error, PartialEq)]
pub enum AuditError {
    #[error("audit.actor_id is empty (set 'system'/'anonymous' explicitly if intended)")]
    EmptyActorId,

    #[error("audit.timestamp_utc_us must be > 0 (was {0})")]
    InvalidTimestamp(i64),

    #[error("audit metadata is still in pending placeholder state — CommandBus did not stamp it before append")]
    PendingNotStamped,
}

impl AuditMetadata {
    /// Construct a new validated [`AuditMetadata`] from raw inputs.
    ///
    /// `timestamp_utc_us` is set from the system clock. Use
    /// [`AuditMetadata::with_timestamp`] to supply an explicit value (testing,
    /// replay, deterministic builds).
    pub fn new(actor_id: impl Into<String>, correlation_id: Uuid) -> Result<Self, AuditError> {
        let s = Self {
            actor_id: actor_id.into(),
            actor_session_id: None,
            source_ip: None,
            user_agent: None,
            timestamp_utc_us: now_us(),
            causation_id: None,
            correlation_id,
        };
        s.validate()?;
        Ok(s)
    }

    /// Convenience constructor for system-internal jobs (cron, seeders, migrations).
    /// Synthesizes a `correlation_id`. Always passes validation.
    pub fn system() -> Self {
        Self {
            actor_id: SYSTEM_ACTOR.to_string(),
            actor_session_id: None,
            source_ip: None,
            user_agent: None,
            timestamp_utc_us: now_us(),
            causation_id: None,
            correlation_id: Uuid::new_v4(),
        }
    }

    /// Placeholder value used by `Event::new`. The `CommandBus` overwrites this
    /// with a real value from the request `CommandContext` before `append`.
    /// Calling `validate()` on a pending value returns
    /// [`AuditError::PendingNotStamped`] — the store rejects the write.
    pub fn pending() -> Self {
        Self {
            actor_id: String::new(),
            actor_session_id: None,
            source_ip: None,
            user_agent: None,
            timestamp_utc_us: 0,
            causation_id: None,
            correlation_id: Uuid::nil(),
        }
    }

    /// True when `audit == pending()`. Stores reject pending audits at append-time.
    pub fn is_pending(&self) -> bool {
        self.actor_id.is_empty() && self.timestamp_utc_us == 0 && self.correlation_id.is_nil()
    }

    /// Defense-in-depth validation. Stores call this at the top of `append`.
    pub fn validate(&self) -> Result<(), AuditError> {
        if self.is_pending() {
            return Err(AuditError::PendingNotStamped);
        }
        if self.actor_id.trim().is_empty() {
            return Err(AuditError::EmptyActorId);
        }
        if self.timestamp_utc_us <= 0 {
            return Err(AuditError::InvalidTimestamp(self.timestamp_utc_us));
        }
        Ok(())
    }

    /// Convenience: produce a cheap, valid `AuditMetadata` for tests.
    /// Sets `actor_id = "test"`, fresh `correlation_id`, current timestamp.
    #[cfg(any(test, feature = "test-utils"))]
    pub fn test_default() -> Self {
        Self {
            actor_id: "test".to_string(),
            actor_session_id: None,
            source_ip: None,
            user_agent: None,
            timestamp_utc_us: now_us(),
            causation_id: None,
            correlation_id: Uuid::new_v4(),
        }
    }
}

/// Microseconds since UNIX epoch.
pub(crate) fn now_us() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_micros() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pending_fails_validation() {
        assert_eq!(
            AuditMetadata::pending().validate(),
            Err(AuditError::PendingNotStamped)
        );
    }

    #[test]
    fn test_empty_actor_rejected() {
        let mut m = AuditMetadata::system();
        m.actor_id = "   ".to_string();
        assert_eq!(m.validate(), Err(AuditError::EmptyActorId));
    }

    #[test]
    fn test_zero_timestamp_rejected() {
        let mut m = AuditMetadata::system();
        m.timestamp_utc_us = 0;
        // is_pending() guards on all-three-zero; here only ts is 0
        m.correlation_id = Uuid::new_v4();
        assert!(matches!(m.validate(), Err(AuditError::InvalidTimestamp(0))));
    }

    #[test]
    fn test_negative_timestamp_rejected() {
        let mut m = AuditMetadata::system();
        m.timestamp_utc_us = -1;
        assert!(matches!(
            m.validate(),
            Err(AuditError::InvalidTimestamp(-1))
        ));
    }

    #[test]
    fn test_system_passes() {
        AuditMetadata::system()
            .validate()
            .expect("system audit must validate");
    }

    #[test]
    fn test_test_default_passes() {
        AuditMetadata::test_default()
            .validate()
            .expect("test_default must validate");
    }

    #[test]
    fn test_legacy_actor_passes() {
        let m = AuditMetadata::new(LEGACY_ACTOR, Uuid::new_v4()).expect("legacy must construct");
        m.validate().expect("legacy must validate");
        assert_eq!(m.actor_id, "legacy-pre-hipaa");
    }

    #[test]
    fn test_new_rejects_empty() {
        assert_eq!(
            AuditMetadata::new("", Uuid::new_v4()),
            Err(AuditError::EmptyActorId)
        );
    }

    #[test]
    fn test_is_pending_recognizes_placeholder() {
        assert!(AuditMetadata::pending().is_pending());
        assert!(!AuditMetadata::system().is_pending());
        assert!(!AuditMetadata::test_default().is_pending());
    }

    #[test]
    fn test_serde_roundtrip() {
        let m = AuditMetadata::system();
        let s = serde_json::to_string(&m).unwrap();
        let back: AuditMetadata = serde_json::from_str(&s).unwrap();
        assert_eq!(m, back);
    }
}
