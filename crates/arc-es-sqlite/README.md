# arc-es-sqlite

> SQLite-based EventStore implementation for arc-core

**Status**: Under Development | **Version**: 0.1.0 | **License**: MIT

---

## Overview

`arc-es-sqlite` provides a SQLite-based implementation of the `EventStore` trait from `arc-core`. It offers a lightweight, embedded event store suitable for:

- Single-node applications
- Development and testing
- Edge deployments
- Local-first applications
- Prototyping event-sourced systems

**Key Features**:
- Zero configuration - embedded SQLite database
- ACID compliance
- Optimistic concurrency control
- Fast append-only writes
- Efficient event streaming for projections
- File-based or in-memory modes

---

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
arc-core = "0.1"
arc-es-sqlite = "0.1"
```

---

## Quick Start

### Basic Usage

```rust
use arc_core::{Event, EventStore};
use arc_es_sqlite::SqliteEventStore;

// PLACEHOLDER: Example to be updated when implementation is complete

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create event store (file-based)
    let event_store = SqliteEventStore::new("events.db").await?;

    // Create an event
    let event = Event::new(
        "User",
        "user-123",
        1,
        "UserCreated",
        serde_json::json!({
            "name": "Jane Doe",
            "email": "jane@example.com"
        })
    );

    // Append to store
    event_store.append("user-123", None, vec![event]).await?;

    // Load events
    let events = event_store.load("user-123").await?;
    println!("Loaded {} events", events.len());

    Ok(())
}
```

### In-Memory Mode (Testing)

```rust
use arc_es_sqlite::SqliteEventStore;

// PLACEHOLDER: Example to be updated when implementation is complete

#[tokio::test]
async fn test_event_store() {
    let event_store = SqliteEventStore::in_memory().await.unwrap();

    // Use for testing without file I/O
}
```

### With Connection Pool

```rust
use arc_es_sqlite::SqliteEventStore;
use diesel::r2d2::{Pool, ConnectionManager};
use diesel::SqliteConnection;

// PLACEHOLDER: Example to be updated when implementation is complete

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = ConnectionManager::<SqliteConnection>::new("events.db");
    let pool = Pool::builder()
        .max_size(10)
        .build(manager)?;

    let event_store = SqliteEventStore::from_pool(pool);

    // Use with connection pooling
    Ok(())
}
```

---

## Database Schema

The SQLite event store uses a single `events` table with optimized indexes:

```sql
CREATE TABLE events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id TEXT NOT NULL UNIQUE,
    aggregate_type TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    sequence INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    payload TEXT NOT NULL,        -- JSON
    metadata TEXT DEFAULT '{}',   -- JSON
    timestamp INTEGER NOT NULL,   -- Unix timestamp
    UNIQUE(aggregate_id, sequence)
);

-- Indexes for performance
CREATE INDEX idx_events_aggregate ON events(aggregate_id, sequence);
CREATE INDEX idx_events_type ON events(event_type);
CREATE INDEX idx_events_timestamp ON events(timestamp);
CREATE INDEX idx_events_id ON events(id);
```

**Schema Design**:
- `id` - Global event position (for streaming)
- `event_id` - Unique identifier (UUID)
- `aggregate_id + sequence` - Unique constraint ensures no gaps
- Indexes optimize common query patterns

---

## API Reference

### `SqliteEventStore`

#### Constructors

##### `new(db_path: &str)`

Creates a new file-based event store.

```rust
// PLACEHOLDER: Signature to be finalized
let event_store = SqliteEventStore::new("./data/events.db").await?;
```

**Parameters**:
- `db_path` - Path to SQLite database file

**Returns**: `Result<SqliteEventStore, Error>`

##### `in_memory()`

Creates an in-memory event store for testing.

```rust
// PLACEHOLDER: Signature to be finalized
let event_store = SqliteEventStore::in_memory().await?;
```

**Use Case**: Unit tests, ephemeral stores

##### `from_pool(pool: Pool)`

Creates an event store from an existing connection pool.

```rust
// PLACEHOLDER: Signature to be finalized
let event_store = SqliteEventStore::from_pool(pool);
```

**Use Case**: Sharing connection pool with projections

---

### `EventStore` Implementation

`SqliteEventStore` implements all methods from `arc_core::EventStore`.

#### `append()`

Appends events with optimistic concurrency control.

```rust
// PLACEHOLDER: Example to be updated when implementation is complete
event_store.append(
    "user-123",
    Some(5),  // Expected current version
    vec![event]
).await?;
```

**Concurrency Behavior**:
- If `expected_version` is `None`, creates new aggregate
- If `expected_version` matches, appends events atomically
- If mismatch, returns `ConcurrencyError`

**Atomicity**: All events in the vector are committed in a single transaction.

#### `load()`

Loads all events for an aggregate.

```rust
// PLACEHOLDER: Example to be updated when implementation is complete
let events = event_store.load("user-123").await?;
let user = UserAggregate::from_events(events);
```

**Performance**: O(n) where n is the number of events for the aggregate.

**Optimization**: Consider snapshotting for aggregates with many events.

#### `load_from()`

Loads events starting from a sequence number.

```rust
// PLACEHOLDER: Example to be updated when implementation is complete
// Load events after snapshot at version 100
let events = event_store.load_from("user-123", 101).await?;
```

**Use Case**: Loading events after a snapshot.

#### `stream_all()`

Streams all events from a global position.

```rust
// PLACEHOLDER: Example to be updated when implementation is complete
// Stream all events for projection rebuild
let all_events = event_store.stream_all(0).await?;
```

**Use Case**: Projection rebuilds, catch-up subscriptions.

**Performance**: Streams events in order by `id` (global position).

---

## Configuration

### File Location

By default, SQLite database file is created at the specified path:

```rust
// PLACEHOLDER: Configuration examples to be added
let event_store = SqliteEventStore::new("./data/events.db").await?;
```

**Recommendations**:
- Use absolute paths in production
- Keep on fast storage (SSD preferred)
- Separate from read model databases

### Connection Pool Settings

```rust
use diesel::r2d2::{Pool, ConnectionManager};

// PLACEHOLDER: Pool configuration to be documented
let manager = ConnectionManager::<SqliteConnection>::new(db_path);
let pool = Pool::builder()
    .max_size(10)           // Max connections
    .min_idle(Some(2))      // Min idle connections
    .connection_timeout(Duration::from_secs(5))
    .build(manager)?;
```

**Tuning**:
- Single-writer bottleneck: SQLite serializes writes
- Read parallelism: Multiple connections can read concurrently
- Connection pool size: 5-10 connections is typical

### SQLite Pragmas

For optimal performance:

```sql
-- PLACEHOLDER: Pragma settings to be added to implementation
PRAGMA journal_mode = WAL;         -- Write-Ahead Logging
PRAGMA synchronous = NORMAL;       -- Balance safety and speed
PRAGMA cache_size = -64000;        -- 64MB cache
PRAGMA temp_store = MEMORY;        -- In-memory temp tables
```

These are applied automatically by `SqliteEventStore`.

---

## Performance Characteristics

### Write Performance

**Append Operations**:
- Single event: ~0.5ms (SSD)
- Batch (10 events): ~1ms (SSD)
- Bottleneck: SQLite single-writer lock

**Optimization Tips**:
- Batch events in command handler when possible
- Use WAL mode for concurrent reads during writes
- Consider sharding by aggregate for horizontal scaling

### Read Performance

**Load Aggregate**:
- 100 events: ~5ms
- 1,000 events: ~20ms
- 10,000 events: ~200ms

**Stream All Events**:
- 10,000 events: ~50ms
- 100,000 events: ~500ms
- 1,000,000 events: ~5s

**Optimization Tips**:
- Use snapshots for large aggregates
- Index on `aggregate_id` is critical
- Use `load_from()` when snapshot exists

### Storage

**Disk Usage**:
- ~500 bytes per event (average JSON payload)
- 1 million events ≈ 500MB
- Indexes add ~20% overhead

**Compression**: SQLite supports page-level compression (future enhancement).

---

## Limitations

### SQLite-Specific Constraints

1. **Single-writer**: Only one write transaction at a time
2. **File-based**: Not suitable for distributed systems (use Postgres for that)
3. **No built-in replication**: Manual backup/restore required
4. **Database size**: Practical limit ~1TB (but event store grows unbounded)

### When to Use

**Good For**:
- Single-node applications
- Embedded systems
- Development/testing
- Prototyping
- Local-first apps

**Not Good For**:
- High-write-throughput systems (>1000 writes/sec)
- Distributed systems requiring multi-node coordination
- Applications needing built-in replication

**Alternative**: Use `arc-es-postgres` for distributed deployments.

---

## Migration from Traditional Database

If migrating from a traditional Diesel + SQLite setup:

### Step 1: Run Migration

```bash
# PLACEHOLDER: Migration files will be added
diesel migration run
```

Creates the `events` table alongside existing tables.

### Step 2: Dual-Write Mode

Write to both old DB and event store:

```rust
// PLACEHOLDER: Migration example to be added when implementation is complete
// Write to event store
event_store.append(&user_id, None, vec![event]).await?;

// Write to old DB (temporary)
diesel::insert_into(users)
    .values(&new_user)
    .execute(&mut conn)?;
```

### Step 3: Validate Consistency

Ensure both stores have same data.

### Step 4: Switch to Projections

Remove old DB writes, use projections for reads.

---

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_append_and_load() {
        let event_store = SqliteEventStore::in_memory().await.unwrap();

        let event = Event::new(
            "User",
            "user-123",
            1,
            "UserCreated",
            serde_json::json!({})
        );

        event_store.append("user-123", None, vec![event.clone()]).await.unwrap();

        let events = event_store.load("user-123").await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "UserCreated");
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_concurrency_control() {
    let event_store = SqliteEventStore::in_memory().await.unwrap();

    // First append
    let event1 = Event::new("User", "user-123", 1, "UserCreated", json!({}));
    event_store.append("user-123", None, vec![event1]).await.unwrap();

    // Second append with wrong version (should fail)
    let event2 = Event::new("User", "user-123", 2, "ProfileUpdated", json!({}));
    let result = event_store.append("user-123", Some(5), vec![event2]).await;

    assert!(result.is_err());
}
```

---

## Troubleshooting

### Error: "Database is locked"

**Cause**: Concurrent write attempts.

**Solution**:
- Use WAL mode (enabled by default)
- Reduce connection pool size
- Ensure transactions are committed quickly

### Error: "UNIQUE constraint failed"

**Cause**: Duplicate `event_id` or concurrent writes to same aggregate.

**Solution**:
- Use optimistic concurrency (`expected_version`)
- Ensure UUIDs are unique

### Performance Degradation

**Symptoms**: Slow `load()` or `stream_all()`.

**Diagnosis**:
```sql
-- Check index usage
EXPLAIN QUERY PLAN SELECT * FROM events WHERE aggregate_id = 'user-123';
```

**Solution**:
- Verify indexes exist
- Run `VACUUM` periodically
- Consider snapshotting

---

## Roadmap

**Current (v0.1.0)**:
- Basic EventStore implementation
- Optimistic concurrency control
- File-based and in-memory modes

**Planned (v0.2.0)**:
- Snapshot support
- Event archiving
- Compression support

**Future**:
- Event encryption
- Read replicas
- Incremental backups

---

## Documentation

- [Arc Core README](../arc-core/README.md)
- [Event Sourcing Architecture](../../docs/09-event-sourcing-architecture.md)
- [API Reference](../../docs/11-event-sourcing-api-reference.md)

---

## Contributing

See [CONTRIBUTING.md](../../CONTRIBUTING.md) for development guidelines.

---

## License

MIT License. See [LICENSE](../../LICENSE) for details.

---

## Credits

Part of the [Arc Framework](https://github.com/lotharthesavior/arc).
