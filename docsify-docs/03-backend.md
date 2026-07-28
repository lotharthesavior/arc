# Backend

**Last updated:** 2026-07-27

## Binary and CLI

The application package is **`arc`** (`crates/arc-app`). Framework CLI/server wiring lives in **`arc-web`**.

Typical commands (via Makefile wrappers preferred):

| Command | Purpose |
|---------|---------|
| `make serve` / `cargo run -p arc -- serve` | HTTP server |
| `make dev` / develop command | Hot reload (cargo-watch + Vite) |
| `make migrate` | Run Diesel migrations |
| `make seed` | Seed users via event-sourced path |
| `make test` | Workspace tests |
| `make doctor` | Boundary + Benthos config freshness |

App-owned subcommands under `crates/arc-app/src/commands/`: **migrate**, **seed**.  
Framework: **serve**, **develop** under `crates/arc-web/src/commands/`.

## Routing

Configured in `crates/arc-app/src/routes.rs`.

| Area | Paths | Auth |
|------|--------|------|
| Public HTML | `GET /`, `GET/POST /signin`, `GET /signout` | Public / session |
| Admin HTML | `/admin`, `/admin/settings`, `/admin/profile`, password POST | Session + idle timeout |
| API v1 | `/api/v1/register`, `/login`, `/protected/*` | JWT on protected |
| Legacy API | `/api/login`, `/api/protected/profile` | JWT (compat; prefer v1) |
| Internal | `POST /internal/projections/users/handle` | Projection bearer token |
| Static | `GET /public/*` | From `dist/` with ETag / cache tiers |
| WebSocket | `GET /ws` | Turbo Streams |
| Health | (see home/static helpers) | — |
| Diagnostics | `/__diag__/health`, `/__diag__/events/{id}` | **Only when `APP_ENV=e2e`** |

## Controllers (`arc-app`)

| Module | Responsibility |
|--------|----------------|
| `home_controller` | Landing page |
| `auth_controller` | Sign-in form/POST, sign-out |
| `admin_controller` | Dashboard, settings, profile GET/POST, password change |
| `api_controller` | Register, login, logout, profile CRUD (JWT) |
| `internal_projection_controller` | Benthos → Arc projection apply |
| `diag_controller` | E2E-only event listing / health |

Controllers orchestrate validation and **commands**; they do not mutate domain tables with Diesel CRUD for users.

## Middleware (`arc-web`)

| Middleware | Role |
|------------|------|
| Session | Cookie session |
| `AuthMiddleware` | Require session user for `/admin` |
| `IdleTimeoutMiddleware` | Expire idle admin sessions (`SessionUser`) |
| `JwtMiddleware` | Bearer JWT for protected API |
| Rate limiting | Login + global IP limits (actix-limitation / helpers) |

CSRF, template helpers, JWT issue/verify, ES stack assembly, and access/audit helpers live under `crates/arc-web/src/helpers/`.

## Domain: User

```
crates/arc-app/src/domain/user/
├── aggregate.rs    # UserAggregate state + handle/apply
├── commands.rs     # Create, update profile, change password, …
├── events.rs       # Domain events
├── projector.rs    # UserProjector → users_view
└── mod.rs
```

Writes: `CommandBus<UserAggregate>`.  
Reads: `users_view` via `ReadModelStore` / services such as `user_service` credential validation.

## Services

`crates/arc-app/src/services/user_service.rs` — credential checks and projection lookups for auth flows. Password hashing uses Argon2; hashes are stored on events / projection rows as designed by the aggregate, not via legacy `users` table CRUD.

## Seeders

`crates/arc-app/src/database/seeders/` issue domain commands (or equivalent event-sourced setup) so seeded users appear in the event log and projections.

## WebSockets

`crates/arc-web/src/websocket/` — connection handling and Turbo Stream helpers. Endpoint mounted at `/ws` from app routes.

## Logging

Prefer `tracing` / `tracing-subscriber` (workspace dependency). Structured logs over ad-hoc `println!`.

## Further reading

- `docs/guides/adding-endpoints.md`
- `docs/guides/idle-timeout.md`
- `docs/guides/session-revocation.md`
- `docs/guides/audit-metadata.md`
- `docs/guides/access-logging.md`
