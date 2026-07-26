# AGENTS.md

## Purpose

Operational guide for agents working in this repository. Keep changes aligned with the current Rust workspace, event-sourced architecture, and roadmap.

## Current State

- Rust workspace with five active crates:
  - `crates/arc-core`: event sourcing primitives, aggregates, command bus, event bus traits, projections, read-model traits, audit/access/session/integrity primitives.
  - `crates/arc-es-sqlite`: SQLite event store, read-model store, snapshot persistence, and JWT session revocation store.
  - `crates/arc-es-postgres`: Postgres event store, read-model store, and snapshot persistence (self-initializing schema).
  - `crates/arc-es-nats`: NATS JetStream `EventBus` implementation (the publish side).
  - `crates/arc-app`: Actix Web application, Tera templates, Vite/Tailwind assets, auth, API/admin routes, user domain wiring.
- Backend framework: Actix Web.
- Write model: command handlers and aggregates through `CommandBus`.
- Event store/read model: SQLite by default; Postgres available via `DATABASE_DRIVER=postgres`.
- Server-side rendering: Tera templates in `crates/arc-app/src/resources/views/`.
- Frontend assets: Vite, Tailwind, Stimulus, Turbo, Toastify.
- Auth modes:
  - Session/cookie auth for HTML/admin routes.
  - JWT bearer auth for `/api/*`.
- Realtime: WebSocket support under `crates/arc-app/src/websocket/`.
- Distributed event lane: `arc-es-nats` publishes persisted events to NATS JetStream; **Benthos (Redpanda Connect)** is the single routing and event-handler delivery layer that consumes `events.>`. See `docs/adr/0001-benthos-only-event-routing.md` and `docs/guides/event-handlers.md`. (The earlier `arc-worker` consumer crate has been removed; treat any reference to it as historical.)

Important: older planning docs can lag behind code. If docs conflict, prefer current source and the source-of-truth order below.

## Repository Map

- `Cargo.toml`: workspace members and shared dependency versions.
- `crates/arc-core/src/`: ES interfaces and framework primitives.
- `crates/arc-es-sqlite/src/`: SQLite implementations.
- `crates/arc-es-nats/src/`: NATS JetStream event bus (publish side).
- `crates/arc-es-postgres/src/`: Postgres event/read-model/snapshot stores.
- `crates/arc-app/src/main.rs`: app entrypoint and command dispatch.
- `crates/arc-app/src/commands/`: serve, migrate, seed, develop commands.
- `crates/arc-app/src/routes.rs`: HTML/API/admin scopes, static asset serving, middleware wiring.
- `crates/arc-app/src/http/controllers/`: HTTP handlers.
- `crates/arc-app/src/http/middlewares/`: auth, JWT, idle timeout, rate limiting.
- `crates/arc-app/src/helpers/`: sessions, CSRF, templates, forms, JWT, ES stack assembly.
- `crates/arc-app/src/domain/user/`: User aggregate, commands, projector.
- `crates/arc-app/src/resources/`: CSS, JS, images, Tera views.
- `config/benthos/`: Benthos (Redpanda Connect) routing pipeline(s) that consume `events.>`.
- `config/handlers/`: event-handler manifests (planned) compiled into Benthos pipelines.
- `migrations/`: Diesel SQL migrations.
- `docs/`: browsable project docs.
- `docsify-docs/`: secondary docsify-oriented docs set; avoid updating both unless explicitly required.
- `progress.md`: canonical project status, remaining work, and roadmap.
- `todo-audit.md`: audit follow-up checklist when present.

## Preferred Commands

- Setup: `make setup`
- Start dev mode: `make dev`
- Run server only: `make serve`
- Run migrations: `make migrate`
- Seed database: `make seed`
- Full DB setup: `make db-setup`
- Tests: `make test`
- Fast compile check: `make check`
- Lint: `make lint`
- Format: `make format`
- Upgradeability drift guard: `make doctor` (alias: `make arc-check`)
- Frontend build: `make frontend-build`

`make test` and `make lint` should match CI-level workspace/all-features coverage.

## Architecture Rules

### Request Handling

- Public HTML routes and admin HTML routes are server-rendered with Tera.
- Admin routes live under `/admin` and require session auth.
- API routes live under `/api`; protected API routes use JWT middleware.
- HTML form POSTs must preserve CSRF protection.

### Event Sourcing

- Writes should go through `CommandBus` and aggregate commands.
- Read models are projection outputs, not authoritative write state.
- `users_view` is maintained by `UserProjector`.
- Event log remains the source of truth.
- Snapshot infrastructure is active: `UserAggregate` serializes/restores snapshots and the app command-bus wiring uses `USER_SNAPSHOT_INTERVAL_EVENTS` (default 50), so user writes create best-effort snapshots at a configurable interval.

### Event Bus / Routing

- `InProcessEventBus` is the default local synchronous path (`EVENT_BUS=inprocess`); it drives projections in the writer process and is read-after-write consistent.
- `arc-es-nats` publishes persisted events to JetStream when `EVENT_BUS=nats` is selected. The writer's responsibility ends at append + publish.
- **Benthos (Redpanda Connect)** is the single durable consumer of `events.>` in distributed mode: it owns routing, filtering, dedupe, retries, dead-lettering, and handler delivery. There is no Rust consumer of `events.>`.
- Benthos must never write directly to Arc databases. Projection writes must run through Arc-owned code paths (for example an internal HTTP projection endpoint/service that uses `Projector`/`ProjectionEngine`/`ReadModelStore`).
- Event handlers are external to Arc. Add one with a handler manifest (`config/handlers/<name>.yaml`) that the generator compiles into a Benthos pipeline — never by editing a Rust crate. See `docs/guides/event-handlers.md`.
- NATS-backed tests spawn a local `nats-server -js`; CI must provision a real `nats-server` binary.

### Auth Rules

- Session auth is the primary path for browser/admin pages.
- JWT auth is the path for API clients.
- Do not weaken one auth flow while changing the other.
- `set_session_user()` stores a projection-backed `SessionUser` under the `"user"` session key; avoid adding unnecessary DB reads to authenticated HTML flows.
- Idle timeout must read `SessionUser`, not the retired `"user_id"` key.

### Database Rules

- SQLite implementations live in `arc-es-sqlite`; Postgres implementations live in `arc-es-postgres`.
- Schema changes require a Diesel migration and affected seeder/test updates (Postgres self-initializes its schema in `build_stores`).
- New storage backends go behind the existing `EventStore` and `ReadModelStore` traits instead of changing domain code.

### Frontend Rules

- Tera templates are the rendering layer.
- Asset references are injected through template helpers and the Vite manifest.
- Public assets are served from `/public/*` via `dist/`.
- Preserve hashed asset caching behavior in `crates/arc-app/src/routes.rs`.

### Testing Rules

- Existing tests rely on SQLite and some use serial execution.
- Auth/profile tests can touch migrations, seeders, session middleware, CSRF behavior, and projections.
- If you change auth, forms, migrations, projection behavior, or session behavior, update or add focused tests.
- If you change NATS publishing behavior, cover publish acks, subject naming (`events.<aggregate_type>.<event_type>`, snake_case), and event serialization. Routing/consumer behavior lives in Benthos pipelines (`config/benthos/`), validated with `benthos lint` and routing integration tests. Projection integration tests should prove Benthos calls Arc-owned projection code; do not test or introduce Benthos SQL/database writes.
- Docker-backed tests must use project-scoped names and labels, for example `arc-nineties-*` plus `arc.project=nineties`, and must remove their containers on every success, skip, timeout, and failure path. Never leave anonymous NATS/Benthos test containers running.

## Current Priorities

### Immediate

- Land `config/benthos/` routing pipelines and the handler-manifest → Benthos-config generator (`make benthos-config`), plus a `benthos lint` CI gate. See `docs/adr/0001-benthos-only-event-routing.md`.
- Reconcile high-level docs that still describe the old MVC-only layout.

### Near-Term

- Decide whether to archive or actively maintain `docsify-docs/`.
- Improve audit checks for stale paths and CI assumptions.
- Tighten controller/service/domain boundaries as more aggregates are added.

### Strategic

- Introduce plugin/hook system.
- Make core/storage crates publishable.
- Broaden the Benthos routing plane (more HTTP/NATS handler manifests, richer DLQ/redrive tooling) without adding database-writing Benthos outputs.

## Known Gaps And Risks

- High-level docs still contain historical MVC-era wording.
- Duplicate/old planning documents can be mistaken for current truth.
- NATS integration tests require a `nats-server` binary to exercise live JetStream behavior; otherwise they skip.

## Change Guidance

- Prefer small, reversible changes unless the task explicitly requires restructuring.
- For auth/security work, verify session, CSRF, JWT, and audit behavior together.
- For template work, verify Tera rendering and asset manifest assumptions.
- For schema work, update migrations, seeders, and tests in one pass.
- For roadmap/documentation work, update the source-of-truth docs instead of adding another planning document.
- Do not commit unless the user explicitly asks or approves.

## Source Of Truth Order

When sources disagree, use this order:

1. Current source code in `crates/` and `migrations/`
2. Accepted ADRs under `docs/adr/` (e.g. `0001-benthos-only-event-routing.md`)
3. `progress.md`
4. Root `AGENTS.md`
5. Current guides in `docs/guides/`
6. Older planning notes under `docs/ark/`, `docs/plans/`, `docs/planning/`, and historical root notes
