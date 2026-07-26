# Event Sourcing Refactor Status

Tracking items for the event sourcing refactor per `docs/ark/refactor-plan.md`.

## 🔴 Priority — Started But Not Finished

These items have landed partial infrastructure or planning and should be resolved before starting broad new roadmap work.

1. **Step 4 routing layer — Benthos (Redpanda Connect).** Decision is locked in `docs/adr/0001-benthos-only-event-routing.md`: Benthos is the single routing and event-handler delivery layer; the `arc-worker` crate has been removed. Benthos must never write directly to Arc databases. The base pipeline (`config/benthos/events.yaml`) and generated runtime pipeline (`config/benthos/generated/events.yaml`) are in place; `make benthos-config` compiles `config/handlers/*.yaml`, CI lints the Benthos configs, and the generated runtime contract validates envelopes plus enriches DLQ messages with `x_arc_dlq`. Publish→Benthos→Arc HTTP projection handler→read-model integration coverage is in place, DLQ/redrive operator docs are written, and `arc-es-nats` provisions `ARC_DLQ` for `dlq.>` automatically. See `docs/guides/event-handlers.md` and `docs/guides/benthos-dlq-redrive.md`.
2. **Documentation reconciliation.** Reference docs still contain stale old MVC/single-crate wording, especially `docs/03-backend.md` and event-sourcing docs `09/10/11`.
3. **CI/audit tooling hardening.** CI works, but `nats-server` install logic is duplicated/fragile; stale-doc detection and `make audit` are planned but not done.
4. **Framework upgrade contract.** Promote Arc from clone-and-edit starter to upgradeable framework. The contract is defined and accepted in `docs/adr/0002-framework-upgrade-contract.md`. Landed: ownership tables match the physical `arc-web` / thin `arc-app` split; `make doctor` / `make arc-check` enforce generated Benthos config banners/freshness **and** the Arc-owned structural boundary (framework runtime must not reappear under `crates/arc-app/src`); user-facing path is `docs/guides/upgrading.md`. Remaining (out of scope for the in-tree split): **publish `arc-web`**, ship **`arc new` CLI**, and finish publishing the other versioned `arc-*` crates under the ADR 0002 SemVer policy.

### Benthos Contract Hardening

- [x] Generator rejects unsupported `idempotency.ordering` / `idempotency.key` / `retry.backoff` values instead of silently ignoring them.
- [x] Generator rejects unknown manifest keys.
- [x] Retry wrapper applies to HTTP and NATS delivery targets; `retry.max_attempts` means total attempts.
- [x] SQL/database delivery targets are rejected; Benthos must not write directly to Arc databases.
- [x] Add envelope validation and enriched DLQ metadata (`x_arc_dlq`) before declaring the handler manifest contract stable.
- [x] Add publish→Benthos→Arc HTTP projection handler→read-model integration coverage.
- [x] Add DLQ/redrive operator documentation and workflow.
- [x] Provision `ARC_DLQ` (`dlq.>`) automatically from `arc-es-nats` so Benthos DLQ messages persist in NATS JetStream.

## 🔴 Blocking — HIPAA Foundations

Interface-level work; full implementation may extend into Step 2.

- [x] **HIPAA-1** `AuditMetadata` struct in `arc-core`. Inline on `Event`, validated at the bus AND at the store, request-scoped via `audit_context::for_actor` / `anonymous`. SQLite migration `2026-04-21-000002_add_hipaa_audit` adds 7 columns + 2 indices. Docs at `docs/guides/audit-metadata.md` with three diagrams (sequence, architecture, ER). 140 workspace tests pass. §164.312(b). ✅
- [x] **HIPAA-2** Generic `AccessLogger` trait + `NoOpAccessLogger` + `RecordingAccessLogger` (test-utils) in `arc-core::access_log`. `Sensitivity` enum (PHI / PCI / PII / Confidential / Internal / Public), `PurposeOfUse`, `Identity`, `AccessedResource`, `AccessLogEntry`. App helper `helpers::access_log` builds identity/correlation from request, runs `record_read` non-fatally. Wired into `GET /api/v1/protected/profile` as the first audited read. 152 workspace tests + 11 E2E pass. Docs at `docs/guides/access-logging.md` with sequence + architecture diagrams. §164.312(b). ✅
  - [x] **HIPAA-2a — Failure policy.** `FailurePolicy::{FailHard, FailOpenWarn}` in `arc-core::access_log`. `FailurePolicy::for_sensitivity` defaults PHI/PCI to `FailHard`, everything else to `FailOpenWarn`. App helper `record_read` returns `RecordReadOutcome::{Ok, FailHard}`; PII profile read returns 503 if the sink fails on PHI. Tests pin both branches. ✅
  - [x] **HIPAA-2b — Compile-time guarantee that controllers call `record_read`.** Implemented via `Sensitive<T>` and `AccessLogged<T>` wrappers in `arc-app::helpers::access_log`. `Sensitive<T>` wraps unaudited data and cannot be returned as a response; `AccessLogged<T>` is the "cleansed" version returned by `record_read` and implements `Responder`. `/profile` controller refactored to use this pattern. ✅
- [x] **HIPAA-3** `IdleTimeoutMiddleware` in `arc-app::http::middlewares`. Reads the post-cutover `SessionUser` plus `last_active_at` from session, purges + redirects to `/signin?reason=idle` past `SESSION_IDLE_TIMEOUT_SECS` (default 900s). Wrapped around `/admin` scope. Regression tests cover real session-user cookies. §164.312(a)(2)(iii). Docs: `docs/guides/idle-timeout.md` + `flow-18-idle-timeout` diagram. ✅
- [x] **HIPAA-4** Server-side `SessionStore` trait in `arc-core::session` + `InMemorySessionStore` (test-utils) + `SqliteSessionStore` in `arc-es-sqlite`. JWT `Claims` carries `jti: Option<Uuid>`. Login records the session; logout revokes; `JwtMiddleware` consults `is_valid` per request and **fails closed (503)** when the store is unavailable. New endpoint `POST /api/v1/protected/logout`. Migration `2026-04-26-000002_create_jwt_sessions`. 9 core + 6 sqlite + 2 E2E tests. §164.312(d). Docs: `docs/guides/session-revocation.md` + `flow-19-session-revocation` diagram. ✅
- [x] **HIPAA-5** Integrity primitives + SQLite & Postgres enforcement complete. `IntegrityChain` trait + `HmacSha256Chain` reference impl + canonical event byte format live in `arc-core::integrity`. `SqliteEventStore` and `PostgresEventStore` sign new rows and verify load/stream when `EVENT_INTEGRITY_KEY` is configured. Framework stance: secure-by-default; enabling integrity on existing data requires a replay. §164.312(c)(1). Docs: `docs/guides/integrity-chain.md` + `architecture-23-integrity-chain` diagram. ✅

## 🟡 Documentation

- [ ] `docs/tutorials/01-adding-your-first-aggregate.md` — Full Task-domain walkthrough (10 sections per plan)
- [ ] `docs/guides/getting-started.md` — Env vars, make targets, first run
- [ ] `docs/guides/event-sourcing-concepts.md` — Plain-English primer
- [ ] `docs/guides/testing-aggregates.md` — InMemory test patterns
- [ ] `README.md` rewrite — "Adding a New Entity" as numbered commands

### Reference docs to reconcile

- [ ] `docs/01-overview.md` — workspace structure, ES as primary architecture
- [ ] `docs/02-architecture.md` — post-refactor layer diagram
- [ ] `docs/03-backend.md` — write-path change note
- [ ] `docs/06-testing.md` — "Testing Event-Sourced Domain Logic" section
- [x] `docs/roadmap.md` — stale status entries reconciled through 2026-06-21, including Benthos maturity, DLQ persistence, and publishable crate metadata status.

## 🟡 Infra

- [x] `docker-compose.yml` audited and rewritten: deprecated `version` removed, NATS JetStream healthcheck, Postgres healthcheck, app service with full env (DATABASE_URL/NATS_URL/SECRET_KEY/JWT_SECRET) + healthcheck. The routing service is now `benthos` (Redpanda Connect) running `config/benthos/events.yaml` on `events.>` (replaced the original worker stub). `Dockerfile` added (multi-stage Rust + Vite). `docker compose config` validates clean.

## 🟢 Production Risks (plan-flagged)

- [x] `es-sqlite/lib.rs` — `i64 → i32` cast on sequence/timestamp removed. Schema migrated via `2026-04-26-000001_widen_event_int_columns` (recreate table with `BIGINT` columns + index restoration). Diesel schema, record types, and queries widened to `i64`. New regression test `test_sequence_above_i32_max_roundtrips_without_truncation` confirms `i32::MAX + N` round-trips intact. ✅
- [x] `ReadModelStore::execute(sql, params)` SQL-dialect leak — redesigned to typed `upsert/delete/get/find_by/list/truncate` before any projector multiplied. ✅
- [x] Snapshot infrastructure + User activation — interface + SQLite/Postgres persistence + CommandBus policy support landed: `Snapshot` struct + `EventStore::save_snapshot/load_snapshot` (`arc-core::snapshot`, `event_store.rs`), `Aggregate::to_snapshot/from_snapshot` (`aggregate.rs`), SQLite/Postgres upsert/load impls, migration `2026-05-31-000001_create_snapshots`. `UserAggregate` now serializes/restores snapshots and app command-bus wiring uses `USER_SNAPSHOT_INTERVAL_EVENTS` with default 50, so user writes create best-effort snapshots at a configurable interval while the event log remains source of truth. ✅
- [x] `InProcessEventBus::publish` blocks write path. Step 3 (`arc-es-nats`) provides the solid NATS JetStream publishing foundation. Durable consumption and routing are owned by Benthos pipelines (ADR 0001); projection writes remain inside Arc-owned handlers/services. The in-process bus is retained only as the zero-dependency single-process dev default. ✅

## ⚪ Transitional Debt (closed)

- [x] Cookie `/signin` Diesel-only — closed 2026-05-08. Cookie auth now reads `users_view`; admin profile + password mutations route through `CommandBus`. Legacy `users` and `user_email_index` Diesel tables dropped (migration `2026-05-08-000001_drop_legacy_users`). `User`/`NewUser` Diesel structs removed. ✅

## ✅ Done (cumulative)

Architecture skeleton · single-hash register · email index · UUID JWT · ES login · aggregate-loaded profile · DELETE path · `CONVENTIONS.md` · `scripts/new-aggregate.sh` · HIPAA-1 audit · HIPAA-2 access logger (incl. 2a failure policy) · HIPAA-3 idle timeout fixed for `SessionUser` cookies · HIPAA-4 server-side session store + jti + logout · HIPAA-5 integrity chain (SQLite & Postgres enforcement, verified from-scratch policy) · es-sqlite i64 widening · `Dockerfile` + compose audit · `users_view` projection + `SqliteReadModelStore` + `UserProjector` + replay-from-zero · cookie `/signin` cutover (SessionUser POD, projection-backed auth, `CommandBus`-driven admin mutations, legacy `users`/`user_email_index` dropped) · snapshot interface + SQLite/Postgres persistence + CommandBus policy support + configurable User snapshots defaulting to 50 events · CI/CD workflows (`ci.yml` clippy/fmt/matrix/tests/frontend + `security.yml` cargo-audit) · Step 3 `arc-es-nats` JetStream event bus (publishing foundation) · NATS publish/consume integration tests + workspace clippy gate passing (validating publishing side) · CI installs pinned `nats-server` release so JetStream tests run live · Step 5 Postgres support (arc-es-postgres, live validation passed, app wiring verified, documentation added) · Benthos adopted as the sole routing layer (ADR 0001); `arc-worker` crate removed · Benthos handler-manifest generator + lint CI + envelope validation + enriched `x_arc_dlq` metadata · SQL/database delivery rejected for Benthos · publish→Benthos→Arc HTTP projection handler integration coverage · DLQ/redrive operator guide · automatic `ARC_DLQ` JetStream provisioning for `dlq.>` · **workspace tests pass**

- [x] Step 3 complete - `arc-es-nats` JetStream EventBus plus `EVENT_BUS=nats` startup wiring (solid publishing foundation).
- [x] Step 4 routing layer = **Benthos (Redpanda Connect)**. ADR 0001 accepted; the `arc-worker` crate was removed from the workspace; `docker-compose.yml` runs a `benthos` service on the generated runtime pipeline. The retired `arc-worker` is historical only — do not extend it. Routing integration coverage, DLQ/redrive docs, and automatic `ARC_DLQ` provisioning are complete. See `docs/ark/refactor-plan.md` (evolved Step 4) and `docs/roadmap.md` §1.4.1.

## Recommended Next

1. **Publish sequence** — `arc-core` metadata and dry-run are ready; publish or cache `arc-core`, then adapter dry-runs for `arc-es-sqlite`, `arc-es-postgres`, and `arc-es-nats`, then **`arc-web`**.
2. **`arc new` CLI** — scaffold a thin app that depends on versioned `arc-web` / `arc-core` instead of clone-and-edit of this monorepo (remaining Framework Readiness work; in-tree split is done).
3. **Documentation cluster** — `docs/tutorials/02-adding-a-projection.md` plus reference doc reconciliation (including AGENTS.md / overview docs for `arc-web`).
4. **Step 5 maintenance** — decide whether to split read-model storage into a separate `arc-rm-postgres` crate before publishing.

## Recent Validation

- 2026-06-16: Step 5 Postgres live validation passed against the bundled compose database: `ARC_POSTGRES_TEST_DATABASE_URL=postgres://arc:password@127.0.0.1:5433/arc_dev cargo test -p arc-es-postgres`.
- 2026-06-16: `DATABASE_DRIVER=postgres` app startup reached schema initialization and projection rebuild under `cargo run -p arc --features postgres -- serve`.
- 2026-06-16: (historical) `DATABASE_DRIVER=postgres` startup validation covered the then-current durable consumer path. That path has since been removed; durable consumption now runs in Benthos, validated separately via `benthos lint` and routing integration tests.
- 2026-06-17: `UserAggregate` snapshot activation passed: `cargo test -p arc --test user_snapshot_policy`; core snapshot mechanics rechecked with `cargo test -p arc-core snapshot`.
- 2026-06-18: Benthos generator/runtime contract hardening passed: `npm run benthos:config:test`, `npm run benthos:config:check`, `make benthos-lint`, plus a temporary generated HTTP-handler config linted with Redpanda Connect.
- 2026-06-19: Architecture direction tightened: Benthos must never write directly to Arc databases; manifests now reject `delivery.type: sql`; projection integration coverage should target an Arc-owned HTTP projection handler/service.
- 2026-06-21: Benthos routing maturity follow-ups completed: publish→Benthos→Arc HTTP projection handler→read-model integration coverage added, DLQ/redrive operator workflow documented, and `arc-es-nats` now provisions `ARC_DLQ` for durable `dlq.>` persistence.
- 2026-06-21: Publishable crate metadata prepared for `arc-core`, `arc-es-sqlite`, `arc-es-postgres`, and `arc-es-nats`; `arc-core` passed `cargo publish --dry-run --allow-dirty`. Adapter dry-runs are gated on publishing or caching `arc-core`.
- 2026-07-04: `make doctor` and `make arc-check` added as ADR 0002 drift guards for generated Benthos config ownership banners and freshness.
- 2026-07-08: `arc-web` / thin `arc-app` split landed; ADR 0002 ownership tables updated; `make doctor` also asserts Arc-owned runtime trees stay in `crates/arc-web` (not under `crates/arc-app/src`); `docs/guides/upgrading.md` added.
