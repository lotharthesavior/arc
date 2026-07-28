# Testing

**Last updated:** 2026-07-27

## Layers

| Layer | How |
|-------|-----|
| Unit / integration (Rust) | `cargo test --workspace --all-features` or `make test` |
| Compile check | `make check` |
| Lint / format | `make lint`, `make format` |
| Doctor / drift | `make doctor` (crate boundaries, generated Benthos config) |
| Benthos | `make benthos-config-check`, `make benthos-lint` |
| E2E | Playwright: `make e2e` / `npx playwright test` |

## Rust tests

- Prefer SQLite for most tests; some use `serial_test` for shared env/files.
- Auth and profile tests often exercise migrations, seeders, session middleware, CSRF, and projections together.
- Shared ES scaffolding lives under `crates/arc-app/src/helpers/test.rs` — builds `CommandBus` + in-process bus with `UserProjector` so commands land in `users_view` before assertions.

### NATS

- `arc-es-nats` integration tests may spawn `nats-server -js`.
- If the binary is missing, tests should **skip** rather than fail spuriously.
- CI must provision a real `nats-server` when exercising JetStream.

### Benthos / projection routing

- Integration coverage exists for publish → Benthos → Arc HTTP projection paths.
- Assert Benthos calls Arc-owned projection code; **do not** introduce Benthos SQL/database writes.
- Docker-backed tests should use project-scoped names/labels (e.g. `arc-nineties-*`, `arc.project=nineties`) and remove containers on every path (success, skip, timeout, failure).

## Playwright e2e

Config: `playwright.config.ts` (canonical).

```bash
make e2e-install    # install @playwright/test + Chromium
make e2e            # headless
make e2e-headed
make e2e-report
```

Specs live under `tests/e2e/` (UI + API flows: sign-in, profile, session, websocket, …).  
E2E env: `.env.e2e` (deterministic secrets for local/CI only).

Diagnostics (`/__diag__/*`) mount only when `APP_ENV=e2e`.

Details: `docs/testing/e2e.md`.

## What to update when you change…

| Change | Update tests for |
|--------|------------------|
| Auth / session / CSRF / JWT | Middleware + controller + e2e auth |
| Projections / User aggregate | Domain + projector + service credential paths |
| Migrations | Seeder + tests that open fresh DBs |
| NATS publish subjects | Publish ack, subject naming, serialization |
| Handler manifests / Benthos | Generator unit tests + lint + routing integration |

## Running a focused crate

```bash
cargo test -p arc-core
cargo test -p arc-es-sqlite
cargo test -p arc --test user_snapshot_policy
```
