# WIP — Resume Point

**Last session:** 2026-05-08. Branch `master`.

## Status

**Step 2 complete.** Cookie `/signin` cuts over to `users_view`. Legacy Diesel `users` and `user_email_index` tables retired. Admin profile + password mutations route through `CommandBus`. **197 Rust workspace tests pass** (E2E suite not re-run this session — the seed/serve smoke check verified the spawn path).

See `todo.md` for the full breakdown. Plan at `docs/ark/refactor-plan.md`.

## Step 2 — what landed (cumulative)

### Earlier in Step 2
- Migration `2026-05-07-000001_create_users_view` — `(id PK, version BIGINT, data JSON)` + `UNIQUE INDEX ON json_extract(data,'$.email')`.
- `arc-core::read_model_store` — typed `upsert/delete/get/find_by/list/truncate` replacing the SQL-leaking `execute(sql, params)` (production risk #3). Version-gated upsert makes replay idempotent.
- `arc-es-sqlite::SqliteReadModelStore` — r2d2+diesel, parameterized SQL, identifier allow-listing, `ON CONFLICT(id) DO UPDATE … WHERE table.version < excluded.version`.
- `domain::user::projector::UserProjector` — handles all five user events, carries unchanged fields forward, deletes on `UserDeleted`.
- `ProjectionEngineHandler` — `EventHandler` adapter so the engine subscribes to any `EventBus`.
- API controllers (`register/login/profile`) read `users_view`; `validate_user_credentials_es` + `lookup_aggregate_id_by_email_view` are projection-backed.
- `replay_from_zero` integration test against real SQLite.

### This session — cookie cutover (2A→2D)
- **Session identity** — new `helpers::session::SessionUser { id: String, name, email }` POD; helpers read/write `users_view`-backed identity from the cookie session. No DB round-trip on `is_authenticated`/`get_session_user` (cookie store is signed/encrypted).
- **`auth_controller::signin_post`** — validates against `users_view` via `validate_user_credentials_es`, populates `SessionUser` from the projection.
- **`admin_controller`** — `profile_post` dispatches `UpdateProfile` and/or `ChangeEmail` through `CommandBus`; `profile_password_post` dispatches `ChangePassword`. After update, the cached `SessionUser` is refreshed from the projection. Reads (`dashboard/settings/profile`) consume `SessionUser`.
- **`auth_middleware`, `home_controller`, `websocket/connection`** — switched to `SessionUser` / `Option<String>` aggregate id (no more `i32 user_id`).
- **WS `BroadcastToUser`** — `user_id: String` end-to-end.
- **`UserSeeder`** — replaced with async `seed_default_user(command_bus, rm_store)` that dispatches `RegisterUser`. Idempotent via projection lookup. CLI `migrate --seed` and `seed` go through it.
- **`helpers::es_stack::build(database_url)`** — shared assembly used by CLI commands and (eventually) the runtime server.
- **`helpers::test::es::build_stack[_with_default_user]`** — test scaffolding building `CommandBus + InMemoryReadModelStore` with the projector subscribed synchronously, optionally seeding the default user.
- **Migration `2026-05-08-000001_drop_legacy_users`** — drops `users` + `user_email_index`. `models/user.rs`, `database/seeders/traits/`, `validate_user_credentials` (Diesel), `User`/`NewUser` Diesel structs, and the `users` schema entry are all gone.
- **`AppState::_user_id`** — removed (vestigial `Mutex<Option<i32>>`).

## Step 2 — deferred

- `InProcessEventBus` sync/async lane split (production risk #2) — fold into Step 3 (JetStream worker).
- `EventStore::save_snapshot/load_snapshot` interface stubs — not load-bearing yet; defer.
- `docs/tutorials/02-adding-a-projection.md` — write once a second projector exists.
- HIPAA-2b compile-time read-logging guarantee — defer until read surface > `/profile`.

## Verifying

```bash
make test             # 197 Rust tests
make lint             # clippy -D warnings
make e2e              # 13 Playwright specs (re-run after this session)
```

### Targeted

```bash
cargo test --bin arc http::controllers::admin_controller::tests   # admin via CommandBus
cargo test --bin arc http::controllers::auth_controller           # cookie signin via users_view
cargo test --bin arc http::middlewares::auth_middleware           # SessionUser-backed auth
cargo test -p arc-core read_model_store                            # in-mem version gate
cargo test -p arc-es-sqlite read_model_store                       # SQLite upsert/find/idempotency
```

### Manual smoke (CLI seed path)

```bash
DATABASE_URL=/tmp/arc.sqlite SECRET_KEY=$(openssl rand -hex 32) JWT_SECRET=$(openssl rand -hex 32) APP_NAME=test \
  ./target/debug/arc migrate --seed

sqlite3 /tmp/arc.sqlite \
  "SELECT json_extract(data,'\$.email') FROM users_view;"  # → jekyll@example.com
sqlite3 /tmp/arc.sqlite \
  "SELECT name FROM sqlite_master WHERE type='table' AND name IN ('users','user_email_index');"  # → empty
```

## Next Steps (priority)

1. **Step 3 — `arc-es-nats` (JetStream `EventBus`).** At that point split `InProcessEventBus` lanes into sync (in-tx projectors + integrity chain) vs async (JetStream + email/Stripe).
2. **Step 4 — `arc-worker` crate** (durable consumer driving `ProjectionEngine` out-of-process).
3. **Snapshot interface** — `EventStore::save_snapshot/load_snapshot`, `Aggregate::to_snapshot/from_snapshot`. Implementation in Step 5 alongside Postgres.
4. **HIPAA-2b** — compile-time read-logging guarantee. Defer until read surface > `/profile`.
5. **Docs cluster** — tutorials/guides/reference reconciliation. `docs/tutorials/02-adding-a-projection.md` is the next addition.

## Then

Step 5 (Postgres + crate publish + HIPAA polish).
