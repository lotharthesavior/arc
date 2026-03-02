//! # Command Bus Module
//!
//! Coordinates command handling through aggregates with event persistence and publishing.
//!
//! ## Overview
//!
//! The CommandBus is the central coordinator in the CQRS pattern. It orchestrates
//! the complete flow from command receipt to event publication:
//!
//! 1. Load events from EventStore for the target aggregate
//! 2. Reconstruct aggregate state using `Aggregate::from_events()`
//! 3. Handle command through `Aggregate::handle()` to produce new events
//! 4. Append events to EventStore with optimistic concurrency check
//! 5. Publish events to EventBus for projections and side effects
//!
//! ## Example
//!
//! ```rust,ignore
//! use nineties_core::command_bus::CommandBus;
//! use nineties_core::event_store::EventStore;
//! use nineties_core::event_bus::EventBus;
//!
//! # async fn example(
//! #     event_store: Box<dyn EventStore>,
//! #     event_bus: Box<dyn EventBus>,
//! #     command: UserCommand,
//! # ) -> Result<(), Box<dyn std::error::Error>> {
//! // Create command bus for UserAggregate
//! let mut command_bus = CommandBus::<UserAggregate>::new(event_store, event_bus);
//!
//! // Dispatch command
//! let events = command_bus.dispatch(command).await?;
//!
//! println!("Produced {} events", events.len());
//! # Ok(())
//! # }
//! ```
//!
//! ## Optimistic Concurrency
//!
//! The CommandBus uses optimistic concurrency control to prevent conflicting writes:
//!
//! - Loads current aggregate version before handling command
//! - Passes expected version to EventStore::append()
//! - If another process modified the aggregate, append fails with ConcurrencyConflict
//! - Caller should retry the command (which will reload the latest state)
//!
//! ## Error Handling
//!
//! The CommandBus can fail at multiple stages:
//!
//! - **Load**: EventStore fails to load events
//! - **Handle**: Aggregate rejects command (validation failure, business rule violation)
//! - **Append**: Concurrency conflict or storage failure
//! - **Publish**: EventBus fails to deliver to handlers
//!
//! All errors are propagated to the caller.

use crate::aggregate::{Aggregate, Command};
use crate::event::Event;
use crate::event_bus::{EventBus, EventBusError};
use crate::event_store::{EventStore, EventStoreError, VersionCheck};
use std::marker::PhantomData;
use thiserror::Error;

/// Errors that can occur during command bus operations.
#[derive(Debug, Error)]
pub enum CommandBusError {
    /// Failed to load aggregate events from event store
    #[error("Failed to load aggregate '{aggregate_id}': {source}")]
    LoadFailed {
        aggregate_id: String,
        #[source]
        source: EventStoreError,
    },

    /// Command handling failed (business rule violation, validation error, etc.)
    #[error("Command handling failed for aggregate '{aggregate_id}': {message}")]
    HandleFailed {
        aggregate_id: String,
        message: String,
    },

    /// Failed to append events to event store
    #[error("Failed to append events for aggregate '{aggregate_id}': {source}")]
    AppendFailed {
        aggregate_id: String,
        #[source]
        source: EventStoreError,
    },

    /// Failed to publish events to event bus
    #[error("Failed to publish events for aggregate '{aggregate_id}': {source}")]
    PublishFailed {
        aggregate_id: String,
        #[source]
        source: EventBusError,
    },

    /// Other command bus errors
    #[error("Command bus error: {message}")]
    Other { message: String },
}

impl CommandBusError {
    /// Create a handle failed error.
    pub fn handle_failed(aggregate_id: impl Into<String>, message: impl Into<String>) -> Self {
        CommandBusError::HandleFailed {
            aggregate_id: aggregate_id.into(),
            message: message.into(),
        }
    }

    /// Create a generic error.
    pub fn other(message: impl Into<String>) -> Self {
        CommandBusError::Other {
            message: message.into(),
        }
    }
}

/// Result type for command bus operations.
pub type CommandBusResult<T> = Result<T, CommandBusError>;

/// Command bus for dispatching commands to aggregates.
///
/// The CommandBus coordinates the complete command handling flow:
/// - Loading aggregate state from event store
/// - Handling commands through the aggregate
/// - Persisting produced events with optimistic concurrency
/// - Publishing events to the event bus
///
/// # Type Parameters
///
/// - `A`: The aggregate type this command bus handles
///
/// # Example
///
/// ```rust,ignore
/// use nineties_core::command_bus::CommandBus;
/// use nineties_core::event_store::InMemoryEventStore;
/// use nineties_core::event_bus::InProcessEventBus;
///
/// # async fn example(command: UserCommand) -> Result<(), Box<dyn std::error::Error>> {
/// let event_store = Box::new(InMemoryEventStore::new());
/// let event_bus = Box::new(InProcessEventBus::new());
///
/// let mut command_bus = CommandBus::<UserAggregate>::new(event_store, event_bus);
///
/// // Dispatch command
/// let events = command_bus.dispatch(command).await?;
/// # Ok(())
/// # }
/// ```
pub struct CommandBus<A: Aggregate> {
    event_store: Box<dyn EventStore>,
    event_bus: Box<dyn EventBus>,
    _phantom: PhantomData<A>,
}

impl<A: Aggregate> CommandBus<A> {
    /// Create a new command bus.
    ///
    /// # Arguments
    ///
    /// - `event_store`: Event store for loading and persisting events
    /// - `event_bus`: Event bus for publishing events to subscribers
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use nineties_core::command_bus::CommandBus;
    ///
    /// # async fn example(
    /// #     event_store: Box<dyn EventStore>,
    /// #     event_bus: Box<dyn EventBus>,
    /// # ) -> Result<(), Box<dyn std::error::Error>> {
    /// let mut command_bus = CommandBus::<UserAggregate>::new(event_store, event_bus);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(event_store: Box<dyn EventStore>, event_bus: Box<dyn EventBus>) -> Self {
        Self {
            event_store,
            event_bus,
            _phantom: PhantomData,
        }
    }

    /// Dispatch a command to its aggregate.
    ///
    /// This method performs the complete command handling flow:
    ///
    /// 1. **Load**: Retrieves all events for the aggregate from the event store
    /// 2. **Reconstruct**: Rebuilds aggregate state by applying loaded events
    /// 3. **Handle**: Processes the command through the aggregate to produce new events
    /// 4. **Append**: Persists new events with optimistic concurrency check
    /// 5. **Publish**: Broadcasts events to event bus subscribers
    ///
    /// # Arguments
    ///
    /// - `command`: The command to dispatch
    ///
    /// # Returns
    ///
    /// - `Ok(Vec<Event>)`: Events produced by the command
    /// - `Err(CommandBusError::LoadFailed)`: Failed to load aggregate events
    /// - `Err(CommandBusError::HandleFailed)`: Command was rejected by aggregate
    /// - `Err(CommandBusError::AppendFailed)`: Failed to persist events (possibly concurrency conflict)
    /// - `Err(CommandBusError::PublishFailed)`: Failed to publish events
    ///
    /// # Optimistic Concurrency
    ///
    /// If a `ConcurrencyConflict` error occurs, it means another process modified
    /// the aggregate after this command loaded its state. The caller should retry
    /// the command, which will reload the latest aggregate state.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use nineties_core::command_bus::CommandBus;
    ///
    /// # async fn example(
    /// #     mut command_bus: CommandBus<UserAggregate>,
    /// #     command: UserCommand,
    /// # ) -> Result<(), Box<dyn std::error::Error>> {
    /// // Dispatch with automatic retry on concurrency conflict
    /// let mut retries = 0;
    /// loop {
    ///     match command_bus.dispatch(command.clone()).await {
    ///         Ok(events) => {
    ///             println!("Command succeeded, produced {} events", events.len());
    ///             break;
    ///         }
    ///         Err(CommandBusError::AppendFailed { source, .. })
    ///             if matches!(source, EventStoreError::ConcurrencyConflict { .. })
    ///                 && retries < 3 =>
    ///         {
    ///             retries += 1;
    ///             println!("Concurrency conflict, retrying ({}/3)", retries);
    ///         }
    ///         Err(e) => return Err(e.into()),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn dispatch(&mut self, command: A::Command) -> CommandBusResult<Vec<Event>> {
        let aggregate_id = command.aggregate_id().to_string();

        // Step 1: Load existing events for the aggregate
        let events = self
            .event_store
            .load(&aggregate_id)
            .await
            .map_err(|source| CommandBusError::LoadFailed {
                aggregate_id: aggregate_id.clone(),
                source,
            })?;

        // Get current version for optimistic concurrency
        let current_version = events.last().map(|e| e.sequence).unwrap_or(0);

        // Step 2: Reconstruct aggregate from events
        let aggregate = A::from_events(events);

        // Step 3: Handle command through aggregate
        let new_events = aggregate
            .handle(command)
            .await
            .map_err(|e| CommandBusError::handle_failed(&aggregate_id, e.to_string()))?;

        // If no events produced, we're done (idempotent command or no-op)
        if new_events.is_empty() {
            return Ok(vec![]);
        }

        // Step 4: Append events to event store with optimistic concurrency check
        let version_check = if current_version == 0 {
            VersionCheck::New
        } else {
            VersionCheck::Expected(current_version)
        };

        self.event_store
            .append(&aggregate_id, version_check, new_events.clone())
            .await
            .map_err(|source| CommandBusError::AppendFailed {
                aggregate_id: aggregate_id.clone(),
                source,
            })?;

        // Step 5: Publish events to event bus
        self.event_bus
            .publish(new_events.clone())
            .await
            .map_err(|source| CommandBusError::PublishFailed {
                aggregate_id: aggregate_id.clone(),
                source,
            })?;

        Ok(new_events)
    }

    /// Get a reference to the event store.
    ///
    /// Useful for testing and diagnostics.
    pub fn event_store(&self) -> &dyn EventStore {
        self.event_store.as_ref()
    }

    /// Get a reference to the event bus.
    ///
    /// Useful for testing and diagnostics.
    pub fn event_bus(&self) -> &dyn EventBus {
        self.event_bus.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aggregate::Aggregate;
    use crate::event::Event;
    use crate::event_bus::{EventHandler, InProcessEventBus};
    use crate::event_store::{EventStore, EventStoreError, EventStoreResult, VersionCheck};
    use async_trait::async_trait;
    use serde_json::json;
    use std::sync::Arc;
    use tokio::sync::Mutex as TokioMutex;

    // Test aggregate: Counter
    #[derive(Debug, Clone, PartialEq)]
    struct CounterCommand {
        id: String,
        increment: i64,
    }

    impl Command for CounterCommand {
        fn aggregate_id(&self) -> &str {
            &self.id
        }
    }

    #[derive(Debug, Clone, Default)]
    struct CounterAggregate {
        id: Option<String>,
        value: i64,
        version: i64,
    }

    #[derive(Debug, thiserror::Error)]
    enum CounterError {
        #[error("Negative increment not allowed")]
        NegativeIncrement,
    }

    #[async_trait]
    impl Aggregate for CounterAggregate {
        type Command = CounterCommand;
        type Event = ();
        type Error = CounterError;

        fn aggregate_type() -> &'static str {
            "Counter"
        }

        fn version(&self) -> i64 {
            self.version
        }

        async fn handle(&self, command: Self::Command) -> Result<Vec<Event>, Self::Error> {
            // Validate
            if command.increment < 0 {
                return Err(CounterError::NegativeIncrement);
            }

            // Produce event
            let event = Event::new(
                "Counter",
                &command.id,
                self.version + 1,
                "CounterIncremented",
                json!({ "increment": command.increment }),
            );

            Ok(vec![event])
        }

        fn apply(&mut self, event: &Event) {
            if event.event_type == "CounterIncremented" {
                self.id = Some(event.aggregate_id.clone());
                self.value += event.payload["increment"].as_i64().unwrap_or(0);
                self.version = event.sequence;
            }
        }
    }

    // In-memory event store for testing
    struct InMemoryEventStore {
        events: Arc<TokioMutex<Vec<Event>>>,
    }

    impl InMemoryEventStore {
        fn new() -> Self {
            Self {
                events: Arc::new(TokioMutex::new(Vec::new())),
            }
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
            let mut store = self.events.lock().await;

            // Get current version
            let current_version = store
                .iter()
                .filter(|e| e.aggregate_id == aggregate_id)
                .map(|e| e.sequence)
                .max()
                .unwrap_or(0);

            // Check version
            if let Some(expected) = version_check.version() {
                if current_version != expected {
                    return Err(EventStoreError::ConcurrencyConflict {
                        aggregate_id: aggregate_id.to_string(),
                        expected,
                        actual: current_version,
                    });
                }
            }

            // Append events
            store.extend(events);
            Ok(())
        }

        async fn load(&self, aggregate_id: &str) -> EventStoreResult<Vec<Event>> {
            let store = self.events.lock().await;
            let events: Vec<Event> = store
                .iter()
                .filter(|e| e.aggregate_id == aggregate_id)
                .cloned()
                .collect();
            Ok(events)
        }

        async fn load_from(
            &self,
            aggregate_id: &str,
            from_sequence: i64,
        ) -> EventStoreResult<Vec<Event>> {
            let store = self.events.lock().await;
            let events: Vec<Event> = store
                .iter()
                .filter(|e| e.aggregate_id == aggregate_id && e.sequence >= from_sequence)
                .cloned()
                .collect();
            Ok(events)
        }

        async fn stream_all(&self, from_position: i64) -> EventStoreResult<Vec<Event>> {
            let store = self.events.lock().await;
            let events: Vec<Event> = store.iter().skip(from_position as usize).cloned().collect();
            Ok(events)
        }

        async fn get_version(&self, aggregate_id: &str) -> EventStoreResult<i64> {
            let store = self.events.lock().await;
            let version = store
                .iter()
                .filter(|e| e.aggregate_id == aggregate_id)
                .map(|e| e.sequence)
                .max()
                .unwrap_or(0);
            Ok(version)
        }
    }

    #[tokio::test]
    async fn test_command_bus_new() {
        let event_store = Box::new(InMemoryEventStore::new());
        let event_bus = Box::new(InProcessEventBus::new());

        let _command_bus = CommandBus::<CounterAggregate>::new(event_store, event_bus);
    }

    #[tokio::test]
    async fn test_dispatch_first_command() {
        let event_store = Box::new(InMemoryEventStore::new());
        let event_bus = Box::new(InProcessEventBus::new());

        let mut command_bus = CommandBus::<CounterAggregate>::new(event_store, event_bus);

        let command = CounterCommand {
            id: "counter-1".to_string(),
            increment: 5,
        };

        let events = command_bus.dispatch(command).await.unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "CounterIncremented");
        assert_eq!(events[0].aggregate_id, "counter-1");
        assert_eq!(events[0].sequence, 1);
        assert_eq!(events[0].payload["increment"], 5);
    }

    #[tokio::test]
    async fn test_dispatch_multiple_commands() {
        let event_store = Box::new(InMemoryEventStore::new());
        let event_bus = Box::new(InProcessEventBus::new());

        let mut command_bus = CommandBus::<CounterAggregate>::new(event_store, event_bus);

        // First command
        let command1 = CounterCommand {
            id: "counter-1".to_string(),
            increment: 5,
        };
        command_bus.dispatch(command1).await.unwrap();

        // Second command
        let command2 = CounterCommand {
            id: "counter-1".to_string(),
            increment: 3,
        };
        let events = command_bus.dispatch(command2).await.unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].sequence, 2); // Second event
    }

    #[tokio::test]
    async fn test_dispatch_validates_command() {
        let event_store = Box::new(InMemoryEventStore::new());
        let event_bus = Box::new(InProcessEventBus::new());

        let mut command_bus = CommandBus::<CounterAggregate>::new(event_store, event_bus);

        let command = CounterCommand {
            id: "counter-1".to_string(),
            increment: -5, // Invalid
        };

        let result = command_bus.dispatch(command).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            CommandBusError::HandleFailed { message, .. } => {
                assert!(message.contains("Negative increment"));
            }
            _ => panic!("Expected HandleFailed error"),
        }
    }

    #[tokio::test]
    async fn test_dispatch_publishes_events() {
        let event_store = Box::new(InMemoryEventStore::new());
        let mut event_bus = InProcessEventBus::new();

        // Track published events
        let published = Arc::new(TokioMutex::new(Vec::new()));
        let published_clone = published.clone();

        struct TestHandler {
            published: Arc<TokioMutex<Vec<String>>>,
        }

        #[async_trait]
        impl EventHandler for TestHandler {
            fn handles(&self) -> Vec<String> {
                vec!["CounterIncremented".to_string()]
            }

            async fn handle(
                &self,
                event: &Event,
            ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                self.published.lock().await.push(event.event_type.clone());
                Ok(())
            }
        }

        event_bus
            .subscribe(Box::new(TestHandler {
                published: published_clone,
            }))
            .await
            .unwrap();

        let mut command_bus = CommandBus::<CounterAggregate>::new(event_store, Box::new(event_bus));

        let command = CounterCommand {
            id: "counter-1".to_string(),
            increment: 5,
        };

        command_bus.dispatch(command).await.unwrap();

        // Verify event was published
        let published_events = published.lock().await;
        assert_eq!(published_events.len(), 1);
        assert_eq!(published_events[0], "CounterIncremented");
    }

    #[tokio::test]
    async fn test_dispatch_empty_events() {
        // Aggregate that produces no events
        struct NoOpAggregate {
            version: i64,
        }

        impl Default for NoOpAggregate {
            fn default() -> Self {
                Self { version: 0 }
            }
        }

        struct NoOpCommand {
            id: String,
        }

        impl Command for NoOpCommand {
            fn aggregate_id(&self) -> &str {
                &self.id
            }
        }

        #[derive(Debug, thiserror::Error)]
        #[error("No-op error")]
        struct NoOpError;

        #[async_trait]
        impl Aggregate for NoOpAggregate {
            type Command = NoOpCommand;
            type Event = ();
            type Error = NoOpError;

            fn aggregate_type() -> &'static str {
                "NoOp"
            }

            fn version(&self) -> i64 {
                self.version
            }

            async fn handle(&self, _command: Self::Command) -> Result<Vec<Event>, Self::Error> {
                Ok(vec![]) // No events
            }

            fn apply(&mut self, _event: &Event) {}
        }

        let event_store = Box::new(InMemoryEventStore::new());
        let event_bus = Box::new(InProcessEventBus::new());

        let mut command_bus = CommandBus::<NoOpAggregate>::new(event_store, event_bus);

        let command = NoOpCommand {
            id: "noop-1".to_string(),
        };

        let events = command_bus.dispatch(command).await.unwrap();

        assert_eq!(events.len(), 0);
    }

    #[tokio::test]
    async fn test_aggregate_state_reconstruction() {
        let event_store = Box::new(InMemoryEventStore::new());
        let event_bus = Box::new(InProcessEventBus::new());

        let mut command_bus = CommandBus::<CounterAggregate>::new(event_store, event_bus);

        // Increment by 5
        let command1 = CounterCommand {
            id: "counter-1".to_string(),
            increment: 5,
        };
        command_bus.dispatch(command1).await.unwrap();

        // Increment by 3
        let command2 = CounterCommand {
            id: "counter-1".to_string(),
            increment: 3,
        };
        command_bus.dispatch(command2).await.unwrap();

        // Load and verify state
        let events = command_bus.event_store().load("counter-1").await.unwrap();
        let aggregate = CounterAggregate::from_events(events);

        assert_eq!(aggregate.value, 8); // 5 + 3
        assert_eq!(aggregate.version, 2);
    }

    #[tokio::test]
    async fn test_optimistic_concurrency() {
        // Create a failing event store that simulates concurrency conflict
        struct ConflictingEventStore;

        #[async_trait]
        impl EventStore for ConflictingEventStore {
            async fn append(
                &self,
                aggregate_id: &str,
                version_check: VersionCheck,
                _events: Vec<Event>,
            ) -> EventStoreResult<()> {
                if let Some(expected) = version_check.version() {
                    Err(EventStoreError::ConcurrencyConflict {
                        aggregate_id: aggregate_id.to_string(),
                        expected,
                        actual: expected + 1,
                    })
                } else {
                    Ok(())
                }
            }

            async fn load(&self, _aggregate_id: &str) -> EventStoreResult<Vec<Event>> {
                Ok(vec![])
            }

            async fn load_from(
                &self,
                _aggregate_id: &str,
                _from_sequence: i64,
            ) -> EventStoreResult<Vec<Event>> {
                Ok(vec![])
            }

            async fn stream_all(&self, _from_position: i64) -> EventStoreResult<Vec<Event>> {
                Ok(vec![])
            }

            async fn get_version(&self, _aggregate_id: &str) -> EventStoreResult<i64> {
                Ok(0)
            }
        }

        let event_store = Box::new(ConflictingEventStore);
        let event_bus = Box::new(InProcessEventBus::new());

        let mut command_bus = CommandBus::<CounterAggregate>::new(event_store, event_bus);

        let command = CounterCommand {
            id: "counter-1".to_string(),
            increment: 5,
        };

        let result = command_bus.dispatch(command).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            CommandBusError::AppendFailed { source, .. } => {
                assert!(matches!(
                    source,
                    EventStoreError::ConcurrencyConflict { .. }
                ));
            }
            _ => panic!("Expected AppendFailed with ConcurrencyConflict"),
        }
    }

    #[test]
    fn test_error_messages() {
        let error = CommandBusError::handle_failed("user-123", "Invalid email");
        let msg = error.to_string();
        assert!(msg.contains("user-123"));
        assert!(msg.contains("Invalid email"));

        let error = CommandBusError::other("Something went wrong");
        assert!(error.to_string().contains("Something went wrong"));
    }
}
