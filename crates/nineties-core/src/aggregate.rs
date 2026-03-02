//! # Aggregate Module
//!
//! Core abstractions for domain aggregates and commands in event sourcing.
//!
//! ## Overview
//!
//! This module provides the traits needed to implement aggregates following the
//! Event Sourcing and CQRS patterns. Aggregates are the fundamental building blocks
//! that encapsulate business logic, enforce invariants, and produce events.
//!
//! ## Design Philosophy
//!
//! **Complexity is Opt-In**: You can use aggregates in two ways:
//!
//! 1. **Simple Path**: Define enums for commands/events, implement the trait
//! 2. **Complex Path**: Add rich domain logic, validation, and business rules
//!
//! Both approaches use the same infrastructure and are first-class citizens.
//!
//! ## Core Concepts
//!
//! ### Commands
//!
//! Commands represent **intent** to change state. They are imperative (e.g., `CreateUser`,
//! `UpdateProfile`) and can be rejected if business rules aren't satisfied.
//!
//! - Commands are validated before producing events
//! - Commands may produce zero events (validation failure)
//! - Commands may produce multiple events (complex operations)
//! - Commands from one aggregate must be atomic
//!
//! ### Events
//!
//! Events represent **facts** about things that have happened. They are past tense
//! (e.g., `UserCreated`, `ProfileUpdated`) and cannot be rejected once produced.
//!
//! - Events are immutable once written
//! - Events are the source of truth
//! - Events are used to reconstruct aggregate state
//! - Events are published to event bus for subscribers
//!
//! ### Aggregates
//!
//! Aggregates are consistency boundaries that:
//!
//! - Encapsulate domain logic and business rules
//! - Validate commands and produce events
//! - Apply events to update internal state
//! - Can be reconstructed from their event stream
//! - Enforce invariants within their boundary
//!
//! ## Quick Start
//!
//! ### 1. Define Your Domain Events
//!
//! ```rust
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! pub enum UserEvent {
//!     UserCreated {
//!         id: String,
//!         name: String,
//!         email: String,
//!     },
//!     ProfileUpdated {
//!         name: String,
//!     },
//!     EmailChanged {
//!         email: String,
//!     },
//! }
//! ```
//!
//! ### 2. Define Your Commands
//!
//! ```rust
//! use nineties_core::aggregate::Command;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! pub enum UserCommand {
//!     CreateUser {
//!         id: String,
//!         name: String,
//!         email: String,
//!     },
//!     UpdateProfile {
//!         id: String,
//!         name: String,
//!     },
//!     ChangeEmail {
//!         id: String,
//!         email: String,
//!     },
//! }
//!
//! impl Command for UserCommand {
//!     fn aggregate_id(&self) -> &str {
//!         match self {
//!             UserCommand::CreateUser { id, .. } => id,
//!             UserCommand::UpdateProfile { id, .. } => id,
//!             UserCommand::ChangeEmail { id, .. } => id,
//!         }
//!     }
//! }
//! ```
//!
//! ### 3. Define Your Aggregate
//!
//! ```rust
//! use nineties_core::aggregate::Aggregate;
//! use nineties_core::event::Event;
//! use thiserror::Error;
//!
//! # use serde::{Deserialize, Serialize};
//! # use nineties_core::aggregate::Command;
//! #
//! # #[derive(Debug, Clone, Serialize, Deserialize)]
//! # pub enum UserEvent {
//! #     UserCreated { id: String, name: String, email: String },
//! #     ProfileUpdated { name: String },
//! #     EmailChanged { email: String },
//! # }
//! #
//! # #[derive(Debug, Clone, Serialize, Deserialize)]
//! # pub enum UserCommand {
//! #     CreateUser { id: String, name: String, email: String },
//! #     UpdateProfile { id: String, name: String },
//! #     ChangeEmail { id: String, email: String },
//! # }
//! #
//! # impl Command for UserCommand {
//! #     fn aggregate_id(&self) -> &str {
//! #         match self {
//! #             UserCommand::CreateUser { id, .. } => id,
//! #             UserCommand::UpdateProfile { id, .. } => id,
//! #             UserCommand::ChangeEmail { id, .. } => id,
//! #         }
//! #     }
//! # }
//! #
//! #[derive(Debug, Error)]
//! pub enum UserError {
//!     #[error("User already exists")]
//!     AlreadyExists,
//!     #[error("User not found")]
//!     NotFound,
//!     #[error("Invalid email format")]
//!     InvalidEmail,
//! }
//!
//! #[derive(Default)]
//! pub struct UserAggregate {
//!     id: Option<String>,
//!     name: Option<String>,
//!     email: Option<String>,
//!     version: i64,
//!     created: bool,
//! }
//!
//! #[async_trait::async_trait]
//! impl Aggregate for UserAggregate {
//!     type Command = UserCommand;
//!     type Event = UserEvent;
//!     type Error = UserError;
//!
//!     fn aggregate_type() -> &'static str {
//!         "User"
//!     }
//!
//!     fn version(&self) -> i64 {
//!         self.version
//!     }
//!
//!     async fn handle(&self, command: Self::Command) -> Result<Vec<Event>, Self::Error> {
//!         match command {
//!             UserCommand::CreateUser { id, name, email } => {
//!                 // Enforce invariant: user cannot be created twice
//!                 if self.created {
//!                     return Err(UserError::AlreadyExists);
//!                 }
//!
//!                 // Validate email
//!                 if !email.contains('@') {
//!                     return Err(UserError::InvalidEmail);
//!                 }
//!
//!                 // Produce event
//!                 Ok(vec![Event::new(
//!                     "User",
//!                     &id,
//!                     self.version + 1,
//!                     "UserCreated",
//!                     serde_json::json!({
//!                         "id": id,
//!                         "name": name,
//!                         "email": email,
//!                     }),
//!                 )])
//!             }
//!             UserCommand::UpdateProfile { id, name } => {
//!                 if !self.created {
//!                     return Err(UserError::NotFound);
//!                 }
//!
//!                 Ok(vec![Event::new(
//!                     "User",
//!                     &id,
//!                     self.version + 1,
//!                     "ProfileUpdated",
//!                     serde_json::json!({ "name": name }),
//!                 )])
//!             }
//!             UserCommand::ChangeEmail { id, email } => {
//!                 if !self.created {
//!                     return Err(UserError::NotFound);
//!                 }
//!
//!                 if !email.contains('@') {
//!                     return Err(UserError::InvalidEmail);
//!                 }
//!
//!                 Ok(vec![Event::new(
//!                     "User",
//!                     &id,
//!                     self.version + 1,
//!                     "EmailChanged",
//!                     serde_json::json!({ "email": email }),
//!                 )])
//!             }
//!         }
//!     }
//!
//!     fn apply(&mut self, event: &Event) {
//!         self.version = event.sequence;
//!
//!         match event.event_type.as_str() {
//!             "UserCreated" => {
//!                 self.id = Some(event.payload["id"].as_str().unwrap().to_string());
//!                 self.name = Some(event.payload["name"].as_str().unwrap().to_string());
//!                 self.email = Some(event.payload["email"].as_str().unwrap().to_string());
//!                 self.created = true;
//!             }
//!             "ProfileUpdated" => {
//!                 self.name = Some(event.payload["name"].as_str().unwrap().to_string());
//!             }
//!             "EmailChanged" => {
//!                 self.email = Some(event.payload["email"].as_str().unwrap().to_string());
//!             }
//!             _ => {}
//!         }
//!     }
//! }
//! ```
//!
//! ### 4. Use Your Aggregate
//!
//! ```rust,no_run
//! # use nineties_core::aggregate::Aggregate;
//! # use nineties_core::event::Event;
//! # use thiserror::Error;
//! # use serde::{Deserialize, Serialize};
//! # use nineties_core::aggregate::Command;
//! #
//! # #[derive(Debug, Clone, Serialize, Deserialize)]
//! # pub enum UserEvent {
//! #     UserCreated { id: String, name: String, email: String },
//! #     ProfileUpdated { name: String },
//! #     EmailChanged { email: String },
//! # }
//! #
//! # #[derive(Debug, Clone, Serialize, Deserialize)]
//! # pub enum UserCommand {
//! #     CreateUser { id: String, name: String, email: String },
//! #     UpdateProfile { id: String, name: String },
//! #     ChangeEmail { id: String, email: String },
//! # }
//! #
//! # impl Command for UserCommand {
//! #     fn aggregate_id(&self) -> &str {
//! #         match self {
//! #             UserCommand::CreateUser { id, .. } => id,
//! #             UserCommand::UpdateProfile { id, .. } => id,
//! #             UserCommand::ChangeEmail { id, .. } => id,
//! #         }
//! #     }
//! # }
//! #
//! # #[derive(Debug, Error)]
//! # pub enum UserError {
//! #     #[error("User already exists")]
//! #     AlreadyExists,
//! #     #[error("User not found")]
//! #     NotFound,
//! #     #[error("Invalid email format")]
//! #     InvalidEmail,
//! # }
//! #
//! # #[derive(Default)]
//! # pub struct UserAggregate {
//! #     id: Option<String>,
//! #     name: Option<String>,
//! #     email: Option<String>,
//! #     version: i64,
//! #     created: bool,
//! # }
//! #
//! # #[async_trait::async_trait]
//! # impl Aggregate for UserAggregate {
//! #     type Command = UserCommand;
//! #     type Event = UserEvent;
//! #     type Error = UserError;
//! #     fn aggregate_type() -> &'static str { "User" }
//! #     fn version(&self) -> i64 { self.version }
//! #     async fn handle(&self, command: Self::Command) -> Result<Vec<Event>, Self::Error> {
//! #         Ok(vec![])
//! #     }
//! #     fn apply(&mut self, event: &Event) {}
//! # }
//! #
//! # async fn example() {
//! // Create a command
//! let command = UserCommand::CreateUser {
//!     id: "user-123".to_string(),
//!     name: "Alice".to_string(),
//!     email: "alice@example.com".to_string(),
//! };
//!
//! // Create aggregate (typically loaded from event store)
//! let aggregate = UserAggregate::default();
//!
//! // Handle command
//! let events = aggregate.handle(command).await.unwrap();
//!
//! // Events would be persisted to event store and published to event bus
//! assert_eq!(events.len(), 1);
//! assert_eq!(events[0].event_type, "UserCreated");
//! # }
//! ```
//!
//! ## Testing Your Aggregates
//!
//! Aggregates are easy to test because they're pure functions (commands → events → state).
//!
//! ```rust
//! # use nineties_core::aggregate::Aggregate;
//! # use nineties_core::event::Event;
//! # use thiserror::Error;
//! # use serde::{Deserialize, Serialize};
//! # use nineties_core::aggregate::Command;
//! #
//! # #[derive(Debug, Clone, Serialize, Deserialize)]
//! # pub enum UserEvent {
//! #     UserCreated { id: String, name: String, email: String },
//! # }
//! #
//! # #[derive(Debug, Clone, Serialize, Deserialize)]
//! # pub enum UserCommand {
//! #     CreateUser { id: String, name: String, email: String },
//! # }
//! #
//! # impl Command for UserCommand {
//! #     fn aggregate_id(&self) -> &str {
//! #         match self {
//! #             UserCommand::CreateUser { id, .. } => id,
//! #         }
//! #     }
//! # }
//! #
//! # #[derive(Debug, Error)]
//! # pub enum UserError {
//! #     #[error("User already exists")]
//! #     AlreadyExists,
//! #     #[error("Invalid email format")]
//! #     InvalidEmail,
//! # }
//! #
//! # #[derive(Default)]
//! # pub struct UserAggregate {
//! #     id: Option<String>,
//! #     version: i64,
//! #     created: bool,
//! # }
//! #
//! # #[async_trait::async_trait]
//! # impl Aggregate for UserAggregate {
//! #     type Command = UserCommand;
//! #     type Event = UserEvent;
//! #     type Error = UserError;
//! #     fn aggregate_type() -> &'static str { "User" }
//! #     fn version(&self) -> i64 { self.version }
//! #     async fn handle(&self, command: Self::Command) -> Result<Vec<Event>, Self::Error> {
//! #         match command {
//! #             UserCommand::CreateUser { id, name, email } => {
//! #                 if self.created {
//! #                     return Err(UserError::AlreadyExists);
//! #                 }
//! #                 if !email.contains('@') {
//! #                     return Err(UserError::InvalidEmail);
//! #                 }
//! #                 Ok(vec![Event::new("User", &id, self.version + 1, "UserCreated",
//! #                     serde_json::json!({ "id": id, "name": name, "email": email }))])
//! #             }
//! #         }
//! #     }
//! #     fn apply(&mut self, event: &Event) {
//! #         self.version = event.sequence;
//! #         if event.event_type == "UserCreated" {
//! #             self.id = Some(event.aggregate_id.clone());
//! #             self.created = true;
//! #         }
//! #     }
//! # }
//! #
//! #[tokio::test]
//! async fn test_user_creation() {
//!     // Given: A new user aggregate
//!     let aggregate = UserAggregate::default();
//!
//!     // When: Creating a user
//!     let command = UserCommand::CreateUser {
//!         id: "user-123".to_string(),
//!         name: "Alice".to_string(),
//!         email: "alice@example.com".to_string(),
//!     };
//!
//!     let events = aggregate.handle(command).await.unwrap();
//!
//!     // Then: UserCreated event is produced
//!     assert_eq!(events.len(), 1);
//!     assert_eq!(events[0].event_type, "UserCreated");
//!     assert_eq!(events[0].aggregate_id, "user-123");
//! }
//!
//! #[tokio::test]
//! async fn test_invalid_email() {
//!     // Given: A new user aggregate
//!     let aggregate = UserAggregate::default();
//!
//!     // When: Creating a user with invalid email
//!     let command = UserCommand::CreateUser {
//!         id: "user-456".to_string(),
//!         name: "Bob".to_string(),
//!         email: "not-an-email".to_string(), // Invalid!
//!     };
//!
//!     let result = aggregate.handle(command).await;
//!
//!     // Then: Error is returned
//!     assert!(result.is_err());
//! }
//!
//! #[tokio::test]
//! async fn test_cannot_create_twice() {
//!     // Given: An existing user (reconstructed from events)
//!     let mut aggregate = UserAggregate::default();
//!     let event = Event::new(
//!         "User",
//!         "user-789",
//!         1,
//!         "UserCreated",
//!         serde_json::json!({
//!             "id": "user-789",
//!             "name": "Charlie",
//!             "email": "charlie@example.com"
//!         }),
//!     );
//!     aggregate.apply(&event);
//!
//!     // When: Trying to create the user again
//!     let command = UserCommand::CreateUser {
//!         id: "user-789".to_string(),
//!         name: "Charlie".to_string(),
//!         email: "charlie@example.com".to_string(),
//!     };
//!
//!     let result = aggregate.handle(command).await;
//!
//!     // Then: Error is returned
//!     assert!(result.is_err());
//! }
//! ```
//!
//! ## Advanced Patterns
//!
//! ### Multiple Events from One Command
//!
//! Sometimes a command should produce multiple events atomically:
//!
//! ```rust,ignore
//! async fn handle(&self, command: Self::Command) -> Result<Vec<Event>, Self::Error> {
//!     match command {
//!         OrderCommand::PlaceOrder { order_id, items } => {
//!             // Validate stock
//!             // ...
//!
//!             // Produce multiple events
//!             Ok(vec![
//!                 Event::new("Order", &order_id, self.version + 1, "OrderPlaced", ...),
//!                 Event::new("Order", &order_id, self.version + 2, "InventoryReserved", ...),
//!                 Event::new("Order", &order_id, self.version + 3, "PaymentRequested", ...),
//!             ])
//!         }
//!     }
//! }
//! ```
//!
//! ### Conditional Events
//!
//! Commands may produce zero events if preconditions aren't met:
//!
//! ```rust,ignore
//! async fn handle(&self, command: Self::Command) -> Result<Vec<Event>, Self::Error> {
//!     match command {
//!         UserCommand::MarkAsActive { id } => {
//!             // If already active, no event needed
//!             if self.is_active {
//!                 return Ok(vec![]);
//!             }
//!
//!             Ok(vec![Event::new("User", &id, self.version + 1, "UserActivated", ...)])
//!         }
//!     }
//! }
//! ```
//!
//! ### Complex Validation
//!
//! Use the aggregate state to enforce complex business rules:
//!
//! ```rust,ignore
//! async fn handle(&self, command: Self::Command) -> Result<Vec<Event>, Self::Error> {
//!     match command {
//!         AccountCommand::Withdraw { amount } => {
//!             // Check balance
//!             if self.balance < amount {
//!                 return Err(AccountError::InsufficientFunds);
//!             }
//!
//!             // Check withdrawal limit
//!             if self.daily_withdrawals + amount > self.daily_limit {
//!                 return Err(AccountError::DailyLimitExceeded);
//!             }
//!
//!             Ok(vec![Event::new("Account", &self.id, self.version + 1, "Withdrawn", ...)])
//!         }
//!     }
//! }
//! ```

use crate::event::Event;
use async_trait::async_trait;
use std::error::Error;

/// Trait for commands that can be dispatched to aggregates.
///
/// Commands represent intent to change aggregate state. They are imperative
/// (e.g., `CreateUser`, `UpdateProfile`) and can be rejected if business rules
/// aren't satisfied.
///
/// # Design Principles
///
/// - **Imperative naming**: Use verbs (Create, Update, Delete, Activate)
/// - **Validation happens in aggregates**: Commands may be rejected
/// - **Identify target**: Commands must know which aggregate instance to operate on
/// - **Serializable**: Commands should be serializable for command sourcing
///
/// # Example
///
/// ```rust
/// use nineties_core::aggregate::Command;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// pub enum UserCommand {
///     CreateUser { id: String, name: String, email: String },
///     UpdateProfile { id: String, name: String },
///     DeleteUser { id: String },
/// }
///
/// impl Command for UserCommand {
///     fn aggregate_id(&self) -> &str {
///         match self {
///             UserCommand::CreateUser { id, .. } => id,
///             UserCommand::UpdateProfile { id, .. } => id,
///             UserCommand::DeleteUser { id } => id,
///         }
///     }
/// }
/// ```
pub trait Command: Send + Sync {
    /// Get the aggregate instance ID this command targets.
    ///
    /// The command bus uses this to load the correct aggregate instance
    /// from the event store.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use nineties_core::aggregate::Command;
    /// # use serde::{Deserialize, Serialize};
    /// #
    /// # #[derive(Debug, Clone, Serialize, Deserialize)]
    /// # pub enum UserCommand {
    /// #     CreateUser { id: String, name: String },
    /// # }
    /// #
    /// # impl Command for UserCommand {
    /// #     fn aggregate_id(&self) -> &str {
    /// #         match self {
    /// #             UserCommand::CreateUser { id, .. } => id,
    /// #         }
    /// #     }
    /// # }
    /// #
    /// let command = UserCommand::CreateUser {
    ///     id: "user-123".to_string(),
    ///     name: "Alice".to_string(),
    /// };
    ///
    /// assert_eq!(command.aggregate_id(), "user-123");
    /// ```
    fn aggregate_id(&self) -> &str;
}

/// Trait for domain aggregates in event sourcing.
///
/// Aggregates are consistency boundaries that encapsulate business logic,
/// enforce invariants, and produce events in response to commands.
///
/// # Lifecycle
///
/// 1. **Load**: Reconstruct aggregate from event stream using `from_events()`
/// 2. **Command**: Handle command with `handle()` to produce new events
/// 3. **Store**: Persist events to event store (done by command bus)
/// 4. **Apply**: Apply new events to update aggregate state using `apply()`
/// 5. **Publish**: Publish events to event bus (done by command bus)
///
/// # Design Principles
///
/// - **State is private**: Aggregate state should not be exposed outside
/// - **Commands produce events**: `handle()` validates and produces events
/// - **Events update state**: `apply()` updates internal state deterministically
/// - **Pure functions**: `handle()` has no side effects (no I/O, no mutations)
/// - **Deterministic apply**: Same events always produce same state
/// - **Default implementation**: Aggregate must implement `Default` for initial state
///
/// # Type Parameters
///
/// - `Command`: The command type this aggregate handles (must implement `Command`)
/// - `Event`: The domain event type (typically an enum, must be serializable)
/// - `Error`: The error type for validation failures (must implement `std::error::Error`)
///
/// # Example
///
/// See the module-level documentation for a complete example.
#[async_trait]
pub trait Aggregate: Send + Sync + Default {
    /// The command type this aggregate handles
    type Command: Command;

    /// The domain event type (your custom enum)
    type Event;

    /// The error type for validation failures
    type Error: Error + Send + Sync + 'static;

    /// Get the aggregate type name (e.g., "User", "Order", "Account").
    ///
    /// This is used for event metadata and debugging. Should be a static string
    /// that uniquely identifies this aggregate type in your domain.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use nineties_core::aggregate::{Aggregate, Command};
    /// # use nineties_core::event::Event;
    /// # use thiserror::Error;
    /// #
    /// # #[derive(Debug)]
    /// # struct DummyCommand;
    /// # impl Command for DummyCommand {
    /// #     fn aggregate_id(&self) -> &str { "dummy" }
    /// # }
    /// #
    /// # #[derive(Debug, Error)]
    /// # #[error("dummy error")]
    /// # struct DummyError;
    /// #
    /// # #[derive(Default)]
    /// # struct UserAggregate;
    /// #
    /// # #[async_trait::async_trait]
    /// # impl Aggregate for UserAggregate {
    /// #     type Command = DummyCommand;
    /// #     type Event = ();
    /// #     type Error = DummyError;
    /// #
    /// fn aggregate_type() -> &'static str {
    ///     "User"
    /// }
    /// #
    /// #     fn version(&self) -> i64 { 0 }
    /// #     async fn handle(&self, _: Self::Command) -> Result<Vec<Event>, Self::Error> { Ok(vec![]) }
    /// #     fn apply(&mut self, _: &Event) {}
    /// # }
    /// #
    /// assert_eq!(UserAggregate::aggregate_type(), "User");
    /// ```
    fn aggregate_type() -> &'static str;

    /// Get the current version (sequence number) of this aggregate.
    ///
    /// The version represents how many events have been applied to this aggregate.
    /// It starts at 0 for a new aggregate and increments with each event.
    ///
    /// This is used for optimistic concurrency control - the event store checks
    /// that the version hasn't changed since the aggregate was loaded.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use nineties_core::aggregate::{Aggregate, Command};
    /// # use nineties_core::event::Event;
    /// # use thiserror::Error;
    /// #
    /// # #[derive(Debug)]
    /// # struct DummyCommand;
    /// # impl Command for DummyCommand {
    /// #     fn aggregate_id(&self) -> &str { "dummy" }
    /// # }
    /// #
    /// # #[derive(Debug, Error)]
    /// # #[error("dummy error")]
    /// # struct DummyError;
    /// #
    /// # #[derive(Default)]
    /// # struct UserAggregate { version: i64 }
    /// #
    /// # #[async_trait::async_trait]
    /// # impl Aggregate for UserAggregate {
    /// #     type Command = DummyCommand;
    /// #     type Event = ();
    /// #     type Error = DummyError;
    /// #     fn aggregate_type() -> &'static str { "User" }
    /// #
    /// fn version(&self) -> i64 {
    ///     self.version
    /// }
    /// #
    /// #     async fn handle(&self, _: Self::Command) -> Result<Vec<Event>, Self::Error> { Ok(vec![]) }
    /// #     fn apply(&mut self, _: &Event) {}
    /// # }
    /// #
    /// let aggregate = UserAggregate::default();
    /// assert_eq!(aggregate.version(), 0); // New aggregate
    /// ```
    fn version(&self) -> i64;

    /// Handle a command and produce events.
    ///
    /// This is where your business logic lives. The method should:
    ///
    /// 1. Inspect current state (`self`) to make decisions
    /// 2. Validate the command against business rules
    /// 3. Return error if validation fails
    /// 4. Produce one or more events if validation succeeds
    /// 5. Return empty vec if command has no effect
    ///
    /// **Important**: This method should have no side effects:
    /// - Don't write to databases
    /// - Don't call external APIs
    /// - Don't mutate state
    ///
    /// Side effects happen in projections and event handlers.
    ///
    /// # Arguments
    ///
    /// - `command`: The command to handle
    ///
    /// # Returns
    ///
    /// - `Ok(Vec<Event>)`: One or more events to persist and publish
    /// - `Err(Self::Error)`: Validation or business rule failure
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// async fn handle(&self, command: Self::Command) -> Result<Vec<Event>, Self::Error> {
    ///     match command {
    ///         UserCommand::CreateUser { id, name, email } => {
    ///             // Check invariant
    ///             if self.created {
    ///                 return Err(UserError::AlreadyExists);
    ///             }
    ///
    ///             // Validate input
    ///             if !email.contains('@') {
    ///                 return Err(UserError::InvalidEmail);
    ///             }
    ///
    ///             // Produce event
    ///             Ok(vec![Event::new("User", &id, self.version + 1, "UserCreated", ...)])
    ///         }
    ///     }
    /// }
    /// ```
    async fn handle(&self, command: Self::Command) -> Result<Vec<Event>, Self::Error>;

    /// Apply an event to update aggregate state.
    ///
    /// This method must be **deterministic** and **side-effect free**:
    /// - Same events always produce same state
    /// - No I/O operations
    /// - No randomness
    /// - No external dependencies
    ///
    /// The event is already persisted when this is called. Your job is to
    /// update the aggregate's internal state to reflect the event.
    ///
    /// # Arguments
    ///
    /// - `event`: The event to apply (already persisted)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// fn apply(&mut self, event: &Event) {
    ///     self.version = event.sequence;
    ///
    ///     match event.event_type.as_str() {
    ///         "UserCreated" => {
    ///             self.id = Some(event.payload["id"].as_str().unwrap().to_string());
    ///             self.name = Some(event.payload["name"].as_str().unwrap().to_string());
    ///             self.created = true;
    ///         }
    ///         "ProfileUpdated" => {
    ///             self.name = Some(event.payload["name"].as_str().unwrap().to_string());
    ///         }
    ///         _ => {} // Unknown event types are ignored
    ///     }
    /// }
    /// ```
    fn apply(&mut self, event: &Event);

    /// Reconstruct aggregate from its event stream.
    ///
    /// This method has a default implementation that:
    /// 1. Creates a new aggregate instance using `Default::default()`
    /// 2. Applies each event in order using `apply()`
    /// 3. Returns the reconstructed aggregate
    ///
    /// You typically don't need to override this unless you have special requirements.
    ///
    /// # Arguments
    ///
    /// - `events`: The complete event stream for this aggregate
    ///
    /// # Returns
    ///
    /// A fully reconstructed aggregate with all events applied
    ///
    /// # Example
    ///
    /// ```rust
    /// # use nineties_core::aggregate::{Aggregate, Command};
    /// # use nineties_core::event::Event;
    /// # use thiserror::Error;
    /// #
    /// # #[derive(Debug)]
    /// # struct DummyCommand;
    /// # impl Command for DummyCommand {
    /// #     fn aggregate_id(&self) -> &str { "dummy" }
    /// # }
    /// #
    /// # #[derive(Debug, Error)]
    /// # #[error("dummy error")]
    /// # struct DummyError;
    /// #
    /// # #[derive(Default)]
    /// # struct UserAggregate {
    /// #     id: Option<String>,
    /// #     version: i64,
    /// #     created: bool,
    /// # }
    /// #
    /// # #[async_trait::async_trait]
    /// # impl Aggregate for UserAggregate {
    /// #     type Command = DummyCommand;
    /// #     type Event = ();
    /// #     type Error = DummyError;
    /// #     fn aggregate_type() -> &'static str { "User" }
    /// #     fn version(&self) -> i64 { self.version }
    /// #     async fn handle(&self, _: Self::Command) -> Result<Vec<Event>, Self::Error> { Ok(vec![]) }
    /// #
    /// #     fn apply(&mut self, event: &Event) {
    /// #         self.version = event.sequence;
    /// #         if event.event_type == "UserCreated" {
    /// #             self.id = Some(event.aggregate_id.clone());
    /// #             self.created = true;
    /// #         }
    /// #     }
    /// # }
    /// #
    /// // Load events from event store
    /// let events = vec![
    ///     Event::new("User", "user-123", 1, "UserCreated", serde_json::json!({"id": "user-123"})),
    ///     Event::new("User", "user-123", 2, "ProfileUpdated", serde_json::json!({"name": "Alice"})),
    /// ];
    ///
    /// // Reconstruct aggregate
    /// let aggregate = UserAggregate::from_events(events);
    ///
    /// assert_eq!(aggregate.version(), 2);
    /// assert!(aggregate.created);
    /// ```
    fn from_events(events: Vec<Event>) -> Self {
        let mut aggregate = Self::default();
        for event in events {
            aggregate.apply(&event);
        }
        aggregate
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use thiserror::Error;

    // Test domain: Simple counter aggregate
    #[derive(Debug, Clone, Serialize, Deserialize)]
    enum CounterCommand {
        Create { id: String },
        Increment { id: String, amount: i32 },
        Decrement { id: String, amount: i32 },
    }

    impl Command for CounterCommand {
        fn aggregate_id(&self) -> &str {
            match self {
                CounterCommand::Create { id } => id,
                CounterCommand::Increment { id, .. } => id,
                CounterCommand::Decrement { id, .. } => id,
            }
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    enum CounterEvent {
        Created { id: String },
        Incremented { amount: i32 },
        Decremented { amount: i32 },
    }

    #[derive(Debug, Error)]
    enum CounterError {
        #[error("Counter already exists")]
        AlreadyExists,
        #[error("Counter not found")]
        NotFound,
        #[error("Amount must be positive")]
        InvalidAmount,
        #[error("Counter would go negative")]
        WouldGoNegative,
    }

    #[derive(Default)]
    struct CounterAggregate {
        id: Option<String>,
        value: i32,
        version: i64,
        created: bool,
    }

    #[async_trait]
    impl Aggregate for CounterAggregate {
        type Command = CounterCommand;
        type Event = CounterEvent;
        type Error = CounterError;

        fn aggregate_type() -> &'static str {
            "Counter"
        }

        fn version(&self) -> i64 {
            self.version
        }

        async fn handle(&self, command: Self::Command) -> Result<Vec<Event>, Self::Error> {
            match command {
                CounterCommand::Create { id } => {
                    if self.created {
                        return Err(CounterError::AlreadyExists);
                    }

                    Ok(vec![Event::new(
                        "Counter",
                        &id,
                        self.version + 1,
                        "Created",
                        serde_json::json!({ "id": id }),
                    )])
                }
                CounterCommand::Increment { id, amount } => {
                    if !self.created {
                        return Err(CounterError::NotFound);
                    }

                    if amount <= 0 {
                        return Err(CounterError::InvalidAmount);
                    }

                    Ok(vec![Event::new(
                        "Counter",
                        &id,
                        self.version + 1,
                        "Incremented",
                        serde_json::json!({ "amount": amount }),
                    )])
                }
                CounterCommand::Decrement { id, amount } => {
                    if !self.created {
                        return Err(CounterError::NotFound);
                    }

                    if amount <= 0 {
                        return Err(CounterError::InvalidAmount);
                    }

                    if self.value - amount < 0 {
                        return Err(CounterError::WouldGoNegative);
                    }

                    Ok(vec![Event::new(
                        "Counter",
                        &id,
                        self.version + 1,
                        "Decremented",
                        serde_json::json!({ "amount": amount }),
                    )])
                }
            }
        }

        fn apply(&mut self, event: &Event) {
            self.version = event.sequence;

            match event.event_type.as_str() {
                "Created" => {
                    self.id = Some(event.aggregate_id.clone());
                    self.value = 0;
                    self.created = true;
                }
                "Incremented" => {
                    let amount = event.payload["amount"].as_i64().unwrap() as i32;
                    self.value += amount;
                }
                "Decremented" => {
                    let amount = event.payload["amount"].as_i64().unwrap() as i32;
                    self.value -= amount;
                }
                _ => {}
            }
        }
    }

    #[tokio::test]
    async fn test_command_aggregate_id() {
        let command = CounterCommand::Create {
            id: "counter-1".to_string(),
        };

        assert_eq!(command.aggregate_id(), "counter-1");
    }

    #[tokio::test]
    async fn test_aggregate_type() {
        assert_eq!(CounterAggregate::aggregate_type(), "Counter");
    }

    #[tokio::test]
    async fn test_create_counter() {
        let aggregate = CounterAggregate::default();

        let command = CounterCommand::Create {
            id: "counter-1".to_string(),
        };

        let events = aggregate.handle(command).await.unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "Created");
        assert_eq!(events[0].aggregate_id, "counter-1");
        assert_eq!(events[0].sequence, 1);
    }

    #[tokio::test]
    async fn test_cannot_create_twice() {
        let mut aggregate = CounterAggregate::default();
        let event = Event::new(
            "Counter",
            "counter-1",
            1,
            "Created",
            serde_json::json!({ "id": "counter-1" }),
        );
        aggregate.apply(&event);

        let command = CounterCommand::Create {
            id: "counter-1".to_string(),
        };

        let result = aggregate.handle(command).await;
        assert!(matches!(result, Err(CounterError::AlreadyExists)));
    }

    #[tokio::test]
    async fn test_increment_counter() {
        let mut aggregate = CounterAggregate::default();
        let event = Event::new(
            "Counter",
            "counter-1",
            1,
            "Created",
            serde_json::json!({ "id": "counter-1" }),
        );
        aggregate.apply(&event);

        let command = CounterCommand::Increment {
            id: "counter-1".to_string(),
            amount: 5,
        };

        let events = aggregate.handle(command).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "Incremented");
        assert_eq!(events[0].sequence, 2);
    }

    #[tokio::test]
    async fn test_increment_not_found() {
        let aggregate = CounterAggregate::default();

        let command = CounterCommand::Increment {
            id: "counter-1".to_string(),
            amount: 5,
        };

        let result = aggregate.handle(command).await;
        assert!(matches!(result, Err(CounterError::NotFound)));
    }

    #[tokio::test]
    async fn test_decrement_counter() {
        let mut aggregate = CounterAggregate::default();

        // Create and increment to 10
        let events = vec![
            Event::new(
                "Counter",
                "counter-1",
                1,
                "Created",
                serde_json::json!({ "id": "counter-1" }),
            ),
            Event::new(
                "Counter",
                "counter-1",
                2,
                "Incremented",
                serde_json::json!({ "amount": 10 }),
            ),
        ];

        for event in events {
            aggregate.apply(&event);
        }

        assert_eq!(aggregate.value, 10);

        // Decrement by 3
        let command = CounterCommand::Decrement {
            id: "counter-1".to_string(),
            amount: 3,
        };

        let events = aggregate.handle(command).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "Decremented");
    }

    #[tokio::test]
    async fn test_decrement_would_go_negative() {
        let mut aggregate = CounterAggregate::default();

        let events = vec![
            Event::new(
                "Counter",
                "counter-1",
                1,
                "Created",
                serde_json::json!({ "id": "counter-1" }),
            ),
            Event::new(
                "Counter",
                "counter-1",
                2,
                "Incremented",
                serde_json::json!({ "amount": 5 }),
            ),
        ];

        for event in events {
            aggregate.apply(&event);
        }

        assert_eq!(aggregate.value, 5);

        // Try to decrement by 10 (would go negative)
        let command = CounterCommand::Decrement {
            id: "counter-1".to_string(),
            amount: 10,
        };

        let result = aggregate.handle(command).await;
        assert!(matches!(result, Err(CounterError::WouldGoNegative)));
    }

    #[tokio::test]
    async fn test_from_events() {
        let events = vec![
            Event::new(
                "Counter",
                "counter-1",
                1,
                "Created",
                serde_json::json!({ "id": "counter-1" }),
            ),
            Event::new(
                "Counter",
                "counter-1",
                2,
                "Incremented",
                serde_json::json!({ "amount": 5 }),
            ),
            Event::new(
                "Counter",
                "counter-1",
                3,
                "Incremented",
                serde_json::json!({ "amount": 3 }),
            ),
            Event::new(
                "Counter",
                "counter-1",
                4,
                "Decremented",
                serde_json::json!({ "amount": 2 }),
            ),
        ];

        let aggregate = CounterAggregate::from_events(events);

        assert_eq!(aggregate.version, 4);
        assert_eq!(aggregate.value, 6); // 0 + 5 + 3 - 2
        assert!(aggregate.created);
    }

    #[tokio::test]
    async fn test_version_increments() {
        let aggregate = CounterAggregate::default();
        assert_eq!(aggregate.version(), 0);

        let mut aggregate = CounterAggregate::default();
        let event1 = Event::new(
            "Counter",
            "counter-1",
            1,
            "Created",
            serde_json::json!({ "id": "counter-1" }),
        );
        aggregate.apply(&event1);
        assert_eq!(aggregate.version(), 1);

        let event2 = Event::new(
            "Counter",
            "counter-1",
            2,
            "Incremented",
            serde_json::json!({ "amount": 5 }),
        );
        aggregate.apply(&event2);
        assert_eq!(aggregate.version(), 2);
    }

    #[tokio::test]
    async fn test_apply_unknown_event() {
        let mut aggregate = CounterAggregate::default();

        // Unknown event type should be silently ignored
        let event = Event::new(
            "Counter",
            "counter-1",
            1,
            "UnknownEvent",
            serde_json::json!({}),
        );

        aggregate.apply(&event);

        // Version still updates (this is important for correctness)
        assert_eq!(aggregate.version, 1);
        // But state is unchanged
        assert!(!aggregate.created);
        assert_eq!(aggregate.value, 0);
    }
}
