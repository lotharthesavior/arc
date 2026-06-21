# arc-es-nats

NATS JetStream event bus adapter for Arc event sourcing.

This crate implements Arc's `EventBus` publishing side on NATS JetStream. It publishes persisted
Arc events to subject names derived from their aggregate and event types.

## What It Provides

- `NatsEventBus`: JetStream-backed `EventBus` publisher.
- Subject convention: `events.<aggregate_type>.<event_type>` using snake_case.
- Idempotent stream creation for the configured event stream and `events.>`.
- Automatic `ARC_DLQ` stream creation for `dlq.>` so Benthos dead-letter messages persist.

Arc uses Benthos/Redpanda Connect as the durable consumer and routing plane. This crate publishes
events; it does not implement a durable Rust consumer.

## Example

```rust
use arc_core::audit::AuditMetadata;
use arc_core::event::Event;
use arc_core::event_bus::EventBus;
use arc_es_nats::NatsEventBus;
use serde_json::json;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let bus = NatsEventBus::new("nats://127.0.0.1:4222", "EVENTS").await?;
let event = Event::new(
    "User",
    "user-123",
    1,
    "UserRegistered",
    json!({ "email": "ada@example.test" }),
)
.with_audit(AuditMetadata::system());

bus.publish(vec![event]).await?;
# Ok(())
# }
```

## Stability

This crate is pre-1.0 and follows `arc-core`'s compatibility line. Breaking changes may ship in
minor versions until `1.0`.

## License

MIT
