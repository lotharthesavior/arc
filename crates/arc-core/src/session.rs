//! # Session Store
//!
//! Server-side JWT session registry (HIPAA-4, §164.312(d) Person Authentication).
//! Where [`AuditMetadata`](crate::audit::AuditMetadata) audits writes and
//! [`AccessLogger`](crate::access_log::AccessLogger) audits reads, this trait
//! makes JWTs **revocable**: a stolen token stays valid only until the
//! corresponding `jti` is removed from the store.
//!
//! ## Failure semantics
//!
//! `is_valid` MUST fail closed when the underlying sink is unreachable:
//! revocation is a security control, and we cannot prove a token is *not*
//! revoked when the store is down. Middleware that consults this trait
//! returns 503, never 200.
//!
//! Note this is the **opposite** of the `AccessLogger` policy where read
//! audit failures fail open — different concerns, different defaults.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Errors emitted by [`SessionStore`] implementations.
#[derive(Debug, Error)]
pub enum SessionStoreError {
    #[error("session store sink failure: {0}")]
    Sink(String),
    #[error("session not found: {0}")]
    NotFound(Uuid),
    #[error("session store validation failure: {0}")]
    Validation(String),
}

/// Persistent record of an issued JWT session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionRecord {
    pub jti: Uuid,
    pub actor_id: String,
    pub created_at_us: i64,
    pub expires_at_us: i64,
    pub revoked_at_us: Option<i64>,
}

impl SessionRecord {
    /// True iff the record exists, has not been revoked, and has not expired
    /// at the supplied wall-clock instant.
    pub fn is_valid_at(&self, now_us: i64) -> bool {
        self.revoked_at_us.is_none() && self.expires_at_us > now_us
    }
}

/// Server-side registry of issued JWTs.
///
/// Implementations:
/// - [`InMemorySessionStore`] — `Arc<Mutex<HashMap>>`, ships in this crate
///   behind `test-utils` and is also a viable single-node production option
/// - `SqliteSessionStore` — in `arc-es-sqlite`, durable
/// - Future: Postgres, Redis
#[async_trait]
pub trait SessionStore: Send + Sync {
    /// Record a freshly-minted token. Called by the login controller before
    /// it returns the token to the client; on failure, the controller must
    /// refuse to hand out the token.
    async fn record_session(&self, record: SessionRecord) -> Result<(), SessionStoreError>;

    /// True if the `jti` is recorded, not revoked, and not expired.
    /// Returns `Err(Sink)` on store unavailability — middleware must fail closed.
    async fn is_valid(&self, jti: Uuid, now_us: i64) -> Result<bool, SessionStoreError>;

    /// Mark the session revoked. Idempotent; revoking an unknown `jti`
    /// returns `Err(NotFound)` so callers can distinguish "already gone"
    /// from "double revoke" if they care.
    async fn revoke(&self, jti: Uuid, now_us: i64) -> Result<(), SessionStoreError>;

    /// Bulk-revoke every active session for an actor (breach response).
    /// Returns the count of records affected.
    async fn revoke_all_for_actor(
        &self,
        actor_id: &str,
        now_us: i64,
    ) -> Result<usize, SessionStoreError>;

    /// Hard-delete records past their `expires_at_us`. Returns rows removed.
    /// Implementations may run this on a schedule; callers may also invoke
    /// it inline at startup.
    async fn prune_expired(&self, now_us: i64) -> Result<usize, SessionStoreError>;
}

// ─────────────────────────────────────────────────────────────────────────────
// In-memory implementation. Public behind the `test-utils` feature so
// downstream tests and small single-node deployments can use it.
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(any(test, feature = "test-utils"))]
mod in_memory {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[derive(Clone, Default)]
    pub struct InMemorySessionStore {
        inner: Arc<Mutex<HashMap<Uuid, SessionRecord>>>,
    }

    impl InMemorySessionStore {
        pub fn new() -> Self {
            Self::default()
        }
    }

    #[async_trait]
    impl SessionStore for InMemorySessionStore {
        async fn record_session(&self, record: SessionRecord) -> Result<(), SessionStoreError> {
            if record.actor_id.trim().is_empty() {
                return Err(SessionStoreError::Validation("actor_id empty".into()));
            }
            if record.expires_at_us <= record.created_at_us {
                return Err(SessionStoreError::Validation(
                    "expires_at_us must be > created_at_us".into(),
                ));
            }
            self.inner.lock().await.insert(record.jti, record);
            Ok(())
        }

        async fn is_valid(&self, jti: Uuid, now_us: i64) -> Result<bool, SessionStoreError> {
            let g = self.inner.lock().await;
            Ok(g.get(&jti).map(|r| r.is_valid_at(now_us)).unwrap_or(false))
        }

        async fn revoke(&self, jti: Uuid, now_us: i64) -> Result<(), SessionStoreError> {
            let mut g = self.inner.lock().await;
            match g.get_mut(&jti) {
                Some(r) => {
                    r.revoked_at_us = Some(now_us);
                    Ok(())
                }
                None => Err(SessionStoreError::NotFound(jti)),
            }
        }

        async fn revoke_all_for_actor(
            &self,
            actor_id: &str,
            now_us: i64,
        ) -> Result<usize, SessionStoreError> {
            let mut g = self.inner.lock().await;
            let mut n = 0;
            for r in g.values_mut() {
                if r.actor_id == actor_id && r.revoked_at_us.is_none() {
                    r.revoked_at_us = Some(now_us);
                    n += 1;
                }
            }
            Ok(n)
        }

        async fn prune_expired(&self, now_us: i64) -> Result<usize, SessionStoreError> {
            let mut g = self.inner.lock().await;
            let before = g.len();
            g.retain(|_, r| r.expires_at_us > now_us);
            Ok(before - g.len())
        }
    }
}

#[cfg(any(test, feature = "test-utils"))]
pub use in_memory::InMemorySessionStore;

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(jti: Uuid, actor: &str, ttl_us: i64) -> SessionRecord {
        let now = 1_700_000_000_000_000;
        SessionRecord {
            jti,
            actor_id: actor.to_string(),
            created_at_us: now,
            expires_at_us: now + ttl_us,
            revoked_at_us: None,
        }
    }

    #[tokio::test]
    async fn test_record_then_is_valid_true() {
        let s = InMemorySessionStore::new();
        let id = Uuid::new_v4();
        s.record_session(rec(id, "alice", 1_000_000)).await.unwrap();
        assert!(s.is_valid(id, 1_700_000_000_000_001).await.unwrap());
    }

    #[tokio::test]
    async fn test_unknown_jti_is_invalid() {
        let s = InMemorySessionStore::new();
        assert!(!s.is_valid(Uuid::new_v4(), 0).await.unwrap());
    }

    #[tokio::test]
    async fn test_revoke_makes_invalid() {
        let s = InMemorySessionStore::new();
        let id = Uuid::new_v4();
        s.record_session(rec(id, "alice", 1_000_000)).await.unwrap();
        s.revoke(id, 1_700_000_000_000_500).await.unwrap();
        assert!(!s.is_valid(id, 1_700_000_000_000_600).await.unwrap());
    }

    #[tokio::test]
    async fn test_revoke_unknown_returns_not_found() {
        let s = InMemorySessionStore::new();
        let id = Uuid::new_v4();
        let err = s.revoke(id, 0).await.unwrap_err();
        assert!(matches!(err, SessionStoreError::NotFound(j) if j == id));
    }

    #[tokio::test]
    async fn test_expired_session_is_invalid_without_revoke() {
        let s = InMemorySessionStore::new();
        let id = Uuid::new_v4();
        let now = 1_700_000_000_000_000;
        let r = SessionRecord {
            jti: id,
            actor_id: "alice".into(),
            created_at_us: now,
            expires_at_us: now + 100,
            revoked_at_us: None,
        };
        s.record_session(r).await.unwrap();
        assert!(s.is_valid(id, now + 50).await.unwrap());
        assert!(!s.is_valid(id, now + 200).await.unwrap());
    }

    #[tokio::test]
    async fn test_revoke_all_for_actor_only_targets_that_actor() {
        let s = InMemorySessionStore::new();
        let alice1 = Uuid::new_v4();
        let alice2 = Uuid::new_v4();
        let bob = Uuid::new_v4();
        s.record_session(rec(alice1, "alice", 1_000_000))
            .await
            .unwrap();
        s.record_session(rec(alice2, "alice", 1_000_000))
            .await
            .unwrap();
        s.record_session(rec(bob, "bob", 1_000_000)).await.unwrap();

        let now = 1_700_000_000_000_500;
        let n = s.revoke_all_for_actor("alice", now).await.unwrap();
        assert_eq!(n, 2);

        assert!(!s.is_valid(alice1, now + 1).await.unwrap());
        assert!(!s.is_valid(alice2, now + 1).await.unwrap());
        assert!(s.is_valid(bob, now + 1).await.unwrap());
    }

    #[tokio::test]
    async fn test_prune_expired_only_removes_expired() {
        let s = InMemorySessionStore::new();
        let now = 1_700_000_000_000_000;

        let live = Uuid::new_v4();
        let expired = Uuid::new_v4();
        s.record_session(SessionRecord {
            jti: live,
            actor_id: "a".into(),
            created_at_us: now,
            expires_at_us: now + 1_000_000,
            revoked_at_us: None,
        })
        .await
        .unwrap();
        s.record_session(SessionRecord {
            jti: expired,
            actor_id: "a".into(),
            created_at_us: now - 2000,
            expires_at_us: now - 1000,
            revoked_at_us: None,
        })
        .await
        .unwrap();

        let removed = s.prune_expired(now).await.unwrap();
        assert_eq!(removed, 1);
        assert!(s.is_valid(live, now + 1).await.unwrap());
        assert!(!s.is_valid(expired, now + 1).await.unwrap());
    }

    #[tokio::test]
    async fn test_record_session_validates_inputs() {
        let s = InMemorySessionStore::new();
        let bad = SessionRecord {
            jti: Uuid::new_v4(),
            actor_id: "  ".into(),
            created_at_us: 100,
            expires_at_us: 200,
            revoked_at_us: None,
        };
        assert!(matches!(
            s.record_session(bad).await.unwrap_err(),
            SessionStoreError::Validation(_)
        ));
    }

    #[tokio::test]
    async fn test_record_session_rejects_inverted_expiry() {
        let s = InMemorySessionStore::new();
        let bad = SessionRecord {
            jti: Uuid::new_v4(),
            actor_id: "alice".into(),
            created_at_us: 200,
            expires_at_us: 100,
            revoked_at_us: None,
        };
        assert!(matches!(
            s.record_session(bad).await.unwrap_err(),
            SessionStoreError::Validation(_)
        ));
    }
}
