# AGENTS.md

## Purpose

Operational guide for agents working in this repository. Keep changes aligned with the current Rust workspace, event-sourced architecture, and roadmap.

## Current State

- Rust workspace with five active crates:
  - `crates/arc-core`: event sourcing primitives, aggregates, command bus, event bus traits, projections, read-model traits, audit/access/session/integrity primitives.
  - `crates/arc-es-sqlite`: SQLite event store, read-model store, snapshot persistence, and JWT session revocation store.
  - `crates/arc-es-nats`: NATS JetStream `EventBus` implementation.
  - `crates/arc-app`: Actix Web application, Tera templates, Vite/Tailwind assets, auth, API/admin routes, user domain wiring.
  - `crates/arc-worker`: durable JetStream consumer that drives `ProjectionEngine`.
- Backend framework: Actix Web.
- Write model: command handlers and aggregates through `CommandBus`.
- Event store/read model: SQLite by default; Postgres is the next planned driver.
- Server-side rendering: Tera templates in `crates/arc-app/src/resources/views/`.
- Frontend assets: Vite, Tailwind, Stimulus, Turbo, Toastify.
- Auth modes:
  - Session/cookie auth for HTML/admin routes.
  - JWT bearer auth for `/api/*`.
- Realtime: WebSocket support under `crates/arc-app/src/websocket/`.
- Distributed event lane: optional NATS JetStream publishing plus `arc-worker` durable consumption.

Important: older planning docs can lag behind code. If docs conflict, prefer current source and the source-of-truth order below.

## Repository Map

- `Cargo.toml`: workspace members and shared dependency versions.
- `crates/arc-core/src/`: ES interfaces and framework primitives.
- `crates/arc-es-sqlite/src/`: SQLite implementations.
- `crates/arc-es-nats/src/`: NATS JetStream event bus.
- `crates/arc-app/src/main.rs`: app entrypoint and command dispatch.
- `crates/arc-app/src/commands/`: serve, migrate, seed, develop commands.
- `crates/arc-app/src/routes.rs`: HTML/API/admin scopes, static asset serving, middleware wiring.
- `crates/arc-app/src/http/controllers/`: HTTP handlers.
- `crates/arc-app/src/http/middlewares/`: auth, JWT, idle timeout, rate limiting.
- `crates/arc-app/src/helpers/`: sessions, CSRF, templates, forms, JWT, ES stack assembly.
- `crates/arc-app/src/domain/user/`: User aggregate, commands, projector.
- `crates/arc-app/src/resources/`: CSS, JS, images, Tera views.
- `crates/arc-worker/src/`: worker configuration, NATS consumer setup, message processing loop.
- `migrations/`: Diesel SQL migrations.
- `docs/`: browsable project docs.
- `docsify-docs/`: secondary docsify-oriented docs set; avoid updating both unless explicitly required.
- `todo.md`: current refactor status and recommended next work.
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
- Snapshot infrastructure exists, but production User currently uses the default disabled policy and rehydrates from zero.

### Event Bus / Worker

- `InProcessEventBus` is the default local synchronous path.
- `arc-es-nats` publishes persisted events to JetStream when NATS mode is selected.
- `arc-worker` owns durable JetStream projection delivery in distributed mode.
- NATS-backed tests spawn a local `nats-server -js`; CI must provision a real `nats-server` binary.

### Auth Rules

- Session auth is the primary path for browser/admin pages.
- JWT auth is the path for API clients.
- Do not weaken one auth flow while changing the other.
- `set_session_user()` stores a projection-backed `SessionUser` under the `"user"` session key; avoid adding unnecessary DB reads to authenticated HTML flows.
- Idle timeout must read `SessionUser`, not the retired `"user_id"` key.

### Database Rules

- SQLite implementations live in `arc-es-sqlite`.
- Schema changes require a Diesel migration and affected seeder/test updates.
- Step 5 should add Postgres stores behind existing `EventStore` and `ReadModelStore` traits instead of changing domain code.

### Frontend Rules

- Tera templates are the rendering layer.
- Asset references are injected through template helpers and the Vite manifest.
- Public assets are served from `/public/*` via `dist/`.
- Preserve hashed asset caching behavior in `crates/arc-app/src/routes.rs`.

### Testing Rules

- Existing tests rely on SQLite and some use serial execution.
- Auth/profile tests can touch migrations, seeders, session middleware, CSRF behavior, and projections.
- If you change auth, forms, migrations, projection behavior, or session behavior, update or add focused tests.
- If you change NATS/worker behavior, cover publish/consume, redelivery, durable consumer setup, and ACK/NAK lifecycle.

## Current Priorities

### Immediate

- Step 5: add Postgres event/read-model stores and `DATABASE_DRIVER=sqlite|postgres`.
- Continue HIPAA-2b: compile-time or mechanical guarantee that regulated read controllers call `record_read`.
- Reconcile high-level docs that still describe the old MVC-only layout.

### Near-Term

- Decide whether to archive or actively maintain `docsify-docs/`.
- Improve audit checks for stale paths and CI assumptions.
- Tighten controller/service/domain boundaries as more aggregates are added.

### Strategic

- Introduce plugin/hook system.
- Make core/storage crates publishable.
- Broaden distributed architecture beyond the current JetStream projection lane.

## Known Gaps And Risks

- HIPAA-5 integrity primitives exist, but event-store signature persistence/enforcement is deferred.
- Snapshot infrastructure exists, but User production wiring does not currently create snapshots.
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
2. `todo.md`
3. `docs/roadmap.md`
4. Root `AGENTS.md`
5. Current guides in `docs/guides/`
6. Older planning notes under `docs/ark/`, `docs/plans/`, `docs/planning/`, and historical root notes
