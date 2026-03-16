//! # Nineties ES SQLite
//!
//! SQLite implementation of the EventStore trait from nineties-core.
//!
//! ## Features
//!
//! - SQLite-backed event store with optimistic concurrency control
//! - Connection pool support via r2d2
//! - Thread-safe implementation
//! - Comprehensive error handling
//!
//! ## Example
//!
//! ```rust,no_run
//! use nineties_es_sqlite::SqliteEventStore;
//! use nineties_core::event::Event;
//! use nineties_core::event_store::{EventStore, VersionCheck};
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create event store
//! let store = SqliteEventStore::new("events.db").await?;
//!
//! // Create and append event
//! let event = Event::new("User", "user-123", 1, "UserCreated", json!({
//!     "name": "Alice",
//!     "email": "alice@example.com"
//! }));
//!
//! store.append("user-123", VersionCheck::New, vec![event]).await?;
//!
//! // Load events
//! let events = store.load("user-123").await?;
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use diesel::sqlite::SqliteConnection;
use nineties_core::event::Event;
use nineties_core::event_store::{EventStore, EventStoreError, EventStoreResult, VersionCheck};
use std::sync::Arc;
use uuid::Uuid;

// Re-export for convenience
pub use nineties_core::{Deserialize, Serialize};

/// Database model for events table (for insertion)
#[derive(Debug, Insertable, Clone)]
#[diesel(table_name = events)]
struct NewEventRecord {
    pub event_id: String,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub sequence: i32,
    pub event_type: String,
    pub payload: String,
    pub metadata: Option<String>,
    pub timestamp: i32,
}

/// Database model for events table (for queries)
#[derive(Debug, Queryable, Clone)]
struct EventRecord {
    #[allow(dead_code)]
    pub id: Option<i32>,
    pub event_id: String,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub sequence: i32,
    pub event_type: String,
    pub payload: String,
    pub metadata: Option<String>,
    pub timestamp: i32,
}

impl NewEventRecord {
    /// Convert from Event to NewEventRecord
    fn from_event(event: &Event) -> Result<Self, EventStoreError> {
        Ok(NewEventRecord {
            event_id: event.event_id.to_string(),
            aggregate_type: event.aggregate_type.clone(),
            aggregate_id: event.aggregate_id.clone(),
            sequence: event.sequence as i32,
            event_type: event.event_type.clone(),
            payload: serde_json::to_string(&event.payload)
                .map_err(|e| EventStoreError::serialization(e.to_string()))?,
            metadata: Some(
                serde_json::to_string(&event.metadata)
                    .map_err(|e| EventStoreError::serialization(e.to_string()))?,
            ),
            timestamp: (event.timestamp / 1000) as i32, // Convert milliseconds to seconds for i32
        })
    }
}

impl EventRecord {
    /// Convert from EventRecord to Event
    fn to_event(&self) -> Result<Event, EventStoreError> {
        let event_id = Uuid::parse_str(&self.event_id)
            .map_err(|e| EventStoreError::serialization(format!("Invalid UUID: {}", e)))?;

        let payload: serde_json::Value = serde_json::from_str(&self.payload)
            .map_err(|e| EventStoreError::serialization(e.to_string()))?;

        let metadata: serde_json::Value = if let Some(ref meta_str) = self.metadata {
            serde_json::from_str(meta_str)
                .map_err(|e| EventStoreError::serialization(e.to_string()))?
        } else {
            serde_json::json!({})
        };

        Ok(Event {
            event_id,
            aggregate_type: self.aggregate_type.clone(),
            aggregate_id: self.aggregate_id.clone(),
            sequence: self.sequence as i64,
            event_type: self.event_type.clone(),
            payload,
            metadata,
            timestamp: (self.timestamp as u64) * 1000, // Convert seconds back to milliseconds
        })
    }
}

// Define the schema inline for the crate
mod schema {
    diesel::table! {
        events (id) {
            id -> Nullable<Integer>,
            event_id -> Text,
            aggregate_type -> Text,
            aggregate_id -> Text,
            sequence -> Integer,
            event_type -> Text,
            payload -> Text,
            metadata -> Nullable<Text>,
            timestamp -> Integer,
        }
    }
}

use schema::events;

type Pool = r2d2::Pool<ConnectionManager<SqliteConnection>>;

/// SQLite implementation of EventStore.
///
/// Provides thread-safe, connection-pooled access to SQLite event storage.
/// Supports optimistic concurrency control via version checking.
#[derive(Clone)]
pub struct SqliteEventStore {
    pool: Arc<Pool>,
}

impl SqliteEventStore {
    /// Create a new SQLite event store with the given database URL.
    ///
    /// # Arguments
    ///
    /// - `database_url`: Path to SQLite database file (e.g., "events.db" or ":memory:")
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// use nineties_es_sqlite::SqliteEventStore;
    ///
    /// // File-based database
    /// let store = SqliteEventStore::new("events.db").await?;
    ///
    /// // In-memory database (for testing)
    /// let test_store = SqliteEventStore::new(":memory:").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(database_url: &str) -> EventStoreResult<Self> {
        let manager = ConnectionManager::<SqliteConnection>::new(database_url);
        let pool = Pool::builder()
            .max_size(10)
            .build(manager)
            .map_err(|e| EventStoreError::database(format!("Failed to create pool: {}", e)))?;

        Ok(SqliteEventStore {
            pool: Arc::new(pool),
        })
    }

    /// Create a new SQLite event store with a custom pool.
    ///
    /// Useful for advanced configuration or testing.
    pub fn with_pool(pool: Pool) -> Self {
        SqliteEventStore {
            pool: Arc::new(pool),
        }
    }
}

#[async_trait]
impl EventStore for SqliteEventStore {
    async fn append(
        &self,
        aggregate_id: &str,
        version_check: VersionCheck,
        new_events: Vec<Event>,
    ) -> EventStoreResult<()> {
        if new_events.is_empty() {
            return Ok(());
        }

        let aggregate_id = aggregate_id.to_string();
        let pool = self.pool.clone();

        // Run blocking database operation in tokio blocking thread
        tokio::task::spawn_blocking(move || -> EventStoreResult<()> {
            use diesel::connection::AnsiTransactionManager;
            use diesel::connection::TransactionManager;

            let mut conn = pool.get().map_err(|e| {
                EventStoreError::database(format!("Failed to get connection: {}", e))
            })?;

            // Begin transaction
            AnsiTransactionManager::begin_transaction(&mut *conn)
                .map_err(|e| EventStoreError::database(e.to_string()))?;

            // Perform operations
            let result = (|| -> EventStoreResult<()> {
                // Check current version
                let current_version = events::table
                    .filter(events::aggregate_id.eq(&aggregate_id))
                    .select(diesel::dsl::max(events::sequence))
                    .first::<Option<i32>>(&mut *conn)
                    .map_err(|e| EventStoreError::database(e.to_string()))?
                    .unwrap_or(0) as i64;

                // Verify version check
                if let Some(expected) = version_check.version() {
                    if current_version != expected {
                        return Err(EventStoreError::ConcurrencyConflict {
                            aggregate_id: aggregate_id.clone(),
                            expected,
                            actual: current_version,
                        });
                    }
                }

                // Verify sequence numbers are sequential
                let mut expected_sequence = current_version + 1;
                for event in &new_events {
                    if event.sequence != expected_sequence {
                        return Err(EventStoreError::InvalidSequence {
                            aggregate_id: aggregate_id.clone(),
                            expected: expected_sequence,
                            actual: event.sequence,
                        });
                    }
                    expected_sequence += 1;
                }

                // Insert events
                for event in &new_events {
                    let record = NewEventRecord::from_event(event)?;
                    diesel::insert_into(events::table)
                        .values(&record)
                        .execute(&mut *conn)
                        .map_err(|e| EventStoreError::database(e.to_string()))?;
                }

                Ok(())
            })();

            // Commit or rollback based on result
            match result {
                Ok(_) => {
                    AnsiTransactionManager::commit_transaction(&mut *conn)
                        .map_err(|e| EventStoreError::database(e.to_string()))?;
                    Ok(())
                }
                Err(e) => {
                    let _ = AnsiTransactionManager::rollback_transaction(&mut *conn);
                    Err(e)
                }
            }
        })
        .await
        .map_err(|e| EventStoreError::other(format!("Task join error: {}", e)))?
    }

    async fn load(&self, aggregate_id: &str) -> EventStoreResult<Vec<Event>> {
        self.load_from(aggregate_id, 1).await
    }

    async fn load_from(
        &self,
        aggregate_id: &str,
        from_sequence: i64,
    ) -> EventStoreResult<Vec<Event>> {
        let aggregate_id = aggregate_id.to_string();
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| {
                EventStoreError::database(format!("Failed to get connection: {}", e))
            })?;

            let records: Vec<EventRecord> = events::table
                .filter(events::aggregate_id.eq(&aggregate_id))
                .filter(events::sequence.ge(from_sequence as i32))
                .order(events::sequence.asc())
                .load(&mut conn)
                .map_err(|e| EventStoreError::database(e.to_string()))?;

            records.iter().map(|r| r.to_event()).collect()
        })
        .await
        .map_err(|e| EventStoreError::other(format!("Task join error: {}", e)))?
    }

    async fn stream_all(&self, from_position: i64) -> EventStoreResult<Vec<Event>> {
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| {
                EventStoreError::database(format!("Failed to get connection: {}", e))
            })?;

            let records: Vec<EventRecord> = events::table
                .filter(events::id.ge(from_position as i32))
                .order(events::id.asc())
                .load(&mut conn)
                .map_err(|e| EventStoreError::database(e.to_string()))?;

            records.iter().map(|r| r.to_event()).collect()
        })
        .await
        .map_err(|e| EventStoreError::other(format!("Task join error: {}", e)))?
    }

    async fn get_version(&self, aggregate_id: &str) -> EventStoreResult<i64> {
        let aggregate_id = aggregate_id.to_string();
        let pool = self.pool.clone();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(|e| {
                EventStoreError::database(format!("Failed to get connection: {}", e))
            })?;

            let version = events::table
                .filter(events::aggregate_id.eq(&aggregate_id))
                .select(diesel::dsl::max(events::sequence))
                .first::<Option<i32>>(&mut conn)
                .map_err(|e| EventStoreError::database(e.to_string()))?
                .unwrap_or(0) as i64;

            Ok(version)
        })
        .await
        .map_err(|e| EventStoreError::other(format!("Task join error: {}", e)))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
    use serde_json::json;

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("../../migrations");

    async fn setup_test_store() -> SqliteEventStore {
        let manager = ConnectionManager::<SqliteConnection>::new(":memory:");
        let pool = Pool::builder()
            .max_size(1)
            .build(manager)
            .expect("Failed to create pool");

        // Run migrations
        let mut conn = pool.get().expect("Failed to get connection");
        conn.run_pending_migrations(MIGRATIONS)
            .expect("Failed to run migrations");
        drop(conn);

        SqliteEventStore::with_pool(pool)
    }

    #[tokio::test]
    async fn test_append_and_load_single_event() {
        let store = setup_test_store().await;

        let event = Event::new(
            "User",
            "user-123",
            1,
            "UserCreated",
            json!({ "name": "Alice" }),
        );

        // Append event
        store
            .append("user-123", VersionCheck::New, vec![event.clone()])
            .await
            .unwrap();

        // Load events
        let loaded = store.load("user-123").await.unwrap();

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].aggregate_id, "user-123");
        assert_eq!(loaded[0].event_type, "UserCreated");
        assert_eq!(loaded[0].sequence, 1);
    }

    #[tokio::test]
    async fn test_append_multiple_events() {
        let store = setup_test_store().await;

        let events = vec![
            Event::new("User", "user-456", 1, "UserCreated", json!({})),
            Event::new("User", "user-456", 2, "ProfileUpdated", json!({})),
            Event::new("User", "user-456", 3, "EmailChanged", json!({})),
        ];

        store
            .append("user-456", VersionCheck::New, events)
            .await
            .unwrap();

        let loaded = store.load("user-456").await.unwrap();

        assert_eq!(loaded.len(), 3);
        assert_eq!(loaded[0].sequence, 1);
        assert_eq!(loaded[1].sequence, 2);
        assert_eq!(loaded[2].sequence, 3);
    }

    #[tokio::test]
    async fn test_optimistic_concurrency_control() {
        let store = setup_test_store().await;

        // First event
        let event1 = Event::new("User", "user-789", 1, "UserCreated", json!({}));
        store
            .append("user-789", VersionCheck::New, vec![event1])
            .await
            .unwrap();

        // Second event with correct version
        let event2 = Event::new("User", "user-789", 2, "ProfileUpdated", json!({}));
        store
            .append("user-789", VersionCheck::Expected(1), vec![event2])
            .await
            .unwrap();

        // Third event with incorrect version - should fail
        let event3 = Event::new("User", "user-789", 3, "EmailChanged", json!({}));
        let result = store
            .append("user-789", VersionCheck::Expected(1), vec![event3])
            .await;

        assert!(result.is_err());
        if let Err(EventStoreError::ConcurrencyConflict {
            expected, actual, ..
        }) = result
        {
            assert_eq!(expected, 1);
            assert_eq!(actual, 2);
        } else {
            panic!("Expected ConcurrencyConflict error");
        }
    }

    #[tokio::test]
    async fn test_invalid_sequence() {
        let store = setup_test_store().await;

        // Event with wrong sequence number
        let event = Event::new("User", "user-999", 5, "UserCreated", json!({}));
        let result = store
            .append("user-999", VersionCheck::New, vec![event])
            .await;

        assert!(result.is_err());
        if let Err(EventStoreError::InvalidSequence {
            expected, actual, ..
        }) = result
        {
            assert_eq!(expected, 1);
            assert_eq!(actual, 5);
        } else {
            panic!("Expected InvalidSequence error");
        }
    }

    #[tokio::test]
    async fn test_load_from_sequence() {
        let store = setup_test_store().await;

        let events = vec![
            Event::new("Order", "order-1", 1, "OrderCreated", json!({})),
            Event::new("Order", "order-1", 2, "ItemAdded", json!({})),
            Event::new("Order", "order-1", 3, "ItemAdded", json!({})),
            Event::new("Order", "order-1", 4, "OrderShipped", json!({})),
        ];

        store
            .append("order-1", VersionCheck::New, events)
            .await
            .unwrap();

        // Load from sequence 3
        let loaded = store.load_from("order-1", 3).await.unwrap();

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].sequence, 3);
        assert_eq!(loaded[1].sequence, 4);
    }

    #[tokio::test]
    async fn test_get_version() {
        let store = setup_test_store().await;

        // New aggregate has version 0
        let version = store.get_version("user-new").await.unwrap();
        assert_eq!(version, 0);

        // After adding 3 events, version is 3
        let events = vec![
            Event::new("User", "user-version", 1, "UserCreated", json!({})),
            Event::new("User", "user-version", 2, "ProfileUpdated", json!({})),
            Event::new("User", "user-version", 3, "EmailChanged", json!({})),
        ];

        store
            .append("user-version", VersionCheck::New, events)
            .await
            .unwrap();

        let version = store.get_version("user-version").await.unwrap();
        assert_eq!(version, 3);
    }

    #[tokio::test]
    async fn test_stream_all() {
        let store = setup_test_store().await;

        // Add events for multiple aggregates
        let user_events = vec![
            Event::new("User", "user-1", 1, "UserCreated", json!({})),
            Event::new("User", "user-1", 2, "ProfileUpdated", json!({})),
        ];

        let order_events = vec![
            Event::new("Order", "order-1", 1, "OrderCreated", json!({})),
            Event::new("Order", "order-1", 2, "OrderShipped", json!({})),
        ];

        store
            .append("user-1", VersionCheck::New, user_events)
            .await
            .unwrap();
        store
            .append("order-1", VersionCheck::New, order_events)
            .await
            .unwrap();

        // Stream all events
        let all_events = store.stream_all(0).await.unwrap();

        assert_eq!(all_events.len(), 4);
    }

    #[tokio::test]
    async fn test_empty_aggregate() {
        let store = setup_test_store().await;

        // Load events for non-existent aggregate
        let events = store.load("non-existent").await.unwrap();
        assert_eq!(events.len(), 0);
    }

    #[tokio::test]
    async fn test_metadata_preservation() {
        let store = setup_test_store().await;

        let event = Event::with_metadata(
            "User",
            "user-meta",
            1,
            "UserCreated",
            json!({ "name": "Alice" }),
            json!({
                "correlation_id": "req-123",
                "user_id": "admin-1"
            }),
        );

        store
            .append("user-meta", VersionCheck::New, vec![event])
            .await
            .unwrap();

        let loaded = store.load("user-meta").await.unwrap();

        assert_eq!(loaded[0].metadata["correlation_id"], "req-123");
        assert_eq!(loaded[0].metadata["user_id"], "admin-1");
    }

    #[tokio::test]
    async fn test_concurrent_appends() {
        let store = setup_test_store().await;

        // First event
        let event1 = Event::new("User", "user-concurrent", 1, "UserCreated", json!({}));
        store
            .append("user-concurrent", VersionCheck::New, vec![event1])
            .await
            .unwrap();

        // Simulate concurrent appends with same expected version
        let store1 = store.clone();
        let store2 = store.clone();

        let handle1 = tokio::spawn(async move {
            let event = Event::new("User", "user-concurrent", 2, "Update1", json!({}));
            store1
                .append("user-concurrent", VersionCheck::Expected(1), vec![event])
                .await
        });

        let handle2 = tokio::spawn(async move {
            let event = Event::new("User", "user-concurrent", 2, "Update2", json!({}));
            store2
                .append("user-concurrent", VersionCheck::Expected(1), vec![event])
                .await
        });

        let result1 = handle1.await.unwrap();
        let result2 = handle2.await.unwrap();

        // One should succeed, one should fail
        assert!(result1.is_ok() != result2.is_ok());
    }

    #[tokio::test]
    async fn test_event_ordering_within_aggregate() {
        let store = setup_test_store().await;

        // Add events in batches
        let batch1 = vec![
            Event::new("User", "user-order", 1, "UserCreated", json!({})),
            Event::new("User", "user-order", 2, "EmailChanged", json!({})),
        ];

        let batch2 = vec![
            Event::new("User", "user-order", 3, "ProfileUpdated", json!({})),
            Event::new("User", "user-order", 4, "PasswordChanged", json!({})),
        ];

        store
            .append("user-order", VersionCheck::New, batch1)
            .await
            .unwrap();
        store
            .append("user-order", VersionCheck::Expected(2), batch2)
            .await
            .unwrap();

        let loaded = store.load("user-order").await.unwrap();

        // Verify order is maintained
        assert_eq!(loaded.len(), 4);
        for (i, event) in loaded.iter().enumerate() {
            assert_eq!(event.sequence, (i + 1) as i64);
        }
    }
}
