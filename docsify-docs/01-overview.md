# Arc — Project Overview

**Last updated:** 2026-07-27  
**Workspace version:** 0.2.2

## Introduction

**Arc** is a composable, event-sourced web application framework and starter built in Rust on Actix Web. Writes go through commands and aggregates into an append-only event log; HTML and API reads use projection-backed read models. The default web surface is server-rendered Tera templates with Vite/Tailwind assets and Hotwire (Stimulus + Turbo).

**Philosophy:** Spend time on your domain on top of a solid foundation — auth, sessions, event store, projections, and frontend tooling are already wired.

> Historical note: Arc started as a single-crate MVC/Diesel starter. That shape is gone. Prefer this docs set, `progress.md`, accepted ADRs under `docs/adr/`, and the code under `crates/` when anything conflicts.

## Key Features

- **Event-sourced writes** — `CommandBus`, aggregates, optimistic concurrency, audit metadata
- **Projections** — e.g. `users_view` maintained by `UserProjector` (events remain source of truth)
- **Storage adapters** — SQLite (`arc-es-sqlite`) by default; Postgres (`arc-es-postgres`) via `DATABASE_DRIVER=postgres`
- **Dual event lanes** — in-process bus for local read-after-write consistency; NATS JetStream publish (`arc-es-nats`) for distributed mode
- **Benthos routing** — Redpanda Connect is the sole durable consumer of `events.>`; handler manifests compile into pipelines (ADR 0001)
- **Auth** — cookie/session for HTML/admin; JWT bearer for `/api/*`; idle timeout, rate limits, session revocation
- **SSR admin app** — dashboard, profile, settings with CSRF-protected forms
- **WebSockets** — Turbo Streams support under `/ws`
- **Snapshots & integrity** — best-effort aggregate snapshots; optional HMAC event integrity signatures
- **Tooling** — Diesel migrations, seeders, Vite asset pipeline, Playwright e2e, `make doctor` upgradeability checks

## Technology Stack

### Backend

| Area | Choice |
|------|--------|
| HTTP | Actix Web 4 |
| Templates | Tera 1.20 |
| Sessions | actix-session (cookie) |
| Passwords | Argon2 |
| JWT | jsonwebtoken |
| Validation | validator |
| Async | Tokio |
| Logging | tracing |
| Migrations | Diesel + diesel_migrations |
| Postgres path | sqlx (in `arc-es-postgres`) |
| Event bus (distributed) | async-nats / JetStream |

### Frontend

| Area | Choice |
|------|--------|
| CSS | Tailwind CSS 3 |
| JS | Stimulus 3 + Turbo 8 |
| Notifications | Toastify |
| Build | Vite 6 |

### Infrastructure (optional distributed)

| Area | Choice |
|------|--------|
| Message bus | NATS JetStream |
| Routing / handlers | Benthos (Redpanda Connect) |
| Local compose | `docker-compose.yml` |

## Workspace Crates

```
crates/
├── arc-core          # ES primitives: Event, Aggregate, CommandBus, EventBus traits,
│                     # projections, read models, audit, session, snapshot, integrity
├── arc-es-sqlite     # SQLite event / read-model / snapshot / JWT-session stores
├── arc-es-postgres   # Postgres event / read-model / snapshot stores
├── arc-es-nats       # NATS JetStream EventBus (publish + stream provisioning)
├── arc-web           # Reusable Actix runtime: middleware, helpers, serve/develop CLI,
│                     # WebSockets
└── arc-app           # Thin application: User domain, routes, controllers, templates,
                      # assets, seeders (package name: arc)
```

Durable event **routing** is not a Rust crate: it lives in `config/benthos/` and is generated from `config/handlers/*.yaml` via `make benthos-config`.

## Quick Start

```bash
# Install deps and prepare env (see Makefile / README)
make setup          # or: npm install && copy .env.example → .env

# Database
make migrate
make seed

# Frontend assets
make frontend-build   # or: npm run build

# Development (cargo-watch + Vite)
make dev
# or production-style binary only:
make serve
```

Useful checks:

```bash
make check          # cargo check --workspace --all-features
make test           # workspace tests
make doctor         # boundary + generated Benthos freshness (alias: make arc-check)
make e2e            # Playwright (requires setup)
```

## Default Seed User

After `make seed` (or `cargo run -p arc -- seed`):

- **Email:** `jekyll@example.com`
- **Password:** `password`

## System Requirements

- Rust (stable)
- Node.js + npm
- SQLite development libraries (default driver)
- `cargo-watch` for `make dev`
- Optional: `nats-server -js`, Benthos/Redpanda Connect, Postgres for distributed/Postgres paths

### Ubuntu/Debian (typical)

```bash
sudo apt install build-essential libssl-dev libsqlite3-dev
cargo install cargo-watch
```

## Repository Map

```
.
├── crates/                 # Workspace members (see above)
├── config/
│   ├── benthos/            # Hand-written + generated pipelines
│   └── handlers/           # Handler manifests → make benthos-config
├── migrations/             # Diesel SQL migrations
├── database/               # Local SQLite files (gitignored)
├── dist/                   # Vite build output (gitignored)
├── scripts/                # benthos generator, arc-check, aggregate scaffold, …
├── tests/e2e/              # Playwright specs
├── docs/                   # Canonical markdown docs + ADRs + guides
├── docsify-docs/           # This browsable docsify set (kept in sync with current code)
├── progress.md             # Project status and roadmap tracker
├── AGENTS.md               # Agent/operator guide
├── Makefile
├── docker-compose.yml
└── package.json
```

## Source of Truth Order

When documents disagree:

1. Code under `crates/` and `migrations/`
2. Accepted ADRs in `docs/adr/`
3. `progress.md`
4. Root `AGENTS.md`
5. Guides in `docs/guides/`
6. Older planning notes (historical only)

## License

MIT — see workspace `Cargo.toml` / repository license.
