//! # Event Store Module
//!
//! Defines the EventStore trait for persisting and retrieving events.
//!
//! ## Design Principles
//!
//! - **Append-only**: Events can only be added, never modified or deleted
//! - **Optimistic concurrency**: Version-based conflict detection
//! - **Stream-based**: Events can be loaded by aggregate or streamed globally
//! - **Pluggable**: Multiple implementations (SQLite, Postgres, in-memory)
//!
//! ## Example
//!
//! ```rust,ignore
//! use nineties_core::event_store::{EventStore, VersionCheck};
//! use nineties_core::event::Event;
//!
//! # async fn example(store: impl EventStore) -> Result<(), Box<dyn std::error::Error>> {
//! // Append first event
//! store.append("user-123", VersionCheck::New, vec![event1]).await?;
//!
//! // Append with version check
//! store.append("user-123", VersionCheck::Expected(1), vec![event2]).await?;
//!
//! // Load all events for aggregate
//! let events = store.load("user-123").await?;
//! # Ok(())
//! # }
//! ```

use crate::event::Event;
use async_trait::async_trait;
use thiserror::Error;

/// Version check strategy for optimistic concurrency control.
///
/// Used when appending events to ensure no conflicting changes have occurred.
///
/// # Variants
///
/// - `New`: This is the first event for the aggregate (sequence should be 1)
/// - `Expected(version)`: Require aggregate to be at this exact version
/// - `Auto`: Load current version automatically (less safe, use sparingly)
///
/// # Example
///
/// ```rust
/// use nineties_core::event_store::VersionCheck;
///
/// // First event for an aggregate
/// let check = VersionCheck::New;
///
/// // Subsequent event - require version 5
/// let check = VersionCheck::Expected(5);
///
/// // Auto-load version (use with caution)
/// let check = VersionCheck::Auto;
/// ```
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
    /// Get the expected version number, or None for Auto mode.
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
    /// Optimistic concurrency conflict - aggregate version doesn't match
    #[error("Concurrency conflict: expected version {expected}, but aggregate is at version {actual} (aggregate_id: {aggregate_id})")]
    ConcurrencyConflict {
        aggregate_id: String,
        expected: i64,
        actual: i64,
    },

    /// Aggregate not found
    #[error("Aggregate not found: {aggregate_id}")]
    AggregateNotFound { aggregate_id: String },

    /// Event sequence violation (non-sequential sequence numbers)
    #[error(
        "Invalid event sequence: expected {expected}, got {actual} (aggregate_id: {aggregate_id})"
    )]
    InvalidSequence {
        aggregate_id: String,
        expected: i64,
        actual: i64,
    },

    /// Database connection error
    #[error("Database error: {message}")]
    DatabaseError { message: String },

    /// Serialization/deserialization error
    #[error("Serialization error: {message}")]
    SerializationError { message: String },

    /// General I/O error
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Other errors
    #[error("Event store error: {message}")]
    Other { message: String },
}

impl EventStoreError {
    /// Create a database error.
    pub fn database(message: impl Into<String>) -> Self {
        EventStoreError::DatabaseError {
            message: message.into(),
        }
    }

    /// Create a serialization error.
    pub fn serialization(message: impl Into<String>) -> Self {
        EventStoreError::SerializationError {
            message: message.into(),
        }
    }

    /// Create a generic error.
    pub fn other(message: impl Into<String>) -> Self {
        EventStoreError::Other {
            message: message.into(),
        }
    }
}

/// Result type for event store operations.
pub type EventStoreResult<T> = Result<T, EventStoreError>;

/// Trait for event store implementations.
///
/// The event store is the core persistence layer for event sourcing.
/// It provides:
/// - Append-only storage of events
/// - Optimistic concurrency control
/// - Event stream retrieval by aggregate or globally
/// - Guaranteed ordering within aggregates
///
/// # Implementation Requirements
///
/// - Events must be stored durably and in order
/// - Concurrent appends to the same aggregate must be detected (optimistic locking)
/// - Events within an aggregate must be loaded in sequence order
/// - Global stream must maintain total ordering (by event ID or timestamp)
///
/// # Thread Safety
///
/// Implementations must be Send + Sync to work with async Rust.
#[async_trait]
pub trait EventStore: Send + Sync {
    /// Append events to the store for a specific aggregate.
    ///
    /// # Arguments
    ///
    /// - `aggregate_id`: ID of the aggregate
    /// - `version_check`: Version check strategy for concurrency control
    /// - `events`: Vector of events to append
    ///
    /// # Returns
    ///
    /// - `Ok(())` if events were successfully appended
    /// - `Err(EventStoreError::ConcurrencyConflict)` if version check fails
    /// - `Err(EventStoreError::InvalidSequence)` if event sequences are invalid
    ///
    /// # Concurrency Guarantee
    ///
    /// If multiple concurrent appends occur for the same aggregate, only one will
    /// succeed. Others will receive a ConcurrencyConflict error and should retry.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use nineties_core::event_store::{EventStore, VersionCheck};
    ///
    /// # async fn example(store: impl EventStore, events: Vec<Event>) -> Result<(), Box<dyn std::error::Error>> {
    /// // First event
    /// store.append("user-123", VersionCheck::New, events).await?;
    ///
    /// // Subsequent event with version check
    /// store.append("user-123", VersionCheck::Expected(1), more_events).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn append(
        &self,
        aggregate_id: &str,
        version_check: VersionCheck,
        events: Vec<Event>,
    ) -> EventStoreResult<()>;

    /// Load all events for a specific aggregate.
    ///
    /// Events are returned in sequence order (oldest first).
    ///
    /// # Arguments
    ///
    /// - `aggregate_id`: ID of the aggregate
    ///
    /// # Returns
    ///
    /// - `Ok(Vec<Event>)` with events in sequence order
    /// - Empty vector if aggregate has no events
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// # async fn example(store: impl EventStore) -> Result<(), Box<dyn std::error::Error>> {
    /// let events = store.load("user-123").await?;
    /// for event in events {
    ///     println!("Event {}: {}", event.sequence, event.event_type);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn load(&self, aggregate_id: &str) -> EventStoreResult<Vec<Event>>;

    /// Load events for an aggregate starting from a specific sequence number.
    ///
    /// Useful for incremental loading or snapshot-based replay.
    ///
    /// # Arguments
    ///
    /// - `aggregate_id`: ID of the aggregate
    /// - `from_sequence`: Starting sequence number (inclusive)
    ///
    /// # Returns
    ///
    /// - `Ok(Vec<Event>)` with events from `from_sequence` onwards
    /// - Empty vector if no events exist from that sequence
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// # async fn example(store: impl EventStore) -> Result<(), Box<dyn std::error::Error>> {
    /// // Load events after snapshot at sequence 100
    /// let events = store.load_from("user-123", 101).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn load_from(
        &self,
        aggregate_id: &str,
        from_sequence: i64,
    ) -> EventStoreResult<Vec<Event>>;

    /// Stream all events globally, starting from a position.
    ///
    /// Used for building projections that need to process all events across
    /// all aggregates. Position is implementation-specific but typically
    /// corresponds to a global event ID or offset.
    ///
    /// # Arguments
    ///
    /// - `from_position`: Starting position (0 for all events)
    ///
    /// # Returns
    ///
    /// - `Ok(Vec<Event>)` with events in global order
    ///
    /// # Note
    ///
    /// For large event stores, consider implementing pagination or streaming
    /// variants of this method.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// # async fn example(store: impl EventStore) -> Result<(), Box<dyn std::error::Error>> {
    /// // Stream all events
    /// let all_events = store.stream_all(0).await?;
    ///
    /// // Stream events after position 1000
    /// let recent_events = store.stream_all(1000).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn stream_all(&self, from_position: i64) -> EventStoreResult<Vec<Event>>;

    /// Get the current version (latest sequence number) for an aggregate.
    ///
    /// Returns 0 if the aggregate has no events.
    ///
    /// # Arguments
    ///
    /// - `aggregate_id`: ID of the aggregate
    ///
    /// # Returns
    ///
    /// - `Ok(i64)` with the current version (0 if no events exist)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// # async fn example(store: impl EventStore) -> Result<(), Box<dyn std::error::Error>> {
    /// let version = store.get_version("user-123").await?;
    /// println!("Current version: {}", version);
    /// # Ok(())
    /// # }
    /// ```
    async fn get_version(&self, aggregate_id: &str) -> EventStoreResult<i64>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_check_new() {
        let check = VersionCheck::New;
        assert_eq!(check.version(), Some(0));
    }

    #[test]
    fn test_version_check_expected() {
        let check = VersionCheck::Expected(5);
        assert_eq!(check.version(), Some(5));
    }

    #[test]
    fn test_version_check_auto() {
        let check = VersionCheck::Auto;
        assert_eq!(check.version(), None);
    }

    #[test]
    fn test_error_messages() {
        let error = EventStoreError::ConcurrencyConflict {
            aggregate_id: "user-123".to_string(),
            expected: 5,
            actual: 6,
        };
        let error_msg = error.to_string();
        assert!(error_msg.contains("expected version 5"));
        assert!(error_msg.contains("aggregate is at version 6"));
        assert!(error_msg.contains("user-123"));

        let error = EventStoreError::database("Connection failed");
        assert!(error.to_string().contains("Connection failed"));

        let error = EventStoreError::serialization("Invalid JSON");
        assert!(error.to_string().contains("Invalid JSON"));
    }
}
