# Arc Documentation (Docsify)

Welcome to the Arc documentation. This site describes the **current** six-crate, event-sourced workspace (version **0.2.3**), not the historical single-crate MVC starter.

Canonical project status: root `progress.md`.  
Operator guide for agents: root `AGENTS.md`.  
Accepted decisions: `docs/adr/`.

## Table of Contents

1. **[Overview](01-overview.md)** — introduction, stack, crates, quick start  
2. **[Architecture](02-architecture.md)** — write/read paths, bus modes, auth  
3. **[Backend](03-backend.md)** — CLI, routing, controllers, middleware  
4. **[Frontend](04-frontend.md)** — Tera, Stimulus, Turbo, Vite  
5. **[Database](05-database.md)** — event store, projections, drivers, migrations  
6. **[Testing](06-testing.md)** — unit/integration and Playwright e2e  
7. **[API Reference](07-api-reference.md)** — HTTP endpoints  
8. **[Gaps & Improvements](08-problems-and-improvements.md)** — known work remaining  
9. **[Event Sourcing (as built)](09-event-sourcing-architecture.md)**  
10. **[Extending Arc](10-event-sourcing-implementation-guide.md)** — aggregates, handlers  
11. **[Roadmap](roadmap.md)** — priorities from `progress.md`

## Quick links

| Task | Where |
|------|--------|
| First run | [Quick start](01-overview.md#quick-start) |
| Add an HTTP handler | `docs/guides/adding-endpoints.md` |
| Add an event handler | `docs/guides/event-handlers.md` |
| Postgres | `docs/guides/postgres-setup.md` |
| Benthos DLQ | `docs/guides/benthos-dlq-redrive.md` |
| Upgrade / boundaries | `docs/guides/upgrading.md`, `make doctor` |

## Contributing to docs

1. Prefer updating **code comments + ADRs + guides** for durable truth.  
2. Keep this docsify set aligned with the workspace (no Alpine/HTMX/MVC-as-current claims).  
3. Label historical material explicitly if retained.

## License

MIT
