# Accomplishments

## Project Structure & Foundations
- Rust workspace with five crates (arc-core, arc-es-sqlite, arc-es-postgres, arc-es-nats, arc-app). Durable event routing is handled by Benthos (Redpanda Connect), not a Rust crate; the earlier `arc-worker` consumer crate was removed (ADR 0001).
- CommandBus + Aggregate architecture for all writes
- Event sourcing primitives (Event, EventStore, Projection, ReadModelStore)
- Two-lane EventBus (InProcess + NATS JetStream lane)
- CONVENTIONS.md and scripts/new-aggregate.sh

## User Domain & Auth Cutover
- UserAggregate with full command/event model
- users_view projection + SqliteReadModelStore + UserProjector
- Replay-from-zero support
- ES login / aggregate-loaded profile
- Full cookie /signin cutover to projection-backed SessionUser
- Legacy users and user_email_index tables dropped
- CommandBus-driven admin profile and password mutations
- DELETE user path

## HIPAA Foundations
- HIPAA-1: AuditMetadata on events with request-scoped audit context
- HIPAA-2: AccessLogger trait + NoOp/Recording implementations + FailurePolicy (incl. 2a)
- HIPAA-3: IdleTimeoutMiddleware (fixed for post-cutover SessionUser)
- HIPAA-4: Server-side SessionStore (Sqlite) + JWT jti + logout + fail-closed middleware
- HIPAA-5: IntegrityChain trait + HmacSha256Chain primitives

## Snapshots & Storage
- Snapshot interface + to_snapshot / from_snapshot hooks
- Sqlite snapshot persistence (migration 2026-05-31-000001_create_snapshots)
- EventStore save/load_snapshot support
- SnapshotPolicy in CommandBus (default Disabled)

## Distributed Eventing (Step 3 + Evolving Routing Layer)
- arc-es-nats: NATS JetStream EventBus implementation (publishing foundation)
- Benthos (Redpanda Connect) pipelines as primary durable consumer, router, filter, and HTTP/NATS handler delivery mechanism (evolved Step 4 direction; Benthos does not write to Arc databases)
- Benthos (Redpanda Connect) is now the sole durable routing layer; the earlier Rust consumer has been removed
- NATS integration tests + CI support for live JetStream (validating publishing side)
- CI installs pinned nats-server release binary

## CI, Tooling & Quality
- Full workspace CI (ci.yml with clippy, fmt, tests, frontend, docs, integration)
- security.yml (cargo-audit + dependency review)
- Makefile targets aligned to CI (test, lint, etc.)
- check-roadmap-claims.sh extended with post-audit drift checks
- CI quality job updated to scan under crates/ and fail on println!/eprintln!
- Duplicate tests.yml workflow removed
- Dockerfile + docker-compose.yml audit and hardening

## Documentation & DX
- Root AGENTS.md updated to current 5-crate workspace + source-of-truth order
- High-level reference docs (01/02/roadmap) updated with historical framing for pre-ES MVC content
- Snapshot and architecture claims qualified in roadmap

**Note**: This is a high-level summary of landed work. See `todo.md` and `PROGRESS.md` for the full cumulative list and current status.
