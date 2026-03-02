//! # Projection Module
//!
//! Projections build read models from event streams. They consume events and materialize
//! optimized views for queries.
//!
//! ## Design Principles
//!
//! - **Event-driven**: Projections react to events, not commands
//! - **Rebuildable**: Can be rebuilt from scratch by replaying events
//! - **Idempotent**: Handling the same event multiple times should be safe
//! - **Query-optimized**: Read models are optimized for specific queries
//!
//! ## Example
//!
//! ```rust,ignore
//! use nineties_core::projection::{Projection, ProjectionEngine};
//! use nineties_core::event::Event;
//!
//! struct UserListProjection {
//!     // Database connection, cache, etc.
//! }
//!
//! #[async_trait]
//! impl Projection for UserListProjection {
//!     fn name(&self) -> &str {
//!         "UserList"
//!     }
//!
//!     fn handles(&self) -> Vec<String> {
//!         vec!["UserCreated".to_string(), "ProfileUpdated".to_string()]
//!     }
//!
//!     async fn handle(&mut self, event: &Event) -> Result<(), Box<dyn std::error::Error>> {
//!         match event.event_type.as_str() {
//!             "UserCreated" => {
//!                 // Insert into users_view table
//!             }
//!             "ProfileUpdated" => {
//!                 // Update users_view table
//!             }
//!             _ => {}
//!         }
//!         Ok(())
//!     }
//!
//!     async fn clear(&mut self) -> Result<(), Box<dyn std::error::Error>> {
//!         // DELETE FROM users_view
//!         Ok(())
//!     }
//! }
//! ```

use crate::event::Event;
use crate::event_store::EventStore;
use async_trait::async_trait;
use thiserror::Error;

/// Errors that can occur during projection operations.
#[derive(Debug, Error)]
pub enum ProjectionError {
    /// Error handling an event
    #[error("Projection '{name}' failed to handle event {event_type} (event_id: {event_id}): {message}")]
    HandleFailed {
        name: String,
        event_type: String,
        event_id: String,
        message: String,
    },

    /// Error clearing projection state
    #[error("Projection '{name}' failed to clear: {message}")]
    ClearFailed { name: String, message: String },

    /// Error rebuilding projection
    #[error("Projection '{name}' failed to rebuild: {message}")]
    RebuildFailed { name: String, message: String },

    /// Event store error during rebuild
    #[error("Failed to load events for rebuild: {0}")]
    EventStoreError(String),

    /// Other errors
    #[error("Projection error: {message}")]
    Other { message: String },
}

impl ProjectionError {
    /// Create a handle failed error.
    pub fn handle_failed(
        name: impl Into<String>,
        event_type: impl Into<String>,
        event_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        ProjectionError::HandleFailed {
            name: name.into(),
            event_type: event_type.into(),
            event_id: event_id.into(),
            message: message.into(),
        }
    }

    /// Create a clear failed error.
    pub fn clear_failed(name: impl Into<String>, message: impl Into<String>) -> Self {
        ProjectionError::ClearFailed {
            name: name.into(),
            message: message.into(),
        }
    }

    /// Create a rebuild failed error.
    pub fn rebuild_failed(name: impl Into<String>, message: impl Into<String>) -> Self {
        ProjectionError::RebuildFailed {
            name: name.into(),
            message: message.into(),
        }
    }

    /// Create a generic error.
    pub fn other(message: impl Into<String>) -> Self {
        ProjectionError::Other {
            message: message.into(),
        }
    }
}

/// Result type for projection operations.
pub type ProjectionResult<T> = Result<T, ProjectionError>;

/// Trait for projections that build read models from events.
///
/// Projections consume events and materialize views optimized for queries.
/// They can be rebuilt at any time by replaying all events from the event store.
///
/// # Idempotency
///
/// Projections should be idempotent - handling the same event multiple times
/// should produce the same result. This is important for:
/// - Replay after bugs are fixed
/// - Handling duplicate events
/// - Eventual consistency in distributed systems
///
/// # Thread Safety
///
/// Implementations must be Send + Sync for async Rust.
#[async_trait]
pub trait Projection: Send + Sync {
    /// Projection name (unique identifier).
    ///
    /// Used for logging, monitoring, and identification.
    fn name(&self) -> &str;

    /// Event types this projection handles.
    ///
    /// Return a vector of event type names (e.g., ["UserCreated", "ProfileUpdated"]).
    /// Only events matching these types will be passed to `handle()`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// fn handles(&self) -> Vec<String> {
    ///     vec![
    ///         "UserCreated".to_string(),
    ///         "ProfileUpdated".to_string(),
    ///         "UserDeleted".to_string(),
    ///     ]
    /// }
    /// ```
    fn handles(&self) -> Vec<String>;

    /// Handle a single event.
    ///
    /// Update the read model based on the event. This method should be idempotent.
    ///
    /// # Arguments
    ///
    /// - `event`: The event to process
    ///
    /// # Returns
    ///
    /// - `Ok(())` if the event was handled successfully
    /// - `Err(ProjectionError)` if handling failed
    ///
    /// # Idempotency
    ///
    /// To make this idempotent, consider:
    /// - Using UPSERT or INSERT ... ON CONFLICT
    /// - Checking if the event was already processed
    /// - Making operations naturally idempotent (SET vs INCREMENT)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// async fn handle(&mut self, event: &Event) -> ProjectionResult<()> {
    ///     match event.event_type.as_str() {
    ///         "UserCreated" => {
    ///             let data: UserCreatedEvent = serde_json::from_value(event.payload.clone())?;
    ///             // INSERT INTO users_view ...
    ///             Ok(())
    ///         }
    ///         _ => Ok(())
    ///     }
    /// }
    /// ```
    async fn handle(&mut self, event: &Event) -> ProjectionResult<()>;

    /// Clear the projection state.
    ///
    /// Remove all data from the read model. Called before rebuild.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// async fn clear(&mut self) -> ProjectionResult<()> {
    ///     // DELETE FROM users_view
    ///     Ok(())
    /// }
    /// ```
    async fn clear(&mut self) -> ProjectionResult<()>;

    /// Rebuild the projection from scratch.
    ///
    /// Default implementation:
    /// 1. Clear existing state
    /// 2. Replay all events
    ///
    /// Override this if you need custom rebuild logic.
    ///
    /// # Arguments
    ///
    /// - `events`: All events to replay
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Default implementation (usually sufficient)
    /// async fn rebuild(&mut self, events: Vec<Event>) -> ProjectionResult<()> {
    ///     self.clear().await?;
    ///     for event in events {
    ///         if self.handles().contains(&event.event_type) {
    ///             self.handle(&event).await?;
    ///         }
    ///     }
    ///     Ok(())
    /// }
    /// ```
    async fn rebuild(&mut self, events: Vec<Event>) -> ProjectionResult<()> {
        self.clear().await?;
        for event in events {
            if self.handles().contains(&event.event_type) {
                self.handle(&event).await?;
            }
        }
        Ok(())
    }
}

/// Engine for managing multiple projections.
///
/// The ProjectionEngine:
/// - Registers multiple projections
/// - Routes events to interested projections
/// - Rebuilds projections from event store
/// - Provides bulk operations
///
/// # Example
///
/// ```rust,ignore
/// use nineties_core::projection::ProjectionEngine;
///
/// let event_store = Box::new(sqlite_event_store);
/// let mut engine = ProjectionEngine::new(event_store);
///
/// // Register projections
/// engine.register(Box::new(UserListProjection::new()));
/// engine.register(Box::new(AuditLogProjection::new()));
///
/// // Process events
/// for event in events {
///     engine.process(&event).await?;
/// }
///
/// // Rebuild all projections
/// engine.rebuild_all().await?;
/// ```
pub struct ProjectionEngine {
    projections: Vec<Box<dyn Projection>>,
    event_store: Box<dyn EventStore>,
}

impl ProjectionEngine {
    /// Create a new projection engine.
    ///
    /// # Arguments
    ///
    /// - `event_store`: Event store for loading events during rebuild
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let event_store = Box::new(InMemoryEventStore::new());
    /// let engine = ProjectionEngine::new(event_store);
    /// ```
    pub fn new(event_store: Box<dyn EventStore>) -> Self {
        Self {
            projections: Vec::new(),
            event_store,
        }
    }

    /// Register a projection.
    ///
    /// The projection will receive all future events that match its `handles()` filter.
    ///
    /// # Arguments
    ///
    /// - `projection`: The projection to register
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// engine.register(Box::new(UserListProjection::new()));
    /// ```
    pub fn register(&mut self, projection: Box<dyn Projection>) {
        tracing::info!("Registering projection: {}", projection.name());
        self.projections.push(projection);
    }

    /// Process a single event through all interested projections.
    ///
    /// Routes the event to projections whose `handles()` includes the event type.
    ///
    /// # Arguments
    ///
    /// - `event`: The event to process
    ///
    /// # Returns
    ///
    /// - `Ok(())` if all projections handled the event successfully
    /// - `Err(ProjectionError)` if any projection failed
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// engine.process(&event).await?;
    /// ```
    pub async fn process(&mut self, event: &Event) -> ProjectionResult<()> {
        for projection in &mut self.projections {
            if projection.handles().contains(&event.event_type) {
                tracing::debug!(
                    "Processing event {} ({}) in projection {}",
                    event.event_type,
                    event.event_id,
                    projection.name()
                );

                projection.handle(event).await.map_err(|e| {
                    ProjectionError::handle_failed(
                        projection.name(),
                        &event.event_type,
                        event.event_id.to_string(),
                        e.to_string(),
                    )
                })?;
            }
        }
        Ok(())
    }

    /// Process multiple events in sequence.
    ///
    /// # Arguments
    ///
    /// - `events`: Vector of events to process
    ///
    /// # Returns
    ///
    /// - `Ok(())` if all events were processed successfully
    /// - `Err(ProjectionError)` on first failure
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// engine.process_batch(events).await?;
    /// ```
    pub async fn process_batch(&mut self, events: Vec<Event>) -> ProjectionResult<()> {
        for event in events {
            self.process(&event).await?;
        }
        Ok(())
    }

    /// Rebuild all registered projections from the event store.
    ///
    /// This will:
    /// 1. Load all events from the event store (from position 0)
    /// 2. Rebuild each projection by calling `projection.rebuild(events)`
    ///
    /// # Returns
    ///
    /// - `Ok(())` if all projections rebuilt successfully
    /// - `Err(ProjectionError)` if any projection failed
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // Rebuild after fixing a bug
    /// engine.rebuild_all().await?;
    /// ```
    pub async fn rebuild_all(&mut self) -> ProjectionResult<()> {
        tracing::info!("Rebuilding all projections");

        // Load all events
        let events = self
            .event_store
            .stream_all(0)
            .await
            .map_err(|e| ProjectionError::EventStoreError(e.to_string()))?;

        tracing::info!("Loaded {} events for rebuild", events.len());

        // Rebuild each projection
        for projection in &mut self.projections {
            tracing::info!("Rebuilding projection: {}", projection.name());

            projection.rebuild(events.clone()).await.map_err(|e| {
                ProjectionError::rebuild_failed(projection.name(), e.to_string())
            })?;

            tracing::info!("Rebuilt projection: {}", projection.name());
        }

        Ok(())
    }

    /// Rebuild a specific projection by name.
    ///
    /// # Arguments
    ///
    /// - `name`: Name of the projection to rebuild
    ///
    /// # Returns
    ///
    /// - `Ok(())` if rebuild succeeded
    /// - `Err(ProjectionError)` if projection not found or rebuild failed
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// engine.rebuild_projection("UserList").await?;
    /// ```
    pub async fn rebuild_projection(&mut self, name: &str) -> ProjectionResult<()> {
        tracing::info!("Rebuilding projection: {}", name);

        // Find projection
        let projection = self
            .projections
            .iter_mut()
            .find(|p| p.name() == name)
            .ok_or_else(|| ProjectionError::other(format!("Projection not found: {}", name)))?;

        // Load all events
        let events = self
            .event_store
            .stream_all(0)
            .await
            .map_err(|e| ProjectionError::EventStoreError(e.to_string()))?;

        // Rebuild
        projection
            .rebuild(events)
            .await
            .map_err(|e| ProjectionError::rebuild_failed(name, e.to_string()))?;

        tracing::info!("Rebuilt projection: {}", name);
        Ok(())
    }

    /// Get number of registered projections.
    pub fn projection_count(&self) -> usize {
        self.projections.len()
    }

    /// Get names of all registered projections.
    pub fn projection_names(&self) -> Vec<String> {
        self.projections
            .iter()
            .map(|p| p.name().to_string())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_store::{EventStore, EventStoreResult, VersionCheck};
    use std::sync::{Arc, Mutex};

    // Mock in-memory event store for testing
    struct MockEventStore {
        events: Arc<Mutex<Vec<Event>>>,
    }

    impl MockEventStore {
        fn new() -> Self {
            Self {
                events: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn add_event(&self, event: Event) {
            self.events.lock().unwrap().push(event);
        }
    }

    #[async_trait]
    impl EventStore for MockEventStore {
        async fn append(
            &self,
            _aggregate_id: &str,
            _version_check: VersionCheck,
            events: Vec<Event>,
        ) -> EventStoreResult<()> {
            self.events.lock().unwrap().extend(events);
            Ok(())
        }

        async fn load(&self, aggregate_id: &str) -> EventStoreResult<Vec<Event>> {
            Ok(self
                .events
                .lock()
                .unwrap()
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
            Ok(self
                .events
                .lock()
                .unwrap()
                .iter()
                .filter(|e| e.aggregate_id == aggregate_id && e.sequence >= from_sequence)
                .cloned()
                .collect())
        }

        async fn stream_all(&self, _from_position: i64) -> EventStoreResult<Vec<Event>> {
            Ok(self.events.lock().unwrap().clone())
        }

        async fn get_version(&self, aggregate_id: &str) -> EventStoreResult<i64> {
            Ok(self
                .events
                .lock()
                .unwrap()
                .iter()
                .filter(|e| e.aggregate_id == aggregate_id)
                .map(|e| e.sequence)
                .max()
                .unwrap_or(0))
        }
    }

    // Mock projection for testing
    struct MockProjection {
        name: String,
        events_handled: Arc<Mutex<Vec<Event>>>,
        cleared: Arc<Mutex<bool>>,
        handles_types: Vec<String>,
    }

    impl MockProjection {
        fn new(name: &str, handles: Vec<String>) -> Self {
            Self {
                name: name.to_string(),
                events_handled: Arc::new(Mutex::new(Vec::new())),
                cleared: Arc::new(Mutex::new(false)),
                handles_types: handles,
            }
        }

        #[allow(dead_code)]
        fn handled_count(&self) -> usize {
            self.events_handled.lock().unwrap().len()
        }

        #[allow(dead_code)]
        fn was_cleared(&self) -> bool {
            *self.cleared.lock().unwrap()
        }
    }

    #[async_trait]
    impl Projection for MockProjection {
        fn name(&self) -> &str {
            &self.name
        }

        fn handles(&self) -> Vec<String> {
            self.handles_types.clone()
        }

        async fn handle(&mut self, event: &Event) -> ProjectionResult<()> {
            self.events_handled.lock().unwrap().push(event.clone());
            Ok(())
        }

        async fn clear(&mut self) -> ProjectionResult<()> {
            self.events_handled.lock().unwrap().clear();
            *self.cleared.lock().unwrap() = true;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_projection_engine_new() {
        let store = Box::new(MockEventStore::new());
        let engine = ProjectionEngine::new(store);
        assert_eq!(engine.projection_count(), 0);
    }

    #[tokio::test]
    async fn test_register_projection() {
        let store = Box::new(MockEventStore::new());
        let mut engine = ProjectionEngine::new(store);

        let projection = Box::new(MockProjection::new("Test", vec!["TestEvent".to_string()]));
        engine.register(projection);

        assert_eq!(engine.projection_count(), 1);
        assert_eq!(engine.projection_names(), vec!["Test"]);
    }

    #[tokio::test]
    async fn test_process_event() {
        let store = Box::new(MockEventStore::new());
        let mut engine = ProjectionEngine::new(store);

        let projection = Box::new(MockProjection::new(
            "Test",
            vec!["UserCreated".to_string()],
        ));
        let proj_clone = projection.events_handled.clone();
        engine.register(projection);

        let event = Event::new(
            "User",
            "user-1",
            1,
            "UserCreated",
            serde_json::json!({"name": "Alice"}),
        );

        engine.process(&event).await.unwrap();

        assert_eq!(proj_clone.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_projection_filtering() {
        let store = Box::new(MockEventStore::new());
        let mut engine = ProjectionEngine::new(store);

        let projection = Box::new(MockProjection::new(
            "Test",
            vec!["UserCreated".to_string()],
        ));
        let proj_clone = projection.events_handled.clone();
        engine.register(projection);

        // Event that should be handled
        let event1 = Event::new("User", "user-1", 1, "UserCreated", serde_json::json!({}));
        engine.process(&event1).await.unwrap();

        // Event that should be filtered out
        let event2 = Event::new("User", "user-1", 2, "UserDeleted", serde_json::json!({}));
        engine.process(&event2).await.unwrap();

        assert_eq!(proj_clone.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_rebuild_all() {
        let store = MockEventStore::new();

        // Add events to store
        store.add_event(Event::new(
            "User",
            "user-1",
            1,
            "UserCreated",
            serde_json::json!({}),
        ));
        store.add_event(Event::new(
            "User",
            "user-2",
            1,
            "UserCreated",
            serde_json::json!({}),
        ));

        let mut engine = ProjectionEngine::new(Box::new(store));

        let projection = Box::new(MockProjection::new(
            "Test",
            vec!["UserCreated".to_string()],
        ));
        let proj_events = projection.events_handled.clone();
        let proj_cleared = projection.cleared.clone();
        engine.register(projection);

        engine.rebuild_all().await.unwrap();

        assert!(*proj_cleared.lock().unwrap());
        assert_eq!(proj_events.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_multiple_projections() {
        let store = Box::new(MockEventStore::new());
        let mut engine = ProjectionEngine::new(store);

        let proj1 = Box::new(MockProjection::new(
            "Projection1",
            vec!["UserCreated".to_string()],
        ));
        let proj2 = Box::new(MockProjection::new(
            "Projection2",
            vec!["UserCreated".to_string(), "UserDeleted".to_string()],
        ));

        let proj1_events = proj1.events_handled.clone();
        let proj2_events = proj2.events_handled.clone();

        engine.register(proj1);
        engine.register(proj2);

        let event = Event::new("User", "user-1", 1, "UserCreated", serde_json::json!({}));
        engine.process(&event).await.unwrap();

        assert_eq!(proj1_events.lock().unwrap().len(), 1);
        assert_eq!(proj2_events.lock().unwrap().len(), 1);
    }

    #[tokio::test]
    async fn test_process_batch() {
        let store = Box::new(MockEventStore::new());
        let mut engine = ProjectionEngine::new(store);

        let projection = Box::new(MockProjection::new(
            "Test",
            vec!["UserCreated".to_string()],
        ));
        let proj_events = projection.events_handled.clone();
        engine.register(projection);

        let events = vec![
            Event::new("User", "user-1", 1, "UserCreated", serde_json::json!({})),
            Event::new("User", "user-2", 1, "UserCreated", serde_json::json!({})),
            Event::new("User", "user-3", 1, "UserCreated", serde_json::json!({})),
        ];

        engine.process_batch(events).await.unwrap();

        assert_eq!(proj_events.lock().unwrap().len(), 3);
    }
}
