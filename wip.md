# WIP — Resume Point

**Last session:** 2026-05-07. Branch `master`.

## Status

Step 2 partial — projector pipeline shipped. **215 tests pass** (202 Rust + 13 Playwright). Cookie `/signin` Diesel-only is the remaining transitional debt.

See `todo.md` for full breakdown. Plan at `docs/ark/refactor-plan.md`.

## Step 2 — what landed

- Migration `2026-05-07-000001_create_users_view` — `(id PK, version BIGINT, data JSON)` with `UNIQUE INDEX ON json_extract(data,'$.email')`. Generic projection-table shape: any future projector reuses it.
- `nineties-core::read_model_store` — trait redesigned. Typed `upsert/delete/get/find_by/list/truncate` replaces the SQL-leaking `execute(sql, params)` (production risk #3). `Upsert` carries `version`; both `InMemoryReadModelStore` and `SqliteReadModelStore` enforce the version gate so replay is idempotent.
- `nineties-es-sqlite::SqliteReadModelStore` — r2d2+diesel, parameterized SQL, identifier validation, `INSERT … ON CONFLICT(id) DO UPDATE … WHERE table.version < excluded.version`.
- `domain::user::projector::UserProjector` — handles `UserRegistered/ProfileUpdated/EmailChanged/PasswordChanged/UserDeleted`. Carries unchanged fields forward via prior-row read; deletes the row on `UserDeleted`.
- `nineties-core::projection::ProjectionEngineHandler` — `EventHandler` adapter that lets `ProjectionEngine` subscribe to any `EventBus` implementation.
- `commands/serve.rs` — wires engine + projector + handler before constructing `CommandBus`; calls `rebuild_all()` at startup so existing event-store data is backfilled.
- Controllers (`register/login/profile`) — take `web::Data<dyn ReadModelStore>`; `validate_user_credentials_es` and `lookup_aggregate_id_by_email_view` now read `users_view`; `create_user` dropped the `user_email_index` insert.
- 14 new tests including `replay_from_zero` (real SQLite + projector, deterministic convergence).

## Step 2 — deferred

- `InProcessEventBus` sync/async lane split (production risk #2) — fold into Step 3 (JetStream worker).
- `EventStore::save_snapshot/load_snapshot` interface stubs — not load-bearing yet; defer.
- Cookie `/signin` Diesel→`users_view` swap — blocked on session refactor (`user_id: i32` → `aggregate_id: String` touches `set_session_user`, idle-timeout middleware, admin pages).
- `docs/tutorials/02-adding-a-projection.md` — write once a second projector exists.

## Verifying

```bash
make test             # 202 Rust tests
make lint             # clippy -D warnings
make e2e              # 13 Playwright specs
```

### Targeted

```bash
cargo test -p nineties-core read_model_store          # in-mem version gate
cargo test -p nineties-es-sqlite read_model_store     # SQLite upsert/find/idempotency
cargo test -p nineties --bin nineties domain::user::projector       # UserProjector + replay-from-zero
```

### Manual smoke

```bash
make migrate && make dev

curl -X POST localhost:8080/api/v1/register -H 'content-type: application/json' \
  -d '{"name":"a","email":"a@b.c","password":"pw123456"}'

# After register, the projector populates users_view synchronously via the bus.
sqlite3 *.db "SELECT id, version, data FROM users_view;"

TOKEN=$(curl -s -X POST localhost:8080/api/v1/login -H 'content-type: application/json' \
  -d '{"email":"a@b.c","password":"pw123456"}' | jq -r .token)

# /profile now reads users_view, not the event stream.
curl localhost:8080/api/v1/protected/profile -H "authorization: Bearer $TOKEN"
```

### Rebuild from zero

Truncate `users_view` then restart `make dev` — startup `rebuild_all` replays the event log into the projection. Or run the test:

```bash
cargo test -p nineties --bin nineties replay_from_zero -- --nocapture
```

## Next Steps (priority)

1. **Cookie /signin cutover** — refactor session storage to UUID-keyed user data, then route signin through `users_view`. Closes the last transitional gap.
2. **Step 3 — `nineties-es-nats` (JetStream `EventBus`).** At that point split `InProcessEventBus` lanes into sync (in-tx projectors + integrity chain) vs async (JetStream + email/Stripe).
3. **Step 4 — `nineties-worker` crate** (durable consumer driving `ProjectionEngine` out-of-process).
4. **Snapshot interface** — `EventStore::save_snapshot/load_snapshot`, `Aggregate::to_snapshot/from_snapshot`. Implementation in Step 5 alongside Postgres.
5. **HIPAA-2b** — compile-time read-logging guarantee. Defer until read surface > `/profile`.
6. **Docs cluster** — tutorials/guides/reference reconciliation. `docs/tutorials/02-adding-a-projection.md` is the next addition.

## Then

Step 5 (Postgres + crate publish + HIPAA polish).
