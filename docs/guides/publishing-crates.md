# Publishing Arc Crates

Arc's reusable crates are published in dependency order:

1. `arc-core`
2. `arc-es-sqlite`
3. `arc-es-postgres`
4. `arc-es-nats`

The storage and NATS crates depend on `arc-core` by version, with a local `path` retained for
workspace development. Publish or make `arc-core` available in the registry before running
`cargo publish --dry-run` for the adapter crates.

## Preflight

Run the compile checks first:

```bash
cargo check -p arc-core
cargo check -p arc-es-sqlite
cargo check -p arc-es-postgres
cargo check -p arc-es-nats
```

Run packaging checks in order:

```bash
cargo publish -p arc-core --dry-run
cargo publish -p arc-es-sqlite --dry-run
cargo publish -p arc-es-postgres --dry-run
cargo publish -p arc-es-nats --dry-run
```

If network access is unavailable, `cargo package -p arc-core --allow-dirty --offline --no-verify`
can still validate the `arc-core` package archive. Adapter crates need registry resolution for
`arc-core`, so offline package checks will fail until `arc-core` is present in the local registry
cache.

## Public API Policy

All four crates are pre-1.0. Within a compatible `0.2.x` line:

- Keep `arc-core` traits source-compatible unless there is a documented migration.
- Treat the event envelope, `EventStore`, `ReadModelStore`, `EventBus`, `Aggregate`, and projection
  traits as public API.
- Treat handler manifest semantics and Benthos envelope fields as integration API.
- Prefer additive changes for read-model, event, audit, session, and integrity structs.
- Document breaking changes in release notes and bump the minor version before `1.0`.

Do not publish adapter crates with direct database-writing Benthos behavior. Benthos remains the
routing/delivery layer; Arc-owned code owns projection writes.
