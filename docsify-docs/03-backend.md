# Backend Documentation

## Entry Point

The application entry point is `src/main.rs`. It handles:

1. **Health Checks**: Validates `.env` file existence and database availability
2. **CLI Commands**: Processes command-line arguments
3. **Server Initialization**: Sets up Actix Web with middleware and routes

### CLI Commands

```bash
cargo run <command> [options]
```

| Command | Description | Options |
|---------|-------------|---------|
| `serve` | Run the production server | - |
| `develop` | Run with hot-reload | - |
| `migrate` | Run database migrations | `--fresh` (reset), `--seed` (populate) |
| `seed` | Run database seeders | - |

### Server Initialization

```rust
HttpServer::new(move || {
    App::new()
        .wrap(SessionMiddleware::new(
            CookieSessionStore::default(),
            secret_key.clone(),
        ))
        .wrap(NormalizePath::trim())
        .app_data(web::Data::new(AppState { ... }))
        .configure(routes::config)
})
.bind((app_url, app_port))?
.run()
.await
```

## Routing

Routes are configured in `src/routes.rs`:

### Public Routes

| Method | Path | Handler | Description |
|--------|------|---------|-------------|
| GET | `/` | `home_controller::home` | Home page |
| GET | `/signin` | `auth_controller::signin` | Sign-in form |
| POST | `/signin` | `auth_controller::signin_post` | Process sign-in |
| GET | `/signout` | `auth_controller::signout` | Sign out user |
| GET | `/public/{filename:.*}` | `static_file` | Static file serving |

### Protected Routes (require authentication)

| Method | Path | Handler | Description |
|--------|------|---------|-------------|
| GET | `/admin` | `admin_controller::dashboard` | Admin dashboard |
| GET | `/admin/settings` | `admin_controller::settings` | Settings page |
| GET | `/admin/profile` | `admin_controller::profile` | User profile |
| POST | `/admin/profile` | `admin_controller::profile_post` | Update profile |
| POST | `/admin/profile-password` | `admin_controller::profile_password_post` | Change password |

## Controllers

### HomeController (`src/http/controllers/home_controller.rs`)

Handles the public landing page.

```rust
#[get("/")]
pub async fn home(data: web::Data<AppState>, session: Session) -> impl Responder
```

**Functionality:**
- Checks if user is authenticated
- Retrieves session messages
- Renders `home.html` template

### AuthController (`src/http/controllers/auth_controller.rs`)

Handles authentication flows.

#### Sign-in Page
```rust
#[get("/signin")]
pub async fn signin(data: web::Data<AppState>, session: Session) -> impl Responder
```
- Redirects authenticated users to `/admin`
- Displays session messages (errors/success)

#### Sign-in Processing
```rust
#[post("/signin")]
pub async fn signin_post(req_body: String, session: Session) -> impl Responder
```
- Validates email and password
- Uses `validate_user_credentials()` from UserService
- Sets `user_id` in session on success
- Redirects to `/admin` or back to `/signin` with error

#### Sign-out
```rust
#[get("/signout")]
pub async fn signout(session: Session) -> impl Responder
```
- Removes `user_id` from session
- Sets success message
- Redirects to home page

### AdminController (`src/http/controllers/admin_controller.rs`)

Handles protected admin functionality.

#### Dashboard
```rust
#[get("")]
pub async fn dashboard(data: web::Data<AppState>, session: Session) -> HttpResponse
```

#### Settings
```rust
#[get("/settings")]
pub async fn settings(_req: HttpRequest, data: web::Data<AppState>, session: Session) -> impl Responder
```

#### Profile View
```rust
#[get("/profile")]
pub async fn profile(_req: HttpRequest, data: web::Data<AppState>, session: Session) -> impl Responder
```

#### Profile Update
```rust
#[post("/profile")]
pub async fn profile_post(
    form: web::Form<UserForm>,
    data: web::Data<AppState>,
    session: Session
) -> impl Responder
```
- Accepts `UserForm` with `name` and `email`
- Updates user in database
- Returns JSON response

#### Password Change
```rust
#[post("/profile-password")]
pub async fn profile_password_post(
    form: web::Form<PasswordForm>,
    data: web::Data<AppState>,
    session: Session
) -> impl Responder
```
- Validates current password
- Hashes and saves new password
- Returns JSON response

## Middleware

### AuthMiddleware (`src/http/middlewares/auth_middleware.rs`)

Implements Actix's `Transform` trait for authentication checking.

**Flow:**
1. Extract `user_id` from session
2. Query database for user
3. If user found: proceed to handler
4. If not found: redirect to `/signin`

```rust
fn call(&self, req: ServiceRequest) -> Self::Future {
    let session = req.get_session();
    let user_id: i32 = session.get::<i32>("user_id").unwrap_or(Some(0)).unwrap_or(0);
    let user = users.find(user_id).first::<User>(&mut get_connection());

    match user {
        Ok(_) => { /* proceed */ },
        Err(_) => { /* redirect to /signin */ }
    }
}
```

## Services

### UserService (`src/services/user_service.rs`)

Encapsulates user-related business logic.

#### Credential Validation
```rust
pub fn validate_user_credentials(user_email: &str, user_password: &str) -> UserValidationResult
```

**Returns:**
- `UserValidationResult::Valid` - Credentials match
- `UserValidationResult::InvalidEmail` - Email not found
- `UserValidationResult::InvalidPasswordHash` - Password hash parsing failed
- `UserValidationResult::Invalid` - Password mismatch

#### Password Hashing
```rust
pub fn prepare_password(password_string: &str) -> String
```
- Uses Argon2 with random salt
- Returns hashed password string

## Helpers

### Database Helper (`src/helpers/database.rs`)

Manages database connection pooling.

```rust
pub fn get_connection() -> PooledConnection<ConnectionManager<SqliteConnection>>
pub fn get_connection_pool() -> Pool<ConnectionManager<SqliteConnection>>
```

**Configuration:**
- Default database: `database/database.sqlite`
- Default pool size: 10 connections
- Tests connection on checkout

### Session Helper (`src/helpers/session.rs`)

Provides session utility functions.

```rust
pub fn is_authenticated(session: &Session) -> bool
pub fn get_session_message(session: &Session, is_json: bool) -> (String, String)
pub fn get_session_user(session: &Session) -> Option<User>
```

### Template Helper (`src/helpers/template.rs`)

Handles Tera template rendering with asset injection.

```rust
pub fn load_template(template: &str, params: Vec<(&str, &str)>, assets: Option<Vec<&str>>) -> String
```

**Features:**
- Loads templates from `src/resources/views/**/*`
- Injects asset URLs from Vite manifest
- Supports selective asset inclusion

### Form Helper (`src/helpers/form.rs`)

Parses URL-encoded form bodies.

```rust
pub fn get_from_form_body(field: String, req_body: String) -> String
```

### Test Helper (`src/helpers/test.rs`)

Provides test utilities.

```rust
pub struct TestFinalizer;
```
- Implements `Drop` to clean up test database after tests

## Models

### User Model (`src/models/user.rs`)

```rust
#[derive(Queryable, Selectable, Debug)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub password: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Insertable)]
pub struct NewUser<'a> {
    pub name: &'a str,
    pub email: &'a str,
    pub password: &'a str,
}
```

Embeds database migrations:
```rust
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();
```

## Development Mode

The `src/console/development.rs` module runs two concurrent processes:

1. **Cargo Watch**: Rebuilds and restarts server on Rust file changes
2. **Vite Watch**: Rebuilds frontend assets on CSS/JS changes

```rust
pub async fn run_development() -> io::Result<()> {
    let cargo_watch_task = tokio::spawn(run_cargo_watch());
    let bundle_task = tokio::spawn(run_vite_bundle());
    try_join!(cargo_watch_task, bundle_task)?;
    Ok(())
}
```

**Ignored directories for cargo-watch:**
- `database/*`
- `dist/*`
- `node_modules/*`

## Error Handling

The codebase uses various error handling patterns:

1. **Result Types**: Most functions return `Result<T, E>`
2. **Option Types**: Used for nullable values (e.g., session user)
3. **Expect/Unwrap**: Used for unrecoverable errors with descriptive messages
4. **HTTP Status Codes**: Controllers return appropriate status codes

Example error response:
```rust
HttpResponse::InternalServerError()
    .json(serde_json::json!({"errors": {"server_error": "Failed to update user"}}))
```
