//! # Event Bus Module
//!
//! Defines the EventBus and EventHandler traits for pub/sub event handling.
//!
//! ## Design Principles
//!
//! - **Decoupled**: Publishers don't know about subscribers
//! - **Synchronous**: InProcessEventBus handles events synchronously in order
//! - **Type-safe**: Event handlers declare which event types they handle
//! - **Extensible**: Multiple handlers can subscribe to the same events
//!
//! ## Example
//!
//! ```rust
//! use arc_core::event_bus::{EventBus, EventHandler, InProcessEventBus};
//! use arc_core::event::Event;
//! use serde_json::json;
//! use async_trait::async_trait;
//!
//! // Define a custom event handler
//! struct WelcomeEmailHandler;
//!
//! #[async_trait]
//! impl EventHandler for WelcomeEmailHandler {
//!     fn handles(&self) -> Vec<String> {
//!         vec!["UserCreated".to_string()]
//!     }
//!
//!     async fn handle(&self, event: &Event) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!         if event.event_type == "UserCreated" {
//!             println!("Sending welcome email for user: {}", event.aggregate_id);
//!         }
//!         Ok(())
//!     }
//! }
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create event bus
//! let mut event_bus = InProcessEventBus::new();
//!
//! // Subscribe handler
//! event_bus.subscribe(Box::new(WelcomeEmailHandler)).await?;
//!
//! // Publish event
//! let event = Event::new(
//!     "User",
//!     "user-123",
//!     1,
//!     "UserCreated",
//!     json!({ "email": "alice@example.com" }),
//! );
//! event_bus.publish(vec![event]).await?;
//! # Ok(())
//! # }
//! ```

use crate::event::Event;
use async_trait::async_trait;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;

/// Errors that can occur during event bus operations.
#[derive(Debug, Error)]
pub enum EventBusError {
    /// Handler execution failed
    #[error("Event handler failed for event '{event_type}' (event_id: {event_id}): {message}")]
    HandlerFailed {
        event_type: String,
        event_id: String,
        message: String,
    },

    /// No handlers registered for event type
    #[error("No handlers registered for event type '{event_type}'")]
    NoHandlers { event_type: String },

    /// Handler subscription failed
    #[error("Failed to subscribe handler: {message}")]
    SubscriptionFailed { message: String },

    /// General event bus error
    #[error("Event bus error: {message}")]
    Other { message: String },
}

impl EventBusError {
    /// Create a handler failed error.
    pub fn handler_failed(
        event_type: impl Into<String>,
        event_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        EventBusError::HandlerFailed {
            event_type: event_type.into(),
            event_id: event_id.into(),
            message: message.into(),
        }
    }

    /// Create a no handlers error.
    pub fn no_handlers(event_type: impl Into<String>) -> Self {
        EventBusError::NoHandlers {
            event_type: event_type.into(),
        }
    }

    /// Create a subscription failed error.
    pub fn subscription_failed(message: impl Into<String>) -> Self {
        EventBusError::SubscriptionFailed {
            message: message.into(),
        }
    }

    /// Create a generic error.
    pub fn other(message: impl Into<String>) -> Self {
        EventBusError::Other {
            message: message.into(),
        }
    }
}

/// Result type for event bus operations.
pub type EventBusResult<T> = Result<T, EventBusError>;

/// Trait for event handlers that process published events.
///
/// Event handlers subscribe to specific event types and execute side effects
/// when those events occur. Handlers should be idempotent where possible.
///
/// # Thread Safety
///
/// Implementations must be Send + Sync to work with async Rust.
///
/// # Example
///
/// ```rust
/// use arc_core::event_bus::EventHandler;
/// use arc_core::event::Event;
/// use async_trait::async_trait;
///
/// struct AuditLogHandler;
///
/// #[async_trait]
/// impl EventHandler for AuditLogHandler {
///     fn handles(&self) -> Vec<String> {
///         // Handle all user-related events
///         vec![
///             "UserCreated".to_string(),
///             "UserUpdated".to_string(),
///             "UserDeleted".to_string(),
///         ]
///     }
///
///     async fn handle(&self, event: &Event) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
///         println!("Audit: {} at {}", event.event_type, event.timestamp);
///         // Write to audit log...
///         Ok(())
///     }
/// }
/// ```
#[async_trait]
pub trait EventHandler: Send + Sync {
    /// Returns the list of event types this handler is interested in.
    ///
    /// The handler's `handle()` method will only be called for events
    /// whose event_type is in this list.
    ///
    /// # Returns
    ///
    /// Vector of event type names (e.g., ["UserCreated", "UserUpdated"])
    fn handles(&self) -> Vec<String>;

    /// Handle a published event.
    ///
    /// This method is called when an event matching one of the types returned
    /// by `handles()` is published to the event bus.
    ///
    /// # Arguments
    ///
    /// - `event`: Reference to the event being handled
    ///
    /// # Returns
    ///
    /// - `Ok(())` if the event was handled successfully
    /// - `Err(...)` if handling failed (error will be propagated to publisher)
    ///
    /// # Error Handling
    ///
    /// If an error is returned, it will stop event processing for subsequent
    /// handlers. Consider logging errors and returning Ok(()) if you want
    /// to allow other handlers to continue.
    async fn handle(&self, event: &Event) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

/// Trait for event bus implementations.
///
/// The event bus provides pub/sub functionality for domain events.
/// Publishers call `publish()` to send events, and subscribers register
/// via `subscribe()` to receive events they're interested in.
///
/// # Thread Safety
///
/// Implementations must be Send + Sync to work with async Rust.
///
/// # Example
///
/// ```rust,ignore
/// use arc_core::event_bus::{EventBus, InProcessEventBus};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut event_bus = InProcessEventBus::new();
///
/// // Subscribe handlers
/// event_bus.subscribe(Box::new(EmailHandler::new())).await?;
/// event_bus.subscribe(Box::new(NotificationHandler::new())).await?;
///
/// // Publish events
/// event_bus.publish(events).await?;
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait EventBus: Send + Sync {
    /// Publish events to all subscribed handlers.
    ///
    /// Events are delivered to all handlers that have registered interest
    /// in their event_type via the `handles()` method.
    ///
    /// # Arguments
    ///
    /// - `events`: Vector of events to publish
    ///
    /// # Returns
    ///
    /// - `Ok(())` if all handlers processed all events successfully
    /// - `Err(EventBusError::HandlerFailed)` if any handler fails
    ///
    /// # Ordering
    ///
    /// Events are delivered in the order provided. Handlers are called
    /// synchronously in subscription order.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// # async fn example(event_bus: impl EventBus, events: Vec<Event>) -> Result<(), Box<dyn std::error::Error>> {
    /// // Publish multiple events
    /// event_bus.publish(events).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn publish(&self, events: Vec<Event>) -> EventBusResult<()>;

    /// Subscribe an event handler to the bus.
    ///
    /// The handler will be called for all future events that match
    /// the types returned by its `handles()` method.
    ///
    /// # Arguments
    ///
    /// - `handler`: Boxed event handler implementation
    ///
    /// # Returns
    ///
    /// - `Ok(())` if subscription succeeded
    /// - `Err(EventBusError::SubscriptionFailed)` if subscription failed
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// # async fn example(mut event_bus: impl EventBus) -> Result<(), Box<dyn std::error::Error>> {
    /// event_bus.subscribe(Box::new(MyHandler::new())).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn subscribe(&mut self, handler: Box<dyn EventHandler>) -> EventBusResult<()>;
}

/// In-process, synchronous event bus implementation.
///
/// This is the default event bus implementation that delivers events
/// synchronously to all registered handlers in the same process.
///
/// # Thread Safety
///
/// Uses Arc<Mutex<>> internally for thread-safe handler management.
///
/// # Performance
///
/// - Synchronous delivery means handlers block the publisher
/// - Handlers are called sequentially in subscription order
/// - For high-throughput scenarios, consider async/queue-based implementations
///
/// # Example
///
/// ```rust
/// use arc_core::event_bus::{EventBus, EventHandler, InProcessEventBus};
/// use arc_core::event::Event;
/// use serde_json::json;
/// use async_trait::async_trait;
///
/// struct LogHandler;
///
/// #[async_trait]
/// impl EventHandler for LogHandler {
///     fn handles(&self) -> Vec<String> {
///         vec!["UserCreated".to_string()]
///     }
///
///     async fn handle(&self, event: &Event) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
///         println!("Event logged: {}", event.event_type);
///         Ok(())
///     }
/// }
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut bus = InProcessEventBus::new();
/// bus.subscribe(Box::new(LogHandler)).await?;
///
/// let event = Event::new("User", "user-1", 1, "UserCreated", json!({}));
/// bus.publish(vec![event]).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct InProcessEventBus {
    handlers: Arc<Mutex<Vec<Box<dyn EventHandler>>>>,
}

impl InProcessEventBus {
    /// Create a new in-process event bus.
    ///
    /// # Example
    ///
    /// ```rust
    /// use arc_core::event_bus::InProcessEventBus;
    ///
    /// let bus = InProcessEventBus::new();
    /// ```
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get the number of registered handlers.
    ///
    /// Useful for testing and diagnostics.
    ///
    /// # Example
    ///
    /// ```rust
    /// use arc_core::event_bus::InProcessEventBus;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let bus = InProcessEventBus::new();
    /// assert_eq!(bus.handler_count().await, 0);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn handler_count(&self) -> usize {
        self.handlers.lock().await.len()
    }
}

impl Default for InProcessEventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EventBus for InProcessEventBus {
    async fn publish(&self, events: Vec<Event>) -> EventBusResult<()> {
        let handlers = self.handlers.lock().await;

        for event in &events {
            // Find all handlers interested in this event type
            for handler in handlers.iter() {
                let handled_types = handler.handles();

                if handled_types.contains(&event.event_type) {
                    // Call the handler
                    handler.handle(event).await.map_err(|e| {
                        EventBusError::handler_failed(
                            &event.event_type,
                            event.event_id.to_string(),
                            e.to_string(),
                        )
                    })?;
                }
            }
        }

        Ok(())
    }

    async fn subscribe(&mut self, handler: Box<dyn EventHandler>) -> EventBusResult<()> {
        let mut handlers = self.handlers.lock().await;
        handlers.push(handler);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::Arc;
    use tokio::sync::Mutex as TokioMutex;

    // Test handler that counts how many times it's called
    struct CountingHandler {
        count: Arc<TokioMutex<usize>>,
        event_types: Vec<String>,
    }

    impl CountingHandler {
        fn new(event_types: Vec<String>) -> Self {
            Self {
                count: Arc::new(TokioMutex::new(0)),
                event_types,
            }
        }
    }

    #[async_trait]
    impl EventHandler for CountingHandler {
        fn handles(&self) -> Vec<String> {
            self.event_types.clone()
        }

        async fn handle(
            &self,
            _event: &Event,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            let mut count = self.count.lock().await;
            *count += 1;
            Ok(())
        }
    }

    // Test handler that fails on specific event types
    struct FailingHandler {
        fail_on: String,
    }

    #[async_trait]
    impl EventHandler for FailingHandler {
        fn handles(&self) -> Vec<String> {
            vec![self.fail_on.clone()]
        }

        async fn handle(
            &self,
            _event: &Event,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Err("Intentional test failure".into())
        }
    }

    #[tokio::test]
    async fn test_new_event_bus() {
        let bus = InProcessEventBus::new();
        assert_eq!(bus.handler_count().await, 0);
    }

    #[tokio::test]
    async fn test_subscribe_handler() {
        let mut bus = InProcessEventBus::new();
        let handler = Box::new(CountingHandler::new(vec!["UserCreated".to_string()]));

        bus.subscribe(handler).await.unwrap();
        assert_eq!(bus.handler_count().await, 1);
    }

    #[tokio::test]
    async fn test_subscribe_multiple_handlers() {
        let mut bus = InProcessEventBus::new();

        bus.subscribe(Box::new(CountingHandler::new(vec![
            "UserCreated".to_string()
        ])))
        .await
        .unwrap();
        bus.subscribe(Box::new(CountingHandler::new(vec![
            "UserUpdated".to_string()
        ])))
        .await
        .unwrap();

        assert_eq!(bus.handler_count().await, 2);
    }

    #[tokio::test]
    async fn test_publish_single_event() {
        let mut bus = InProcessEventBus::new();
        let counter = Arc::new(TokioMutex::new(0));
        let counter_clone = counter.clone();

        let handler = CountingHandler {
            count: counter_clone,
            event_types: vec!["UserCreated".to_string()],
        };

        bus.subscribe(Box::new(handler)).await.unwrap();

        let event = Event::new(
            "User",
            "user-123",
            1,
            "UserCreated",
            json!({ "name": "Alice" }),
        );

        bus.publish(vec![event]).await.unwrap();

        let count = *counter.lock().await;
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_publish_multiple_events() {
        let mut bus = InProcessEventBus::new();
        let counter = Arc::new(TokioMutex::new(0));
        let counter_clone = counter.clone();

        let handler = CountingHandler {
            count: counter_clone,
            event_types: vec!["UserCreated".to_string(), "UserUpdated".to_string()],
        };

        bus.subscribe(Box::new(handler)).await.unwrap();

        let events = vec![
            Event::new(
                "User",
                "user-1",
                1,
                "UserCreated",
                json!({ "name": "Alice" }),
            ),
            Event::new(
                "User",
                "user-1",
                2,
                "UserUpdated",
                json!({ "name": "Alice Smith" }),
            ),
            Event::new("User", "user-2", 1, "UserCreated", json!({ "name": "Bob" })),
        ];

        bus.publish(events).await.unwrap();

        let count = *counter.lock().await;
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_handler_filters_event_types() {
        let mut bus = InProcessEventBus::new();
        let counter = Arc::new(TokioMutex::new(0));
        let counter_clone = counter.clone();

        // Handler only interested in UserCreated
        let handler = CountingHandler {
            count: counter_clone,
            event_types: vec!["UserCreated".to_string()],
        };

        bus.subscribe(Box::new(handler)).await.unwrap();

        let events = vec![
            Event::new("User", "user-1", 1, "UserCreated", json!({})),
            Event::new("User", "user-1", 2, "UserUpdated", json!({})),
            Event::new("User", "user-1", 3, "UserDeleted", json!({})),
        ];

        bus.publish(events).await.unwrap();

        // Should only count UserCreated event
        let count = *counter.lock().await;
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_multiple_handlers_same_event() {
        let mut bus = InProcessEventBus::new();
        let counter1 = Arc::new(TokioMutex::new(0));
        let counter2 = Arc::new(TokioMutex::new(0));

        let handler1 = CountingHandler {
            count: counter1.clone(),
            event_types: vec!["UserCreated".to_string()],
        };

        let handler2 = CountingHandler {
            count: counter2.clone(),
            event_types: vec!["UserCreated".to_string()],
        };

        bus.subscribe(Box::new(handler1)).await.unwrap();
        bus.subscribe(Box::new(handler2)).await.unwrap();

        let event = Event::new("User", "user-1", 1, "UserCreated", json!({}));
        bus.publish(vec![event]).await.unwrap();

        // Both handlers should be called
        assert_eq!(*counter1.lock().await, 1);
        assert_eq!(*counter2.lock().await, 1);
    }

    #[tokio::test]
    async fn test_handler_failure_propagates() {
        let mut bus = InProcessEventBus::new();

        let failing_handler = Box::new(FailingHandler {
            fail_on: "UserCreated".to_string(),
        });

        bus.subscribe(failing_handler).await.unwrap();

        let event = Event::new("User", "user-1", 1, "UserCreated", json!({}));
        let result = bus.publish(vec![event]).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            EventBusError::HandlerFailed {
                event_type,
                event_id,
                message,
            } => {
                assert_eq!(event_type, "UserCreated");
                assert!(!event_id.is_empty());
                assert!(message.contains("Intentional test failure"));
            }
            _ => panic!("Expected HandlerFailed error"),
        }
    }

    #[tokio::test]
    async fn test_no_handlers_for_event_type() {
        let bus = InProcessEventBus::new();

        // No handlers subscribed
        let event = Event::new("User", "user-1", 1, "UserCreated", json!({}));
        let result = bus.publish(vec![event]).await;

        // Should succeed - no handlers is not an error
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handler_called_in_order() {
        let mut bus = InProcessEventBus::new();
        let order = Arc::new(TokioMutex::new(Vec::new()));

        struct OrderTracker {
            id: usize,
            order: Arc<TokioMutex<Vec<usize>>>,
        }

        #[async_trait]
        impl EventHandler for OrderTracker {
            fn handles(&self) -> Vec<String> {
                vec!["TestEvent".to_string()]
            }

            async fn handle(
                &self,
                _event: &Event,
            ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                self.order.lock().await.push(self.id);
                Ok(())
            }
        }

        // Subscribe 3 handlers
        for i in 1..=3 {
            bus.subscribe(Box::new(OrderTracker {
                id: i,
                order: order.clone(),
            }))
            .await
            .unwrap();
        }

        let event = Event::new("Test", "test-1", 1, "TestEvent", json!({}));
        bus.publish(vec![event]).await.unwrap();

        // Handlers should be called in subscription order
        let call_order = order.lock().await;
        assert_eq!(*call_order, vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_event_bus_clone() {
        let mut bus1 = InProcessEventBus::new();
        let counter = Arc::new(TokioMutex::new(0));

        let handler = CountingHandler {
            count: counter.clone(),
            event_types: vec!["UserCreated".to_string()],
        };

        bus1.subscribe(Box::new(handler)).await.unwrap();

        // Clone the bus
        let bus2 = bus1.clone();

        // Both should share the same handlers
        assert_eq!(bus1.handler_count().await, 1);
        assert_eq!(bus2.handler_count().await, 1);

        // Publishing through either should work
        let event = Event::new("User", "user-1", 1, "UserCreated", json!({}));
        bus2.publish(vec![event]).await.unwrap();

        assert_eq!(*counter.lock().await, 1);
    }

    #[test]
    fn test_error_messages() {
        let error = EventBusError::handler_failed("UserCreated", "event-123", "Connection timeout");
        let msg = error.to_string();
        assert!(msg.contains("UserCreated"));
        assert!(msg.contains("event-123"));
        assert!(msg.contains("Connection timeout"));

        let error = EventBusError::no_handlers("UnknownEvent");
        assert!(error.to_string().contains("UnknownEvent"));

        let error = EventBusError::subscription_failed("Handler invalid");
        assert!(error.to_string().contains("Handler invalid"));
    }
}
