# Roadmap

**Last updated:** 2026-07-27  
**Source:** root `progress.md` (canonical). Update `progress.md` first when status changes.

## Executive status

Arc is a **six-crate**, event-sourced Rust workspace (v0.2.3). Core foundations are implemented: command bus, user aggregate, projections, SQLite/Postgres stores, NATS publish, Benthos-only durable routing, session + JWT auth, snapshots, optional integrity, and an `arc-web` / thin `arc-app` split in the tree.

Framework packaging, documentation hygiene, and product scaffolding remain open.

## Done (foundations) — do not re-plan as greenfield

- [x] Event / Aggregate / CommandBus foundations  
- [x] User aggregate + projection-backed `users_view`  
- [x] Legacy mutable users path removed  
- [x] SQLite + Postgres storage adapters  
- [x] NATS JetStream publishing + subject naming  
- [x] Benthos as sole durable router (ADR 0001); `arc-worker` removed  
- [x] Handler-manifest generator + lint gates  
- [x] Internal HTTP projection delivery  
- [x] User snapshots + integrity hooks  
- [x] Rate limiting, idle timeout, session revocation primitives  
- [x] Playwright e2e harness  

## Current priorities

### 1. Land framework split

- Review/commit `arc-web` extraction; keep doctor boundaries.  
- Docs and upgrade guides match physical layout (`docs/guides/upgrading.md`, ADR 0002).

### 2. Green distributed integration

- Stabilize `benthos_projection_routing` against real `nats-server`.  
- Harden spawn/cleanup for CI and local Docker.

### 3. Publish crates

- Publish/cache `arc-core`; dry-run remaining `arc-es-*`.  
- Publish `arc-web` under lockstep SemVer (ADR 0002).  
- Guide: `docs/guides/publishing-crates.md`.

### 4. `arc new`

- Scaffold thin apps depending on versioned crates.  
- Stop requiring monorepo forks for new products.

### 5. Documentation

- Keep high-level docs free of MVC-as-current wording.  
- Single status surface: `progress.md`.  
- Archive or clearly label historical plans.

### 6. CI and audits

- Reliable NATS provisioning in CI.  
- Stale-doc detection; finish audit workflow intent.

## Near-term product

| Item | Notes |
|------|--------|
| More handler manifests | HTTP/NATS examples, richer DLQ tooling |
| E2E consolidation | Prefer one Playwright layout |
| Postgres session registry | Today JWT revocation is SQLite-only |

## Strategic (later)

| Item | Notes |
|------|--------|
| Plugin / hook system | Domain extension without forking core |
| Publishable framework ergonomics | Docs, versioning, upgrade contract |
| Additional aggregates | Beyond User, as product needs dictate |
| Multi-region / advanced ops | Build on Benthos + NATS, not custom Rust consumers |

## Explicitly out of direction

- Benthos writing Arc databases  
- Restoring `arc-worker` as the durable consumer  
- Treating Diesel CRUD as the write model for domain aggregates  

## How to use this roadmap

1. Change status in **`progress.md`**.  
2. Mirror only durable bullets here if this docsify set is published.  
3. Prefer ADRs for decision history over checkbox archaeology.
