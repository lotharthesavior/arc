# Architecture

## Overview

Arc now follows an **event-sourced workspace architecture**: HTTP handlers dispatch commands, aggregates emit events, event stores persist those events, and projections build read models for HTML/API reads. The Actix/Tera web app remains the primary user-facing surface.

> Historical note: some diagrams and examples below still describe the original single-crate MVC starter. Treat them as historical context unless they are explicitly tied to files under `crates/`.

## Architectural Diagram

![Architecture Diagram - MVC Request Flow - Shows HTTP request flowing through Actix Web server, middleware layer, routes, controllers, services and helpers, models, Diesel ORM, to SQLite database](diagrams/architecture-20-mvc-request-flow.svg)

## Design Patterns

### 1. Event-Sourced Write Path

**Commands and aggregates** (`crates/arc-app/src/domain/user/`, `crates/arc-core/src/aggregate.rs`)
- Validate state transitions
- Emit immutable domain events
- Rehydrate state from the event log

**Command bus** (`crates/arc-core/src/command_bus.rs`)
- Loads aggregate history
- Persists new events through `EventStore`
- Publishes persisted events through `EventBus`

**Projections** (`crates/arc-core/src/projection.rs`, `crates/arc-app/src/domain/user/projector.rs`)
- Consume events and build read-model rows such as `users_view`
- Run in-process for `EVENT_BUS=inprocess`; in distributed mode, Benthos pipelines own durable routing and projection/event-handler delivery

### 2. Server-Rendered Web Surface

**Views** (`crates/arc-app/src/resources/views/`)
- Tera HTML templates for server-side rendering
- Organized by section (admin, parts)
- Support for template inheritance and includes

**Controllers** (`crates/arc-app/src/http/controllers/`)
- Handle HTTP requests and responses
- Dispatch commands and read projection rows
- Return rendered templates or JSON responses

### 2. Service Layer Pattern

Services (`src/services/`) encapsulate business logic separate from controllers:

```rust
// user_service.rs
pub fn validate_user_credentials(email: &str, password: &str) -> UserValidationResult
pub fn prepare_password(password: &str) -> String
```

This separation keeps controllers thin and business logic testable.

### 3. Middleware Pattern

The `AuthMiddleware` implements Actix's `Transform` trait to intercept requests:

```rust
pub struct AuthMiddleware;

impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
```

It validates session-based authentication before allowing access to protected routes.

### 4. Repository/ORM Pattern

Diesel ORM provides type-safe database operations:

```rust
// Query with Diesel DSL
let user = users
    .filter(email.eq(&email_param))
    .load::<User>(conn)?;
```

### 5. Seeder Pattern

Database seeders implement a common trait for consistent population:

```rust
pub trait Seeder {
    fn execute(conn: &mut SqliteConnection) -> Result<(), Box<dyn Error>>;
}
```

### 6. Helper/Utility Pattern

Cross-cutting concerns are organized into helper modules:
- `database.rs` - Connection pooling
- `session.rs` - Session management utilities
- `template.rs` - Template rendering with asset injection
- `form.rs` - Form body parsing

## Application State

The application maintains state through `AppState`:

```rust
#[derive(Debug)]
struct AppState {
    app_name: Mutex<String>,
    user_id: Mutex<Option<i32>>,
}
```

This state is shared across handlers via Actix's `web::Data` extractor.

## Request Flow

1. **Request Arrives**: HTTP request hits the Actix server
2. **Middleware Processing**:
   - `NormalizePath` trims trailing slashes
   - `SessionMiddleware` manages cookie sessions
   - `AuthMiddleware` validates protected routes
3. **Route Matching**: Request matched to controller handler
4. **Controller Logic**:
   - Extract session data
   - Call services for business logic
   - Query database via models
5. **Response Generation**:
   - Render Tera template with context
   - Or return JSON for API endpoints
6. **Response Sent**: HTTP response returned to client

## Module Organization

```
src/
├── main.rs              # Entry point, CLI commands
├── routes.rs            # Route configuration
├── schema.rs            # Diesel schema (auto-generated)
│
├── http/
│   ├── controllers/     # Request handlers
│   │   ├── home_controller.rs
│   │   ├── auth_controller.rs
│   │   └── admin_controller.rs
│   └── middlewares/
│       └── auth_middleware.rs
│
├── models/
│   └── user.rs          # User model + migrations
│
├── services/
│   └── user_service.rs  # User business logic
│
├── helpers/
│   ├── database.rs      # Connection pooling
│   ├── session.rs       # Session utilities
│   ├── template.rs      # Template rendering
│   ├── form.rs          # Form parsing
│   ├── general.rs       # General utilities
│   └── test.rs          # Test utilities
│
├── database/
│   └── seeders/
│       ├── create_users.rs
│       └── traits/
│           └── seeder.rs
│
├── console/
│   └── development.rs   # Dev server runner
│
└── resources/
    ├── views/           # Tera templates
    ├── css/             # Stylesheets
    ├── js/              # JavaScript
    └── imgs/            # Images
```

## Configuration

The application uses environment variables for configuration:

| Variable | Description | Default |
|----------|-------------|---------|
| `APP_NAME` | Application name | (none) |
| `APP_URL` | Bind address | (required) |
| `APP_PORT` | Server port | 8080 |
| `DATABASE_URL` | SQLite database path | database/database.sqlite |
| `DATABASE_POOL_LIMIT` | Connection pool size | 10 |
| `SECRET_KEY` | Session encryption key | (required) |
