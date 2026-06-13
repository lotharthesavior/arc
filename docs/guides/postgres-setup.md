# Postgres Setup Guide

## Overview

Arc supports **Postgres** as an alternative storage backend for both events and read models. This is controlled via the `DATABASE_DRIVER` environment variable.

## Configuration

To enable Postgres support:

1. **Build with the `postgres` feature**:
   ```bash
   cargo build --features postgres
   ```

2. **Configure environment variables**:
   ```env
   DATABASE_DRIVER=postgres
   DATABASE_URL=postgres://user:password@localhost:5432/database_name
   ```

## Schema Initialization

Unlike the SQLite implementation which uses Diesel migrations, the Postgres implementation (`arc-es-postgres`) handles its own schema initialization.

The `initialize_schema()` method is called during application startup (in `build_stores`) and creates the following tables if they do not exist:

- `events`: The append-only event log.
- `snapshots`: The aggregate snapshots table.
- `users_view`: The default projection table for users.

This "self-healing" schema boundary ensures that the database is always ready for use without requiring manual migration steps during initial setup or testing.

## Live Testing

To run the Postgres-specific tests against a live database, set the `ARC_POSTGRES_TEST_DATABASE_URL` environment variable:

```bash
ARC_POSTGRES_TEST_DATABASE_URL=postgres://user:password@localhost:5432/test_db cargo test -p arc-es-postgres
```

## Using Docker Compose

The project includes a `docker-compose.yml` with a Postgres service. To start it:

```bash
docker compose up -d postgres
```

By default, it is configured to use port **5433** to avoid conflicts with local Postgres instances. You can adjust this in `docker-compose.yml`.

The default connection URL for local development with Docker is:
`postgres://arc:password@localhost:5433/arc_dev`

## Production Readiness

- **Optimistic Concurrency**: Postgres implementation uses `UNIQUE(aggregate_id, sequence)` and a version-check query within a transaction to ensure consistency.
- **Idempotent Upserts**: Read-model upserts use `ON CONFLICT (id) DO UPDATE ... WHERE version < EXCLUDED.version` to ensure that replay and out-of-order delivery converge to the correct state.
- **Audit Metadata**: HIPAA-compliant audit fields are persisted inline on each event row.
