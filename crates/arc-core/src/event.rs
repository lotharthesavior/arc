//! # Event Module
//!
//! Core event type for event sourcing. Events represent immutable facts about
//! things that have happened in the system.
//!
//! ## Audit metadata
//!
//! Every event carries an [`AuditMetadata`](crate::audit::AuditMetadata) value.
//! Aggregates produce events with `AuditMetadata::pending()`; the
//! `CommandBus::dispatch` implementation overwrites that placeholder with a
//! validated audit struct sourced from the request `CommandContext` before
//! calling `EventStore::append`. Stores reject events whose audit fails
//! validation.
//!
//! ## Design Principles
//!
//! - **Immutable**: Once persisted, events cannot be changed.
//! - **Serializable**: All events can be stored as JSON.
//! - **Self-describing**: Events contain all metadata needed to understand them.
//! - **Ordered**: Events have a sequence number within their aggregate.
//! - **Audited**: Every persisted event carries `who/when/where/why` audit data.

use crate::audit::AuditMetadata;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Core event type representing an immutable domain event.
///
/// # Fields
///
/// - `event_id`: Unique identifier for this specific event occurrence
/// - `aggregate_type`: Type of aggregate this event belongs to (e.g., "User")
/// - `aggregate_id`: ID of the specific aggregate instance
/// - `sequence`: Sequential number within the aggregate stream (starts at 1)
/// - `event_type`: Type of event (e.g., "UserRegistered")
/// - `payload`: Event data as JSON (flexible, evolvable schema)
/// - `audit`: HIPAA audit metadata. `pending()` until the bus stamps it.
/// - `timestamp`: When the event occurred (milliseconds since UNIX epoch)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Event {
    /// Unique identifier for this event
    pub event_id: Uuid,

    /// Aggregate type (e.g., "User", "Order")
    pub aggregate_type: String,

    /// Aggregate instance identifier
    pub aggregate_id: String,

    /// Sequence number within the aggregate (starts at 1)
    pub sequence: i64,

    /// Event type (e.g., "UserCreated", "ProfileUpdated")
    /// Convention: Past tense, PascalCase
    pub event_type: String,

    /// Event payload as JSON
    pub payload: serde_json::Value,

    /// HIPAA audit metadata. `AuditMetadata::pending()` until the
    /// `CommandBus` overwrites it before `append`. `EventStore::append`
    /// implementations call `audit.validate()` and reject pending values.
    pub audit: AuditMetadata,

    /// Wall-clock timestamp (milliseconds since UNIX epoch). For HIPAA
    /// audit-quality time, use `audit.timestamp_utc_us` (microsecond precision).
    pub timestamp: u64,
}

impl Event {
    /// Create a new event with `audit = AuditMetadata::pending()`.
    ///
    /// Aggregates call this from `handle()` and the `CommandBus` overwrites the
    /// audit field with a request-scoped value before persisting. The
    /// pending placeholder fails store-side validation, so a forgotten stamp
    /// is impossible to commit.
    ///
    /// # Example
    ///
    /// ```rust
    /// use arc_core::event::Event;
    /// use serde_json::json;
    ///
    /// let event = Event::new(
    ///     "User",
    ///     "user-456",
    ///     1,
    ///     "UserCreated",
    ///     json!({ "name": "Bob", "email": "bob@example.com" }),
    /// );
    /// assert!(event.audit.is_pending());
    /// ```
    pub fn new(
        aggregate_type: impl Into<String>,
        aggregate_id: impl Into<String>,
        sequence: i64,
        event_type: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            aggregate_type: aggregate_type.into(),
            aggregate_id: aggregate_id.into(),
            sequence,
            event_type: event_type.into(),
            payload,
            audit: AuditMetadata::pending(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis() as u64,
        }
    }

    /// Replace `audit` with a fully-stamped value. Used by `CommandBus`
    /// before calling `EventStore::append`.
    pub fn with_audit(mut self, audit: AuditMetadata) -> Self {
        self.audit = audit;
        self
    }

    /// Serialize event to JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize event from JSON string.
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::AuditMetadata;
    use serde_json::json;

    #[test]
    fn test_event_creation() {
        let event = Event::new(
            "User",
            "user-123",
            1,
            "UserCreated",
            json!({
                "name": "Test User",
                "email": "test@example.com"
            }),
        );

        assert_eq!(event.aggregate_type, "User");
        assert_eq!(event.aggregate_id, "user-123");
        assert_eq!(event.sequence, 1);
        assert_eq!(event.event_type, "UserCreated");
        assert_eq!(event.payload["name"], "Test User");
        assert!(event.timestamp > 0);
        assert!(event.audit.is_pending());
    }

    #[test]
    fn test_with_audit_overwrites_pending() {
        let stamp = AuditMetadata::test_default();
        let event =
            Event::new("User", "user-1", 1, "UserCreated", json!({})).with_audit(stamp.clone());
        assert!(!event.audit.is_pending());
        assert_eq!(event.audit, stamp);
    }

    #[test]
    fn test_event_serialization_roundtrips_audit() {
        let event = Event::new(
            "User",
            "user-789",
            3,
            "ProfileUpdated",
            json!({"name": "X"}),
        )
        .with_audit(AuditMetadata::test_default());

        let json_str = event.to_json().unwrap();
        let deserialized = Event::from_json(&json_str).unwrap();

        assert_eq!(event.event_id, deserialized.event_id);
        assert_eq!(event.audit, deserialized.audit);
    }

    #[test]
    fn test_event_ordering() {
        let event1 = Event::new("User", "user-1", 1, "UserCreated", json!({}));
        let event2 = Event::new("User", "user-1", 2, "ProfileUpdated", json!({}));

        assert!(event1.sequence < event2.sequence);
    }

    #[test]
    fn test_event_uniqueness() {
        let event1 = Event::new("User", "user-1", 1, "UserCreated", json!({}));
        let event2 = Event::new("User", "user-1", 1, "UserCreated", json!({}));
        assert_ne!(event1.event_id, event2.event_id);
    }
}
