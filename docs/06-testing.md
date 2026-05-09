# Testing Documentation

## Overview

Arc includes a comprehensive test suite covering models, controllers, middleware, and routes. Tests use Actix Web's testing utilities and run serially to avoid database conflicts.

## Test Configuration

### Dependencies

```toml
# Cargo.toml
[dev-dependencies]
serial_test = "2.0"
```

### Environment

Tests use a separate environment file:

```bash
# .env.test
DATABASE_URL=database/test.sqlite
SECRET_KEY=your-test-secret-key-here
APP_NAME=Test App
```

## Test Structure

```
src/
├── models/
│   └── user.rs              # User model tests
├── http/
│   ├── controllers/
│   │   ├── home_controller.rs    # Home controller tests
│   │   ├── auth_controller.rs    # Auth controller tests
│   │   └── admin_controller.rs   # Admin controller tests
│   └── middlewares/
│       └── auth_middleware.rs    # Auth middleware tests
└── routes.rs                # Route tests
```

## Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_can_create_user

# Run tests in specific file
cargo test --test user  # if using separate test files
```

## Test Utilities

### TestFinalizer

Automatically cleans up the test database after test execution:

```rust
// src/helpers/test.rs
pub struct TestFinalizer;

impl Drop for TestFinalizer {
    fn drop(&mut self) {
        let database: String = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "database/database.sqlite".to_string());
        let _ = std::fs::remove_file(database);
    }
}
```

**Usage:**
```rust
#[actix_web::test]
async fn my_test() {
    let _finalizer = TestFinalizer;
    // Test code...
    // Database is cleaned up when _finalizer goes out of scope
}
```

### Test Database Preparation

```rust
fn prepare_test_db() -> PooledConnection<ConnectionManager<SqliteConnection>> {
    dotenv::from_filename(".env.test").ok();
    let mut conn = get_connection();
    conn.run_pending_migrations(MIGRATIONS).expect("Failed to run migrations");
    conn
}

fn seed_users_table() {
    let mut conn = prepare_test_db();
    UserSeeder::execute(&mut conn).expect("Failed to seed users table");
}
```

## Test Patterns

### Serial Test Execution

Tests that modify the database run serially to avoid conflicts:

```rust
use serial_test::serial;

#[serial]
#[actix_web::test]
async fn test_something() {
    // This test runs alone, not in parallel
}
```

### Transaction Testing

Use Diesel's `test_transaction` for isolated tests:

```rust
conn.test_transaction::<_, Error, _>(|conn| {
    // Database operations here
    // Automatically rolled back after test
    Ok(())
});
```

### Actix Web Test Service

```rust
use actix_web::{test, App};

let app = test::init_service(
    App::new()
        .app_data(web::Data::new(AppState { ... }))
        .wrap(SessionMiddleware::new(...))
        .service(my_controller::handler)
).await;
```

### Making Test Requests

```rust
// GET request
let req = test::TestRequest::get()
    .uri("/path")
    .to_request();
let resp = test::call_service(&app, req).await;

// POST request with form data
let req = test::TestRequest::post()
    .uri("/signin")
    .set_form(&[("email", "test@example.com"), ("password", "secret")])
    .to_request();
let resp = test::call_service(&app, req).await;

// Request with cookie
let req = test::TestRequest::get()
    .cookie(parsed_cookie)
    .uri("/admin")
    .to_request();
```

### Asserting Responses

```rust
use actix_web::http;

assert_eq!(resp.status(), http::StatusCode::OK);
assert_eq!(resp.status(), http::StatusCode::FOUND);  // 302 redirect
assert_eq!(resp.status(), http::StatusCode::NOT_FOUND);
```

### Cookie/Session Handling

```rust
// Extract session cookie from response
let headers = resp.headers().clone();
let cookie_header = headers.get("set-cookie").unwrap().to_str().unwrap();
let parsed_cookie = Cookie::parse_encoded(cookie_header).unwrap();

// Use cookie in next request
let next_req = test::TestRequest::get()
    .cookie(parsed_cookie)
    .uri("/admin")
    .to_request();
```

## Test Examples

### Model Tests

```rust
// src/models/user.rs

#[serial]
#[actix_web::test]
async fn test_can_create_user() {
    let _finalizer = TestFinalizer;
    let mut conn = prepare_test_db();

    conn.test_transaction::<_, Error, _>(|conn| {
        let expected_email = "john@email.com";

        diesel::insert_into(users).values(NewUser {
            name: "John Doe",
            email: expected_email,
            password: "password",
        }).execute(conn)?;

        let results: Vec<User> = users
            .filter(email.eq(expected_email))
            .load::<User>(conn)?;

        assert!(results.len() > 0);
        Ok(())
    });
}

#[serial]
#[actix_web::test]
async fn test_can_update_user() {
    let _finalizer = TestFinalizer;
    let mut conn = prepare_test_db();

    conn.test_transaction::<_, Error, _>(|conn| {
        seed_users_table();

        let all_users: Vec<i32> = users.select(id).load::<i32>(conn)?;
        let user_id = all_users[0];

        let new_email = "newemail@example.com";
        diesel::update(users.find(user_id))
            .set(email.eq(new_email))
            .execute(conn)?;

        let user: User = users.filter(email.eq(new_email)).first(conn)?;
        assert_eq!(user.email, new_email);

        Ok(())
    });
}
```

### Controller Tests

```rust
// src/http/controllers/auth_controller.rs

#[serial]
#[actix_web::test]
async fn test_signin_route() {
    let _finalizer = TestFinalizer;
    prepare_test_db();
    seed_users_table();

    let secret_key = Key::from(env::var("SECRET_KEY")
        .expect("SECRET_KEY must be set")
        .as_bytes());

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(AppState {
                app_name: Mutex::from("Test App".to_string()),
                user_id: Mutex::from(None),
            }))
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                secret_key.clone(),
            ))
            .service(auth_controller::signin)
            .service(auth_controller::signin_post)
    ).await;

    // Test GET /signin returns OK
    let req1 = test::TestRequest::get().uri("/signin").to_request();
    let resp1 = test::call_service(&app, req1).await;
    assert_eq!(resp1.status(), http::StatusCode::OK);

    // Test POST /signin redirects on success
    let req2 = test::TestRequest::post()
        .uri("/signin")
        .set_form(&[("email", "jekyll@example.com"), ("password", "password")])
        .to_request();
    let resp2 = test::call_service(&app, req2).await;
    assert_eq!(resp2.status(), http::StatusCode::FOUND);
}
```

### Middleware Tests

```rust
// src/http/middlewares/auth_middleware.rs

#[serial]
#[actix_web::test]
async fn test_auth_middleware() {
    let _finalizer = TestFinalizer;
    prepare_test_db();
    seed_users_table();

    let app = test::init_service(
        App::new()
            .wrap(SessionMiddleware::new(...))
            .service(
                web::resource("/protected")
                    .wrap(AuthMiddleware)
                    .route(web::get().to(|| async { HttpResponse::Ok() }))
            )
    ).await;

    // Unauthenticated request should redirect
    let req = test::TestRequest::get().uri("/protected").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), http::StatusCode::FOUND);
}
```

### Route Tests

```rust
// src/routes.rs

#[actix_web::test]
async fn test_static_file_ok() {
    let app = test::init_service(
        App::new().service(static_file)
    ).await;

    // Create test file
    fs::create_dir_all("./dist").unwrap();
    fs::write("./dist/styles.css", "").unwrap();

    let req = test::TestRequest::get().uri("/public/styles.css").to_request();
    let resp = test::call_service(&app, req).await;

    // Cleanup
    fs::remove_file("./dist/styles.css").unwrap();

    assert_eq!(resp.status(), http::StatusCode::OK);
}

#[actix_web::test]
async fn test_static_file_not_found() {
    let app = test::init_service(
        App::new().service(static_file)
    ).await;

    let req = test::TestRequest::get().uri("/public/nonexistent.css").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), http::StatusCode::NOT_FOUND);
}
```

## Test Coverage Summary

| Module | Tests | Coverage Areas |
|--------|-------|----------------|
| `models/user.rs` | 4 | Create, Read, Update, Delete |
| `controllers/home_controller.rs` | 1 | Home page rendering |
| `controllers/auth_controller.rs` | 1 | Sign-in flow, session management |
| `controllers/admin_controller.rs` | 4 | Dashboard, settings, profile, password change |
| `middlewares/auth_middleware.rs` | 1 | Authentication check |
| `routes.rs` | 2 | Static file serving |

## Best Practices

1. **Use Serial Tests**: Mark database-modifying tests with `#[serial]`
2. **Clean Up**: Use `TestFinalizer` or `test_transaction` for cleanup
3. **Separate Test Environment**: Use `.env.test` for test configuration
4. **Test Both Success and Failure**: Include tests for error cases
5. **Mock External Services**: Avoid real external API calls in tests
6. **Descriptive Names**: Use clear test function names that describe behavior
7. **Arrange-Act-Assert**: Follow the AAA pattern in test structure

## Debugging Tests

```bash
# Run with verbose output
cargo test -- --nocapture

# Run single test with backtrace
RUST_BACKTRACE=1 cargo test test_name -- --nocapture

# Show test output even on success
cargo test -- --show-output
```
