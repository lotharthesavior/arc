# Event Sourcing — As Built

**Last updated:** 2026-07-27

This document describes the **implemented** event-sourcing architecture. For migration history and rejected alternatives, see ADR 0001 and `progress.md`. Older “MVC is current / ES is future” narratives are obsolete.

## Current state (summary)

| Component | Status |
|-----------|--------|
| `Event`, aggregate traits, `CommandBus` | Implemented in `arc-core` |
| Optimistic concurrency + audit metadata | Implemented |
| User aggregate + commands/events | Implemented in `arc-app` |
| `UserProjector` → `users_view` | Implemented |
| SQLite event / RM / snapshot / session stores | `arc-es-sqlite` |
| Postgres event / RM / snapshot stores | `arc-es-postgres` |
| In-process event bus | Default path |
| NATS JetStream publish | `arc-es-nats` |
| Durable consume + route + DLQ | **Benthos only** (not Rust) |
| User snapshots | Configurable interval |
| Event integrity signatures | Optional env-gated |
| Legacy mutable `users` table | Removed |

## Core principles (live)

1. **Events are the source of truth** — projections are derived and rebuildable.  
2. **Commands produce events** — domain writes go through `CommandBus` + aggregates.  
3. **Read models serve queries** — HTML/API use views such as `users_view`.  
4. **Traits bound storage** — domain code does not hard-code SQL dialects.  
5. **Writer responsibility ends at append + publish** — routing is someone else’s job in distributed mode.  
6. **Benthos never owns Arc DB writes** — projection apply stays in Arc-owned code.

## Write path

```text
HTTP / service
    → validate input
    → CommandBus::dispatch(command, context)
        → load aggregate (snapshot? + events)
        → aggregate.handle(command) → Vec<Event>
        → EventStore::append (optimistic concurrency)
        → EventBus::publish (in-process and/or NATS)
    → return result to controller
```

Audit context (actor, IP, causation, …) is attached through command context helpers so events carry compliance-friendly metadata.

## Read path

```text
HTTP
    → controller / service
    → ReadModelStore query (e.g. users_view by email)
    → render Tera or JSON
```

Credential verification for sign-in uses the projection (password hash stored as part of the projected user row), not a legacy ORM User model write path.

## Event bus topologies

### A. In-process

```text
append → InProcessEventBus → ProjectionEngineHandler → UserProjector → users_view
```

- Default for local development  
- Synchronous; read-after-write consistent in the writer process  

### B. Distributed

```text
append → arc-es-nats → JetStream subject events.<aggregate>.<event>
                              ↓
                     Benthos (events.>)
                     · envelope validate
                     · dedupe(event_id)
                     · route / filter
                              ↓
              ┌───────────────┴────────────────┐
              ▼                                ▼
   POST /internal/projections/…        external HTTP/NATS handlers
   (Arc projection code)               (manifest-defined)
              │
              └→ DLQ subjects on failure (see operator guide)
```

- Subject naming: `events.<aggregate_type>.<event_type>` in **snake_case**  
- Handler manifests: `config/handlers/*.yaml` → `make benthos-config`  
- ADR: `docs/adr/0001-benthos-only-event-routing.md`  
- Guide: `docs/guides/event-handlers.md`  

**Rejected:** custom Rust durable consumer (`arc-worker`) as the long-term routing plane.

## Crate map (ES-focused)

```text
arc-core          traits + command bus + projection engine + integrity/session primitives
arc-es-sqlite     SQLite EventStore, ReadModelStore, SnapshotStore, session revocation
arc-es-postgres   Postgres EventStore, ReadModelStore, SnapshotStore
arc-es-nats       JetStream publish + stream/DLQ provisioning
arc-web           wires ES stack into Actix (helpers/es_stack, serve)
arc-app           UserAggregate, projector registration, internal projection HTTP
config/benthos    pipelines
config/handlers   user handler manifests (when present)
```

## Snapshots

- Interval: `USER_SNAPSHOT_INTERVAL_EVENTS` (default 50).  
- Best-effort: command correctness does not depend on snapshot presence.  
- Serialize/restore implemented for `UserAggregate`; stores persist blobs.

## Integrity

Optional HMAC over event bytes when `EVENT_INTEGRITY_KEY` is set. Verification on load/stream depends on store implementation and key configuration. See `docs/guides/integrity-chain.md`.

## What remains aspirational

Not claiming these as shipped:

- Full plugin marketplace / `the-hook` domain event plugins  
- Multi-aggregate product domains beyond User (framework ready; app domain is User today)  
- `arc new` app generator  
- Non-SQLite JWT revocation registry  

Track those in [roadmap](roadmap.md) / `progress.md`.
