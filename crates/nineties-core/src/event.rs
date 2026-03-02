//! # Event Module
//!
//! Core event type for event sourcing. Events represent immutable facts about things
//! that have happened in the system.
//!
//! ## Design Principles
//!
//! - **Immutable**: Once created, events cannot be changed
//! - **Serializable**: All events can be stored as JSON
//! - **Self-describing**: Events contain all metadata needed to understand them
//! - **Ordered**: Events have a sequence number within their aggregate
//!
//! ## Example
//!
//! ```rust
//! use nineties_core::event::Event;
//! use serde_json::json;
//!
//! let event = Event::new(
//!     "User",
//!     "user-123",
//!     1,
//!     "UserCreated",
//!     json!({
//!         "id": "user-123",
//!         "name": "Alice",
//!         "email": "alice@example.com"
//!     }),
//! );
//!
//! assert_eq!(event.aggregate_type, "User");
//! assert_eq!(event.sequence, 1);
//! ```

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Core event type representing an immutable domain event.
///
/// Events are the fundamental unit of event sourcing. They capture facts about
/// things that have happened in the system. Events are:
/// - Immutable once created
/// - Stored in an append-only event store
/// - Used to reconstruct aggregate state
/// - Published to event bus for subscribers
///
/// # Fields
///
/// - `event_id`: Unique identifier for this specific event occurrence
/// - `aggregate_type`: Type of aggregate this event belongs to (e.g., "User", "Order")
/// - `aggregate_id`: ID of the specific aggregate instance
/// - `sequence`: Sequential number within the aggregate stream (starts at 1)
/// - `event_type`: Type of event (e.g., "UserCreated", "OrderShipped")
/// - `payload`: Event data as JSON (flexible, evolvable schema)
/// - `metadata`: Additional context (causation_id, correlation_id, user_id, etc.)
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

    /// Event metadata (causation, correlation, user context, etc.)
    pub metadata: serde_json::Value,

    /// When the event occurred (milliseconds since UNIX epoch)
    pub timestamp: u64,
}

impl Event {
    /// Create a new event with the given parameters.
    ///
    /// Generates a unique event_id and sets timestamp to now.
    /// Metadata is initialized as an empty JSON object.
    ///
    /// # Arguments
    ///
    /// - `aggregate_type`: Type of aggregate (e.g., "User")
    /// - `aggregate_id`: ID of the aggregate instance
    /// - `sequence`: Sequence number (1 for first event, 2 for second, etc.)
    /// - `event_type`: Type of event (e.g., "UserCreated")
    /// - `payload`: Event data as JSON
    ///
    /// # Example
    ///
    /// ```rust
    /// use nineties_core::event::Event;
    /// use serde_json::json;
    ///
    /// let event = Event::new(
    ///     "User",
    ///     "user-456",
    ///     1,
    ///     "UserCreated",
    ///     json!({ "name": "Bob", "email": "bob@example.com" }),
    /// );
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
            metadata: serde_json::json!({}),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis() as u64,
        }
    }

    /// Create an event with custom metadata.
    ///
    /// Useful for adding causation_id, correlation_id, user_id, etc.
    ///
    /// # Example
    ///
    /// ```rust
    /// use nineties_core::event::Event;
    /// use serde_json::json;
    ///
    /// let event = Event::with_metadata(
    ///     "User",
    ///     "user-789",
    ///     2,
    ///     "ProfileUpdated",
    ///     json!({ "name": "New Name" }),
    ///     json!({
    ///         "user_id": "admin-1",
    ///         "correlation_id": "req-abc-123",
    ///         "ip_address": "192.168.1.1"
    ///     }),
    /// );
    /// ```
    pub fn with_metadata(
        aggregate_type: impl Into<String>,
        aggregate_id: impl Into<String>,
        sequence: i64,
        event_type: impl Into<String>,
        payload: serde_json::Value,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            aggregate_type: aggregate_type.into(),
            aggregate_id: aggregate_id.into(),
            sequence,
            event_type: event_type.into(),
            payload,
            metadata,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis() as u64,
        }
    }

    /// Add or update metadata field.
    ///
    /// # Example
    ///
    /// ```rust
    /// use nineties_core::event::Event;
    /// use serde_json::json;
    ///
    /// let mut event = Event::new("User", "user-1", 1, "UserCreated", json!({}));
    /// event.add_metadata("user_id", json!("admin-1"));
    /// ```
    pub fn add_metadata(&mut self, key: impl Into<String>, value: serde_json::Value) {
        if let serde_json::Value::Object(ref mut map) = self.metadata {
            map.insert(key.into(), value);
        }
    }

    /// Get metadata value by key.
    ///
    /// # Example
    ///
    /// ```rust
    /// use nineties_core::event::Event;
    /// use serde_json::json;
    ///
    /// let event = Event::with_metadata(
    ///     "User",
    ///     "user-1",
    ///     1,
    ///     "UserCreated",
    ///     json!({}),
    ///     json!({ "user_id": "admin-1" }),
    /// );
    ///
    /// assert_eq!(event.get_metadata("user_id"), Some(&json!("admin-1")));
    /// ```
    pub fn get_metadata(&self, key: &str) -> Option<&serde_json::Value> {
        if let serde_json::Value::Object(ref map) = self.metadata {
            map.get(key)
        } else {
            None
        }
    }

    /// Serialize event to JSON string.
    ///
    /// Useful for storage or debugging.
    ///
    /// # Example
    ///
    /// ```rust
    /// use nineties_core::event::Event;
    /// use serde_json::json;
    ///
    /// let event = Event::new("User", "user-1", 1, "UserCreated", json!({}));
    /// let json_str = event.to_json().unwrap();
    /// assert!(json_str.contains("UserCreated"));
    /// ```
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize event from JSON string.
    ///
    /// # Example
    ///
    /// ```rust
    /// use nineties_core::event::Event;
    /// use serde_json::json;
    ///
    /// let original = Event::new("User", "user-1", 1, "UserCreated", json!({}));
    /// let json_str = original.to_json().unwrap();
    /// let deserialized = Event::from_json(&json_str).unwrap();
    ///
    /// assert_eq!(original.event_type, deserialized.event_type);
    /// assert_eq!(original.aggregate_id, deserialized.aggregate_id);
    /// ```
    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    }

    #[test]
    fn test_event_with_metadata() {
        let event = Event::with_metadata(
            "Order",
            "order-456",
            2,
            "OrderShipped",
            json!({ "tracking": "ABC123" }),
            json!({
                "user_id": "admin-1",
                "correlation_id": "req-xyz"
            }),
        );

        assert_eq!(event.metadata["user_id"], "admin-1");
        assert_eq!(event.metadata["correlation_id"], "req-xyz");
    }

    #[test]
    fn test_add_metadata() {
        let mut event = Event::new("User", "user-1", 1, "UserCreated", json!({}));

        event.add_metadata("ip_address", json!("192.168.1.1"));
        event.add_metadata("user_agent", json!("Mozilla/5.0"));

        assert_eq!(
            event.get_metadata("ip_address"),
            Some(&json!("192.168.1.1"))
        );
        assert_eq!(
            event.get_metadata("user_agent"),
            Some(&json!("Mozilla/5.0"))
        );
        assert_eq!(event.get_metadata("nonexistent"), None);
    }

    #[test]
    fn test_event_serialization() {
        let event = Event::new(
            "User",
            "user-789",
            3,
            "ProfileUpdated",
            json!({ "name": "New Name" }),
        );

        let json_str = event.to_json().unwrap();
        let deserialized = Event::from_json(&json_str).unwrap();

        assert_eq!(event.event_id, deserialized.event_id);
        assert_eq!(event.aggregate_type, deserialized.aggregate_type);
        assert_eq!(event.aggregate_id, deserialized.aggregate_id);
        assert_eq!(event.sequence, deserialized.sequence);
        assert_eq!(event.event_type, deserialized.event_type);
        assert_eq!(event.payload, deserialized.payload);
        assert_eq!(event.timestamp, deserialized.timestamp);
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

        // Event IDs should be unique even for identical content
        assert_ne!(event1.event_id, event2.event_id);
    }

    #[test]
    fn test_event_immutability_concept() {
        // Events should represent immutable facts
        // This test documents the concept even though Rust doesn't enforce it at runtime
        let event = Event::new("User", "user-1", 1, "UserCreated", json!({}));

        // In practice, events should never be modified after creation
        // The event store will reject updates to existing events
        assert_eq!(event.sequence, 1);
    }
}
