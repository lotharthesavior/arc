# Problems and Improvements

This document outlines identified issues, potential improvements, and "low-hanging fruit" optimizations for the Arc codebase.

---

## Critical Issues

### 1. Duplicate Password Hashing Functions

**Location**: `src/helpers/general.rs:5` and `src/services/user_service.rs:50`

**Problem**: Two identical password hashing functions exist:
- `hash_password()` in `helpers/general.rs`
- `prepare_password()` in `services/user_service.rs`

Both functions do exactly the same thing - hash passwords using Argon2.

**Impact**: Code duplication, maintenance burden, potential for inconsistency.

**Fix**: Remove one and use a single function throughout the codebase.

---

### 2. Connection Pool Recreation on Every Request

**Location**: `src/helpers/database.rs:5-10`

**Problem**: The `get_connection()` function creates a new connection pool on every call:

```rust
pub fn get_connection() -> PooledConnection<ConnectionManager<SqliteConnection>> {
    let pool = get_connection_pool();  // Creates new pool every time!
    let pool = pool.clone();
    pool.get().unwrap()
}
```

**Impact**: Severe performance degradation, defeats purpose of connection pooling.

**Fix**: Use a lazy static or application state to hold the pool:

```rust
use once_cell::sync::Lazy;

static POOL: Lazy<Pool<ConnectionManager<SqliteConnection>>> = Lazy::new(|| {
    get_connection_pool()
});

pub fn get_connection() -> PooledConnection<ConnectionManager<SqliteConnection>> {
    POOL.get().unwrap()
}
```

---

### 3. Bug in Session Message Handling

**Location**: `src/helpers/session.rs:43-47`

**Problem**: When a success message exists, the code incorrectly returns the error field:

```rust
if session_message["success"].is_string() && !session_message["success"].as_str().unwrap().is_empty() {
    session.remove("message");
    return ("error".to_string(), session_message["error"].as_str().unwrap().to_string());
    //      ^^^^^^^ Should be "success"           ^^^^^^^ Should be "success"
}
```

**Impact**: Success messages are never displayed correctly; may panic if error field is missing.

**Fix**:
```rust
return ("success".to_string(), session_message["success"].as_str().unwrap().to_string());
```

---

### 4. Sensitive Data Logging

**Location**: `src/http/controllers/admin_controller.rs:132`

**Problem**: New password is logged to console:

```rust
println!("new password: {}", new_password);
```

**Impact**: Security vulnerability - passwords appear in server logs.

**Fix**: Remove the debug print statement.

---

## Security Concerns

### 5. Missing CSRF Protection

**Status**: ✅ RESOLVED

**Problem**: Forms lack CSRF token validation, making the application vulnerable to Cross-Site Request Forgery attacks.

**Solution Implemented**:
- Created `src/helpers/csrf.rs` module with token generation and validation
- Added CSRF hidden inputs to all forms
- Added validation to all POST endpoints
- Uses constant-time comparison to prevent timing attacks

---

### 6. SQL Injection Protection

**Status**: ✅ SECURE (No action needed)

**Analysis**: The codebase is protected against SQL injection through proper use of Diesel ORM:
- All queries use type-safe query builder (e.g., `.filter(email.eq(&user_email))`)
- No raw SQL queries (`sql_query!`) are used
- User inputs are always passed as parameters, never interpolated into query strings

**Examples of secure patterns used**:
```rust
// Parameterized query - SAFE
users.filter(email.eq(&user_email)).load::<User>(conn)

// Update with parameters - SAFE
diesel::update(users.find(user.id))
    .set((email.eq(user_email.clone()), name.eq(user_name.clone())))
    .execute(&mut get_connection())
```

**Best Practices to Maintain**:
- Continue using Diesel's query builder exclusively
- Never use `diesel::sql_query()` with string interpolation
- Always pass user input as parameters to `.eq()`, `.like()`, etc.

---

### 7. No Rate Limiting

**Problem**: Login endpoint has no rate limiting, enabling brute-force attacks.

**Fix**: Add rate limiting middleware (e.g., `actix-limitation`).

---

### 8. Weak Session Configuration

**Location**: `src/main.rs:118-121`

**Problem**: Session middleware uses default configuration which may not be production-ready.

**Improvements**:
- Set explicit cookie security attributes (HttpOnly, Secure, SameSite)
- Configure session expiration
- Consider using Redis-backed sessions for distributed deployments

---

### 9. Insecure Password Logging in Validation

**Location**: `src/services/user_service.rs:34`

**Problem**: Password hash parsing errors are logged:

```rust
println!("Invalid credentials: Couldn't parse password hash");
```

**Impact**: Reveals information about authentication failures.

**Fix**: Log to a proper logging system with appropriate levels, or remove entirely.

---

## Code Quality Issues

### 10. Unused Import

**Location**: `src/models/user.rs:2`

**Problem**: `SelectStatementAccessor` is imported but never used:

```rust
use diesel::internal::derives::multiconnection::SelectStatementAccessor;
```

**Fix**: Remove the unused import.

---

### 11. Unused Variable in AuthMiddleware

**Location**: `src/http/middlewares/auth_middleware.rs:51-52`

**Problem**: The `user` variable from the Ok branch is unused:

```rust
match user {
    Ok(user) => {},  // user is not used
    Err(e) => { ... }
}
```

**Fix**: Use underscore prefix: `Ok(_user)` or `Ok(_)`.

---

### 12. Inconsistent Error Handling

**Problem**: Mix of `unwrap()`, `expect()`, and `?` operator throughout the codebase.

**Examples**:
- `src/helpers/database.rs:9` - `.unwrap()` on pool.get()
- `src/http/controllers/admin_controller.rs:95` - `.unwrap()` on database operation

**Fix**: Use consistent error handling with `?` operator and proper Result types.

---

### 13. Hardcoded Values

**Location**: Various files

**Problems**:
- Default database path hardcoded in multiple places
- Default pool size hardcoded

**Fix**: Centralize configuration constants or use configuration struct.

---

### 14. Manual Form Parsing Instead of Using Actix Extractors

**Location**: `src/http/controllers/auth_controller.rs:44-46`

**Problem**: Manual form body parsing:

```rust
let email_param: String = get_from_form_body("email".to_string(), req_body.clone());
let password_param: String = get_from_form_body("password".to_string(), req_body);
```

While `admin_controller.rs` properly uses `web::Form<T>`.

**Fix**: Use `web::Form<T>` extractor consistently.

---

### 15. Unused `_req` Parameters

**Locations**:
- `src/http/controllers/admin_controller.rs:30`
- `src/http/controllers/admin_controller.rs:45`

**Problem**: `_req: HttpRequest` parameters are declared but unused.

**Fix**: Remove if not needed.

---

### 16. Template Reinitialization on Every Render

**Location**: `src/helpers/template.rs:7-13`

**Problem**: Tera engine is created fresh on every `load_template()` call:

```rust
let tera = match Tera::new("src/resources/views/**/*") {
    Ok(t) => t,
    Err(e) => { ... }
};
```

**Impact**: Performance overhead from parsing templates repeatedly.

**Fix**: Use a lazy static or application state to cache the Tera instance.

---

## Low-Hanging Fruit Improvements

### 17. Add Input Validation

**Priority**: High

**Problem**: No validation on user input (email format, password strength, name length).

**Fix**: Add validation using a library like `validator` crate:

```rust
use validator::Validate;

#[derive(Validate, Deserialize)]
pub struct UserForm {
    #[validate(length(min = 1, max = 100))]
    name: String,
    #[validate(email)]
    email: String,
}
```

---

### 18. Add Proper Logging

**Priority**: High

**Problem**: Using `println!` for logging instead of a proper logging framework.

**Fix**: Use `tracing` or `log` crate:

```rust
use tracing::{info, warn, error};

info!("User logged in: {}", email);
warn!("Failed login attempt for: {}", email);
```

---

### 19. Environment Variable Validation

**Priority**: Medium

**Problem**: Missing environment variables cause panics at runtime.

**Fix**: Add startup validation to check all required variables exist:

```rust
fn validate_env() -> Result<(), String> {
    let required = ["APP_URL", "SECRET_KEY", "DATABASE_URL"];
    for var in required {
        env::var(var).map_err(|_| format!("{} must be set", var))?;
    }
    Ok(())
}
```

---

### 20. Add Health Check Endpoint

**Priority**: Medium

**Problem**: No health check endpoint for monitoring/load balancers.

**Fix**: Add `/health` endpoint:

```rust
#[get("/health")]
pub async fn health() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({"status": "healthy"}))
}
```

---

### 21. Update `diesel.toml` Path

**Location**: `diesel.toml`

**Problem**: Hardcoded absolute path in configuration:

```toml
file = "/var/www/.../schema.rs"
```

**Fix**: Use relative path:

```toml
file = "src/schema.rs"
```

---

### 22. Add API Versioning

**Priority**: Low

**Problem**: No API versioning for future compatibility.

**Fix**: Add version prefix to routes: `/api/v1/...`

---

### 23. Missing `updated_at` Auto-Update

**Problem**: The `updated_at` column doesn't auto-update on record changes.

**Fix**: Add SQLite trigger or update in application code.

---

### 24. Test Isolation Improvements

**Priority**: Medium

**Problem**: Tests rely on global state and serial execution.

**Fix**: Consider using test containers or in-memory SQLite for better isolation.

---

## Documentation Gaps

### 25. Missing Inline Documentation

**Problem**: Most functions lack doc comments.

**Fix**: Add Rust doc comments:

```rust
/// Validates user credentials against the database.
///
/// # Arguments
/// * `user_email` - The email to validate
/// * `user_password` - The plaintext password to check
///
/// # Returns
/// `UserValidationResult` indicating success or failure type
pub fn validate_user_credentials(user_email: &str, user_password: &str) -> UserValidationResult
```

---

### 26. Missing `.env.example` Complete Template

**Problem**: `.env.example` may not include all variables.

**Fix**: Ensure all required variables are documented:

```env
APP_NAME=Arc
APP_URL=127.0.0.1
APP_PORT=8080
DATABASE_URL=database/database.sqlite
DATABASE_POOL_LIMIT=10
SECRET_KEY=your-secret-key-at-least-64-bytes-long-for-security
```

---

## Performance Improvements

### 27. Static Asset Caching Headers

**Problem**: Static files served without cache headers.

**Fix**: Add cache-control headers for static files:

```rust
file.set_content_disposition(ContentDisposition::inline())
    .customize()
    .insert_header(("Cache-Control", "public, max-age=31536000"))
```

---

### 28. Gzip Compression

**Problem**: Responses are not compressed.

**Fix**: Add compression middleware:

```rust
use actix_web::middleware::Compress;

App::new()
    .wrap(Compress::default())
```

---

## Summary

| Category | Count | Status |
|----------|-------|--------|
| Critical Issues | 4 | See below |
| Security Concerns | 5 | 2 Resolved |
| Code Quality | 7 | Medium Priority |
| Low-Hanging Fruit | 8 | Low-Medium Priority |
| Documentation | 2 | Low Priority |
| Performance | 2 | Low Priority |

### Resolved Issues

| Issue | Status |
|-------|--------|
| #1 - Duplicate Password Hashing | ✅ Resolved |
| #2 - Connection Pool Recreation | ✅ Resolved |
| #3 - Session Message Bug | ✅ Resolved |
| #4 - Sensitive Data Logging | ✅ Resolved |
| #5 - Missing CSRF Protection | ✅ Resolved |
| #6 - SQL Injection | ✅ Secure (no action needed) |

### Remaining Priority Items

1. **High Priority**: Add rate limiting, strengthen session configuration
2. **Medium Priority**: Add logging, input validation, fix inconsistent error handling
3. **Lower Priority**: Documentation, performance optimizations
