# nineties-core

Core event sourcing primitives for the Nineties framework.

## Status

🚧 **Under Development** - Implementation planned for Q1 2026

## Overview

`nineties-core` is the foundational crate providing event sourcing, CQRS, and composability primitives for the Nineties framework. It has **zero web dependencies** and can be used standalone or with the `nineties-web` plugin.

## Key Components

- **EventStore** - Append-only event storage with optimistic concurrency
- **EventBus** - Publish/subscribe event distribution
- **Aggregate** - Domain logic encapsulation with command handling
- **CommandBus** - Command dispatching and coordination
- **Projection** - Read model materialization from event streams
- **ProjectionEngine** - Multi-projection management and rebuilding

## Documentation

Before implementing features in this crate, please review:

1. **[DX_GUIDELINES.md](./DX_GUIDELINES.md)** - Developer experience patterns and best practices
2. **[DX_REVIEW_FEEDBACK.md](./DX_REVIEW_FEEDBACK.md)** - Priority recommendations from DX review
3. **[Architecture Docs](../../docs/09-event-sourcing-architecture.md)** - System design and concepts
4. **[Implementation Guide](../../docs/10-event-sourcing-implementation-guide.md)** - Phase-by-phase implementation plan

## Design Principles

1. **Events are immutable facts** - The event store is the source of truth
2. **Type safety over convenience** - Use the type system to prevent mistakes
3. **Progressive disclosure** - Simple path for simple cases, full CQRS for complex domains
4. **Explicit over implicit** - No magic, clear control flow
5. **Helpful errors** - Every error should guide developers to a solution

## Quick Start (Planned API)

### Simple Event Store Usage

```rust
use nineties_core::{Event, EventStore, SqliteEventStore};

let event_store = SqliteEventStore::new("events.db").await?;

let event = Event::new(
    "User",
    "user-123",
    1,
    "UserCreated",
    json!({"name": "Alice", "email": "alice@example.com"}),
);

event_store.append("user-123", None, vec![event]).await?;
```

### Full CQRS with Aggregates

```rust
use nineties_core::{Aggregate, CommandBus, EventBus, EventStore};

// Define domain types
pub enum UserCommand {
    CreateUser { id: String, name: String, email: String },
}

pub struct UserAggregate {
    // Aggregate state
}

impl Aggregate for UserAggregate {
    type Command = UserCommand;
    type Event = UserEvent;
    type Error = UserError;

    async fn handle(&self, command: Self::Command) -> Result<Vec<Event>, Self::Error> {
        // Validate and produce events
    }

    fn apply(&mut self, event: &Event) {
        // Update state from events
    }
}

// Wire up infrastructure
let event_store = Box::new(SqliteEventStore::new("events.db").await?);
let event_bus = Box::new(InProcessEventBus::new());
let mut command_bus = CommandBus::<UserAggregate>::new(event_store, event_bus);

// Execute commands
let events = command_bus.dispatch(UserCommand::CreateUser { ... }).await?;
```

## Testing

Use the provided test utilities for easy aggregate testing:

```rust
use nineties_core::testing::AggregateFixture;

#[tokio::test]
async fn test_user_creation() {
    AggregateFixture::<UserAggregate>::new()
        .when(UserCommand::CreateUser { ... })
        .await
        .then_expect_events(vec![/* expected events */]);
}
```

## Contributing

When adding features to this crate:

1. Follow naming conventions from DX_GUIDELINES.md
2. Add comprehensive error messages with context
3. Include doc comments with working examples
4. Write tests using the testing utilities
5. Run `cargo test --doc` to verify examples compile

## Dependencies

Minimal dependencies for maximum compatibility:

- `async-trait` - Async trait support
- `serde` + `serde_json` - Serialization
- `uuid` - Event IDs
- `thiserror` - Error handling

See `Cargo.toml` for complete list.

## License

MIT
