# Extending Arc (Implementation Guide)

**Last updated:** 2026-07-27

Practical guide for working **with the current codebase**. For theory and topology, see [09-event-sourcing-architecture](09-event-sourcing-architecture.md).

## Prerequisites

- Read `AGENTS.md` architecture rules  
- Know the crate split: domain in `arc-app`, framework in `arc-web` / `arc-core`  
- Prefer `make doctor` after structural moves  

## 1. Add or change a User command

1. Extend `crates/arc-app/src/domain/user/commands.rs` with a new command variant.  
2. Handle it in `aggregate.rs` (validate invariants → emit events).  
3. Add event type(s) in `events.rs` and `apply` logic.  
4. Update `UserProjector` if the read model must change (`users_view` columns / rows).  
5. Add a migration if the projection schema changes.  
6. Wire the controller/API to dispatch via `CommandBus`.  
7. Tests: unit on aggregate; integration via test helper that includes projector; e2e if user-visible.

Do **not** update `users_view` with hand-written SQL from the controller.

## 2. Scaffold a new aggregate (tooling)

```bash
scripts/new-aggregate.sh   # if present; follow its output layout
```

Typical layout mirroring User:

```text
crates/arc-app/src/domain/<name>/
  aggregate.rs
  commands.rs
  events.rs
  projector.rs
  mod.rs
```

Register the projector where the app builds the ES stack / projector list (`main.rs` / es stack wiring).

## 3. Add an HTTP endpoint

1. Controller in `crates/arc-app/src/http/controllers/`.  
2. Route in `routes.rs` (choose public, admin+session, or API+JWT).  
3. CSRF on HTML POST; JWT middleware on protected API.  
4. Writes → commands; reads → projections.  

Guide: `docs/guides/adding-endpoints.md`.

## 4. Add an external event handler (distributed)

You **do not** edit Rust routing for fan-out.

1. Implement your service (HTTP or NATS consumer).  
2. Add `config/handlers/<name>.yaml` describing match + delivery.  
3. Run `make benthos-config` (and commit or CI-check generated output per project policy).  
4. `make benthos-lint`.  
5. Deploy Benthos with NATS URL and secrets.

Contract details: `docs/guides/event-handlers.md`.  
DLQ: `docs/guides/benthos-dlq-redrive.md`.

**Forbidden:** Benthos SQL/database outputs against Arc databases.

## 5. Projection delivery in distributed mode

Benthos should call Arc-owned HTTP such as:

```http
POST /internal/projections/users/handle
Authorization: Bearer <INTERNAL_PROJECTION_TOKEN>
```

Extend internal controllers carefully; keep auth strict; reuse `Projector` / `ProjectionEngine` / `ReadModelStore`.

## 6. Switch storage driver

```bash
# SQLite (default)
DATABASE_DRIVER=sqlite
DATABASE_URL=database/database.sqlite

# Postgres (build with postgres feature as required by app Cargo.toml)
DATABASE_DRIVER=postgres
DATABASE_URL=postgres://arc:password@127.0.0.1:5432/arc_dev
# SESSION_DATABASE_URL=database/sessions.sqlite   # JWT revocation still SQLite
```

See `docs/guides/postgres-setup.md`.

## 7. Event bus selection

```bash
EVENT_BUS=inprocess   # local default
EVENT_BUS=nats        # requires NATS; Benthos for durable consumers
```

Publish subjects: `events.<aggregate_type>.<event_type>` (snake_case).

## 8. Snapshots & integrity

```bash
USER_SNAPSHOT_INTERVAL_EVENTS=50   # or <=0 to disable
EVENT_INTEGRITY_KEY=...            # >= 32 bytes
EVENT_INTEGRITY_KEY_ID=default
```

Backfill/sign existing rows before enforcing integrity on old databases.

## 9. Common pitfalls

| Pitfall | Prefer |
|---------|--------|
| Diesel insert into domain tables for User | CommandBus |
| Editing Benthos YAML by hand for handlers | Handler manifest + generator |
| Subscribing a second Rust consumer to `events.>` | Benthos only |
| Putting middleware under `arc-app` | `arc-web` + doctor |
| Documenting Alpine/HTMX as stack | Stimulus + Turbo |
| Dual-updating outdated docsify + docs without care | One accurate story |

## 10. Verification checklist

```bash
make check
make test
make doctor
make benthos-config-check   # if you touched handlers/generator
# optional:
make e2e
```
