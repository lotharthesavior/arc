# Nineties: Event Sourcing & Composable Architecture

## 1. Current State Analysis

Nineties is a Rust/Actix-Web MVC starter with:
- Diesel ORM + SQLite (CRUD, mutable state)
- Tera templates, Tailwind, Alpine.js, HTMX
- WebSocket support (Turbo Streams)
- Planned plugin system (`the-hook` filters)
- Monolithic binary with feature flags

### Current Architecture

![Architecture Diagram - Current Nineties MVC - HTTP requests flow through middleware to routes, controllers, and services, using Diesel ORM for SQLite database access and Tera for template rendering](diagrams/architecture-04-current-nineties-mvc.svg)

### Key Problems

1. **Tight coupling**: Controllers → Services → Diesel → SQLite is a single pipeline
2. **Mutable state**: CRUD operations lose history — no audit trail, no replay
3. **Monolithic**: UI and backend are one binary, can't run headless microservices
4. **No event bus**: Components can't react to domain events asynchronously
5. **Plugin system is filter-only**: `the-hook` transforms values but doesn't model domain events

---

## 2. Target Architecture: Event-Sourced, Composable Framework

### 2.1 Core Principles

- **Events are the source of truth** — state is derived, never mutated directly
- **Commands produce Events** — every write goes through a command handler
- **Projections build read models** — optimized views materialized from event streams
- **Plugins are optional compositions** — UI, projections, and even event stores are pluggable
- **Core is headless by default** — web UI is a plugin, not a requirement
- **Complexity is opt-in** — simple services can emit events directly; complex domains use full aggregates + command bus

### 2.2 High-Level Architecture

![Architecture Diagram - Event-Sourced Target Architecture - Core library contains Command Bus, Event Store trait, Event Bus, and Projection Engine with Read Model Store trait, with optional web and CLI plugins, pluggable event store backends (SQLite, Postgres, File) and pluggable read model store backends (SQLite, Postgres, dqlite, in-memory)](diagrams/architecture-05-event-sourced-target.svg)

### 2.3 Dual Complexity Paths

The framework supports both simple and complex patterns. Developers choose based on their domain:

![Comparison Diagram - Complexity Paths - Simple path shows controllers emitting events directly to event store via services, while complex path uses Command Bus and Aggregates for domain logic enforcement, both converging at the Event Bus for projection updates](diagrams/comparison-01-complexity-paths.svg)

- **Simple path**: Services validate and emit events directly — minimal ceremony
- **Complex path**: Full aggregates + command bus — strong consistency, domain invariants enforced

### 2.4 Package / Crate Structure

![Architecture Diagram - Workspace Crates - nineties-core is the foundation crate with no dependencies, implemented by event stores (SQLite, Postgres), extended by web and CLI crates, and composed by plugins and the main application](diagrams/architecture-06-workspace-crates.svg)

Proposed `Cargo.toml` workspace:

```toml
[workspace]
members = [
    "crates/nineties-core",
    "crates/nineties-es-sqlite",
    "crates/nineties-es-postgres",
    "crates/nineties-rm-sqlite",
    "crates/nineties-rm-postgres",
    "crates/nineties-rm-dqlite",
    "crates/nineties-web",
    "crates/nineties-cli",
    "crates/nineties-app",
    "plugins/*",
]
```

---

## 3. Core Components

### 3.1 Event Store

The foundational component. All domain state changes are persisted as an append-only log of events.

![Architecture Diagram - Event Store Classes - Event class contains event metadata and payload, EventStore trait defines append and load operations, implemented by SQLite, Postgres, and in-memory stores](diagrams/architecture-07-event-store-classes.svg)

**Schema for SQLite event store:**

```sql
CREATE TABLE events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    aggregate_type TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    sequence INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    payload TEXT NOT NULL,        -- JSON
    metadata TEXT DEFAULT '{}',   -- JSON (causation_id, correlation_id, user_id, etc.)
    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(aggregate_id, sequence)
);

CREATE INDEX idx_events_aggregate ON events(aggregate_id, sequence);
CREATE INDEX idx_events_type ON events(event_type);
CREATE INDEX idx_events_timestamp ON events(timestamp);
```

### 3.2 Aggregates & Commands

Aggregates encapsulate domain logic. Commands are validated and produce events.

![Architecture Diagram - Aggregate and Command Classes - Aggregate trait handles commands and produces events, CommandBus dispatches commands to aggregates and coordinates with EventStore and EventBus](diagrams/architecture-08-aggregate-command-classes.svg)

**Example — User aggregate:**

```rust
pub struct UserAggregate {
    id: Option<String>,
    email: Option<String>,
    name: Option<String>,
    password_hash: Option<String>,
    created: bool,
}

// Commands
pub enum UserCommand {
    CreateUser { id: String, name: String, email: String, password: String },
    UpdateProfile { id: String, name: String },
    ChangePassword { id: String, old_password: String, new_password: String },
    DeleteUser { id: String },
}

// Events
pub enum UserEvent {
    UserCreated { id: String, name: String, email: String, password_hash: String, at: DateTime },
    ProfileUpdated { id: String, name: String, at: DateTime },
    PasswordChanged { id: String, at: DateTime },
    UserDeleted { id: String, at: DateTime },
}
```

### 3.3 Event Bus

Decouples event producers from consumers. Supports sync and async subscribers.

![Flow Diagram - Event Bus Flow - Command handler publishes events to EventBus, which notifies multiple subscribers including projections (User, Audit Log) and side effects (Email, WebSocket, Webhooks)](diagrams/flow-05-event-bus-flow.svg)

![Architecture Diagram - Event Bus Classes - EventBus trait defines publish and subscribe operations, EventHandler trait specifies event handling interface, implemented by InProcessEventBus for synchronous handling and ChannelEventBus for async handling](diagrams/architecture-09-event-bus-classes.svg)

### 3.4 Projections (Read Models)

Projections consume events and build query-optimized read models. The projection system uses a **three-trait architecture** that separates concerns cleanly:

- **`Projector`** — stateless event handler (the "machine"). Contains the pure logic for transforming events into read model writes. Takes `&self`.
- **`Projection`** — composed read model unit (the "output"). Ties a projector to its storage backend. Takes `&self`.
- **`ReadModelStore`** — backend-agnostic persistence layer. Implementations handle SQLite, Postgres, dqlite, or in-memory storage.

`ProjectionUnit` is the standard glue struct that composes a `Projector` + `Arc<dyn ReadModelStore>` + table name into a `Projection`.

![Flow Diagram - Projection Event Flow - Event stream (UserCreated, ProfileUpdated, UserDeleted) flows into multiple projections (UserList, AuditLog, Stats) which materialize different read models through a pluggable backend supporting SQLite, Postgres, dqlite, and in-memory stores](diagrams/flow-06-projection-event-flow.svg)

![Architecture Diagram - Projection Classes - ReadModelStore trait defines storage operations implemented by SQLite, Postgres, dqlite, and in-memory stores; Projector trait defines stateless event handling; Projection trait defines composed read model unit with handle, clear, and rebuild; ProjectionUnit glues Projector and ReadModelStore; ProjectionEngine manages multiple projections, processes events, and coordinates rebuilding from EventStore](diagrams/architecture-10-projection-classes.svg)

**Projector trait** — stateless event handler:

```rust
#[async_trait]
pub trait Projector: Send + Sync {
    /// Unique name identifying this projector.
    fn name(&self) -> &str;

    /// Event types this projector handles.
    fn handles(&self) -> Vec<String>;

    /// Apply a single event to the read model via the store.
    /// Must be idempotent: applying the same event twice produces the same result.
    async fn apply(&self, event: &Event, store: &dyn ReadModelStore) -> ProjectionResult<()>;

    /// Initialize the read model schema (CREATE TABLE IF NOT EXISTS, etc.).
    /// Default implementation is a no-op.
    async fn init(&self, _store: &dyn ReadModelStore) -> ProjectionResult<()> {
        Ok(())
    }
}
```

**Projection trait** — composed read model unit:

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
    async fn rebuild(&self, events: Vec<Event>) -> ProjectionResult<()>;
}
```

All methods take `&self`, not `&mut self`. Mutable state lives in the `ReadModelStore`, which handles interior mutability via connection pools, `Mutex`, etc.

**ProjectionUnit** — standard composition glue:

```rust
pub struct ProjectionUnit {
    projector: Box<dyn Projector>,
    store: Arc<dyn ReadModelStore>,
    table: String, // truncate target for clear()
}

impl ProjectionUnit {
    pub fn new(
        projector: Box<dyn Projector>,
        store: Arc<dyn ReadModelStore>,
        table: impl Into<String>,
    ) -> Self;
}

// ProjectionUnit implements Projection by delegating:
//   name()   → projector.name()
//   handle() → projector.apply(event, store)
//   clear()  → store.truncate(table)
```

**Read Model Store trait:**

```rust
#[async_trait]
pub trait ReadModelStore: Send + Sync {
    /// Execute a write operation (INSERT, UPDATE, DELETE)
    async fn execute(&self, sql: &str, params: Vec<Value>) -> ReadModelResult<()>;

    /// Execute a query and return rows
    async fn query(&self, sql: &str, params: Vec<Value>) -> ReadModelResult<Vec<Row>>;

    /// Truncate a table (used during projection rebuild)
    async fn truncate(&self, table: &str) -> ReadModelResult<()>;
}
```

`ReadModelError` and `ReadModelResult<T>` are defined in the `read_model_store` module. `ProjectionError` includes a `ReadModelError` variant for propagating store errors up through the projection layer.

**Available backends:**

| Backend | Crate | Best For |
|---------|-------|----------|
| **SQLite** | `nineties-rm-sqlite` | Local development, single-node, embedded |
| **Postgres** | `nineties-rm-postgres` | Production, headless microservices |
| **dqlite** | `nineties-rm-dqlite` | Distributed/edge, replicated SQLite |
| **In-Memory** | `nineties-core` (built-in) | Testing, ephemeral projections |

Adding a new backend requires implementing the `ReadModelStore` trait — no changes to projectors or the engine.

**ProjectionEngine** — orchestrates multiple projections:

The `ProjectionEngine` takes `&self` (not `&mut self`) for `process()`, `process_batch()`, `rebuild_all()`, and `rebuild_projection()`. It provides two registration paths:

- `register()` — accepts a fully composed `Box<dyn Projection>`
- `register_projector()` — convenience method that accepts a `Box<dyn Projector>` + `Arc<dyn ReadModelStore>` + table name, wraps them in a `ProjectionUnit`, and registers the result

```rust
// Option 1: register a pre-composed projection
let projection = ProjectionUnit::new(Box::new(UserListProjector), store.clone(), "users_view");
engine.register(Box::new(projection));

// Option 2: convenience — register projector + store directly
engine.register_projector(Box::new(UserListProjector), store.clone(), "users_view");
```

**Key capability**: Projections can be rebuilt from scratch by replaying the entire event stream. This enables:
- Schema changes without data migration
- New read models added retroactively
- Bug fixes by replaying with corrected projection logic
- Switching storage backends without rewriting projectors

### 3.5 Snapshot Store (Optional)

For aggregates with many events, snapshots avoid replaying the full history.

![Architecture Diagram - Snapshot Store Classes - SnapshotStore trait provides save and load operations for Snapshot entities containing aggregate state at specific versions](diagrams/architecture-11-snapshot-store-classes.svg)

---

## 4. Plugin / Composability System

### 4.1 Plugin Trait

Building on the existing [plugin-system.md](planning/plugin-system.md), plugins now interact with the ES core:

![Architecture Diagram - Plugin Classes - Plugin trait defines name, version, and registration, PluginRegistry provides registration methods for aggregates, projections, event handlers, routes, middleware, and CLI commands](diagrams/architecture-12-plugin-classes.svg)

**Example: A "Blog" plugin as a separate crate:**

```rust
// plugins/nineties-plugin-blog/src/lib.rs
pub struct BlogPlugin;

impl Plugin for BlogPlugin {
    fn name(&self) -> &str { "blog" }
    fn version(&self) -> &str { "0.1.0" }

    fn register(&self, reg: &mut PluginRegistry) {
        // Domain
        reg.register_aggregate::<BlogPostAggregate>();
        reg.register_projection(Box::new(BlogListProjection::new()));
        reg.register_event_handler(Box::new(BlogSearchIndexer::new()));

        // Web (only if nineties-web is present)
        #[cfg(feature = "web")]
        {
            reg.register_routes(blog_routes::config);
        }

        // CLI
        reg.register_cli_command("blog:rebuild", blog_cli::rebuild_projections);
    }
}
```

### 4.2 Composition Modes

![Architecture Diagram - Composition Modes - Four deployment modes: Mode A combines core with web plugin and SQLite for full-stack apps, Mode B uses Postgres for headless microservices, Mode C focuses on CLI tools for replay and rebuild, Mode D uses dqlite for distributed/edge deployments](diagrams/architecture-13-composition-modes.svg)

---

## 5. Data Flow: Full Request Lifecycle

### 5.1 Write Path (Command)

![Flow Diagram - Write Path Sequence - Client POST request flows through Controller to CommandBus, which loads aggregate from EventStore, handles command, appends events, publishes to EventBus for projection updates and WebSocket notifications, returning 201 Created](diagrams/flow-07-write-path-sequence.svg)

### 5.2 Read Path (Query)

![Flow Diagram - Read Path Sequence - Client GET request flows through Controller to ReadModel, queries users_view, returns UserView vector, rendered as HTML via Tera templates](diagrams/flow-08-read-path-sequence.svg)

### 5.3 Projection Rebuild

![Flow Diagram - Projection Rebuild Sequence - CLI triggers ProjectionEngine to rebuild projection, truncates read model, streams all events from EventStore, processes each event through Projection handler, completes with event count](diagrams/flow-09-projection-rebuild-sequence.svg)

---

## 6. Migration Strategy from Current State

### Phase 1: Extract Core Library

```
nineties/                    nineties/
├── src/                     ├── crates/
│   ├── main.rs              │   ├── nineties-core/
│   ├── routes.rs    ──►     │   │   ├── src/
│   ├── models/              │   │   │   ├── aggregate.rs
│   ├── services/            │   │   │   ├── command.rs
│   └── helpers/             │   │   │   ├── event.rs
└── ...                      │   │   │   ├── event_store.rs
                             │   │   │   ├── event_bus.rs
                             │   │   │   ├── projection.rs
                             │   │   │   └── lib.rs
                             │   ├── nineties-web/
                             │   │   ├── src/ (actix, tera, routes)
                             │   └── nineties-app/
                             │       └── src/main.rs
                             └── plugins/
```

### Phase 2: Introduce Event Store alongside Diesel

Keep Diesel for read models. Add event store for writes. Dual-write during transition:

![Flow Diagram - Migration Extract Core - Commands flow through Aggregate to EventStore for appending and EventBus for publishing, Projections use Diesel to write read tables, Queries use Diesel to read from view tables](diagrams/flow-10-migration-extract-core.svg)

### Phase 3: Full ES — Remove Direct Diesel Writes

All write operations go through commands. Diesel is used only in projections for read model tables.

### Phase 4: Extract Plugins

Move features (pages, blog, auth) into plugin crates that register their own aggregates, projections, and routes.

---

## 7. Component Checklist

| Component | Crate | Priority | Status |
|-----------|-------|----------|--------|
| `Event` type + serialization | `nineties-core` | P0 | New |
| `EventStore` trait | `nineties-core` | P0 | New |
| SQLite EventStore impl | `nineties-es-sqlite` | P0 | New |
| `EventBus` trait + in-process impl | `nineties-core` | P0 | New |
| `Projector` + `Projection` traits + engine | `nineties-core` | P0 | New |
| `ReadModelStore` trait | `nineties-core` | P0 | New |
| SQLite ReadModelStore impl | `nineties-rm-sqlite` | P0 | New |
| `Aggregate` trait | `nineties-core` | P1 | New |
| `CommandBus` | `nineties-core` | P1 | New |
| `PluginRegistry` (ES-aware) | `nineties-core` | P1 | Extend existing plan |
| Postgres ReadModelStore impl | `nineties-rm-postgres` | P1 | New |
| Snapshot store | `nineties-core` | P2 | New |
| Postgres EventStore impl | `nineties-es-postgres` | P2 | New |
| dqlite ReadModelStore impl | `nineties-rm-dqlite` | P2 | New |
| Web crate extraction | `nineties-web` | P1 | Refactor |
| CLI crate (replay, rebuild) | `nineties-cli` | P1 | Refactor |
| Async event bus (tokio channels) | `nineties-core` | P2 | New |
| `the-hook` async support | `the-hook` | P2 | Extend |
| Saga / Process Manager | `nineties-core` | P3 | New |

---

## 8. Example: Full User Domain with ES

![Flow Diagram - User Domain Example - Write side commands (CreateUser, UpdateProfile, ChangePassword) flow through UserAggregate to EventStore, producing events (UserCreated, ProfileUpdated, PasswordChanged) consumed by projections (UserList, AuditLog) and side effects (WelcomeEmail), queryable via GET endpoints](diagrams/flow-11-user-domain-example.svg)

---

## 9. Distributed Nodes & Cluster Architecture

### 9.1 The Problem Space

When multiple Nineties apps need to work together — scaling horizontally, distributing workload, and sharing events across nodes.

![Architecture Diagram - Single Node - Current single-node architecture with Nineties App connected to local Event Store](diagrams/architecture-14-single-node.svg)

### 9.2 Architecture: Pluggable Cluster with Local-First Storage

Each node owns its local SQLite event store. Cluster traits in `nineties-core` define how nodes discover each other, sync events, and distribute workload. Backend implementations are swappable.

![Architecture Diagram - Cluster Architecture - Cluster control plane with Node Registry, Coordinator/Leader, and Heartbeat Monitor coordinates three nodes (A as Leader, B and C as Workers), each with local SQLite Event Store and Event Bus, synchronized via pluggable sync layer (NATS/gRPC/Gossip)](diagrams/architecture-15-cluster-architecture.svg)

### 9.3 Core Distributed Components

![Architecture Diagram - Distributed Node Classes - NodeIdentity contains node metadata, NodeRegistry trait manages node registration and discovery, ClusterEventBus handles remote event distribution, WorkloadDistributor assigns commands to nodes, LeaderElection coordinates cluster leadership](diagrams/architecture-16-distributed-node-classes.svg)

### 9.4 Local SQLite per Node with Aggregate Partitioning

Each node owns its local SQLite event store. Aggregate partitioning via consistent hashing ensures only one node writes to a given aggregate — eliminating conflicts without distributed locking.

![Architecture Diagram - Aggregate Partitioning - Three nodes (A, B, C) each own specific aggregate partitions (0-3, 4-7, 8-11), commands flow through aggregates to local SQLite Event Store, events published via sync layer (NATS/gRPC) to projections on all nodes](diagrams/architecture-17-aggregate-partitioning.svg)

**How it works:**
- Each node runs its own embedded SQLite — zero shared infrastructure
- Aggregates are partitioned across nodes via consistent hashing of `aggregate_id`
- Commands arriving at the wrong node are forwarded to the owning node
- The owning node appends events to its local SQLite and publishes them to the sync layer
- All nodes receive all events and update their local projections (eventual consistency)
- If a node goes down, its partitions are reassigned and the new owner replays from the sync layer's retention

**Command forwarding flow:**

![Flow Diagram - Command Forwarding Sequence - Client request through Load Balancer to Node A, which hashes aggregate ID to determine owner Node B, forwards command, Node B processes and appends to local SQLite, publishes event via sync layer for projection updates on all nodes](diagrams/flow-12-command-forwarding-sequence.svg)

### 9.5 Pluggable Cluster Backends

![Architecture Diagram - Cluster Backend Implementations - nineties-core defines traits (NodeRegistry, ClusterEventBus, WorkloadDistributor, LeaderElection), implemented by three backends: NATS (with JetStream), P2P (with Gossip and gRPC), and K8s (native discovery)](diagrams/architecture-18-cluster-backend-implementations.svg)

Available backends:

| Backend | Discovery | Sync | Leader Election | Best For |
|---------|-----------|------|-----------------|----------|
| **NATS JetStream** | NATS subjects | NATS pub/sub with persistence | Lease via NATS KV | Cloud, k8s, general purpose |
| **P2P / Gossip** | SWIM gossip + seed nodes | gRPC streaming | Raft consensus | Edge, IoT, zero-dependency |
| **K8s Native** | Headless Service DNS / API | Combine with NATS or gRPC | Lease via k8s ConfigMap | Kubernetes-only deployments |
| **Postgres** | Registry table | LISTEN/NOTIFY | Lease via advisory lock | When Postgres is already available |

### 9.6 Workload Distribution Strategies

![Flow Diagram - Workload Distribution Strategy - Aggregate partitioning strategy using consistent hashing of aggregate ID (user-abc) modulo N nodes to determine partition ownership, routing command to owning node (Node B owns partition 2)](diagrams/flow-13-workload-distribution-strategy.svg)

| Strategy | How | Trade-off |
|----------|-----|-----------|
| **Aggregate partitioning** | Consistent hash of aggregate_id → node | Single owner per aggregate, no conflicts, rebalance on node change |
| **Round-robin commands** | Load balancer distributes commands evenly | Needs distributed locking per aggregate |
| **Sticky sessions** | Route all commands for an aggregate to same node (via gateway) | Simple, but uneven load |
| **Claim-based** | Node "claims" aggregates on first access, registers in registry | Self-organizing, but needs lease/expiry |

**Recommended**: Aggregate partitioning — deterministic, conflict-free, rebalances cleanly.

### 9.7 Event Synchronization

![Flow Diagram - Event Synchronization Sequence - Node A (owner) processes command through aggregate, appends to local EventStore, publishes to Sync Layer (broker), which delivers to Node B and C (subscribers) for projection updates, achieving eventual consistency with sync latency](diagrams/flow-14-event-synchronization-sequence.svg)

**Consistency guarantees by layer:**

| Layer | Consistency | Mechanism |
|-------|-------------|-----------|
| Single aggregate | Strong | Optimistic concurrency (expected_version) |
| Local projections | Strong | Same-process event handling |
| Cross-node projections | Eventual | Sync delivery + idempotent handlers |
| Cross-aggregate queries | Eventual | Projection convergence |

### 9.8 Node Lifecycle

![State Diagram - Node Lifecycle - Node progresses through states: Joining (startup) → Registering (connect to registry) → Syncing (receive partition map) → Active (caught up), with transitions for graceful shutdown (Draining → Deregistered) and failure recovery (Active ↔ Suspected → Failed → Deregistered)](diagrams/state-04-node-lifecycle.svg)

### 9.9 Kubernetes Integration

![Deployment Diagram - Kubernetes Integration - StatefulSet nineties with three pods (nineties-0 Leader, nineties-1 and nineties-2 Workers), exposed via LoadBalancer Service for web traffic and Headless Service for pod discovery, with optional NATS StatefulSet for event synchronization](diagrams/deployment-01-kubernetes-integration.svg)

**K8s-native discovery:**

```rust
// nineties-cluster-k8s crate
pub struct K8sNodeRegistry {
    namespace: String,
    service_name: String,  // headless service
    kube_client: kube::Client,
}

impl NodeRegistry for K8sNodeRegistry {
    async fn discover(&self) -> Result<Vec<NodeIdentity>> {
        // DNS SRV lookup on headless service
        // or use kube API to list pod endpoints
        let endpoints = self.kube_client
            .list::<Endpoints>(&self.namespace)
            .await?;
        // ...
    }
}
```

**Autoscaling:** HPA (Horizontal Pod Autoscaler) scales pods based on command queue depth or event throughput:

```yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: nineties
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: StatefulSet
    name: nineties
  minReplicas: 2
  maxReplicas: 20
  metrics:
    - type: Pods
      pods:
        metric:
          name: nineties_command_queue_depth
        target:
          type: AverageValue
          averageValue: "100"
```

---

## 10. Implementation Roadmap

![Flow Diagram - Implementation Roadmap Gantt - Four-phase timeline: Phase 1 (Q1 2025) ES Core fundamentals, Phase 2 (Q2) Composability and workspace structure, Phase 3 (Q3) Cluster features and partitioning, Phase 4 (Q4) Advanced distributed capabilities](diagrams/flow-15-implementation-roadmap-gantt.svg)

---

## 11. Final Crate Map

![Architecture Diagram - Final Crate Map - nineties-core (events, aggregates, traits) as foundation, extended by Event Stores (SQLite, Postgres), Read Model Stores (SQLite, Postgres, dqlite), Cluster Backends (NATS, P2P, K8s), Surfaces (Web, CLI), and Plugins (auth, blog, pages)](diagrams/architecture-19-final-crate-map.svg)

---

## 12. Summary

Nineties evolves from a monolithic MVC starter into a **composable, event-sourced framework** where:

1. **Events are first-class citizens** — every state change is an event
2. **Core is headless** — `nineties-core` has zero web dependencies
3. **Web is a plugin** — `nineties-web` adds Actix, Tera, WebSocket
4. **Each node is self-contained** — local event store, no shared DB dependency
5. **Nodes sync via eventual consistency** — events replicated through pluggable sync layer
6. **Aggregate ownership eliminates conflicts** — consistent hashing assigns one writer per aggregate
7. **Event storage is pluggable** — SQLite (primary), Postgres (optional), or in-memory event stores
8. **Read model storage is pluggable** — SQLite, Postgres, dqlite, or in-memory via the `ReadModelStore` trait; new backends require only a single trait implementation
9. **Cluster backends are pluggable** — NATS, gRPC, P2P gossip, or k8s-native discovery
10. **Plugins compose freely** — register aggregates, projections, routes, CLI commands
11. **Complexity is opt-in** — simple services or full CQRS, developer's choice
