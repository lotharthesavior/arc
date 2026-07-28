# Database & Storage

**Last updated:** 2026-07-27

## Mental model

| Concern | Where |
|---------|--------|
| **Source of truth** | Append-only **events** table (event store) |
| **Read models** | Projection tables such as **`users_view`** |
| **Snapshots** | Optional rehydrate cache for aggregates (events still win) |
| **JWT session revocation** | SQLite registry (own file when primary driver is Postgres) |
| **Migrations** | Diesel SQL under `migrations/` (SQLite-oriented app path) |
| **Postgres schema** | Largely self-initialized by `arc-es-postgres` store builders |

There is **no** authoritative mutable `users` write table for the domain anymore (legacy tables were dropped via migrations). Cookie sign-in and profile UIs read **projections**.

## Drivers

Set `DATABASE_DRIVER`:

| Value | Implementation | `DATABASE_URL` |
|-------|----------------|----------------|
| `sqlite` (default) | `arc-es-sqlite` | Filesystem path, e.g. `database/database.sqlite` |
| `postgres` | `arc-es-postgres` (feature-gated in app) | Connection string, e.g. `postgres://…` |

See `docs/guides/postgres-setup.md` for Compose and validation notes.

JWT revocation store remains SQLite-backed today (`SESSION_DATABASE_URL` when primary is Postgres).

## Event store (concept)

Events are stored with aggregate identity, monotonic **sequence**, type, JSON payload, metadata (audit), timestamps, and optional integrity signature fields.

Writers always go through `EventStore::append` after aggregate handling (via `CommandBus`), not ad-hoc SQL inserts from controllers.

### Integrity (optional)

- `EVENT_INTEGRITY_KEY` (≥ 32 bytes) enables HMAC signing/verification.
- `EVENT_INTEGRITY_KEY_ID` labels which key signed a row (rotation-friendly).

Guide: `docs/guides/integrity-chain.md`.

## Read models

Example: **`users_view`** — rows maintained by `UserProjector` from user domain events. Auth and admin profile flows query this view.

Projection rebuild / deterministic replay is supported through core projection machinery; distributed delivery applies events via Arc-owned HTTP handlers so Benthos never runs SQL against Arc DBs.

## Snapshots

- Controlled by `USER_SNAPSHOT_INTERVAL_EVENTS` (default **50**).
- `<= 0` disables user snapshots without changing command correctness.
- Snapshots are a **best-effort cache**; missing or stale snapshots fall back to event replay.

## Migrations

```
migrations/
├── …_create_events_table/
├── …_add_hipaa_audit/
├── …_create_jwt_sessions/
├── …_create_users_view/
├── …_drop_legacy_users/
├── …_create_snapshots/
├── …_add_event_integrity_signatures/
└── …
```

```bash
make migrate
# or
cargo run -p arc -- migrate
```

Schema changes that affect storage crates need migrations **and** seeder/test updates. New backends implement the existing traits rather than forking domain code.

## Diesel’s role today

Diesel remains used for:

- Running versioned SQL migrations
- Some SQLite-oriented access paths and generated `schema.rs` helpers

It is **not** the domain write model. Domain writes are event-sourced. Postgres store code uses sqlx-oriented paths inside `arc-es-postgres`.

## Local files

| Path | Role |
|------|------|
| `database/database.sqlite` | Default app DB |
| `database/sessions.sqlite` | Default session/JWT registry when separated |
| `database/*-test*.sqlite` / e2e DBs | Tests |

All `*.sqlite` under `database/` are gitignored.

## Seed data

```bash
make seed
```

Creates the default demo user through the event-sourced path (see overview for credentials).
