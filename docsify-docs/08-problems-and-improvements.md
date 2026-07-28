# Gaps & Improvements

**Last updated:** 2026-07-27

This page tracks **remaining** work relative to the current codebase. Completed foundations (ES core, SQLite/Postgres, NATS publish, Benthos routing, user aggregate, snapshots, integrity, arc-web split in tree, etc.) are listed in root `progress.md` — do not re-open them as if unfinished.

## Immediate priorities

1. **Land / review the `arc-web` ↔ thin `arc-app` split**  
   Physical split exists in the working tree; keep `make doctor` boundaries green and commit/review as needed.

2. **Green distributed integration**  
   `benthos_projection_routing` has failed when local `nats-server` did not accept connections. Re-run in a clean NATS environment; harden startup/cleanup if it reproduces.

3. **Publish framework crates**  
   Dry-run/publish `arc-core`, `arc-es-sqlite`, `arc-es-postgres`, `arc-es-nats`, and eventually `arc-web` under ADR 0002 lockstep SemVer policy. See `docs/guides/publishing-crates.md`.

4. **`arc new` scaffolding**  
   Generate thin apps that depend on versioned crates instead of forking the monorepo.

5. **Documentation reconciliation**  
   High-level docs still lag in places (this docsify set is updated; keep `docs/` and agent notes aligned). Prefer code + ADRs + `progress.md`.

6. **CI / audit hardening**  
   Consolidate NATS install/startup in CI; stale-document detection; complete `make audit` intent.

## Architectural / product backlog

| Area | Status |
|------|--------|
| Plugin / hook system | Strategic — not shipped as a framework extension surface |
| Broader Benthos handler ecosystem | Base pipelines + generator exist; more manifests/tooling welcome |
| SQL outputs from Benthos | **Rejected by design** (ADR 0001) |
| JWT revocation on non-SQLite primary | Registry is SQLite-only today (separate file under Postgres driver) |
| Dual e2e style (`.mjs` vs `specs/*.ts`) | Consolidate over time |
| Historical MVC wording in old markdown | Clean or archive |

## Security / ops checklist (ongoing)

- Rotate any secrets that ever landed in shared history; use `.env.example` as the only committed template for production-shaped values.
- Keep `INTERNAL_PROJECTION_TOKEN` and integrity keys strong and out of client bundles.
- Ensure production `SECRET_KEY` / `JWT_SECRET` differ from test/e2e defaults.
- DLQ/redrive: follow `docs/guides/benthos-dlq-redrive.md` rather than reimplementing consumers in Rust.

## What not to “fix”

- Reintroducing Diesel CRUD as the user write path  
- Reintroducing `arc-worker` as the durable consumer  
- Letting Benthos write Arc databases directly  
- Dual-maintaining contradictory architecture stories without marking history

## Source for live status

Always prefer **`progress.md`** over this page if dates diverge.
