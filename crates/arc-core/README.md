# arc-core

Headless event-sourcing primitives for Arc.

`arc-core` contains the framework contracts and in-memory helpers used by Arc applications and
storage adapters. It has no web framework or database dependency.

## What It Provides

- `Event`: immutable domain event envelope with audit metadata.
- `EventStore`: append/load/stream/snapshot persistence trait.
- `EventBus`: publish/subscribe trait plus in-process implementations.
- `Aggregate`: command handling and state reconstruction contract.
- `CommandBus`: load aggregate, handle command, append events, publish events.
- `ReadModelStore`: backend-neutral read-model persistence trait.
- `Projector` and `ProjectionEngine`: rebuildable projection support.
- Audit, access-log, session, snapshot, and integrity-chain primitives.

## Minimal Example

```rust
use arc_core::audit::AuditMetadata;
use arc_core::event::Event;
use serde_json::json;

let event = Event::new(
    "User",
    "user-123",
    1,
    "UserRegistered",
    json!({ "email": "ada@example.test" }),
)
.with_audit(AuditMetadata::system());

assert_eq!(event.aggregate_type, "User");
assert_eq!(event.event_type, "UserRegistered");
```

## Feature Flags

- `test-utils`: exposes in-memory/test helpers for downstream integration tests.

## Stability

This crate is pre-1.0. Public traits are intended to be stable within a compatible `0.2.x` line,
but breaking API changes may ship in minor versions until `1.0`.

## License

MIT
