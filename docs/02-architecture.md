# Architecture

## Overview

Arc follows a traditional **MVC (Model-View-Controller)** architecture pattern with additional layers for services and helpers. The application is built around the Actix Web framework, which provides an actor-based, asynchronous HTTP server.

## Architectural Diagram

![Architecture Diagram - MVC Request Flow - Shows HTTP request flowing through Actix Web server, middleware layer, routes, controllers, services and helpers, models, Diesel ORM, to SQLite database](diagrams/architecture-20-mvc-request-flow.svg)

## Design Patterns

### 1. MVC Pattern

**Models** (`src/models/`)
- Define data structures that map to database tables
- Handle ORM-related logic via Diesel derives
- Example: `User` struct with `Queryable`, `Selectable`, `Insertable` derives

**Views** (`src/resources/views/`)
- Tera HTML templates for server-side rendering
- Organized by section (admin, parts)
- Support for template inheritance and includes

**Controllers** (`src/http/controllers/`)
- Handle HTTP requests and responses
- Orchestrate data flow between models and views
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
