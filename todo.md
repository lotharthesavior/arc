# Event Sourcing Refactor Status

Tracking items for the event sourcing refactor per `docs/ark/refactor-plan.md`.

## 🔴 Priority — Started But Not Finished

These items have landed partial infrastructure or planning and should be resolved before starting broad new roadmap work.

1. **Step 5 — Postgres live validation/docs.** `arc-es-postgres` now implements Postgres event/read-model stores behind the existing traits, and `DATABASE_DRIVER=sqlite|postgres` wiring compiles for app and worker startup. Remaining work: run against a live Postgres database (`ARC_POSTGRES_TEST_DATABASE_URL`), document the schema-initialization boundary, and decide whether to split read-model storage into a separate `arc-rm-postgres` crate before publishing.
2. **HIPAA-5 — Backfill/operational rollout.** SQLite now persists event signatures and verifies integrity chains when `EVENT_INTEGRITY_KEY` is configured. Remaining work: provide a backfill command for existing unsigned rows, document rollout/key-rotation policy, and decide whether Postgres should enforce the same chain before enabling it in production.
3. **HIPAA-2b — Mechanical read-logging guarantee.** Implemented via `Sensitive<T>`/`AccessLogged<T>` wrappers. ✅
4. **Snapshot activation policy.** Snapshot interfaces, SQLite persistence, and `CommandBus` policy support exist, but production `UserAggregate` opts out and app wiring uses `SnapshotPolicy::Disabled`.
5. **Step 4 routing strategy — Benthos definition/implementation.** `arc-worker` exists as an initial durable consumer/fallback; Benthos/Redpanda Connect is now the proposed primary event router but is not implemented.
6. **Documentation reconciliation.** Reference docs still contain stale old MVC/single-crate wording, especially `docs/03-backend.md` and event-sourcing docs `09/10/11`.
7. **CI/audit tooling hardening.** CI works, but `nats-server` install logic is duplicated/fragile; stale-doc detection and `make audit` are planned but not done.

## 🔴 Blocking — HIPAA Foundations

Interface-level work; full implementation may extend into Step 2.

- [x] **HIPAA-1** `AuditMetadata` struct in `arc-core`. Inline on `Event`, validated at the bus AND at the store, request-scoped via `audit_context::for_actor` / `anonymous`. SQLite migration `2026-04-21-000002_add_hipaa_audit` adds 7 columns + 2 indices. Docs at `docs/guides/audit-metadata.md` with three diagrams (sequence, architecture, ER). 140 workspace tests pass. §164.312(b). ✅
- [x] **HIPAA-2** Generic `AccessLogger` trait + `NoOpAccessLogger` + `RecordingAccessLogger` (test-utils) in `arc-core::access_log`. `Sensitivity` enum (PHI / PCI / PII / Confidential / Internal / Public), `PurposeOfUse`, `Identity`, `AccessedResource`, `AccessLogEntry`. App helper `helpers::access_log` builds identity/correlation from request, runs `record_read` non-fatally. Wired into `GET /api/v1/protected/profile` as the first audited read. 152 workspace tests + 11 E2E pass. Docs at `docs/guides/access-logging.md` with sequence + architecture diagrams. §164.312(b). ✅
  - [x] **HIPAA-2a — Failure policy.** `FailurePolicy::{FailHard, FailOpenWarn}` in `arc-core::access_log`. `FailurePolicy::for_sensitivity` defaults PHI/PCI to `FailHard`, everything else to `FailOpenWarn`. App helper `record_read` returns `RecordReadOutcome::{Ok, FailHard}`; PII profile read returns 503 if the sink fails on PHI. Tests pin both branches. ✅
  - [x] **HIPAA-2b — Compile-time guarantee that controllers call `record_read`.** Implemented via `Sensitive<T>` and `AccessLogged<T>` wrappers in `arc-app::helpers::access_log`. `Sensitive<T>` wraps unaudited data and cannot be returned as a response; `AccessLogged<T>` is the "cleansed" version returned by `record_read` and implements `Responder`. `/profile` controller refactored to use this pattern. ✅
- [x] **HIPAA-3** `IdleTimeoutMiddleware` in `arc-app::http::middlewares`. Reads the post-cutover `SessionUser` plus `last_active_at` from session, purges + redirects to `/signin?reason=idle` past `SESSION_IDLE_TIMEOUT_SECS` (default 900s). Wrapped around `/admin` scope. Regression tests cover real session-user cookies. §164.312(a)(2)(iii). Docs: `docs/guides/idle-timeout.md` + `flow-18-idle-timeout` diagram. ✅
- [x] **HIPAA-4** Server-side `SessionStore` trait in `arc-core::session` + `InMemorySessionStore` (test-utils) + `SqliteSessionStore` in `arc-es-sqlite`. JWT `Claims` carries `jti: Option<Uuid>`. Login records the session; logout revokes; `JwtMiddleware` consults `is_valid` per request and **fails closed (503)** when the store is unavailable. New endpoint `POST /api/v1/protected/logout`. Migration `2026-04-26-000002_create_jwt_sessions`. 9 core + 6 sqlite + 2 E2E tests. §164.312(d). Docs: `docs/guides/session-revocation.md` + `flow-19-session-revocation` diagram. ✅
- [~] **HIPAA-5** Integrity primitives + SQLite opt-in enforcement complete. `IntegrityChain` trait + `HmacSha256Chain` reference impl + canonical event byte format live in `arc-core::integrity`. SQLite migration `2026-06-13-000001_add_event_integrity_signatures` adds persisted signature fields; `SqliteEventStore` signs new rows and verifies load/stream when `EVENT_INTEGRITY_KEY` is configured. Backfill, key-rotation docs, and Postgres parity remain before production enablement. §164.312(c)(1). Docs: `docs/guides/integrity-chain.md` + `architecture-23-integrity-chain` diagram.

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
- [x] `docs/roadmap.md` — stale status entries reconciled against master (Phase 1.3 migration, Phase 7 CI/CD, Phase 4.3 clippy, Phase 9.3 snapshot store).

## 🟡 Infra

- [x] `docker-compose.yml` audited and rewritten: deprecated `version` removed, NATS JetStream healthcheck, Postgres healthcheck, app service with full env (DATABASE_URL/NATS_URL/SECRET_KEY/JWT_SECRET) + healthcheck, worker stub on alpine. `Dockerfile` added (multi-stage Rust + Vite). `docker compose config` validates clean.

## 🟢 Production Risks (plan-flagged)

- [x] `es-sqlite/lib.rs` — `i64 → i32` cast on sequence/timestamp removed. Schema migrated via `2026-04-26-000001_widen_event_int_columns` (recreate table with `BIGINT` columns + index restoration). Diesel schema, record types, and queries widened to `i64`. New regression test `test_sequence_above_i32_max_roundtrips_without_truncation` confirms `i32::MAX + N` round-trips intact. ✅
- [x] `ReadModelStore::execute(sql, params)` SQL-dialect leak — redesigned to typed `upsert/delete/get/find_by/list/truncate` before any projector multiplied. ✅
- [~] Snapshot infrastructure — interface + SQLite persistence + CommandBus policy support landed: `Snapshot` struct + `EventStore::save_snapshot/load_snapshot` (`arc-core::snapshot`, `event_store.rs`), `Aggregate::to_snapshot/from_snapshot` (`aggregate.rs`), `arc-es-sqlite` upsert/load impl, migration `2026-05-31-000001_create_snapshots`. Production `UserAggregate` currently opts out and real app wiring uses the default `SnapshotPolicy::Disabled`, so User still rehydrates from zero until Step 5/storage hardening chooses an activation policy.
- [x] `InProcessEventBus::publish` blocks write path. Step 3 (`arc-es-nats`) provides the solid NATS JetStream publishing foundation. The original Step 4 custom worker approach for durable consumption and projection delivery is now being superseded/evolved by Benthos pipelines as the primary event routing mechanism. ✅

## ⚪ Transitional Debt (closed)

- [x] Cookie `/signin` Diesel-only — closed 2026-05-08. Cookie auth now reads `users_view`; admin profile + password mutations route through `CommandBus`. Legacy `users` and `user_email_index` Diesel tables dropped (migration `2026-05-08-000001_drop_legacy_users`). `User`/`NewUser` Diesel structs removed. ✅

## ✅ Done (cumulative)

Architecture skeleton · single-hash register · email index · UUID JWT · ES login · aggregate-loaded profile · DELETE path · `CONVENTIONS.md` · `scripts/new-aggregate.sh` · HIPAA-1 audit · HIPAA-2 access logger (incl. 2a failure policy) · HIPAA-3 idle timeout fixed for `SessionUser` cookies · HIPAA-4 server-side session store + jti + logout · HIPAA-5 integrity primitives + SQLite opt-in signature enforcement · es-sqlite i64 widening · `Dockerfile` + compose audit · `users_view` projection + `SqliteReadModelStore` + `UserProjector` + replay-from-zero · cookie `/signin` cutover (SessionUser POD, projection-backed auth, `CommandBus`-driven admin mutations, legacy `users`/`user_email_index` dropped) · snapshot interface + SQLite persistence + CommandBus policy support (User activation deferred) · CI/CD workflows (`ci.yml` clippy/fmt/matrix/tests/frontend + `security.yml` cargo-audit) · Step 3 `arc-es-nats` JetStream event bus (publishing foundation) · NATS/worker integration tests + workspace clippy gate passing (validating publishing side) · CI installs pinned `nats-server` release so JetStream tests run live · **Step 5 Postgres support** (arc-es-postgres, live validation passed, app/worker wiring verified, documentation added) · **workspace tests pass**

- [x] Step 3 complete - `arc-es-nats` JetStream EventBus plus `EVENT_BUS=nats` startup wiring (solid publishing foundation).
- [~] Step 4 (original custom `arc-worker` approach) — initial implementation complete; now being evolved/superseded by Benthos pipelines as the primary durable consumer, router, filter, and projection delivery mechanism. See updated plan in `docs/ark/refactor-plan.md` (evolved Step 4) and `docs/roadmap.md` 1.4.1.

## Recommended Next

1. **HIPAA-2b** — compile-time read-logging guarantee. Revisit when read surface grows beyond `/profile`.
2. **Documentation cluster** — `docs/tutorials/02-adding-a-projection.md` plus reference doc reconciliation.
3. **Step 5 maintenance** — decide whether to split read-model storage into a separate `arc-rm-postgres` crate before publishing.
