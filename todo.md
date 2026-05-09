# Step 1 — Remaining Work

Tracking items for completing Step 1 of the event sourcing refactor per `docs/ark/refactor-plan.md`.

## 🔴 Blocking — HIPAA Foundations

Interface-level work; full implementation may extend into Step 2.

- [x] **HIPAA-1** `AuditMetadata` struct in `nineties-core`. Inline on `Event`, validated at the bus AND at the store, request-scoped via `audit_context::for_actor` / `anonymous`. SQLite migration `2026-04-21-000002_add_hipaa_audit` adds 7 columns + 2 indices. Docs at `docs/guides/audit-metadata.md` with three diagrams (sequence, architecture, ER). 140 workspace tests pass. §164.312(b). ✅
- [x] **HIPAA-2** Generic `AccessLogger` trait + `NoOpAccessLogger` + `RecordingAccessLogger` (test-utils) in `nineties-core::access_log`. `Sensitivity` enum (PHI / PCI / PII / Confidential / Internal / Public), `PurposeOfUse`, `Identity`, `AccessedResource`, `AccessLogEntry`. App helper `helpers::access_log` builds identity/correlation from request, runs `record_read` non-fatally. Wired into `GET /api/v1/protected/profile` as the first audited read. 152 workspace tests + 11 E2E pass. Docs at `docs/guides/access-logging.md` with sequence + architecture diagrams. §164.312(b). ✅
  - [x] **HIPAA-2a — Failure policy.** `FailurePolicy::{FailHard, FailOpenWarn}` in `nineties-core::access_log`. `FailurePolicy::for_sensitivity` defaults PHI/PCI to `FailHard`, everything else to `FailOpenWarn`. App helper `record_read` returns `RecordReadOutcome::{Ok, FailHard}`; PII profile read returns 503 if the sink fails on PHI. Tests pin both branches. ✅
  - [ ] **HIPAA-2b — Compile-time guarantee that controllers call `record_read`.** Today a new read controller can return PHI without logging — relies on developer discipline. Options: (a) wrapper response type `AccessLogged<T>` constructible only via the logger, (b) marker trait + proc-macro, (c) clippy lint over `#[get]` handlers returning regulated DTOs. Pick one once the read surface grows beyond `/profile`.
- [x] **HIPAA-3** `IdleTimeoutMiddleware` in `nineties-app::http::middlewares`. Reads `last_active_at` from session, purges + redirects to `/signin?reason=idle` past `SESSION_IDLE_TIMEOUT_SECS` (default 900s). Wrapped around `/admin` scope. 5 unit tests. §164.312(a)(2)(iii). Docs: `docs/guides/idle-timeout.md` + `flow-18-idle-timeout` diagram. ✅
- [x] **HIPAA-4** Server-side `SessionStore` trait in `nineties-core::session` + `InMemorySessionStore` (test-utils) + `SqliteSessionStore` in `nineties-es-sqlite`. JWT `Claims` carries `jti: Option<Uuid>`. Login records the session; logout revokes; `JwtMiddleware` consults `is_valid` per request and **fails closed (503)** when the store is unavailable. New endpoint `POST /api/v1/protected/logout`. Migration `2026-04-26-000002_create_jwt_sessions`. 9 core + 6 sqlite + 2 E2E tests. §164.312(d). Docs: `docs/guides/session-revocation.md` + `flow-19-session-revocation` diagram. ✅
- [x] **HIPAA-5** `IntegrityChain` trait + `HmacSha256Chain` reference impl + canonical event byte format in `nineties-core::integrity`. `EventSignature` (hex-encoded HMAC), `verify_chain` reports `BrokenAt` and `OutOfOrder`. 12 unit tests including a pinned cross-version test vector. Wiring into `EventStore` deferred to Step 2. §164.312(c)(1). Docs: `docs/guides/integrity-chain.md` + `architecture-23-integrity-chain` diagram. ✅

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
- [ ] `docs/roadmap.md` — stale status entries

## 🟡 Infra (Step 1 deliverable, used by Step 3)

- [x] `docker-compose.yml` audited and rewritten: deprecated `version` removed, NATS JetStream healthcheck, Postgres healthcheck, app service with full env (DATABASE_URL/NATS_URL/SECRET_KEY/JWT_SECRET) + healthcheck, worker stub on alpine. `Dockerfile` added (multi-stage Rust + Vite). `docker compose config` validates clean.

## 🟢 Production Risks (plan-flagged)

- [x] `es-sqlite/lib.rs` — `i64 → i32` cast on sequence/timestamp removed. Schema migrated via `2026-04-26-000001_widen_event_int_columns` (recreate table with `BIGINT` columns + index restoration). Diesel schema, record types, and queries widened to `i64`. New regression test `test_sequence_above_i32_max_roundtrips_without_truncation` confirms `i32::MAX + N` round-trips intact. ✅
- [x] `ReadModelStore::execute(sql, params)` SQL-dialect leak — redesigned to typed `upsert/delete/get/find_by/list/truncate` before any projector multiplied. ✅
- [ ] Snapshot support — `EventStore::save_snapshot/load_snapshot`, `Aggregate::to_snapshot/from_snapshot`. Defer until Step 5 alongside Postgres.
- [ ] `InProcessEventBus::publish` blocks write path. Separate synchronous in-transaction handlers from async side-effects (email/Stripe/JetStream). Fold into Step 3.

## ⚪ Transitional Debt (closed)

- [x] Cookie `/signin` Diesel-only — closed 2026-05-08. Cookie auth now reads `users_view`; admin profile + password mutations route through `CommandBus`. Legacy `users` and `user_email_index` Diesel tables dropped (migration `2026-05-08-000001_drop_legacy_users`). `User`/`NewUser` Diesel structs removed. ✅

## ✅ Done (cumulative)

Architecture skeleton · single-hash register · email index · UUID JWT · ES login · aggregate-loaded profile · DELETE path · `CONVENTIONS.md` · `scripts/new-aggregate.sh` · HIPAA-1 audit · HIPAA-2 access logger (incl. 2a failure policy) · HIPAA-3 idle timeout · HIPAA-4 server-side session store + jti + logout · HIPAA-5 integrity chain · es-sqlite i64 widening · `Dockerfile` + compose audit · `users_view` projection + `SqliteReadModelStore` + `UserProjector` + replay-from-zero · cookie `/signin` cutover (SessionUser POD, projection-backed auth, `CommandBus`-driven admin mutations, legacy `users`/`user_email_index` dropped) · **197 Rust workspace tests pass**

## Recommended Next

1. **Step 3 — `nineties-es-nats` (JetStream `EventBus`).** Split `InProcessEventBus` into sync (in-tx projectors + integrity chain) vs async (JetStream + email/Stripe).
2. **Step 4 — `nineties-worker` crate** (durable consumer driving `ProjectionEngine` out-of-process).
3. **Snapshot interface** — defer to Step 5 alongside Postgres.
4. **HIPAA-2b** — compile-time read-logging guarantee. Revisit when read surface grows beyond `/profile`.
5. **Documentation cluster** — `docs/tutorials/02-adding-a-projection.md` plus reference doc reconciliation.
