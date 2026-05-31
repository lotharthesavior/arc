# AGENTS.md

## Purpose

This repository is a Rust web application starter built around Actix Web, Diesel + SQLite, Tera templates, and a Vite/Tailwind frontend pipeline. Use this file as the operational guide for agents working in the repo so changes stay aligned with the current architecture and roadmap.

## Current State

- Backend framework: Actix Web
- Database: SQLite via Diesel and Diesel migrations
- Server-side rendering: Tera templates in `src/resources/views/`
- Frontend assets: Vite, Tailwind, Stimulus, Turbo, Toastify
- Auth modes:
  - Session/cookie auth for HTML routes
  - JWT bearer auth for `/api/*`
- Realtime: WebSocket support under `src/websocket/`
- Dev tooling: `Makefile`, `cargo watch`, Vite watch build

Important: some planning docs lag behind the code. Validation scaffolding, tracing-based logging, and template caching are already implemented in source. If docs conflict with code, prefer the code and `PROGRESS.md`.

## Repository Map

- `src/main.rs`: app entrypoint, command dispatch, startup checks, tracing init
- `src/routes.rs`: route wiring, API/admin scopes, static asset serving and caching
- `src/http/controllers/`: HTTP handlers for home, auth, admin, API
- `src/http/middlewares/`: auth and JWT middleware
- `src/helpers/`: database pool, sessions, CSRF, templates, forms, JWT helpers
- `src/services/`: business logic, currently user credential/password handling
- `src/validation/`: form validation structs and validation rules
- `src/websocket/`: websocket connection/server logic
- `src/database/seeders/`: development seed data
- `migrations/`: Diesel SQL migrations
- `src/resources/`: CSS, JS, images, Tera views
- `docs/`: browsable project docs
- `docsify-docs/`: duplicate docsify-oriented docs set; avoid updating both unless necessary

## How To Work In This Repo

### Preferred Commands

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

### Expected Local Flow

1. Ensure `.env` exists. `src/main.rs` will copy `.env.example` if missing.
2. Run migrations before relying on auth or seeded users.
3. Seed if a task depends on the default login.
4. Use `make dev` when changing both Rust and frontend assets.

## Architectural Rules

### Request Handling

- Public HTML routes and admin HTML routes are server-rendered with Tera.
- Admin routes live under `/admin` and require session auth.
- API routes live under `/api`; protected API routes use JWT middleware.
- HTML form POSTs must preserve CSRF protection.

### Auth Rules

- Session auth is the primary path for browser/admin pages.
- JWT auth is an alternative for API clients.
- Do not weaken one auth flow while changing the other.
- `set_session_user()` caches user data in the session; avoid adding unnecessary DB reads to authenticated HTML flows.

### Database Rules

- Connection access goes through `src/helpers/database.rs`.
- The pool is cached and can be reset in tests; do not reintroduce per-request pool creation.
- Schema changes require a Diesel migration and any affected seeder/test updates.

### Frontend Rules

- Tera templates are the rendering layer.
- Asset references are injected through `load_template()` and the Vite manifest.
- Public assets are served from `/public/*` via `dist/`.
- Preserve hashed asset caching behavior in `src/routes.rs`.

### Testing Rules

- Existing tests rely on SQLite and some use serial execution.
- Auth/profile tests currently touch migrations, seeders, session middleware, and CSRF behavior.
- If you change auth, forms, migrations, or session behavior, update or add tests in the affected controller/helper modules.

## Current Priorities

### Immediate

- Integrate `src/validation/` into auth/admin form handlers
- Add rate limiting around login and API endpoints
- Harden session and security headers
- Expand tests around auth, profile updates, and validation failures

### Near-Term

- Reduce duplication between `docs/` and `docsify-docs/`
- Tighten controller/service boundaries
- Improve operational checks, monitoring, and performance baselines

### Strategic

- Move toward event-sourcing architecture
- Introduce plugin/hook system
- Split the project into a workspace with core/web/app crates

## Known Gaps And Risks

- Validation exists but is not fully wired into all controllers.
- Planning docs contain completed items still marked as pending.
- Documentation exists in two parallel trees.
- The app is still structurally an MVC starter even though long-term docs target event sourcing.
- Test coverage exists but is not yet broad enough for larger auth/security refactors.

## Change Guidance For Agents

- Prefer small, reversible changes unless the task explicitly requires restructuring.
- For auth or security work, verify session, CSRF, and JWT behavior together.
- For template work, verify both Tera rendering and asset manifest assumptions.
- For schema work, update migrations, seeders, and tests in one pass.
- For roadmap or documentation work, update the source-of-truth docs rather than adding a third planning document.

## Source Of Truth Order

When sources disagree, use this order:

1. Current source code in `src/`
2. `PROGRESS.md`
3. `README.md`
4. `docs/`
5. older planning notes in the repository root
