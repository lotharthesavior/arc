# Nineties Framework Conventions

Durable rules for how code is organized and named in this repository. These are
the rules that the framework itself depends on — breaking them breaks the
architecture, not just aesthetics.

## Aggregate layout

Every aggregate lives under `crates/nineties-app/src/domain/<entity>/` with
these files:

```
domain/<entity>/
├── mod.rs          pub mod aggregate; commands; events;
├── aggregate.rs    the <Entity>Aggregate struct + Aggregate impl
├── commands.rs     the <Entity>Command enum + Command impl
└── events.rs       the <Entity>DomainEvent enum (wire shape)
```

No business logic outside `aggregate.rs`. Controllers dispatch commands; they
do not touch the event store directly.

## Naming

| Concept    | Form                    | Example                              |
|------------|-------------------------|--------------------------------------|
| Command    | Imperative, PascalCase  | `RegisterUser`, `UpdateProfile`      |
| Event      | Past tense, PascalCase  | `UserRegistered`, `ProfileUpdated`   |
| Aggregate  | `<Entity>Aggregate`     | `UserAggregate`                      |
| Error      | `<Entity>AggregateError`| `UserAggregateError`                 |
| NATS subject (Step 3+) | `events.<entity>.<event_type>` (lowercase, snake_case) | `events.user.user_registered` |

## Command → Event → State

Three invariants every aggregate must satisfy:

1. **`handle()` never mutates `self`.** It inspects state, validates, and
   returns `Vec<Event>` or `Err`. No DB calls, no HTTP calls, no randomness
   beyond event id / timestamp (which `Event::new` handles).
2. **`apply()` is deterministic and side-effect free.** Given the same event
   stream, the aggregate must always end in the same state. No I/O, no clock
   reads, no RNG.
3. **Every `apply()` branch sets `self.version = event.sequence`.** The default
   impl in aggregate.rs does this before the match — don't override unless you
   know why.

## Where things live

| Thing                              | Crate / path                                       |
|------------------------------------|----------------------------------------------------|
| Trait: `Aggregate`, `Command`, `EventStore`, `EventBus` | `nineties-core`    |
| `Event` struct                     | `nineties-core`                                    |
| SQLite event store                 | `nineties-es-sqlite`                               |
| Domain aggregates                  | `nineties-app/src/domain/<entity>/`                |
| HTTP controllers                   | `nineties-app/src/http/controllers/`               |
| Service helpers (password hash, email index) | `nineties-app/src/services/`             |
| `AppError` + `ResponseError` impl  | `nineties-app/src/http/errors.rs`                  |
| Migrations                         | `migrations/` (workspace root)                     |

Never reach across: a domain module must not import a controller; a controller
must not query the event store directly (dispatch a command or load through
`command_bus.event_store()`).

## Write path

Writes go through `CommandBus` only. A handler that mutates user data through
Diesel is a bug — delete the Diesel call, dispatch a command. The `users`
Diesel table is transitional and read-only for new registrations; Step 2's
projector replaces it with `users_view`.

## HTTP error mapping

All `CommandBusError` variants map to HTTP via `impl ResponseError for AppError`
in `http/errors.rs`. Do not hand-roll `HttpResponse::BadRequest` from a
controller when the failure is a command error — return `AppError` and let the
mapping handle it.

| `CommandBusError`                      | HTTP status |
|----------------------------------------|-------------|
| `HandleFailed`                         | 422         |
| `AppendFailed(ConcurrencyConflict)`    | 409         |
| `LoadFailed`                           | 404         |
| Everything else                        | 500         |

## Adding a new aggregate

Run `make new-aggregate NAME=Task` to scaffold `domain/task/`. Edit the
commands, events, and `handle()`/`apply()` implementations. Register the
aggregate's `CommandBus` in `commands/serve.rs`. Then add routes that dispatch
to it.
