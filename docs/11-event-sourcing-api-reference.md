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
6. [Projection API](#projection-api) — [Projector](#projector-trait) | [ReadModelStore](#readmodelstore-trait) | [Projection](#projection-trait) | [ProjectionUnit](#projectionunit) | [ProjectionEngine](#projectionengine)
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

The projection system uses a three-trait architecture to separate concerns:

- **`Projector`** — stateless event handler containing pure transformation logic
- **`Projection`** — composed read model unit tying a projector to its storage
- **`ReadModelStore`** — backend-agnostic persistence layer for read models

`ProjectionUnit` is the standard glue struct that composes a `Projector` + `ReadModelStore` into a `Projection`.

### `Projector` Trait

Stateless event handler containing the pure logic for transforming events into read model writes. Projectors take `&self` (not `&mut self`) — all mutable state lives in the `ReadModelStore`.

**Location**: `nineties-core::projection::Projector`

**Definition**:
```rust
#[async_trait]
pub trait Projector: Send + Sync {
    /// Unique name identifying this projector.
    fn name(&self) -> &str;

    /// Event types this projector handles.
    fn handles(&self) -> Vec<String>;

    /// Apply a single event to the read model via the store.
    /// Must be idempotent.
    async fn apply(
        &self,
        event: &Event,
        store: &dyn ReadModelStore,
    ) -> ProjectionResult<()>;

    /// Initialize the read model schema (CREATE TABLE IF NOT EXISTS, etc.).
    /// Default implementation is a no-op.
    async fn init(&self, _store: &dyn ReadModelStore) -> ProjectionResult<()> {
        Ok(())
    }
}
```

**Methods**:

| Method | Signature | Description |
|--------|-----------|-------------|
| `name` | `fn name(&self) -> &str` | Unique name for logging, monitoring, and rebuild targeting |
| `handles` | `fn handles(&self) -> Vec<String>` | Event types this projector cares about |
| `apply` | `async fn apply(&self, event, store) -> ProjectionResult<()>` | Apply one event to the read model via the store. Must be idempotent |
| `init` | `async fn init(&self, store) -> ProjectionResult<()>` | Optional schema setup (default no-op) |

**Design Rules**:
- **Stateless**: all mutable state lives in the `ReadModelStore`
- **Deterministic**: same events + empty store = same read model
- **Idempotent**: handling the same event twice must produce the same result (use UPSERT, check event_id, etc.)
- **`&self`**: safe to share across threads

**Example**:
```rust
use nineties_core::projection::{Projector, ProjectionResult, ProjectionError};
use nineties_core::read_model_store::ReadModelStore;
use nineties_core::event::Event;

struct UserListProjector;

#[async_trait]
impl Projector for UserListProjector {
    fn name(&self) -> &str { "UserList" }

    fn handles(&self) -> Vec<String> {
        vec!["UserCreated".to_string(), "ProfileUpdated".to_string()]
    }

    async fn apply(&self, event: &Event, store: &dyn ReadModelStore) -> ProjectionResult<()> {
        match event.event_type.as_str() {
            "UserCreated" => {
                store.execute("users_view", vec![event.payload.clone()]).await
                    .map_err(|e| ProjectionError::handle_failed(
                        "UserList", &event.event_type,
                        &event.event_id.to_string(), e.to_string()
                    ))?;
            }
            _ => {}
        }
        Ok(())
    }
}
```

### `ReadModelStore` Trait

Backend-agnostic persistence layer for projection read models. Provides a uniform interface for projectors to persist and query materialized views.

**Location**: `nineties-core::read_model_store::ReadModelStore`

**Definition**:
```rust
#[async_trait]
pub trait ReadModelStore: Send + Sync {
    /// Execute a write operation (INSERT, UPDATE, DELETE).
    async fn execute(
        &self,
        sql: &str,
        params: Vec<serde_json::Value>,
    ) -> ReadModelResult<()>;

    /// Execute a query and return rows.
    async fn query(
        &self,
        sql: &str,
        params: Vec<serde_json::Value>,
    ) -> ReadModelResult<Vec<Row>>;

    /// Truncate/clear a table or collection.
    /// Used during projection rebuilds to wipe the read model before replay.
    async fn truncate(&self, table: &str) -> ReadModelResult<()>;
}
```

**Methods**:

| Method | Signature | Description |
|--------|-----------|-------------|
| `execute` | `async fn execute(&self, sql, params) -> ReadModelResult<()>` | Write operation (INSERT, UPDATE, DELETE) |
| `query` | `async fn query(&self, sql, params) -> ReadModelResult<Vec<Row>>` | Query returning rows as JSON values |
| `truncate` | `async fn truncate(&self, table) -> ReadModelResult<()>` | Clear a table; used during rebuilds |

**Thread Safety**: implementations must be `Send + Sync`. Interior mutability (connection pools, `Mutex`, etc.) is expected.

**Types**:
- `Row` — alias for `serde_json::Value`
- `ReadModelResult<T>` — alias for `Result<T, ReadModelError>`

#### `InMemoryReadModelStore`

Built-in in-memory implementation for testing and ephemeral projections. Ships with `nineties-core`.

**Location**: `nineties-core::read_model_store::InMemoryReadModelStore`

```rust
pub struct InMemoryReadModelStore {
    tables: Mutex<HashMap<String, Vec<Row>>>,
}

impl InMemoryReadModelStore {
    pub fn new() -> Self;

    /// Test helper: get all rows in a table.
    pub fn get_rows(&self, table: &str) -> Vec<Row>;

    /// Test helper: total row count across all tables.
    pub fn total_rows(&self) -> usize;
}
```

**Note**: In `InMemoryReadModelStore`, the `sql` parameter in `execute()` and `query()` is treated as the table name (no SQL parsing).

### `Projection` Trait

A composed read model unit representing a projector paired with its storage. All methods take `&self` — mutable state lives in the `ReadModelStore` via interior mutability.

Most users do not implement this trait directly. Instead, implement `Projector` and compose it with a `ReadModelStore` via `ProjectionUnit`.

**Location**: `nineties-core::projection::Projection`

**Definition**:
```rust
#[async_trait]
pub trait Projection: Send + Sync {
    /// Projection name (delegates to the projector).
    fn name(&self) -> &str;

    /// Event types this projection handles (delegates to the projector).
    fn handles(&self) -> Vec<String>;

    /// Handle a single event by applying it through the projector to the store.
    async fn handle(&self, event: &Event) -> ProjectionResult<()>;

    /// Clear all read model state for this projection.
    async fn clear(&self) -> ProjectionResult<()>;

    /// Rebuild from a set of events: clear, then replay matching events.
    /// Default implementation calls clear() then handle() for each matching event.
    async fn rebuild(&self, events: Vec<Event>) -> ProjectionResult<()>;
}
```

**Methods**:

| Method | Signature | Description |
|--------|-----------|-------------|
| `name` | `fn name(&self) -> &str` | Delegates to the projector's name |
| `handles` | `fn handles(&self) -> Vec<String>` | Delegates to the projector's event types |
| `handle` | `async fn handle(&self, event) -> ProjectionResult<()>` | Route one event through projector to store |
| `clear` | `async fn clear(&self) -> ProjectionResult<()>` | Wipe the read model (truncate) |
| `rebuild` | `async fn rebuild(&self, events) -> ProjectionResult<()>` | Clear + replay matching events (has default impl) |

### `ProjectionUnit`

Standard composition glue that wires a `Projector` + `Arc<dyn ReadModelStore>` + table name into a `Projection`.

**Location**: `nineties-core::projection::ProjectionUnit`

**Definition**:
```rust
pub struct ProjectionUnit {
    projector: Box<dyn Projector>,
    store: Arc<dyn ReadModelStore>,
    table: String,
}

impl ProjectionUnit {
    pub fn new(
        projector: Box<dyn Projector>,
        store: Arc<dyn ReadModelStore>,
        table: impl Into<String>,
    ) -> Self;
}
```

**Parameters**:
- `projector` — the stateless event handler
- `store` — the read model storage backend (shared via `Arc`)
- `table` — table/collection name to truncate on `clear()`

`ProjectionUnit` implements `Projection` by:
- Delegating `name()` and `handles()` to the projector
- Calling `projector.apply(event, store)` in `handle()`
- Calling `store.truncate(table)` in `clear()`

**Example**:
```rust
use nineties_core::projection::{ProjectionUnit, Projection};
use nineties_core::read_model_store::InMemoryReadModelStore;
use std::sync::Arc;

let store = Arc::new(InMemoryReadModelStore::new());
let projection = ProjectionUnit::new(
    Box::new(UserListProjector),
    store,
    "users_view",
);

// projection implements Projection
projection.handle(&event).await?;
```

### `ProjectionEngine`

Manages multiple projections. Routes events to interested projections, handles rebuilds from the event store. All methods take `&self` (not `&mut self`).

**Location**: `nineties-core::projection::ProjectionEngine`

**Definition**:
```rust
pub struct ProjectionEngine {
    projections: Vec<Box<dyn Projection>>,
    event_store: Box<dyn EventStore>,
}

impl ProjectionEngine {
    pub fn new(event_store: Box<dyn EventStore>) -> Self;

    /// Register a fully composed projection.
    pub fn register(&mut self, projection: Box<dyn Projection>);

    /// Convenience: register a projector + store as a ProjectionUnit.
    pub fn register_projector(
        &mut self,
        projector: Box<dyn Projector>,
        store: Arc<dyn ReadModelStore>,
        table: impl Into<String>,
    );

    /// Process a single event through all interested projections.
    pub async fn process(&self, event: &Event) -> ProjectionResult<()>;

    /// Process multiple events in sequence.
    pub async fn process_batch(&self, events: Vec<Event>) -> ProjectionResult<()>;

    /// Rebuild all registered projections from the event store.
    pub async fn rebuild_all(&self) -> ProjectionResult<()>;

    /// Rebuild a specific projection by name.
    pub async fn rebuild_projection(&self, name: &str) -> ProjectionResult<()>;

    /// Get number of registered projections.
    pub fn projection_count(&self) -> usize;

    /// Get names of all registered projections.
    pub fn projection_names(&self) -> Vec<String>;
}
```

**Methods**:

| Method | Description |
|--------|-------------|
| `register` | Register a pre-composed `Projection` |
| `register_projector` | Convenience: wraps a `Projector` + store into a `ProjectionUnit` and registers it |
| `process` | Route one event to matching projections |
| `process_batch` | Process a vec of events in sequence |
| `rebuild_all` | Load all events from the event store and rebuild every projection |
| `rebuild_projection` | Rebuild a single projection by name |

**Example**:
```rust
use nineties_core::projection::{ProjectionEngine, ProjectionUnit};
use nineties_core::read_model_store::InMemoryReadModelStore;
use std::sync::Arc;

let mut engine = ProjectionEngine::new(event_store);

// Option 1: register a pre-composed projection
let store = Arc::new(InMemoryReadModelStore::new());
let projection = ProjectionUnit::new(Box::new(UserListProjector), store.clone(), "users_view");
engine.register(Box::new(projection));

// Option 2: convenience — register projector + store directly
engine.register_projector(Box::new(UserListProjector), store, "users_view");

// Process events
engine.process(&event).await?;

// Rebuild all projections from event store
engine.rebuild_all().await?;
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
    HandleFailed { name: String, event_type: String, event_id: String, message: String },
    ClearFailed { name: String, message: String },
    RebuildFailed { name: String, message: String },
    EventStoreError(String),
    ReadModelError { name: String, message: String },
    Other { message: String },
}

pub enum ReadModelError {
    WriteFailed { message: String },
    QueryFailed { message: String },
    SchemaFailed { message: String },
    Other { message: String },
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

### Example 3: Building a Projection (Three-Trait Pattern)

```rust
use nineties_core::projection::{
    Projector, Projection, ProjectionUnit, ProjectionEngine,
    ProjectionResult, ProjectionError,
};
use nineties_core::read_model_store::{ReadModelStore, InMemoryReadModelStore};
use nineties_core::event::Event;
use std::sync::Arc;

// Step 1: Implement a Projector (stateless event handler)
struct UserListProjector;

#[async_trait]
impl Projector for UserListProjector {
    fn name(&self) -> &str { "UserList" }

    fn handles(&self) -> Vec<String> {
        vec!["UserCreated".to_string(), "ProfileUpdated".to_string()]
    }

    async fn apply(&self, event: &Event, store: &dyn ReadModelStore) -> ProjectionResult<()> {
        match event.event_type.as_str() {
            "UserCreated" => {
                store.execute(
                    "INSERT OR REPLACE INTO users_view (id, name, email) VALUES (?1, ?2, ?3)",
                    vec![
                        serde_json::json!(event.aggregate_id),
                        event.payload["name"].clone(),
                        event.payload["email"].clone(),
                    ],
                ).await.map_err(|e| ProjectionError::handle_failed(
                    "UserList", &event.event_type,
                    &event.event_id.to_string(), e.to_string(),
                ))?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn init(&self, store: &dyn ReadModelStore) -> ProjectionResult<()> {
        store.execute(
            "CREATE TABLE IF NOT EXISTS users_view (id TEXT PRIMARY KEY, name TEXT, email TEXT)",
            vec![],
        ).await.map_err(|e| ProjectionError::other(e.to_string()))?;
        Ok(())
    }
}

// Step 2: Compose projector + store = projection
let store: Arc<dyn ReadModelStore> = Arc::new(InMemoryReadModelStore::new());
let projection = ProjectionUnit::new(Box::new(UserListProjector), store.clone(), "users_view");

// Step 3: Register with engine
let mut engine = ProjectionEngine::new(event_store);
engine.register(Box::new(projection));

// Or use the convenience method:
// engine.register_projector(Box::new(UserListProjector), store, "users_view");

// Process events
engine.process(&event).await?;

// Rebuild all projections from scratch
engine.rebuild_all().await?;
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
