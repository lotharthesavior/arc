# Arc Project Progress

**Last updated:** 2026-07-25  
**Current version:** 0.2.2  
**Status:** Core framework foundations are implemented. Framework packaging, documentation
reconciliation, and later product roadmap phases remain.

This is the canonical project-status and roadmap tracker. Current source code and accepted ADRs
take precedence when a claim here becomes stale.

## Executive Summary

Arc has evolved from a traditional Actix/Diesel MVC starter into a six-crate, event-sourced Rust
workspace:

- `arc-core`: event sourcing, aggregates, commands, projections, read models, audit, access,
  session, snapshot, and integrity primitives.
- `arc-es-sqlite`: SQLite event, read-model, snapshot, and session storage.
- `arc-es-postgres`: Postgres event, read-model, and snapshot storage.
- `arc-es-nats`: NATS JetStream event publishing and stream provisioning.
- `arc-web`: reusable Actix runtime, middleware, helpers, CLI/server wiring, and WebSockets.
- `arc-app`: thin user-owned application containing the User domain, controllers, routes,
  validation, templates, assets, and seeders.

The physical `arc-web` / thin `arc-app` split is implemented in the current working tree but is
not committed as of this update. Benthos (Redpanda Connect) is the sole durable consumer and
routing layer in distributed mode; the removed `arc-worker` is historical.

## Current Priorities

1. **Land the framework split**
   - Review and commit the current `arc-web` / thin `arc-app` extraction.
   - Keep framework runtime out of `crates/arc-app/src`; `make doctor` enforces this boundary.

2. **Restore a fully green distributed integration run**
   - The latest full workspace run passed ordinary unit and documentation tests.
   - `benthos_projection_routing` failed because its spawned `nats-server` did not accept
     connections.
   - Re-run in a clean NATS environment and harden startup/cleanup if the failure reproduces.

3. **Publish the framework**
   - Publish or cache `arc-core`.
   - Run publish dry-runs for `arc-es-sqlite`, `arc-es-postgres`, and `arc-es-nats`.
   - Publish `arc-web` under the ADR 0002 lockstep SemVer policy.

4. **Create `arc new`**
   - Scaffold thin applications that depend on versioned `arc-web` and `arc-core`.
   - Stop requiring users to clone and edit the framework monorepo.

5. **Reconcile documentation**
   - Correct old MVC and single-crate descriptions.
   - Update module paths after the `arc-web` extraction.
   - Clearly label historical plans and QA reports.

6. **Harden CI and audits**
   - Consolidate duplicated/fragile `nats-server` installation and startup logic.
   - Add stale-document detection.
   - Complete the intended `make audit` workflow.

## Validation

### Latest local validation

- 2026-07-22: `make doctor` passed.
- 2026-07-22: `cargo check --workspace --all-features` passed.
- 2026-07-22: `cargo test --workspace --all-features --no-fail-fast` passed all ordinary unit
  and documentation tests, but the Benthos projection-routing integration target failed to
  connect to its spawned `nats-server`.

Do not claim a completely green workspace until that distributed integration target is rerun
successfully.

### Earlier infrastructure validation

- 2026-06-16: Postgres live adapter validation passed against the Compose database.
- 2026-06-16: Postgres application startup reached schema initialization and projection rebuild.
- 2026-06-17: User snapshot policy and core snapshot mechanics passed.
- 2026-06-18: Benthos generation, contract tests, config freshness, and lint checks passed.
- 2026-06-21: Publishable metadata was prepared for the four core/storage/event crates;
  `arc-core` passed a publish dry-run.
- 2026-07-04: `make doctor` / `make arc-check` generated-file drift checks landed.

## Completed Foundations

### Event-sourced domain

- [x] `Event`, `EventStore`, `Aggregate`, `Command`, and `CommandBus` foundations.
- [x] Optimistic concurrency and audit validation.
- [x] User aggregate commands and event-sourced write path.
- [x] Projection-backed `users_view`.
- [x] Typed `ReadModelStore` operations.
- [x] Deterministic projection replay from the event log.
- [x] Cookie sign-in reads the projection; profile/password mutations use `CommandBus`.
- [x] Legacy mutable `users` and `user_email_index` tables removed.
- [x] User snapshots at configurable intervals (`USER_SNAPSHOT_INTERVAL_EVENTS`, default 50).

### Storage

- [x] SQLite event, read-model, snapshot, and JWT-session stores.
- [x] Postgres event, read-model, and snapshot stores.
- [x] `DATABASE_DRIVER=sqlite|postgres` application selection.
- [x] Sequence/timestamp storage widened to `i64`.
- [x] Event integrity signatures and verification for SQLite and Postgres.

### Distributed eventing and routing

- [x] NATS JetStream publishing through `arc-es-nats`.
- [x] Subject naming: `events.<aggregate_type>.<event_type>` in snake_case.
- [x] Idempotent event-stream and `ARC_DLQ` provisioning.
- [x] Benthos selected as the sole durable routing layer in ADR 0001.
- [x] Removed the bespoke Rust `arc-worker` consumer.
- [x] Base and generated Benthos pipelines.
- [x] Handler-manifest compiler with unknown-key and unsupported-value rejection.
- [x] HTTP and NATS delivery; SQL/database delivery prohibited.
- [x] Envelope validation, retry semantics, dedupe, and enriched `x_arc_dlq` metadata.
- [x] Arc-owned HTTP projection endpoint and routing integration coverage.
- [x] DLQ/redrive operator guide.

### Security and compliance foundations

- [x] Event `AuditMetadata`, validated by command bus and event stores.
- [x] Access logging primitives, sensitivity classification, and failure policies.
- [x] `Sensitive<T>` → `AccessLogged<T>` response discipline for audited reads.
- [x] Session/cookie authentication for browser and admin routes.
- [x] JWT bearer authentication and server-side revocation for API routes.
- [x] CSRF protection for HTML forms.
- [x] Rate limiting.
- [x] Admin idle timeout using projection-backed `SessionUser`.
- [x] HMAC-SHA256 event integrity chain.

### Framework readiness

- [x] ADR 0002 defines ownership, SemVer, public API, and upgrades.
- [x] `arc-web` / thin `arc-app` boundary implemented in the working tree.
- [x] `make doctor` checks generated Benthos freshness and structural ownership.
- [x] User-facing upgrade guide.
- [ ] Commit the current framework split.
- [ ] Publish all versioned `arc-*` crates, including `arc-web`.
- [ ] Ship `arc new`.
- [ ] Extend generated-file checks to Diesel schema and frontend output where practical.

### Delivery and operations

- [x] CI checks formatting, Clippy, tests, and frontend builds.
- [x] Security workflow runs dependency auditing.
- [x] Dockerfile and health-checked Compose services for app, Postgres, NATS, and Benthos.
- [x] Browser E2E infrastructure.
- [x] Tera/Vite/Tailwind/Stimulus/Turbo frontend stack.
- [x] Hashed asset caching and `/public/*` serving.

## Documentation Work

### Missing user documentation

- [ ] `docs/tutorials/01-adding-your-first-aggregate.md`
- [ ] `docs/tutorials/02-adding-a-projection.md`
- [ ] `docs/guides/getting-started.md`
- [ ] `docs/guides/event-sourcing-concepts.md`
- [ ] `docs/guides/testing-aggregates.md`
- [ ] Rewrite the root README “Adding a New Entity” workflow.

### Reference reconciliation

- [ ] `docs/01-overview.md`: six-crate workspace and ES-first architecture.
- [ ] `docs/02-architecture.md`: current layer and deployment diagrams.
- [ ] `docs/03-backend.md`: `arc-web` paths and CommandBus write path.
- [ ] `docs/04-frontend.md`: verify actual JS stack and build commands.
- [ ] `docs/05-database.md`: remove legacy mutable User model/table descriptions.
- [ ] `docs/06-testing.md`: event-sourced domain, projection, NATS, and Benthos testing.
- [ ] `docs/07-api-reference.md`: verify routes, auth modes, and response shapes.
- [ ] `docs/08-problems-and-improvements.md`: close or remove resolved findings.
- [ ] `docs/09-event-sourcing-architecture.md`: distinguish current design from proposals.
- [ ] `docs/10-event-sourcing-implementation-guide.md`: replace pre-implementation phases.
- [ ] `docs/11-event-sourcing-api-reference.md`: align traits with current source.
- [ ] Label old QA, implementation, and planning documents as historical.
- [ ] Decide whether to maintain or archive `docsify-docs/`.

## Remaining Product Roadmap

### Phase 2 — Security hardening

Core HIPAA-oriented foundations are complete. Remaining work:

- [ ] Security headers and a documented CSP posture.
- [ ] Broader input sanitization and adversarial security coverage.
- [ ] Production review of access-log persistence and operations.
- [ ] Formal threat modeling and release security checklist.

### Phase 3 — Plugin and hook system

- [ ] Finalize hook extension boundaries against ADR 0002 ownership rules.
- [ ] Define async hook behavior and failure isolation.
- [ ] Implement plugin discovery/loading without requiring framework forks.
- [ ] Provide at least one maintained example plugin.

### Phase 4 — Testing and quality

- [ ] Restore and keep the full distributed integration suite green.
- [ ] Improve test isolation and external-service lifecycle cleanup.
- [ ] Add explicit coverage targets and reporting.
- [ ] Add documentation and architecture drift audits.
- [ ] Keep Docker resources project-scoped and reliably cleaned up.

### Phase 5 — Performance and observability

- [ ] Establish reproducible performance baselines and budgets.
- [ ] Add metrics for commands, projections, NATS publishing, Benthos delivery, and DLQ depth.
- [ ] Add distributed tracing/correlation across writer, Benthos, and handlers.
- [ ] Document production logging, alerting, and health-check expectations.

### Phase 6 — PWA

- [ ] Decide whether PWA support belongs in the core framework or an optional template/plugin.
- [ ] Add manifest/installability if accepted.
- [ ] Add safe asset/offline caching compatible with authenticated and Turbo flows.

### Phase 7 — CI/CD and releases

- [x] Core CI and security workflows.
- [ ] Stabilize external NATS/Benthos provisioning.
- [ ] Validate crate publishing in dependency order.
- [ ] Establish release notes and migration-note enforcement.
- [ ] Exercise the ADR 0002 upgrade path against a generated sample application.

### Phase 8 — Developer experience

- [ ] `arc new`.
- [ ] Aggregate and projection generators with current module paths.
- [ ] Complete getting-started and first-feature tutorials.
- [ ] Improve actionable diagnostics from `make doctor`.
- [ ] Make crate and framework API documentation publish-ready.

### Phase 9 — Strategic architecture

- [ ] Decide whether Postgres read-model storage should become `arc-rm-postgres`.
- [ ] Broaden handler manifests and DLQ/redrive tooling without database-writing Benthos outputs.
- [ ] Evaluate additional storage backends behind existing traits.
- [ ] Define the 1.0 compatibility and support policy.

## Decisions and Boundaries

- The event log is authoritative; projections are rebuildable read models.
- Writes go through commands and aggregates.
- In-process event delivery is the synchronous, read-after-write-consistent local default.
- NATS is the distributed publish lane.
- Benthos is the only durable consumer/router of `events.>`.
- Benthos never writes Arc databases.
- Event handlers are external and declared through manifests.
- Browser/admin authentication uses sessions; APIs use JWT bearer authentication.
- Framework code belongs in versioned `arc-*` crates; application/domain code belongs in the
  thin app.
- Generated artifacts are not hand-edited.

## Source of Truth

When sources disagree:

1. Current code in `crates/` and `migrations/`
2. Accepted ADRs in `docs/adr/`
3. This file
4. Current guides in `docs/guides/`
5. Historical plans, implementation notes, and QA reports

