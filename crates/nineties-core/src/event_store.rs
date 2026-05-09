//! # Event Store Module
//!
//! Defines the [`EventStore`] trait for persisting and retrieving events.
//!
//! ## Design Principles
//!
//! - **Append-only**: events can only be added, never modified or deleted
//! - **Optimistic concurrency**: version-based conflict detection
//! - **Stream-based**: events can be loaded by aggregate or streamed globally
//! - **Audited**: every event must carry valid [`AuditMetadata`](crate::audit::AuditMetadata)
//!   when appended (HIPAA §164.312(b))
//! - **Pluggable**: multiple implementations (SQLite, Postgres, in-memory)
//!
//! ## HIPAA defense-in-depth
//!
//! `EventStore::append` MUST call `event.audit.validate()?` for each event
//! before persisting. The `CommandBus` validates first, but the store is the
//! durable boundary — it must not trust upstream.

use crate::audit::AuditError;
use crate::event::Event;
use async_trait::async_trait;
use thiserror::Error;

/// Version check strategy for optimistic concurrency control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionCheck {
    /// First event for this aggregate (expected version is 0)
    New,
    /// Require aggregate to be at this exact version
    Expected(i64),
    /// Automatically load and use current version (use sparingly)
    Auto,
}

impl VersionCheck {
    pub fn version(&self) -> Option<i64> {
        match self {
            VersionCheck::New => Some(0),
            VersionCheck::Expected(v) => Some(*v),
            VersionCheck::Auto => None,
        }
    }
}

/// Errors that can occur during event store operations.
#[derive(Debug, Error)]
pub enum EventStoreError {
    /// Optimistic concurrency conflict.
    #[error("Concurrency conflict: expected version {expected}, but aggregate is at version {actual} (aggregate_id: {aggregate_id})")]
    ConcurrencyConflict {
        aggregate_id: String,
        expected: i64,
        actual: i64,
    },

    #[error("Aggregate not found: {aggregate_id}")]
    AggregateNotFound { aggregate_id: String },

    #[error(
        "Invalid event sequence: expected {expected}, got {actual} (aggregate_id: {aggregate_id})"
    )]
    InvalidSequence {
        aggregate_id: String,
        expected: i64,
        actual: i64,
    },

    /// One or more events in an `append` batch had invalid audit metadata.
    /// Defense-in-depth: the bus should have caught this first.
    #[error(
        "Audit metadata validation failed for event {event_index} (aggregate_id: {aggregate_id}): {source}"
    )]
    InvalidAudit {
        aggregate_id: String,
        event_index: usize,
        #[source]
        source: AuditError,
    },

    #[error("Database error: {message}")]
    DatabaseError { message: String },

    #[error("Serialization error: {message}")]
    SerializationError { message: String },

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Event store error: {message}")]
    Other { message: String },
}

impl EventStoreError {
    pub fn database(message: impl Into<String>) -> Self {
        EventStoreError::DatabaseError {
            message: message.into(),
        }
    }

    pub fn serialization(message: impl Into<String>) -> Self {
        EventStoreError::SerializationError {
            message: message.into(),
        }
    }

    pub fn other(message: impl Into<String>) -> Self {
        EventStoreError::Other {
            message: message.into(),
        }
    }

    pub fn invalid_audit(
        aggregate_id: impl Into<String>,
        event_index: usize,
        source: AuditError,
    ) -> Self {
        EventStoreError::InvalidAudit {
            aggregate_id: aggregate_id.into(),
            event_index,
            source,
        }
    }
}

/// Result type for event store operations.
pub type EventStoreResult<T> = Result<T, EventStoreError>;

/// Helper for store implementations: validate every event's audit before persisting.
/// Returns `Err(EventStoreError::InvalidAudit)` on the first failure.
pub fn validate_audit_batch(aggregate_id: &str, events: &[Event]) -> EventStoreResult<()> {
    for (idx, ev) in events.iter().enumerate() {
        ev.audit
            .validate()
            .map_err(|e| EventStoreError::invalid_audit(aggregate_id, idx, e))?;
    }
    Ok(())
}

/// Trait for event store implementations.
///
/// `append` MUST invoke `validate_audit_batch` before persisting (HIPAA defense
/// in depth). The `CommandBus` also validates upstream — both layers run.
#[async_trait]
pub trait EventStore: Send + Sync {
    /// Append events to the store for a specific aggregate.
    ///
    /// Implementations must call `validate_audit_batch(aggregate_id, &events)?`
    /// before any persistence work.
    async fn append(
        &self,
        aggregate_id: &str,
        version_check: VersionCheck,
        events: Vec<Event>,
    ) -> EventStoreResult<()>;

    async fn load(&self, aggregate_id: &str) -> EventStoreResult<Vec<Event>>;

    async fn load_from(
        &self,
        aggregate_id: &str,
        from_sequence: i64,
    ) -> EventStoreResult<Vec<Event>>;

    async fn stream_all(&self, from_position: i64) -> EventStoreResult<Vec<Event>>;

    async fn get_version(&self, aggregate_id: &str) -> EventStoreResult<i64>;
}

// ─────────────────────────────────────────────────────────────────────────────
// In-memory implementation, public for downstream test code.
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(any(test, feature = "test-utils"))]
mod in_memory {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex as TokioMutex;

    /// In-memory event store. Available to downstream crates via the
    /// `test-utils` feature flag.
    ///
    /// Validates audit metadata on every append (same contract as production
    /// stores) so behavior matches what real implementations enforce.
    #[derive(Clone, Default)]
    pub struct InMemoryEventStore {
        events: Arc<TokioMutex<Vec<Event>>>,
    }

    impl InMemoryEventStore {
        pub fn new() -> Self {
            Self::default()
        }
    }

    #[async_trait]
    impl EventStore for InMemoryEventStore {
        async fn append(
            &self,
            aggregate_id: &str,
            version_check: VersionCheck,
            events: Vec<Event>,
        ) -> EventStoreResult<()> {
            validate_audit_batch(aggregate_id, &events)?;

            let mut store = self.events.lock().await;

            let current_version = store
                .iter()
                .filter(|e| e.aggregate_id == aggregate_id)
                .map(|e| e.sequence)
                .max()
                .unwrap_or(0);

            if let Some(expected) = version_check.version() {
                if current_version != expected {
                    return Err(EventStoreError::ConcurrencyConflict {
                        aggregate_id: aggregate_id.to_string(),
                        expected,
                        actual: current_version,
                    });
                }
            }

            store.extend(events);
            Ok(())
        }

        async fn load(&self, aggregate_id: &str) -> EventStoreResult<Vec<Event>> {
            let store = self.events.lock().await;
            Ok(store
                .iter()
                .filter(|e| e.aggregate_id == aggregate_id)
                .cloned()
                .collect())
        }

        async fn load_from(
            &self,
            aggregate_id: &str,
            from_sequence: i64,
        ) -> EventStoreResult<Vec<Event>> {
            let store = self.events.lock().await;
            Ok(store
                .iter()
                .filter(|e| e.aggregate_id == aggregate_id && e.sequence >= from_sequence)
                .cloned()
                .collect())
        }

        async fn stream_all(&self, from_position: i64) -> EventStoreResult<Vec<Event>> {
            let store = self.events.lock().await;
            Ok(store.iter().skip(from_position as usize).cloned().collect())
        }

        async fn get_version(&self, aggregate_id: &str) -> EventStoreResult<i64> {
            let store = self.events.lock().await;
            Ok(store
                .iter()
                .filter(|e| e.aggregate_id == aggregate_id)
                .map(|e| e.sequence)
                .max()
                .unwrap_or(0))
        }
    }
}

#[cfg(any(test, feature = "test-utils"))]
pub use in_memory::InMemoryEventStore;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::AuditMetadata;
    use crate::event::Event;
    use serde_json::json;

    #[test]
    fn test_version_check_new() {
        assert_eq!(VersionCheck::New.version(), Some(0));
    }

    #[test]
    fn test_version_check_expected() {
        assert_eq!(VersionCheck::Expected(5).version(), Some(5));
    }

    #[test]
    fn test_version_check_auto() {
        assert_eq!(VersionCheck::Auto.version(), None);
    }

    #[test]
    fn test_error_messages() {
        let error = EventStoreError::ConcurrencyConflict {
            aggregate_id: "user-123".to_string(),
            expected: 5,
            actual: 6,
        };
        let msg = error.to_string();
        assert!(msg.contains("expected version 5"));
        assert!(msg.contains("aggregate is at version 6"));
        assert!(msg.contains("user-123"));

        assert!(EventStoreError::database("X").to_string().contains("X"));
        assert!(EventStoreError::serialization("Y")
            .to_string()
            .contains("Y"));
    }

    #[test]
    fn test_validate_audit_batch_rejects_pending() {
        let mut e = Event::new("User", "u1", 1, "X", json!({}));
        e.audit = AuditMetadata::pending();
        let err = validate_audit_batch("u1", &[e]).unwrap_err();
        assert!(matches!(
            err,
            EventStoreError::InvalidAudit { event_index: 0, .. }
        ));
    }

    #[test]
    fn test_validate_audit_batch_passes_stamped() {
        let e =
            Event::new("User", "u1", 1, "X", json!({})).with_audit(AuditMetadata::test_default());
        validate_audit_batch("u1", &[e]).expect("stamped audit must pass");
    }

    #[tokio::test]
    async fn test_in_memory_store_rejects_pending_audit() {
        let store = InMemoryEventStore::new();
        let e = Event::new("User", "u1", 1, "X", json!({})); // pending
        let err = store
            .append("u1", VersionCheck::New, vec![e])
            .await
            .unwrap_err();
        assert!(matches!(err, EventStoreError::InvalidAudit { .. }));
    }

    #[tokio::test]
    async fn test_in_memory_store_persists_stamped_event() {
        let store = InMemoryEventStore::new();
        let e =
            Event::new("User", "u1", 1, "X", json!({})).with_audit(AuditMetadata::test_default());
        store
            .append("u1", VersionCheck::New, vec![e])
            .await
            .unwrap();
        let loaded = store.load("u1").await.unwrap();
        assert_eq!(loaded.len(), 1);
    }
}
