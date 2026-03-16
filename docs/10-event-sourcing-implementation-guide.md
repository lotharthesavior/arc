# Event Sourcing Implementation Guide

> **Prepared by**: Software Architecture Team (Event Sourcing Specialist + Documentation Specialist + Source Code Specialist)
>
> **Date**: 2026-02-27
>
> **Status**: Ready for Implementation

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Current State Analysis](#current-state-analysis)
3. [Implementation Phases](#implementation-phases)
4. [Technical Requirements](#technical-requirements)
5. [Code Structure Changes](#code-structure-changes)
6. [Migration Strategy](#migration-strategy)
7. [Testing Strategy](#testing-strategy)
8. [Rollback Plan](#rollback-plan)

---

## Prerequisites

### Knowledge Requirements

**Team members should understand:**
- Event Sourcing and CQRS patterns
- Rust ownership, lifetimes, and async programming
- Actix-Web framework
- Diesel ORM (current) and its limitations
- SQLite (current) and migration to event store

**Recommended Reading:**
- [Event Sourcing Pattern](https://martinfowler.com/eaaDev/EventSourcing.html)
- [CQRS Pattern](https://martinfowler.com/bliki/CQRS.html)
- [Rust Async Book](https://rust-lang.github.io/async-book/)
- `docs/09-event-sourcing-architecture.md` - Our architecture plan

### Technical Prerequisites

**Before starting implementation:**
- [x] Comprehensive roadmap created (`docs/roadmap.md`)
- [x] Logging infrastructure with tracing
- [x] Input validation framework
- [ ] Pre-commit hooks setup
- [ ] CI/CD pipeline established
- [ ] Full test coverage of current features (baseline)

### Environment Setup

**Required tools:**
```bash
# Rust toolchain
rustup update stable
rustup component add rustfmt clippy

# Testing tools
cargo install cargo-tarpaulin  # Code coverage
cargo install cargo-watch      # Auto-rebuild

# Database tools (current)
sqlite3 --version

# Documentation
npm install  # For docsify-cli
```

---

## Current State Analysis

### Existing Architecture (MVC)

**Current Structure:**
```
src/
├── main.rs                    # Entry point
├── routes.rs                  # Route configuration
├── http/
│   ├── controllers/          # Request handlers
│   ├── middlewares/          # Auth, JWT middleware
├── models/                    # Diesel models
├── services/                  # Business logic
├── helpers/                   # Utilities
├── schema.rs                 # Diesel schema
└── database/
    └── seeders/              # Test data
```

**Key Components to Migrate:**

| Component | Current | Event Sourcing Target |
|-----------|---------|----------------------|
| State Storage | Diesel ORM (mutable) | Event Store (append-only) |
| Business Logic | Services (CRUD) | Aggregates (Commands) |
| Read Operations | Direct DB queries | Projections (Read Models) |
| Async Operations | None | Event Bus subscribers |
| History | None | Full event replay |

### Current Database Schema

**`users` table** (from `src/schema.rs`):
```rust
diesel::table! {
    users (id) {
        id -> Integer,
        name -> Text,
        email -> Text,
        password -> Text,
        created_at -> Nullable<Timestamp>,
        updated_at -> Nullable<Timestamp>,
    }
}
```

This will become:
1. **Event Store**: `events` table (append-only log)
2. **Read Model**: `users_view` table (projection)

### Current User Operations

**From `src/services/user_service.rs`:**
```rust
// Current: Direct mutation
pub fn validate_user_credentials(user_email: &str, user_password: &str) -> UserValidationResult {
    let conn = &mut get_connection();
    let user = users.filter(email.eq(&user_email)).load::<User>(conn)?;
    // ... validation logic
}
```

**Target: Command → Event → Projection**
```rust
// Step 1: Command
ValidateUserCredentials { email, password }
    ↓
// Step 2: Event (if valid)
UserCredentialsValidated { user_id, timestamp }
    ↓
// Step 3: Query read model
users_view.filter(email.eq(&user_email)).first()
```

---

## Implementation Phases

### Phase 1: Foundation (Weeks 1-4)

**Goal**: Create core event sourcing primitives

#### Week 1: Workspace Setup

**Tasks:**
1. Create Cargo workspace structure
2. Create `nineties-core` crate
3. Setup crate dependencies
4. Configure feature flags

**Deliverables:**
```
nineties/
├── Cargo.toml              # Workspace manifest
├── crates/
│   ├── nineties-core/     # ES primitives
│   ├── nineties-app/      # Main application
│   └── nineties-web/      # Web layer (future)
```

**Cargo.toml (workspace):**
```toml
[workspace]
members = [
    "crates/nineties-core",
    "crates/nineties-app",
]
resolver = "2"

[workspace.dependencies]
actix-web = "4"
diesel = { version = "2.2.6", features = ["sqlite"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
uuid = { version = "1.0", features = ["v4", "serde"] }
```

#### Week 2: Event Store Trait & Types

**Tasks:**
1. Define `Event` type
2. Define `EventStore` trait
3. Implement SQLite EventStore
4. Add unit tests

**Code to Create:**

`crates/nineties-core/src/event.rs`:
```rust
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// Unique event identifier
    pub event_id: Uuid,

    /// Aggregate type (e.g., "User", "Order")
    pub aggregate_type: String,

    /// Aggregate instance ID
    pub aggregate_id: String,

    /// Sequence number within aggregate
    pub sequence: i64,

    /// Event type (e.g., "UserCreated", "UserUpdated")
    pub event_type: String,

    /// Event payload (JSON)
    pub payload: serde_json::Value,

    /// Metadata (causation_id, correlation_id, user_id, etc.)
    pub metadata: serde_json::Value,

    /// When event occurred
    pub timestamp: SystemTime,
}

impl Event {
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
            timestamp: SystemTime::now(),
        }
    }
}
```

`crates/nineties-core/src/event_store.rs`:
```rust
use crate::event::Event;
use async_trait::async_trait;
use std::error::Error;

#[async_trait]
pub trait EventStore: Send + Sync {
    /// Append events to the store
    /// Returns error if expected_version doesn't match (optimistic concurrency)
    async fn append(
        &self,
        aggregate_id: &str,
        expected_version: Option<i64>,
        events: Vec<Event>,
    ) -> Result<(), Box<dyn Error>>;

    /// Load all events for an aggregate
    async fn load(&self, aggregate_id: &str) -> Result<Vec<Event>, Box<dyn Error>>;

    /// Load events from a specific sequence number
    async fn load_from(
        &self,
        aggregate_id: &str,
        from_sequence: i64,
    ) -> Result<Vec<Event>, Box<dyn Error>>;

    /// Stream all events (for projections)
    async fn stream_all(
        &self,
        from_position: i64,
    ) -> Result<Vec<Event>, Box<dyn Error>>;
}
```

**SQLite Schema:**
```sql
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

CREATE INDEX idx_events_aggregate ON events(aggregate_id, sequence);
CREATE INDEX idx_events_type ON events(event_type);
CREATE INDEX idx_events_timestamp ON events(timestamp);
CREATE INDEX idx_events_id ON events(id);
```

#### Week 3: Event Bus

**Tasks:**
1. Define `EventBus` trait
2. Implement in-process event bus
3. Define `EventHandler` trait
4. Add tests

**Code to Create:**

`crates/nineties-core/src/event_bus.rs`:
```rust
use crate::event::Event;
use async_trait::async_trait;
use std::error::Error;

#[async_trait]
pub trait EventHandler: Send + Sync {
    /// Handle an event
    async fn handle(&self, event: &Event) -> Result<(), Box<dyn Error>>;

    /// Event types this handler subscribes to
    fn handles(&self) -> Vec<String>;
}

#[async_trait]
pub trait EventBus: Send + Sync {
    /// Publish events to all subscribers
    async fn publish(&self, events: Vec<Event>) -> Result<(), Box<dyn Error>>;

    /// Subscribe a handler
    async fn subscribe(&mut self, handler: Box<dyn EventHandler>);
}
```

#### Week 4: Projections

**Tasks:**
1. Define three-trait projection architecture:
   - `Projector` trait (stateless event handler)
   - `Projection` trait (composed read model unit)
   - `ReadModelStore` trait (persistence layer)
2. Implement `ProjectionUnit` (glue struct)
3. Implement `ProjectionEngine`
4. Build `InMemoryReadModelStore` for testing
5. Add rebuild capability

**Design Principles:**
- **Separation of concerns**: Handler logic (projector) is separate from storage (read model store) and orchestration (projection engine)
- **Stateless projectors**: Projectors take `&self`, not `&mut self`. All mutable state lives in the `ReadModelStore` via interior mutability.
- **Composable**: One projector per read model concern; swap backends freely

**Code to Create:**

`crates/nineties-core/src/read_model_store.rs`:
```rust
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Mutex;

pub type Row = serde_json::Value;

#[async_trait]
pub trait ReadModelStore: Send + Sync {
    /// Execute a write operation (INSERT, UPDATE, DELETE).
    async fn execute(
        &self,
        sql: &str,
        params: Vec<serde_json::Value>,
    ) -> Result<(), ReadModelError>;

    /// Execute a query and return rows.
    async fn query(
        &self,
        sql: &str,
        params: Vec<serde_json::Value>,
    ) -> Result<Vec<Row>, ReadModelError>;

    /// Truncate/clear a table or collection.
    async fn truncate(&self, table: &str) -> Result<(), ReadModelError>;
}

/// In-memory read model store for testing (built into nineties-core).
pub struct InMemoryReadModelStore {
    tables: Mutex<HashMap<String, Vec<Row>>>,
}

impl InMemoryReadModelStore {
    pub fn new() -> Self {
        Self {
            tables: Mutex::new(HashMap::new()),
        }
    }

    pub fn get_rows(&self, table: &str) -> Vec<Row> {
        self.tables.lock().unwrap().get(table).cloned().unwrap_or_default()
    }
}
```

`crates/nineties-core/src/projection.rs`:
```rust
use crate::event::Event;
use crate::event_store::EventStore;
use crate::read_model_store::ReadModelStore;
use async_trait::async_trait;
use std::sync::Arc;

/// Stateless event handler — the "machine".
///
/// Contains the pure logic for transforming events into read model writes.
/// Takes `&self` (not `&mut self`) — all mutable state lives in the ReadModelStore.
#[async_trait]
pub trait Projector: Send + Sync {
    /// Unique name identifying this projector.
    fn name(&self) -> &str;

    /// Event types this projector handles.
    fn handles(&self) -> Vec<String>;

    /// Apply a single event to the read model via the store.
    /// Should be idempotent.
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

/// Composed read model unit — the "output".
///
/// Ties a projector to its storage backend. All methods take `&self`.
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
    async fn rebuild(&self, events: Vec<Event>) -> ProjectionResult<()> {
        self.clear().await?;
        for event in events {
            if self.handles().contains(&event.event_type) {
                self.handle(&event).await?;
            }
        }
        Ok(())
    }
}

/// Standard composition glue: Projector + Arc<dyn ReadModelStore> + table name = Projection.
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
    ) -> Self {
        Self { projector, store, table: table.into() }
    }
}

#[async_trait]
impl Projection for ProjectionUnit {
    fn name(&self) -> &str { self.projector.name() }
    fn handles(&self) -> Vec<String> { self.projector.handles() }

    async fn handle(&self, event: &Event) -> ProjectionResult<()> {
        self.projector.apply(event, self.store.as_ref()).await
    }

    async fn clear(&self) -> ProjectionResult<()> {
        self.store.truncate(&self.table).await
            .map_err(|e| ProjectionError::clear_failed(self.projector.name(), e.to_string()))
    }
}

pub struct ProjectionEngine {
    projections: Vec<Box<dyn Projection>>,
    event_store: Box<dyn EventStore>,
}

impl ProjectionEngine {
    pub fn new(event_store: Box<dyn EventStore>) -> Self {
        Self {
            projections: Vec::new(),
            event_store,
        }
    }

    pub fn register(&mut self, projection: Box<dyn Projection>) {
        self.projections.push(projection);
    }

    /// Convenience: register a projector + store as a ProjectionUnit.
    pub fn register_projector(
        &mut self,
        projector: Box<dyn Projector>,
        store: Arc<dyn ReadModelStore>,
        table: impl Into<String>,
    ) {
        let unit = ProjectionUnit::new(projector, store, table);
        self.register(Box::new(unit));
    }

    /// Process a single event. Takes `&self` (not `&mut self`).
    pub async fn process(&self, event: &Event) -> ProjectionResult<()> {
        for projection in &self.projections {
            if projection.handles().contains(&event.event_type) {
                projection.handle(event).await?;
            }
        }
        Ok(())
    }

    /// Rebuild all projections from the event store. Takes `&self`.
    pub async fn rebuild_all(&self) -> ProjectionResult<()> {
        let events = self.event_store.stream_all(0).await
            .map_err(|e| ProjectionError::EventStoreError(e.to_string()))?;
        for projection in &self.projections {
            projection.rebuild(events.clone()).await?;
        }
        Ok(())
    }
}
```

### Phase 2: Aggregates & Commands (Weeks 5-8)

#### Week 5: Aggregate Trait

**Tasks:**
1. Define `Aggregate` trait
2. Define `Command` trait
3. Implement UserAggregate
4. Add tests

**Code to Create:**

`crates/nineties-core/src/aggregate.rs`:
```rust
use crate::event::Event;
use async_trait::async_trait;
use std::error::Error;

#[async_trait]
pub trait Command: Send + Sync {
    fn aggregate_id(&self) -> &str;
}

#[async_trait]
pub trait Aggregate: Send + Sync + Default {
    type Command: Command;
    type Event;
    type Error: Error + Send + Sync + 'static;

    /// Aggregate type name
    fn aggregate_type() -> &'static str;

    /// Current version (sequence number)
    fn version(&self) -> i64;

    /// Handle a command and produce events
    async fn handle(&self, command: Self::Command) -> Result<Vec<Event>, Self::Error>;

    /// Apply an event to update state
    fn apply(&mut self, event: &Event);

    /// Reconstruct aggregate from events
    fn from_events(events: Vec<Event>) -> Self {
        let mut aggregate = Self::default();
        for event in events {
            aggregate.apply(&event);
        }
        aggregate
    }
}
```

#### Week 6: User Aggregate Implementation

**Example Implementation:**

`crates/nineties-app/src/domain/user.rs`:
```rust
use nineties_core::{Aggregate, Command, Event};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UserError {
    #[error("User already exists")]
    AlreadyExists,

    #[error("User not found")]
    NotFound,

    #[error("Invalid email format")]
    InvalidEmail,

    #[error("Password too weak")]
    WeakPassword,
}

// Aggregate State
#[derive(Default)]
pub struct UserAggregate {
    id: Option<String>,
    email: Option<String>,
    name: Option<String>,
    password_hash: Option<String>,
    version: i64,
    created: bool,
}

// Commands
#[derive(Debug, Serialize, Deserialize)]
pub enum UserCommand {
    CreateUser {
        id: String,
        name: String,
        email: String,
        password: String,
    },
    UpdateProfile {
        id: String,
        name: String,
    },
    ChangePassword {
        id: String,
        old_password: String,
        new_password: String,
    },
}

impl Command for UserCommand {
    fn aggregate_id(&self) -> &str {
        match self {
            UserCommand::CreateUser { id, .. } => id,
            UserCommand::UpdateProfile { id, .. } => id,
            UserCommand::ChangePassword { id, .. } => id,
        }
    }
}

// Events
#[derive(Debug, Serialize, Deserialize)]
pub enum UserEvent {
    UserCreated {
        id: String,
        name: String,
        email: String,
        password_hash: String,
    },
    ProfileUpdated {
        id: String,
        name: String,
    },
    PasswordChanged {
        id: String,
    },
}

#[async_trait::async_trait]
impl Aggregate for UserAggregate {
    type Command = UserCommand;
    type Event = UserEvent;
    type Error = UserError;

    fn aggregate_type() -> &'static str {
        "User"
    }

    fn version(&self) -> i64 {
        self.version
    }

    async fn handle(&self, command: Self::Command) -> Result<Vec<Event>, Self::Error> {
        match command {
            UserCommand::CreateUser { id, name, email, password } => {
                if self.created {
                    return Err(UserError::AlreadyExists);
                }

                // Validate email
                if !email.contains('@') {
                    return Err(UserError::InvalidEmail);
                }

                // Hash password
                let password_hash = hash_password(&password);

                Ok(vec![Event::new(
                    "User",
                    &id,
                    self.version + 1,
                    "UserCreated",
                    serde_json::to_value(UserEvent::UserCreated {
                        id,
                        name,
                        email,
                        password_hash,
                    }).unwrap(),
                )])
            }

            UserCommand::UpdateProfile { id, name } => {
                if !self.created {
                    return Err(UserError::NotFound);
                }

                Ok(vec![Event::new(
                    "User",
                    &id,
                    self.version + 1,
                    "ProfileUpdated",
                    serde_json::to_value(UserEvent::ProfileUpdated { id, name }).unwrap(),
                )])
            }

            UserCommand::ChangePassword { id, old_password, new_password } => {
                if !self.created {
                    return Err(UserError::NotFound);
                }

                // Verify old password
                if !verify_password(&old_password, self.password_hash.as_ref().unwrap()) {
                    return Err(UserError::WeakPassword);
                }

                Ok(vec![Event::new(
                    "User",
                    &id,
                    self.version + 1,
                    "PasswordChanged",
                    serde_json::to_value(UserEvent::PasswordChanged { id }).unwrap(),
                )])
            }
        }
    }

    fn apply(&mut self, event: &Event) {
        self.version = event.sequence;

        match event.event_type.as_str() {
            "UserCreated" => {
                let data: UserEvent = serde_json::from_value(event.payload.clone()).unwrap();
                if let UserEvent::UserCreated { id, name, email, password_hash } = data {
                    self.id = Some(id);
                    self.name = Some(name);
                    self.email = Some(email);
                    self.password_hash = Some(password_hash);
                    self.created = true;
                }
            }
            "ProfileUpdated" => {
                let data: UserEvent = serde_json::from_value(event.payload.clone()).unwrap();
                if let UserEvent::ProfileUpdated { name, .. } = data {
                    self.name = Some(name);
                }
            }
            "PasswordChanged" => {
                // Password hash would be in metadata or separate secure store
            }
            _ => {}
        }
    }
}
```

#### Week 7: Command Bus

**Tasks:**
1. Implement CommandBus
2. Connect to EventStore and EventBus
3. Add optimistic concurrency control
4. Add tests

`crates/nineties-core/src/command_bus.rs`:
```rust
use crate::{Aggregate, Command, Event, EventBus, EventStore};
use std::error::Error;
use std::marker::PhantomData;

pub struct CommandBus<A: Aggregate> {
    event_store: Box<dyn EventStore>,
    event_bus: Box<dyn EventBus>,
    _phantom: PhantomData<A>,
}

impl<A: Aggregate> CommandBus<A> {
    pub fn new(
        event_store: Box<dyn EventStore>,
        event_bus: Box<dyn EventBus>,
    ) -> Self {
        Self {
            event_store,
            event_bus,
            _phantom: PhantomData,
        }
    }

    pub async fn dispatch(&mut self, command: A::Command) -> Result<Vec<Event>, Box<dyn Error>> {
        let aggregate_id = command.aggregate_id();

        // Load existing events
        let events = self.event_store.load(aggregate_id).await?;
        let current_version = events.last().map(|e| e.sequence).unwrap_or(0);

        // Reconstruct aggregate
        let aggregate = A::from_events(events);

        // Handle command
        let new_events = aggregate.handle(command).await?;

        // Append events (with optimistic concurrency check)
        self.event_store
            .append(aggregate_id, Some(current_version), new_events.clone())
            .await?;

        // Publish events
        self.event_bus.publish(new_events.clone()).await?;

        Ok(new_events)
    }
}
```

#### Week 8: User Projection (Read Model)

**Tasks:**
1. Create `UserListProjector` (stateless event handler)
2. Compose it with a `ReadModelStore` via `ProjectionUnit`
3. Register with `ProjectionEngine`
4. Add tests

`crates/nineties-app/src/projections/user_list.rs`:
```rust
use nineties_core::event::Event;
use nineties_core::projection::{Projector, ProjectionResult, ProjectionError};
use nineties_core::read_model_store::ReadModelStore;
use async_trait::async_trait;

/// Stateless projector for the user list read model.
///
/// Transforms UserCreated and ProfileUpdated events into writes against
/// a ReadModelStore. Does not own any state — the store handles persistence.
pub struct UserListProjector;

#[async_trait]
impl Projector for UserListProjector {
    fn name(&self) -> &str {
        "UserList"
    }

    fn handles(&self) -> Vec<String> {
        vec![
            "UserCreated".to_string(),
            "ProfileUpdated".to_string(),
        ]
    }

    async fn apply(
        &self,
        event: &Event,
        store: &dyn ReadModelStore,
    ) -> ProjectionResult<()> {
        match event.event_type.as_str() {
            "UserCreated" => {
                let data = &event.payload;

                store.execute(
                    "INSERT INTO users_view (id, name, email, created_at) VALUES (?1, ?2, ?3, ?4)",
                    vec![
                        serde_json::json!(data["id"].as_str().unwrap()),
                        serde_json::json!(data["name"].as_str().unwrap()),
                        serde_json::json!(data["email"].as_str().unwrap()),
                        serde_json::json!(event.timestamp.duration_since(UNIX_EPOCH)?.as_secs() as i64),
                    ],
                ).await.map_err(|e| ProjectionError::handle_failed(
                    "UserList", &event.event_type, &event.event_id.to_string(), e.to_string()
                ))?;
            }
            "ProfileUpdated" => {
                let data = &event.payload;

                store.execute(
                    "UPDATE users_view SET name = ?1, updated_at = ?2 WHERE id = ?3",
                    vec![
                        serde_json::json!(data["name"].as_str().unwrap()),
                        serde_json::json!(event.timestamp.duration_since(UNIX_EPOCH)?.as_secs() as i64),
                        serde_json::json!(event.aggregate_id.as_str()),
                    ],
                ).await.map_err(|e| ProjectionError::handle_failed(
                    "UserList", &event.event_type, &event.event_id.to_string(), e.to_string()
                ))?;
            }
            _ => {}
        }

        Ok(())
    }

    async fn init(&self, store: &dyn ReadModelStore) -> ProjectionResult<()> {
        store.execute(
            "CREATE TABLE IF NOT EXISTS users_view (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                email TEXT NOT NULL UNIQUE,
                created_at INTEGER NOT NULL,
                updated_at INTEGER
            )",
            vec![],
        ).await.map_err(|e| ProjectionError::read_model_error("UserList", e.to_string()))?;
        Ok(())
    }
}
```

**Composing the projection:**
```rust
use nineties_core::projection::{ProjectionUnit, ProjectionEngine};
use nineties_core::read_model_store::ReadModelStore;
use std::sync::Arc;

// Compose: projector + store = projection
let store: Arc<dyn ReadModelStore> = Arc::new(sqlite_read_model_store);
let projection = ProjectionUnit::new(
    Box::new(UserListProjector),
    store.clone(),
    "users_view",
);

// Register with engine
let mut engine = ProjectionEngine::new(event_store);
engine.register(Box::new(projection));

// Or use the convenience method:
engine.register_projector(Box::new(UserListProjector), store, "users_view");
```

### Phase 3: Integration (Weeks 9-12)

#### Week 9: Update Controllers

**Tasks:**
1. Refactor auth_controller to use CommandBus
2. Refactor admin_controller
3. Keep Diesel for reads (from projections)
4. Add tests

**Before (Current):**
```rust
#[post("/admin/users")]
pub async fn create_user(form: web::Form<UserForm>) -> impl Responder {
    let new_user = NewUser {
        name: &form.name,
        email: &form.email,
        password: &hash_password(&form.password),
    };

    diesel::insert_into(users)
        .values(&new_user)
        .execute(&mut get_connection())?;

    HttpResponse::Ok().finish()
}
```

**After (Event Sourcing):**
```rust
#[post("/admin/users")]
pub async fn create_user(
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
        Ok(_) => HttpResponse::Ok().json(json!({"success": true})),
        Err(e) => HttpResponse::BadRequest().json(json!({"error": e.to_string()})),
    }
}
```

#### Week 10: Dual-Write Mode

**Tasks:**
1. Write to both event store AND Diesel
2. Validate consistency
3. Monitor for issues
4. Prepare rollback if needed

**Implementation:**
```rust
// Temporary: Write to both systems
async fn create_user_dual_write(command: UserCommand) -> Result<(), Box<dyn Error>> {
    // 1. Event sourcing (new)
    let events = command_bus.dispatch(command.clone()).await?;

    // 2. Traditional DB (old) - for safety during transition
    diesel::insert_into(users)
        .values(&NewUser::from_command(&command))
        .execute(&mut get_connection())?;

    // 3. Validate they match
    verify_consistency(&command.aggregate_id()).await?;

    Ok(())
}
```

#### Week 11: Switch to Projections

**Tasks:**
1. Stop dual-writes
2. Use projections for all reads
3. Monitor query performance
4. Optimize indexes if needed

#### Week 12: Cleanup & Documentation

**Tasks:**
1. Remove old Diesel write code
2. Update documentation
3. Performance benchmarks
4. Team training on ES patterns

---

## Technical Requirements

### Dependencies

**Add to `Cargo.toml`:**
```toml
[dependencies]
# Event sourcing
async-trait = "0.1"
thiserror = "1.0"
uuid = { version = "1.0", features = ["v4", "serde"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Existing (keep)
actix-web = "4"
diesel = { version = "2.2.6", features = ["sqlite"] }
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
```

### Database Migrations

**Create migration:**
```bash
diesel migration generate create_events_table
diesel migration generate create_users_view_table
```

**Up migration (`*_create_events_table/up.sql`):**
```sql
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

CREATE INDEX idx_events_aggregate ON events(aggregate_id, sequence);
CREATE INDEX idx_events_type ON events(event_type);
CREATE INDEX idx_events_timestamp ON events(timestamp);
CREATE INDEX idx_events_id ON events(id);
```

**Up migration (`*_create_users_view_table/up.sql`):**
```sql
-- Rename existing users table
ALTER TABLE users RENAME TO users_backup;

-- Create new users_view table (projection)
CREATE TABLE users_view (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    created_at INTEGER NOT NULL,
    updated_at INTEGER,
    INDEX idx_users_email (email)
);

-- Migrate existing data (optional, or rebuild from events)
INSERT INTO users_view (id, name, email, created_at)
SELECT CAST(id AS TEXT), name, email, CAST(strftime('%s', created_at) AS INTEGER)
FROM users_backup;
```

### Testing Requirements

**Unit Tests:**
- Event serialization/deserialization
- Aggregate command handling
- Aggregate event application
- Projector event handling (apply)

**Integration Tests:**
- EventStore append/load
- EventBus publish/subscribe
- CommandBus dispatch
- ProjectionUnit composition and rebuild

**End-to-End Tests:**
- Full user creation flow
- Full user update flow
- Query read models

**Example Test:**
```rust
#[tokio::test]
async fn test_user_creation_flow() {
    // Setup
    let event_store = Box::new(InMemoryEventStore::new());
    let event_bus = Box::new(InProcessEventBus::new());
    let mut command_bus = CommandBus::<UserAggregate>::new(event_store, event_bus);

    // Execute
    let command = UserCommand::CreateUser {
        id: "user-123".to_string(),
        name: "Test User".to_string(),
        email: "test@example.com".to_string(),
        password: "SecurePass123".to_string(),
    };

    let events = command_bus.dispatch(command).await.unwrap();

    // Verify
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type, "UserCreated");
    assert_eq!(events[0].aggregate_id, "user-123");
}
```

---

## Migration Strategy

### Step-by-Step Migration

**Phase 1: Preparation (Week 1)**
1. Create feature branch: `feature/event-sourcing`
2. Setup workspace structure
3. Create empty crates
4. Add dependencies

**Phase 2: Build Core (Weeks 2-4)**
1. Implement Event, EventStore, EventBus
2. Add comprehensive tests
3. Code review

**Phase 3: Build Domain (Weeks 5-8)**
1. Implement UserAggregate
2. Implement UserListProjector + compose via ProjectionUnit
3. Add integration tests
4. Code review

**Phase 4: Integrate (Weeks 9-10)**
1. Update one controller (auth_controller)
2. Deploy to staging with dual-write
3. Monitor for 1 week
4. Validate consistency

**Phase 5: Complete Migration (Weeks 11-12)**
1. Migrate remaining controllers
2. Remove dual-write code
3. Remove old users table
4. Deploy to production

### Rollback Procedures

**If issues occur in Phase 4 (Dual-Write):**
1. Disable event sourcing code path
2. Revert to direct Diesel writes
3. No data loss (both systems have data)

**If issues occur in Phase 5:**
1. Re-enable dual-write mode
2. Validate data consistency
3. Fix issues
4. Try again

---

## Testing Strategy

### Test Pyramid

```
        /\
       /E2E\         5% - Full system tests
      /------\
     / Integ  \       15% - Component integration
    /----------\
   /   Unit     \     80% - Unit tests
  /--------------\
```

### Test Coverage Goals

| Component | Target Coverage | Critical Paths |
|-----------|----------------|----------------|
| Event Store | 95% | append, load, concurrency |
| Aggregates | 95% | command handling, events |
| Projections | 90% | projector apply, rebuild, ReadModelStore |
| Controllers | 80% | happy path, error handling |
| Overall | 85% | - |

### Performance Benchmarks

**Before migration (baseline):**
- User creation: ~50ms
- User query: ~5ms
- User update: ~40ms

**After migration (target):**
- User creation: <100ms (includes event store + projection)
- User query: <10ms (from read model)
- User update: <80ms
- Projection rebuild: <1s per 10k events

---

## Rollback Plan

### Trigger Conditions

**Rollback if:**
- Data inconsistency detected (>0.1%)
- Performance degradation (>2x slower)
- Critical bug in production
- Team consensus to pause

### Rollback Steps

1. **Immediate**: Disable ES code path via feature flag
2. **Verify**: Confirm old path works
3. **Analyze**: Investigate root cause
4. **Fix**: Address issues in staging
5. **Retry**: Re-enable with fixes

### Data Recovery

**Scenario: Lost events**
- Backup event store hourly
- Can rebuild read models from events
- Can replay events from backup

**Scenario: Corrupted read model**
- Simply rebuild from event store
- No data loss (events are source of truth)

---

## Next Steps

### Immediate Actions (This Week)

1. ✅ Review this document with team
2. ✅ Get architectural approval
3. ✅ Create GitHub project for tracking
4. ✅ Schedule kickoff meeting

### Week 1 Actions

1. Create feature branch
2. Setup workspace structure
3. Begin Event type implementation
4. Daily standup to track progress

### Resources Needed

**Team:**
- 2 senior Rust engineers (full-time)
- 1 architect (50% time)
- 1 QA engineer (50% time)

**Time:**
- 12 weeks to full ES implementation
- 4 weeks minimum for MVP (Event Store + 1 aggregate)

**Tools:**
- Staging environment for testing
- Database backups
- Monitoring dashboard

---

## Summary

**Event Sourcing Implementation is:**
- ✅ Well-documented in architecture docs
- ✅ Broken down into manageable phases
- ✅ Testable at each step
- ✅ Reversible if needed
- ✅ Ready to begin

**Key Success Factors:**
1. Comprehensive testing at every phase
2. Dual-write mode for safety
3. Team training on ES patterns
4. Monitoring and observability
5. Clear rollback procedures

**Estimated Timeline:**
- 4 weeks: MVP (Event Store + 1 aggregate)
- 8 weeks: User aggregate fully migrated
- 12 weeks: Production-ready event sourcing

This guide should be reviewed and updated as implementation progresses.
