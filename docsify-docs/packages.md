# Published Packages

Generated applications use published crates; they do not depend on an Arc repository checkout.

| Package | Purpose |
|---|---|
| `arc-web-cli` | Installs the `arc` generator command |
| `arc-core` | Commands, aggregates, events, command bus, projections, read models |
| `arc-web` | Actix runtime, server builder, middleware, and application wiring |
| `arc-es-sqlite` | SQLite event, read-model, snapshot, and session stores |
| `arc-es-postgres` | Optional Postgres stores |
| `arc-es-nats` | **Work in progress:** optional NATS JetStream event publishing; the complete NATS + Benthos distributed workflow is not ready for general use |

> **Work in progress:** NATS publishing and Benthos (Redpanda Connect) event routing are still being completed and documented. Use Arc's default in-process event bus for normal applications unless you are actively developing or testing the distributed event path.

## Installation model

The CLI writes normal Cargo dependencies:

```toml
[dependencies]
arc-core = "0.4.0"
arc-web = "0.4.0"
```

You may create the same application manually, but then you must also supply the environment bootstrap, migrations, aggregate, routes, and runtime entry point that the CLI normally generates.

## Versioning

Keep Arc packages on the same version unless release notes explicitly say otherwise. Review dependency updates like any framework upgrade and run:

```bash
make setup
make check
make test
```

## Upgrading from 0.3 to 0.4

Arc 0.4 replaces positional event constructor arguments with the named-field `NewEvent` parameter struct:

```rust
use arc_core::event::{Event, NewEvent};

let event = Event::new(NewEvent {
    aggregate_type: "Product",
    aggregate_id: product_id,
    sequence: next_sequence,
    event_type: "ProductCreated",
    payload,
});
```

Update every `Event::new(...)` call when moving an application from 0.3 to 0.4.
