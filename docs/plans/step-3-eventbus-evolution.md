# Step 3 — Event Bus Evolution: Sync/Async Split → NATS JetStream → Worker

> **Historical note (2026-06-19):** This plan predates ADR 0001. The custom
> `arc-worker` consumer path described below is superseded and removed from the
> forward architecture. Current distributed routing is Benthos-only. Benthos
> must never write directly to Arc databases; projection delivery must call an
> Arc-owned HTTP/NATS handler or service that runs `Projector` /
> `ProjectionEngine` / `ReadModelStore` code. Use
> `docs/adr/0001-benthos-only-event-routing.md`, `docs/guides/event-handlers.md`,
> `todo.md`, and `docs/roadmap.md` for current planning.

> **Status:** Design proposal (plan). No source under `crates/` changes as part of this
> document — it specifies the work for refactor Steps 3 and 4.
> **Scope:** the synchronous/asynchronous split of the event bus, the `arc-es-nats`
> JetStream backend, and the `arc-worker` consumer process.
> **Canonical source:** `docs/ark/refactor-plan.md` Steps 3–4; production risk #2
> ("`InProcessEventBus::publish` blocks the write path").

This document describes where the event bus **comes from** (today's single synchronous
in-process lane) and where it **goes to** (two lanes — a synchronous in-transaction lane
and an asynchronous side-effect lane carried over NATS JetStream and drained by an
out-of-process worker). Every claim about the current system is grounded in the code
cited inline; every claim about the target is written as a proposal ("will", "introduces").

## Table of contents

1. [Context & motivation](#1-context--motivation)
2. [As-is architecture](#2-as-is-architecture)
3. [To-be architecture](#3-to-be-architecture)
4. [Data flow / sequence](#4-data-flow--sequence)
5. [Infrastructure](#5-infrastructure)
6. [Where it comes from / where it goes to](#6-where-it-comes-from--where-it-goes-to)
7. [Implementation process / workflow](#7-implementation-process--workflow)
8. [Testing & validation](#8-testing--validation)
9. [Risks & open questions](#9-risks--open-questions)

---

## 1. Context & motivation

The write path today is `CommandBus::dispatch` in
`crates/arc-core/src/command_bus.rs`. Its seven documented steps are: load → reconstruct
(`from_events`, optionally snapshot + tail) → `handle` → stamp `AuditMetadata` → `EventStore::append`
(SQLite transaction, optimistic concurrency) → `event_bus.publish` → best-effort snapshot
(`command_bus.rs:241-368`).

Step 6 is the problem. `dispatch` calls:

```rust
// crates/arc-core/src/command_bus.rs:330
self.event_bus
    .publish(new_events.clone())
    .await
    .map_err(|source| CommandBusError::PublishFailed { aggregate_id: ..., source })?;
```

The only production `EventBus` is `InProcessEventBus`
(`crates/arc-core/src/event_bus.rs:374-404`). Its `publish` holds the handler lock and runs
**every** registered handler **inline and in subscription order**, propagating the first
error:

```rust
// crates/arc-core/src/event_bus.rs:375-397
async fn publish(&self, events: Vec<Event>) -> EventBusResult<()> {
    let handlers = self.handlers.lock().await;
    for event in &events {
        for handler in handlers.iter() {
            if handler.handles().contains(&event.event_type) {
                handler.handle(event).await.map_err(|e| {
                    EventBusError::handler_failed(/* ... */)
                })?;          // <-- first failure short-circuits the whole publish
            }
        }
    }
    Ok(())
}
```

At application startup (`crates/arc-app/src/commands/serve.rs:99-119`) the only subscribed
handler is `ProjectionEngineHandler`, which drives `ProjectionEngine` → `UserProjector` →
the `users_view` read model. So **projections currently run synchronously on the command's
write path**, inside the `publish` call, immediately after the append commit.

Two consequences follow, both flagged as production risk #2 in the refactor plan
(`docs/ark/refactor-plan.md:452`) and as the open `todo.md` line "`InProcessEventBus::publish`
blocks write path":

1. **Latency coupling.** Any handler we add for a genuine side-effect — a welcome email,
   a Stripe call, a search-index update — runs inline and blocks the HTTP response for as
   long as that side-effect takes. The `EventHandler` doc comment in
   `event_bus.rs:1-10` even names "email/Stripe/search index" as the motivating example.

2. **Consistency violation on post-commit failure.** The events are already durably
   appended (Step 5 committed the SQLite transaction) *before* `publish` runs (Step 6). If
   a handler then fails, `dispatch` returns `CommandBusError::PublishFailed` even though the
   write is permanent. The caller sees an error for a command that actually succeeded — a
   write-side/read-side consistency violation.

**Where this goes.** Per `refactor-plan.md` Steps 3–4 we split the single lane into two:

- A **synchronous lane** for work that must be consistent with the commit. The genuine
  in-transaction concern is the **integrity chain** (`crates/arc-core/src/integrity.rs`,
  HMAC-SHA256 over `prev_signature || canonical_event_bytes`), plus any handler whose
  failure *should* roll the command back.
- An **asynchronous lane** for side-effects and projections, delivered over **NATS
  JetStream** and drained by a new out-of-process **`arc-worker`**. The bus publishes to
  JetStream (fast, just a persistence ack); the worker consumes at-least-once, drives
  `ProjectionEngine`, and runs external side-effects, acking/​nacking each message.

![Current vs target architecture](diagrams/01-context-current-vs-target.png)

---

## 2. As-is architecture

**Write path (single lane).** A controller dispatches a command. `CommandBus` loads the
stream, reconstructs the aggregate, calls `handle`, stamps audit, appends inside the SQLite
transaction (`crates/arc-es-sqlite/src/lib.rs:252-332` — `spawn_blocking` + `AnsiTransactionManager`
+ optimistic concurrency via `max(sequence)`), and then publishes. Publishing is the only
post-commit work and it is synchronous.

**Event delivery.** `InProcessEventBus` keeps `Arc<Mutex<Vec<Box<dyn EventHandler>>>>`
(`event_bus.rs:327-329`). Handlers declare interest via `handles() -> Vec<String>`
(`event_bus.rs:172`) and process events through
`handle(&self, &Event) -> Result<(), Box<dyn Error + Send + Sync>>` (`event_bus.rs:193`).
Delivery is in-order, in-process, and fail-fast.

**Projections.** `ProjectionEngineHandler` (`crates/arc-core/src/projection.rs:524-548`)
adapts a `ProjectionEngine` into an `EventHandler`; its `handles()` is the union of every
registered projector's event types (`projection.rs:504-509`). At startup `serve.rs` builds
the engine with `UserProjector`, subscribes the handler to the in-process bus, and runs
`rebuild_all()` once to backfill `users_view` from the event store (`serve.rs:91-116`).
`UserProjector` is already idempotent and version-gated — re-applying a known event is a
no-op because each upsert carries the event `sequence` as `version`
(`crates/arc-app/src/domain/user/projector.rs:1-9`, and the `duplicate_event_delivery_is_idempotent`
and `out_of_order_replay_does_not_regress` tests at `projector.rs:206-264`).

**Integrity chain (not yet wired).** `IntegrityChain` is defined and unit-tested in
`integrity.rs` but, per its own module note (`integrity.rs:10-11`), wiring into
`EventStore` is deferred. It is the canonical example of a **synchronous, in-transaction**
concern: signing must happen as part of the commit so the chain can never have a gap.

The as-is shape is the left half of the [context diagram](#1-context--motivation): one lane,
everything inline, projections on the write path.

---

## 3. To-be architecture

### 3.1 The two lanes

| | **Sync lane** | **Async lane** |
|---|---|---|
| Runs | in-process, on the write path | out-of-process, after the commit |
| Carrier | direct call (or `EventStore::append` itself) | NATS JetStream → `arc-worker` |
| Failure semantics | propagates → fails `dispatch` | retried via redelivery; never fails `dispatch` |
| Members | integrity chain (in `append`); consistency-critical handlers | projections; email / Stripe / search index |
| Consistency | strongly consistent with commit | eventually consistent (at-least-once) |

The genuinely transactional member — the integrity chain — belongs **inside
`EventStore::append`**, signed within the same SQLite transaction that persists the rows.
This document keeps that placement (it is a store concern, not a bus concern) and treats
the **sync lane on the bus** as "handlers that must run synchronously and whose failure must
surface to the caller". We are explicit about the nuance: bus-level sync handlers run
*post-commit* (Step 6), so they cannot truly be transactional; anything that must be atomic
with the write goes in the store, not the bus.

![Event flow across the two lanes](diagrams/04-event-flow-two-lanes.png)

### 3.2 Trait & type design

The design is **additive to `arc-core`** and preserves today's behavior by default.

**(a) Lane classification on `EventHandler`.** Extend the existing trait with one
defaulted method — no existing impl changes, and the default reproduces current semantics
exactly:

```rust
// arc-core/src/event_bus.rs (proposed addition)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandlerLane {
    /// In-process, on the write path. Failure fails the dispatch.
    Sync,
    /// Off the write path. Delivered via the async carrier; failure is retried.
    Async,
}

pub trait EventHandler: Send + Sync {
    fn handles(&self) -> Vec<String>;
    async fn handle(&self, event: &Event) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// Default `Sync` ⇒ a handler that does not opt in behaves exactly as today.
    fn lane(&self) -> HandlerLane { HandlerLane::Sync }
}
```

Because the default is `Sync`, the current `ProjectionEngineHandler` (and every test
handler in `event_bus.rs`/`command_bus.rs`) keeps its present behavior with zero edits.

**(b) `TwoLaneEventBus` — a composite `EventBus`.** `CommandBus` holds a single
`Box<dyn EventBus>` (`command_bus.rs:194`). Rather than change `CommandBus`, introduce a
composite that *is* an `EventBus`:

```rust
// arc-core/src/event_bus.rs (proposed)
pub struct TwoLaneEventBus {
    sync: InProcessEventBus,          // Sync-lane handlers, run inline
    async_bus: Box<dyn EventBus>,     // Async carrier (NatsEventBus, or a no-op in dev)
}

#[async_trait]
impl EventBus for TwoLaneEventBus {
    async fn publish(&self, events: Vec<Event>) -> EventBusResult<()> {
        // 1. Sync lane: run inline; first error propagates → fails dispatch.
        self.sync.publish(events.clone()).await?;
        // 2. Async lane: hand off to the carrier (JetStream publish + ack).
        //    Handling (projection/side-effects) happens out-of-process.
        self.async_bus.publish(events).await
    }

    async fn subscribe(&mut self, handler: Box<dyn EventHandler>) -> EventBusResult<()> {
        match handler.lane() {
            HandlerLane::Sync  => self.sync.subscribe(handler).await,
            HandlerLane::Async => self.async_bus.subscribe(handler).await,
        }
    }
}
```

This keeps the `EventBus` trait (`event_bus.rs:224-278`) and `CommandBus` untouched: the
composite slots into the same `Box<dyn EventBus>` already passed to `CommandBus::new`
(`command_bus.rs:201`).

**(c) `NatsEventBus` — the JetStream carrier.** A new crate `crates/arc-es-nats` implements
`EventBus` using `async-nats` JetStream:

```rust
// crates/arc-es-nats/src/lib.rs (proposed)
pub struct NatsEventBus {
    jetstream: async_nats::jetstream::Context,
    stream: String,         // from NATS_STREAM, default "EVENTS"
}

impl NatsEventBus {
    /// Idempotent: get_or_create the stream covering `events.>`.
    pub async fn new(url: &str, stream: &str) -> EventBusResult<Self> { /* ... */ }
}

#[async_trait]
impl EventBus for NatsEventBus {
    async fn publish(&self, events: Vec<Event>) -> EventBusResult<()> {
        for e in &events {
            let subject = subject_for(e);                 // events.<aggregate_type>.<event_type>
            let payload = serde_json::to_vec(e)?;         // the full Event, audit included
            let ack = self.jetstream
                .publish_with_headers(subject, msg_id_header(e.event_id), payload.into())
                .await?;
            ack.await?;                                   // await PubAck = persisted on stream
        }
        Ok(())
    }
    // `subscribe` is a no-op / unsupported on the publisher side: consumption is the
    // worker's job (Step 4). It returns Ok so the composite can register Async handlers
    // as a logical declaration without binding an in-process callback.
    async fn subscribe(&mut self, _h: Box<dyn EventHandler>) -> EventBusResult<()> { Ok(()) }
}
```

**Subject scheme.** Per `CONVENTIONS.md` ("NATS subject (Step 3+): `events.<entity>.<event_type>`
lowercase, snake_case") the mapping lowercases the aggregate type and snake-cases the event
type:

```
subject_for(Event { aggregate_type: "User", event_type: "UserRegistered", .. })
    => "events.user.user_registered"
```

`Nats-Msg-Id` is set to the event's `event_id` (a UUID, `event.rs:43`) so JetStream's
publish-side dedup window collapses accidental double-publishes of the same event.

### 3.3 Where the bus splits

The split is the `TwoLaneEventBus::publish` body above:

1. **Sync lane** runs first, inline, and a failure propagates as
   `CommandBusError::PublishFailed` — preserving today's "failure surfaces to caller"
   contract for handlers that genuinely need it.
2. **Async lane** publishes each event to JetStream and awaits the `PubAck`. Only the
   *publish* is on the write path (one RTT + a stream persist); the actual *handling*
   (projection + side-effects) is fully decoupled and owned by `arc-worker`.

Projections, which are side-effect-shaped and idempotent, move to the async lane in Phase C
(see [§7](#7-implementation-process--workflow)). This is a deliberate move from
strong read-after-write consistency (single process, projection inline) to eventual
consistency (worker drains JetStream), and is called out as such.

---

## 4. Data flow / sequence

A command end-to-end in the target design:

![Command dispatch sequence](diagrams/02-command-dispatch-sequence.png)

1. Controller calls `CommandBus::dispatch(cmd, ctx)`.
2. `CommandBus` loads prior events, reconstructs the aggregate (`from_events` / snapshot+tail),
   calls `handle`, and stamps a validated `AuditMetadata` (`command_bus.rs:241-312`).
3. `EventStore::append` runs the SQLite transaction with optimistic concurrency
   (`arc-es-sqlite/src/lib.rs:279-316`). **The integrity-chain signature is computed here,
   inside the transaction** (Step-2 wiring per `integrity.rs`), so the committed rows and
   their signatures are atomic. This is the sync, in-transaction lane.
4. On commit, `CommandBus` calls `TwoLaneEventBus::publish`.
5. **Sync lane:** any consistency-critical in-process handlers run inline; an error here
   becomes `PublishFailed`.
6. **Async lane:** each event is published to `events.<type>.<event>` with
   `Nats-Msg-Id = event_id`; the bus awaits the `PubAck` confirming the event is persisted
   on the stream. `dispatch` returns; the controller responds `201`.
7. **Worker:** the `arc-worker` durable consumer (`AckExplicit`, `DeliverAll`, 30s
   `ack_wait`) receives the message, deserializes the `Event`, and calls
   `ProjectionEngine::process(&event)` (`projection.rs:406-427`).
8. **Ack/nak:** on `Ok` → `msg.ack()`; on `Err` → `msg.nak()` → redelivery after `ack_wait`.

**Guarantees and how they hold:**

- **At-least-once:** `AckExplicit` + redelivery means a message is delivered until acked.
  Crash-after-process-before-ack ⇒ redelivery ⇒ the event is re-applied.
- **Idempotency:** safe because `UserProjector` is version-gated — a re-applied event does
  not regress or duplicate state (`projector.rs:206-264`). New projectors must preserve this
  (use the store's version-gated `upsert`, never blind increments).
- **Per-aggregate ordering:** JetStream assigns a monotonic stream sequence in publish
  order. `CommandBus` appends and publishes a given aggregate's events in `sequence` order,
  and optimistic concurrency (`arc-es-sqlite/src/lib.rs:287-305`) serializes concurrent
  writers to the same aggregate. With a **single durable consumer and `max_ack_pending = 1`**
  the worker processes the stream strictly in publish order, so per-aggregate order is
  preserved. Throughput scaling (subject-per-aggregate or hashed partitions with
  per-partition serialization) is an [open question](#9-risks--open-questions).

---

## 5. Infrastructure

### 5.1 NATS JetStream service

`docker-compose.yml` already ships a JetStream-enabled NATS with monitoring on `:8222`
(`docker-compose.yml:2-14`), and the `app` service already exports `NATS_URL: nats://nats:4222`
(`docker-compose.yml:38`). A `worker` **stub** is present and explicitly waiting to be
replaced once Step 4 lands (`docker-compose.yml:55-61`). So the infra slot exists; this step
fills it.

**Stream (created idempotently in `NatsEventBus::new`):**

| Field | Value |
|---|---|
| name | `NATS_STREAM` (default `EVENTS`) |
| subjects | `events.>` |
| storage | `file` (durable across restarts) |
| retention | `Limits` (events also live in SQLite, the source of truth) |
| `duplicate_window` | a few minutes — pairs with `Nats-Msg-Id = event_id` for publish dedup |

**Durable consumer (created by `arc-worker`, per `refactor-plan.md` Step 4):**

| Field | Value |
|---|---|
| durable name | e.g. `arc-worker` |
| `ack_policy` | `AckExplicit` |
| `deliver_policy` | `DeliverAll` |
| `ack_wait` | 30s |
| `max_ack_pending` | `1` (v1, ordering-first) |
| `max_deliver` | bounded; exhausted messages routed to a DLQ subject / advisory |

### 5.2 New config

Two variables join the existing config surface (read like `crate::helpers::config::database_url()`,
`serve.rs:76`):

- `NATS_URL` — already present in compose; consumed by `NatsEventBus` and `arc-worker`.
- `NATS_STREAM` — new; stream name, default `EVENTS`.
- `EVENT_BUS` — new; `inprocess` (default, zero-infra dev) or `nats` (composite + worker).
  This is the migration/rollback switch (see [§7](#7-implementation-process--workflow)).

### 5.3 The `arc-worker` process

`crates/arc-worker` is a standalone binary (Step 4). It connects to `NATS_URL`, ensures the
durable consumer, then loops: deserialize `Event` → `projection_engine.process(&event).await`
→ `ack`/`nak`. It owns a `ProjectionEngine` built exactly like `serve.rs` does today
(`serve.rs:91-97`) but driven by JetStream delivery instead of the in-process bus. The
startup `rebuild_all()` that `serve.rs` runs (`serve.rs:112`) moves to the worker for
distributed deployments.

![Deployment / infrastructure topology](diagrams/03-deployment-infrastructure.png)

### 5.4 Dev vs prod topology & failure handling

- **Dev (`EVENT_BUS=inprocess`):** no NATS, no worker. `CommandBus` uses the in-process bus
  and projections stay synchronous and read-after-write consistent — today's behavior,
  preserved. This is the zero-infra default so `make dev` keeps working.
- **Prod (`EVENT_BUS=nats`):** `CommandBus` uses `TwoLaneEventBus` (sync lane + `NatsEventBus`);
  `arc-worker` runs as a separate process/container and owns projections + side-effects.
- **Redelivery / failure:** a failed projection `nak`s and is redelivered after `ack_wait`;
  bounded `max_deliver` prevents poison-message loops, with exhausted messages parked on a
  DLQ subject for inspection. Because SQLite remains the source of truth, an operator can
  always recover a read model by replaying from the store (`ProjectionEngine::rebuild_all`,
  `projection.rs:438-461`).

---

## 6. Where it comes from / where it goes to

A component/context view of the boundaries this step touches:

- **Upstream (unchanged):** HTTP controllers → `CommandBus<UserAggregate>` →
  `SqliteEventStore`. The append remains the transactional source of truth; the integrity
  chain signs within that transaction.
- **This component (the event bus):** changes from a single `InProcessEventBus` to a
  `TwoLaneEventBus` whose async lane is a `NatsEventBus` publishing to `events.<type>.<event>`
  on the `EVENTS` JetStream.
- **Downstream (relocated):** `ProjectionEngine` + `UserProjector` → `users_view`, plus
  external side-effects, move **out of the writer process** into `arc-worker`, fed by the
  durable JetStream consumer.

The [deployment diagram](#53-the-arc-worker-process) shows the writer process, SQLite
(events / users_view / snapshots), the JetStream stream + durable consumer, and the worker,
with the dev-only in-process projection path drawn as the fallback.

---

## 7. Implementation process / workflow

Three phases, each independently shippable and cargo-gateable. Phase A needs **no new
infrastructure** and is the immediately implementable slice.

### Phase A — infra-free sync/async split (no new crates)

1. Add `HandlerLane` and the defaulted `EventHandler::lane()` to `arc-core` (default `Sync`
   ⇒ behavior unchanged; all existing handlers and tests keep passing untouched).
2. Add `TwoLaneEventBus` to `arc-core`. Its async carrier in this phase is a small
   in-process **spawned** executor (or an in-memory queue) that runs `Async`-lane handlers
   off the write path, isolating their latency and failures from `dispatch`.
3. Classify genuine side-effects (email / Stripe / search) as `Async`. **Keep projections
   `Sync` in this phase** so single-process read-after-write consistency is preserved — only
   the slow/fallible side-effects leave the write path. This directly closes the `todo.md`
   "blocks write path" item without introducing eventual consistency yet.

**Boundary:** entirely within `arc-core` + `arc-app` wiring; no NATS, no worker. Rollback is
trivial (handlers default back to `Sync`).

### Phase B — `arc-es-nats` JetStream backend crate

1. Create `crates/arc-es-nats` (new workspace member alongside `arc-core`, `arc-es-sqlite`,
   `arc-app` — `Cargo.toml:3-7`), depending on `async-nats`.
2. Implement `NatsEventBus` (`EventBus` impl, idempotent `get_or_create` stream, subject
   mapping, `Nats-Msg-Id` dedup) and `subject_for`.
3. Make `TwoLaneEventBus`'s async carrier the `NatsEventBus` when `EVENT_BUS=nats`. Events
   now flow to JetStream **in addition** to whatever the async lane already did — JetStream
   is additive; projections can still run in-process at this stage.
4. Gate the `arc-es-nats` dependency in `arc-app` behind a cargo feature (e.g. `nats`) so a
   build without NATS has zero `async-nats` footprint.

**Boundary:** new crate + one feature flag + config plumbing (`NATS_URL`, `NATS_STREAM`).

### Phase C — `arc-worker` crate (JetStream projector)

1. Create `crates/arc-worker` (standalone binary, Step 4).
2. Durable consumer (`AckExplicit`, `DeliverAll`, 30s `ack_wait`, `max_ack_pending=1`),
   consume loop driving `ProjectionEngine::process`, `ack`/`nak`.
3. **Cutover:** when `EVENT_BUS=nats`, `serve.rs` stops subscribing `ProjectionEngineHandler`
   to the in-process bus (`serve.rs:99-105`); the worker owns projections. The read path
   becomes eventually consistent — document this in the relevant guide.
4. Replace the compose `worker` stub (`docker-compose.yml:55-61`) with the real binary.

**Boundary:** new binary crate; the `EVENT_BUS` switch flips ownership of projections.

### Migration, compatibility & rollback

- **Default stays `inprocess`** at every phase, so `master` behavior is the fallback and no
  deployment is forced onto NATS before it is ready.
- **Rollback** is flipping `EVENT_BUS=inprocess` (and, if needed, disabling the `nats`
  cargo feature). SQLite is the source of truth throughout, so no data is stranded.
- **Compat:** `arc-core` changes are additive (one defaulted trait method, one new type);
  `EventBus`/`CommandBus`/`EventStore` signatures are unchanged, so `EventStoreContract`
  and existing tests stay green.

![Phased rollout](diagrams/05-phased-rollout.png)

---

## 8. Testing & validation

Aligned with `refactor-plan.md` "Step 3 — QA Requirements" and the validation notes for
Steps 3–4.

### Unit (Phase A, no infra)

- Sync-lane handler error → `publish` returns `Err` → `dispatch` returns `PublishFailed`
  (preserves `event_bus.rs::test_handler_failure_propagates` semantics).
- Async-lane handler error → `publish` returns `Ok` → `dispatch` succeeds (failure is
  isolated, not fatal). Assert the side-effect was still attempted.
- `lane()` defaulting: an `EventHandler` that does not override `lane()` is treated as
  `Sync` (behavior-unchanged guarantee).
- Ordering within the sync lane is preserved (mirror `test_handler_called_in_order`).

### NATS integration (Phase B/C, Testcontainers-rs `nats:latest --jetstream`)

- **Publish/consume roundtrip:** dispatch a command → assert a message lands on
  `events.user.user_registered` and deserializes back to the original `Event` (audit
  included).
- **Idempotent stream/consumer creation:** call `NatsEventBus::new` and the worker's
  consumer setup twice → no error, same config (the refactor plan's "stream/consumer
  idempotent creation" check).
- **At-least-once redelivery:** publish, force the worker to error/crash mid-process before
  ack, restart → assert the event is processed and the final `users_view` is correct exactly
  once *in effect* (idempotent projector absorbs the duplicate).
- **Per-aggregate ordering:** publish 10 events for one `aggregate_id` in sequence → assert
  the worker applies them in order and `users_view.version` advances monotonically with no
  gaps.
- **`EventStoreContract` stays green** across stores; SQLite append/load/concurrency tests
  (`arc-es-sqlite/src/lib.rs:494-1065`) are unaffected.

### Green-bar discipline

`cargo test --workspace` and `cargo clippy --all-targets --all-features -- -D warnings`
(the gates already enforced in CI per `roadmap.md:387-389`) must stay green at every phase.
NATS integration tests are feature/marker-gated so a no-NATS `cargo test` still passes.

---

## 9. Risks & open questions

1. **Dual-write between SQLite and JetStream (highest risk).** Append (Step 5) commits to
   SQLite; the JetStream publish (Step 6) is a separate write. If the process dies between
   them, the event is durable in the store but never reaches the stream, so the worker never
   projects it. **Mitigations:** (a) the **transactional outbox** pattern — a poller reads
   the `events` table and publishes unsent rows, making SQLite the single commit point
   (recommended hardening); (b) interim reliance on `arc-worker` driving `rebuild_all` /
   `stream_all` from the store to reconcile gaps, leveraging the startup backfill that
   already exists (`serve.rs:112`). v1 may accept a small window; the outbox closes it.

2. **Read-after-write consistency regression.** Moving projections to the async lane
   (Phase C) makes a `GET` immediately after a `POST` possibly stale. This is an intentional
   architectural shift; it must be documented, and UI/flows that assume read-your-writes may
   need adjustment. Phase A deliberately keeps projections synchronous to defer this.

3. **Ordering vs throughput.** `max_ack_pending=1` guarantees per-aggregate order but caps
   throughput at one in-flight message. Subject-per-aggregate (`events.<type>.<aggregate_id>`)
   or hashed partitions with per-partition serialization would scale, at the cost of subject
   cardinality and complexity. **Open:** which partitioning to adopt, and when.

4. **Subject namespace leakage (HIPAA).** `refactor-plan.md` flags that subject names can
   reveal PHI. The scheme here uses `aggregate_type` + `event_type` (no natural keys), but if
   `aggregate_id` enters subjects for partitioning (#3) it must be an opaque UUID, never a
   natural key.

5. **DLQ / poison messages.** A deterministically failing event would redeliver forever
   without a bound. `max_deliver` + a DLQ subject are specified, but the operational
   runbook (alerting, replay-from-DLQ) is out of scope here and needs its own note.

6. **Exactly-once is not offered.** The contract is at-least-once + idempotent handlers.
   Every async handler (not just `UserProjector`) must be idempotent; this should be a
   documented requirement in `CONVENTIONS.md` when async handlers proliferate.

---

### Diagram index

All sources live in [`diagrams/`](diagrams/) as `.mmd` and render to sibling `.png`:

| Source | Rendered | Used in |
|---|---|---|
| `01-context-current-vs-target.mmd` | `01-context-current-vs-target.png` | §1 |
| `02-command-dispatch-sequence.mmd` | `02-command-dispatch-sequence.png` | §4 |
| `03-deployment-infrastructure.mmd` | `03-deployment-infrastructure.png` | §5 |
| `04-event-flow-two-lanes.mmd` | `04-event-flow-two-lanes.png` | §3 |
| `05-phased-rollout.mmd` | `05-phased-rollout.png` | §7 |
