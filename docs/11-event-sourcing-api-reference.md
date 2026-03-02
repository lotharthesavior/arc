# Event Sourcing API Reference

> **Status**: Template - To be filled as code is implemented
>
> **Last Updated**: 2026-03-01
>
> **Related Docs**: [Architecture](09-event-sourcing-architecture.md) | [Implementation Guide](10-event-sourcing-implementation-guide.md)

---

## Table of Contents

1. [Core Traits](#core-traits)
2. [Event Types](#event-types)
3. [EventStore API](#eventstore-api)
4. [EventBus API](#eventbus-api)
5. [Aggregate API](#aggregate-api)
6. [Projection API](#projection-api)
7. [CommandBus API](#commandbus-api)
8. [Error Types](#error-types)
9. [Usage Examples](#usage-examples)

---

## Core Traits

### Overview

The `nineties-core` crate provides trait-based abstractions for event sourcing patterns. All traits are async-compatible and designed for composability.

**Key Design Principles:**
- Traits over concrete types for maximum flexibility
- Async-first API using `async-trait`
- Zero-cost abstractions where possible
- Backend-agnostic design

---

## Event Types

### `Event`

The fundamental event type representing a domain event in the system.

**Location**: `nineties-core::event::Event`

**Definition**:
```rust
// PLACEHOLDER: To be filled when implementation is complete
pub struct Event {
    pub event_id: Uuid,
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub sequence: i64,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub metadata: serde_json::Value,
    pub timestamp: SystemTime,
}
```

**Fields**:

| Field | Type | Description |
|-------|------|-------------|
| `event_id` | `Uuid` | Unique identifier for this event |
| `aggregate_type` | `String` | Type of aggregate (e.g., "User", "Order") |
| `aggregate_id` | `String` | Instance ID of the aggregate |
| `sequence` | `i64` | Version number within aggregate stream |
| `event_type` | `String` | Event type name (e.g., "UserCreated") |
| `payload` | `Value` | JSON payload containing event data |
| `metadata` | `Value` | Metadata (causation_id, correlation_id, user_id, etc.) |
| `timestamp` | `SystemTime` | When the event occurred |

**Constructor**:
```rust
impl Event {
    pub fn new(
        aggregate_type: impl Into<String>,
        aggregate_id: impl Into<String>,
        sequence: i64,
        event_type: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self

    // PLACEHOLDER: Additional constructors will be documented here
}
```

**Example**:
```rust
// PLACEHOLDER: To be filled with real implementation example
use nineties_core::Event;
use serde_json::json;

let event = Event::new(
    "User",
    "user-123",
    1,
    "UserCreated",
    json!({
        "name": "Jane Doe",
        "email": "jane@example.com"
    })
);
```

---

## EventStore API

### `EventStore` Trait

Defines the contract for persisting and retrieving events.

**Location**: `nineties-core::event_store::EventStore`

**Definition**:
```rust
// PLACEHOLDER: To be filled when implementation is complete
#[async_trait]
pub trait EventStore: Send + Sync {
    async fn append(
        &self,
        aggregate_id: &str,
        expected_version: Option<i64>,
        events: Vec<Event>,
    ) -> Result<(), Box<dyn Error>>;

    async fn load(&self, aggregate_id: &str) -> Result<Vec<Event>, Box<dyn Error>>;

    async fn load_from(
        &self,
        aggregate_id: &str,
        from_sequence: i64,
    ) -> Result<Vec<Event>, Box<dyn Error>>;

    async fn stream_all(
        &self,
        from_position: i64,
    ) -> Result<Vec<Event>, Box<dyn Error>>;
}
```

### Methods

#### `append`

Appends events to an aggregate's event stream with optimistic concurrency control.

**Signature**:
```rust
async fn append(
    &self,
    aggregate_id: &str,
    expected_version: Option<i64>,
    events: Vec<Event>,
) -> Result<(), Box<dyn Error>>
```

**Parameters**:
- `aggregate_id` - The aggregate instance identifier
- `expected_version` - Expected current version (for optimistic locking), `None` for new aggregates
- `events` - Events to append

**Returns**: `Ok(())` on success, error on version conflict or storage failure

**Errors**:
- `ConcurrencyError` - When `expected_version` doesn't match current version
- `StorageError` - Database or storage backend error

**Example**:
```rust
// PLACEHOLDER: To be filled with real implementation example
let events = vec![
    Event::new("User", "user-123", 1, "UserCreated", payload)
];

event_store.append("user-123", None, events).await?;
```

#### `load`

Loads all events for an aggregate from the beginning.

**Signature**:
```rust
async fn load(&self, aggregate_id: &str) -> Result<Vec<Event>, Box<dyn Error>>
```

**Parameters**:
- `aggregate_id` - The aggregate instance identifier

**Returns**: Vector of events in sequence order

**Example**:
```rust
// PLACEHOLDER: To be filled with real implementation example
let events = event_store.load("user-123").await?;
let user = UserAggregate::from_events(events);
```

#### `load_from`

Loads events starting from a specific sequence number.

**Signature**:
```rust
async fn load_from(
    &self,
    aggregate_id: &str,
    from_sequence: i64,
) -> Result<Vec<Event>, Box<dyn Error>>
```

**Use Case**: Loading events after a snapshot

**Example**:
```rust
// PLACEHOLDER: To be filled with real implementation example
// Load snapshot at version 100, then load remaining events
let snapshot = snapshot_store.load("user-123").await?;
let remaining_events = event_store.load_from("user-123", snapshot.version + 1).await?;
```

#### `stream_all`

Streams all events across all aggregates from a global position.

**Signature**:
```rust
async fn stream_all(
    &self,
    from_position: i64,
) -> Result<Vec<Event>, Box<dyn Error>>
```

**Use Case**: Projection rebuilds, catching up read models

**Example**:
```rust
// PLACEHOLDER: To be filled with real implementation example
let all_events = event_store.stream_all(0).await?;
for event in all_events {
    projection.handle(&event).await?;
}
```

### Implementations

#### `SqliteEventStore`

**Location**: `nineties-es-sqlite::SqliteEventStore`

SQLite-based implementation of `EventStore`.

**Constructor**:
```rust
// PLACEHOLDER: To be documented when implemented
impl SqliteEventStore {
    pub fn new(db_path: &str) -> Result<Self, Error>
    pub fn from_pool(pool: Pool) -> Self
}
```

**Database Schema**:
```sql
-- PLACEHOLDER: Schema will be documented here
CREATE TABLE events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id TEXT NOT NULL UNIQUE,
    aggregate_type TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    sequence INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    metadata TEXT DEFAULT '{}',
    timestamp INTEGER NOT NULL,
    UNIQUE(aggregate_id, sequence)
);
```

---

## EventBus API

### `EventBus` Trait

Publishes events to subscribers for asynchronous processing.

**Location**: `nineties-core::event_bus::EventBus`

**Definition**:
```rust
// PLACEHOLDER: To be filled when implementation is complete
#[async_trait]
pub trait EventBus: Send + Sync {
    async fn publish(&self, events: Vec<Event>) -> Result<(), Box<dyn Error>>;
    async fn subscribe(&mut self, handler: Box<dyn EventHandler>);
}
```

### `EventHandler` Trait

Interface for event subscribers.

**Definition**:
```rust
// PLACEHOLDER: To be filled when implementation is complete
#[async_trait]
pub trait EventHandler: Send + Sync {
    async fn handle(&self, event: &Event) -> Result<(), Box<dyn Error>>;
    fn handles(&self) -> Vec<String>;
}
```

**Methods**:

#### `publish`

Publishes events to all registered subscribers.

**Signature**:
```rust
async fn publish(&self, events: Vec<Event>) -> Result<(), Box<dyn Error>>
```

**Example**:
```rust
// PLACEHOLDER: To be filled with real implementation example
let events = vec![event];
event_bus.publish(events).await?;
```

#### `subscribe`

Registers an event handler.

**Signature**:
```rust
async fn subscribe(&mut self, handler: Box<dyn EventHandler>)
```

**Example**:
```rust
// PLACEHOLDER: To be filled with real implementation example
struct EmailNotificationHandler;

#[async_trait]
impl EventHandler for EmailNotificationHandler {
    fn handles(&self) -> Vec<String> {
        vec!["UserCreated".to_string()]
    }

    async fn handle(&self, event: &Event) -> Result<(), Box<dyn Error>> {
        // Send welcome email
        Ok(())
    }
}

event_bus.subscribe(Box::new(EmailNotificationHandler)).await;
```

### Implementations

#### `InProcessEventBus`

Synchronous in-process event bus.

**Use Case**: Single-node deployments, testing

#### `ChannelEventBus`

Async event bus using Tokio channels.

**Use Case**: Background processing, async handlers

---

## Aggregate API

### `Aggregate` Trait

Defines domain aggregate with command handling and event application.

**Location**: `nineties-core::aggregate::Aggregate`

**Definition**:
```rust
// PLACEHOLDER: To be filled when implementation is complete
#[async_trait]
pub trait Aggregate: Send + Sync + Default {
    type Command: Command;
    type Event;
    type Error: Error + Send + Sync + 'static;

    fn aggregate_type() -> &'static str;
    fn version(&self) -> i64;

    async fn handle(&self, command: Self::Command) -> Result<Vec<Event>, Self::Error>;
    fn apply(&mut self, event: &Event);

    fn from_events(events: Vec<Event>) -> Self {
        // Default implementation
    }
}
```

### `Command` Trait

Marker trait for commands.

**Definition**:
```rust
// PLACEHOLDER: To be filled when implementation is complete
pub trait Command: Send + Sync {
    fn aggregate_id(&self) -> &str;
}
```

### Methods

#### `handle`

Process a command and produce events.

**Rules**:
- MUST validate business rules
- MUST be side-effect free
- MUST return events or error
- MUST NOT modify aggregate state (use `apply` for that)

**Example**:
```rust
// PLACEHOLDER: To be filled with real implementation example
async fn handle(&self, command: Self::Command) -> Result<Vec<Event>, Self::Error> {
    match command {
        UserCommand::CreateUser { id, name, email } => {
            if self.created {
                return Err(UserError::AlreadyExists);
            }

            Ok(vec![Event::new(
                "User",
                &id,
                self.version + 1,
                "UserCreated",
                json!({ "name": name, "email": email })
            )])
        }
    }
}
```

#### `apply`

Apply an event to update aggregate state.

**Rules**:
- MUST be deterministic
- MUST be side-effect free
- MUST update state based on event data only

**Example**:
```rust
// PLACEHOLDER: To be filled with real implementation example
fn apply(&mut self, event: &Event) {
    self.version = event.sequence;

    match event.event_type.as_str() {
        "UserCreated" => {
            let data: UserEvent = serde_json::from_value(event.payload.clone()).unwrap();
            self.id = Some(data.id);
            self.name = Some(data.name);
            self.created = true;
        }
        _ => {}
    }
}
```

---

## Projection API

### `Projection` Trait

Builds read models from event streams.

**Location**: `nineties-core::projection::Projection`

**Definition**:
```rust
// PLACEHOLDER: To be filled when implementation is complete
#[async_trait]
pub trait Projection: Send + Sync {
    fn name(&self) -> &str;
    fn handles(&self) -> Vec<String>;

    async fn handle(&mut self, event: &Event) -> Result<(), Box<dyn Error>>;
    async fn rebuild(&mut self, events: Vec<Event>) -> Result<(), Box<dyn Error>>;
    async fn clear(&mut self) -> Result<(), Box<dyn Error>>;
}
```

### `ProjectionEngine`

Manages multiple projections.

**Definition**:
```rust
// PLACEHOLDER: To be filled when implementation is complete
pub struct ProjectionEngine {
    // Internal fields
}

impl ProjectionEngine {
    pub fn new(event_store: Box<dyn EventStore>) -> Self;
    pub fn register(&mut self, projection: Box<dyn Projection>);
    pub async fn process(&mut self, event: &Event) -> Result<(), Box<dyn Error>>;
    pub async fn rebuild_all(&mut self) -> Result<(), Box<dyn Error>>;
}
```

### Methods

#### `handle`

Process a single event to update the read model.

**Example**:
```rust
// PLACEHOLDER: To be filled with real implementation example
async fn handle(&mut self, event: &Event) -> Result<(), Box<dyn Error>> {
    match event.event_type.as_str() {
        "UserCreated" => {
            // Insert into users_view table
            diesel::sql_query("INSERT INTO users_view ...")
                .execute(&mut self.conn)?;
        }
        _ => {}
    }
    Ok(())
}
```

#### `rebuild`

Rebuild projection from scratch by replaying all events.

**Use Cases**:
- Schema changes
- Bug fixes in projection logic
- Adding new projections

**Example**:
```rust
// PLACEHOLDER: To be filled with real implementation example
let engine = projection_engine.rebuild_all().await?;
```

---

## CommandBus API

### `CommandBus`

Orchestrates command handling with aggregates, event store, and event bus.

**Location**: `nineties-core::command_bus::CommandBus`

**Definition**:
```rust
// PLACEHOLDER: To be filled when implementation is complete
pub struct CommandBus<A: Aggregate> {
    // Internal fields
}

impl<A: Aggregate> CommandBus<A> {
    pub fn new(
        event_store: Box<dyn EventStore>,
        event_bus: Box<dyn EventBus>,
    ) -> Self;

    pub async fn dispatch(&mut self, command: A::Command) -> Result<Vec<Event>, Box<dyn Error>>;
}
```

### Workflow

1. Load aggregate events from EventStore
2. Reconstruct aggregate state
3. Handle command (produces events)
4. Append events to EventStore (with optimistic concurrency)
5. Publish events to EventBus

**Example**:
```rust
// PLACEHOLDER: To be filled with real implementation example
let mut command_bus = CommandBus::<UserAggregate>::new(event_store, event_bus);

let command = UserCommand::CreateUser {
    id: "user-123".to_string(),
    name: "Jane Doe".to_string(),
    email: "jane@example.com".to_string(),
};

let events = command_bus.dispatch(command).await?;
```

---

## Error Types

### Core Errors

**PLACEHOLDER**: Error types will be documented as they are implemented

```rust
// nineties-core::error
pub enum EventStoreError {
    ConcurrencyError { expected: i64, actual: i64 },
    StorageError(String),
    SerializationError(String),
}

pub enum ProjectionError {
    EventHandlingError(String),
    RebuildError(String),
}
```

---

## Usage Examples

### Example 1: Simple Path - Direct Event Emission

For simple use cases where full aggregate complexity isn't needed.

```rust
// PLACEHOLDER: To be filled with real implementation example

// In controller
#[post("/users")]
async fn create_user(
    form: web::Form<UserForm>,
    event_store: web::Data<Arc<EventStore>>,
    event_bus: web::Data<Arc<EventBus>>,
) -> impl Responder {
    // Validate
    if form.email.is_empty() {
        return HttpResponse::BadRequest().json(json!({"error": "Email required"}));
    }

    // Create event directly
    let event = Event::new(
        "User",
        &Uuid::new_v4().to_string(),
        1,
        "UserCreated",
        json!({
            "name": form.name,
            "email": form.email,
        })
    );

    // Append and publish
    event_store.append(&event.aggregate_id, None, vec![event.clone()]).await?;
    event_bus.publish(vec![event]).await?;

    HttpResponse::Created().json(json!({"success": true}))
}
```

### Example 2: Complex Path - Full Aggregate with CommandBus

For domains requiring strong business rules and consistency.

```rust
// PLACEHOLDER: To be filled with real implementation example

// In controller
#[post("/users")]
async fn create_user(
    form: web::Form<UserForm>,
    command_bus: web::Data<Arc<Mutex<CommandBus<UserAggregate>>>>,
) -> impl Responder {
    let command = UserCommand::CreateUser {
        id: Uuid::new_v4().to_string(),
        name: form.name.clone(),
        email: form.email.clone(),
        password: form.password.clone(),
    };

    match command_bus.lock().await.dispatch(command).await {
        Ok(events) => HttpResponse::Created().json(json!({
            "id": events[0].aggregate_id,
            "success": true
        })),
        Err(e) => HttpResponse::BadRequest().json(json!({
            "error": e.to_string()
        }))
    }
}

// Domain layer
#[derive(Default)]
pub struct UserAggregate {
    id: Option<String>,
    email: Option<String>,
    created: bool,
    version: i64,
}

#[async_trait]
impl Aggregate for UserAggregate {
    type Command = UserCommand;
    type Event = UserEvent;
    type Error = UserError;

    fn aggregate_type() -> &'static str { "User" }
    fn version(&self) -> i64 { self.version }

    async fn handle(&self, command: Self::Command) -> Result<Vec<Event>, Self::Error> {
        match command {
            UserCommand::CreateUser { id, name, email, password } => {
                // Business rules
                if self.created {
                    return Err(UserError::AlreadyExists);
                }

                if !email.contains('@') {
                    return Err(UserError::InvalidEmail);
                }

                // Produce event
                Ok(vec![Event::new(
                    "User",
                    &id,
                    1,
                    "UserCreated",
                    json!({ "name": name, "email": email })
                )])
            }
        }
    }

    fn apply(&mut self, event: &Event) {
        self.version = event.sequence;
        match event.event_type.as_str() {
            "UserCreated" => {
                self.id = Some(event.aggregate_id.clone());
                self.created = true;
            }
            _ => {}
        }
    }
}
```

### Example 3: Building a Projection

```rust
// PLACEHOLDER: To be filled with real implementation example

pub struct UserListProjection {
    pool: Pool<ConnectionManager<SqliteConnection>>,
}

#[async_trait]
impl Projection for UserListProjection {
    fn name(&self) -> &str { "UserList" }

    fn handles(&self) -> Vec<String> {
        vec!["UserCreated".to_string(), "ProfileUpdated".to_string()]
    }

    async fn handle(&mut self, event: &Event) -> Result<(), Box<dyn Error>> {
        let mut conn = self.pool.get()?;

        match event.event_type.as_str() {
            "UserCreated" => {
                diesel::sql_query(
                    "INSERT INTO users_view (id, name, email) VALUES (?1, ?2, ?3)"
                )
                .bind::<Text, _>(&event.aggregate_id)
                .bind::<Text, _>(event.payload["name"].as_str().unwrap())
                .bind::<Text, _>(event.payload["email"].as_str().unwrap())
                .execute(&mut conn)?;
            }
            _ => {}
        }

        Ok(())
    }

    async fn clear(&mut self) -> Result<(), Box<dyn Error>> {
        let mut conn = self.pool.get()?;
        diesel::sql_query("DELETE FROM users_view").execute(&mut conn)?;
        Ok(())
    }
}
```

### Example 4: Testing with Event Sourcing

```rust
// PLACEHOLDER: To be filled with real implementation example

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_user_creation() {
        // Arrange
        let event_store = Box::new(InMemoryEventStore::new());
        let event_bus = Box::new(InProcessEventBus::new());
        let mut command_bus = CommandBus::<UserAggregate>::new(event_store, event_bus);

        // Act
        let command = UserCommand::CreateUser {
            id: "user-123".to_string(),
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };

        let events = command_bus.dispatch(command).await.unwrap();

        // Assert
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "UserCreated");
        assert_eq!(events[0].aggregate_id, "user-123");
    }

    #[tokio::test]
    async fn test_duplicate_user_fails() {
        // Arrange
        let event_store = Box::new(InMemoryEventStore::new());
        let event_bus = Box::new(InProcessEventBus::new());
        let mut command_bus = CommandBus::<UserAggregate>::new(event_store, event_bus);

        let command = UserCommand::CreateUser {
            id: "user-123".to_string(),
            name: "Test User".to_string(),
            email: "test@example.com".to_string(),
            password: "password123".to_string(),
        };

        // Act - first creation
        command_bus.dispatch(command.clone()).await.unwrap();

        // Act - second creation (should fail)
        let result = command_bus.dispatch(command).await;

        // Assert
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), UserError::AlreadyExists));
    }
}
```

---

## Advanced Topics

### Optimistic Concurrency Control

**PLACEHOLDER**: To be documented with implementation

### Snapshotting

**PLACEHOLDER**: To be documented with implementation

### Event Upcasting

**PLACEHOLDER**: To be documented with implementation

### Saga / Process Managers

**PLACEHOLDER**: To be documented with implementation

---

## Migration from Traditional CRUD

See [Migration Guide](#migration-guide) section below.

---

## Performance Considerations

**PLACEHOLDER**: Benchmarks and optimization guides will be added here

---

## Related Documentation

- [Event Sourcing Architecture](09-event-sourcing-architecture.md)
- [Implementation Guide](10-event-sourcing-implementation-guide.md)
- [Nineties Core README](../crates/nineties-core/README.md)
- [SQLite EventStore README](../crates/nineties-es-sqlite/README.md)

---

**Note**: This is a living document. As the event sourcing library is implemented, this API reference will be updated with actual code examples, performance benchmarks, and complete usage patterns.
