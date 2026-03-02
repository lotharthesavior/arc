# Problems and Improvements

This document outlines identified issues, potential improvements, and "low-hanging fruit" optimizations for the Nineties codebase.

---

## Critical Issues

### 1. Duplicate Password Hashing Functions

**Status: ✅ RESOLVED**

**Location**: `src/helpers/general.rs:5` and `src/services/user_service.rs:50`

**Problem**: Two identical password hashing functions exist:
- `hash_password()` in `helpers/general.rs`
- `prepare_password()` in `services/user_service.rs`

Both functions do exactly the same thing - hash passwords using Argon2.

**Impact**: Code duplication, maintenance burden, potential for inconsistency.

**Fix**: Remove one and use a single function throughout the codebase.

---

### 2. Connection Pool Recreation on Every Request

**Status: ✅ RESOLVED**

**Location**: `src/helpers/database.rs`

**Problem**: The `get_connection()` function used to create a new connection pool on every call.

**Solution Implemented**: Uses `OnceLock<RwLock<Option<PoolState>>>` for a singleton connection pool with dynamic database URL support (test-friendly). Pool configuration centralized in `src/helpers/config.rs`.

---

### 3. Bug in Session Message Handling

**Status: ✅ RESOLVED**

**Location**: `src/helpers/session.rs`

**Problem**: When a success message existed, the code incorrectly returned the error field.

**Solution Implemented**: Corrected field references in session message handling.

---

### 4. Sensitive Data Logging

**Status: ✅ RESOLVED**

**Location**: `src/http/controllers/admin_controller.rs`

**Problem**: New password was logged to console.

**Solution Implemented**: Debug print statement removed.

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

**Best Practices to Maintain**:
- Continue using Diesel's query builder exclusively
- Never use `diesel::sql_query()` with string interpolation
- Always pass user input as parameters to `.eq()`, `.like()`, etc.

---

### 7. No Rate Limiting

**Status: ✅ RESOLVED** — login endpoints use strict rate limiting via `actix-limitation`, and a global rate limiting middleware (`GlobalRateLimit`) protects all endpoints via `actix-web-lab::from_fn`.

**Solution Implemented**:
- Login rate limit: configurable via `RATE_LIMIT_MAX_REQUESTS` (default: 5) and `RATE_LIMIT_PERIOD_SECS` (default: 60)
- Global rate limit: configurable via `GLOBAL_RATE_LIMIT_MAX_REQUESTS` (default: 100) and `GLOBAL_RATE_LIMIT_PERIOD_SECS` (default: 60)
- Global middleware skips `/health` and `/public/*` static file paths
- All rate limit responses include `Retry-After` header (RFC 6585)
- Middleware defined in `src/http/middlewares/rate_limit_middleware.rs`

---

### 8. Weak Session Configuration

**Status: ✅ RESOLVED** — HttpOnly, Secure (prod), SameSite (configurable), 24hr TTL, custom cookie name. See docs/SESSION_CONFIGURATION.md.

---

### 9. Insecure Password Logging in Validation

**Status: ✅ RESOLVED** — replaced with `tracing::warn!` logging user ID only.

---

## Code Quality Issues

### 10. Unused Import

**Status: ✅ RESOLVED** — removed.

---

### 11. Unused Variable in AuthMiddleware

**Status: ✅ RESOLVED** — middleware rewritten with `is_authenticated()` bool check.

---

### 12. Inconsistent Error Handling

**Status: ✅ RESOLVED** — critical paths now use proper `match` with error handling instead of bare `unwrap()`.

**Solution Implemented**:
- `user_service.rs`: `validate_user_credentials()` uses `match` for DB errors and password hash parsing
- `admin_controller.rs`: All `diesel::update()` calls wrapped in `match` with `tracing::error!` logging
- `prepare_password()` uses descriptive `.expect()` for the infallible argon2 hash operation
- Session/mutex operations retain `.unwrap()` as these are framework-level failures

---

### 13. Hardcoded Values

**Status: ✅ RESOLVED** — centralized in `src/helpers/config.rs`.

**Solution Implemented**:
- Created `src/helpers/config.rs` with `DEFAULT_DATABASE_URL`, `DEFAULT_POOL_LIMIT`, `database_url()`, and `database_pool_limit()` functions
- `database.rs` now uses `config::database_url()` and `config::database_pool_limit()` instead of hardcoded values
- `main.rs` uses `helpers::config::database_url()` for database health check

---

### 14. Manual Form Parsing Instead of Using Actix Extractors

**Status: ✅ RESOLVED** — `auth_controller.rs` now uses `web::Form<SigninForm>` extractor.

**Solution Implemented**:
- Created `SigninForm` struct with `Deserialize` derive
- Replaced manual `get_from_form_body()` calls with `web::Form<SigninForm>` extractor in `signin_post()`
- Form fields accessed via `form.email`, `form.password`, `form.csrf_token`

---

### 15. Unused `_req` Parameters

**Status: ✅ RESOLVED** — removed from `settings()` and `profile()` handlers.

---

### 16. Template Reinitialization on Every Render

**Status: ✅ RESOLVED** — uses `once_cell::sync::Lazy` for singleton Tera instance.

---

## Low-Hanging Fruit Improvements

### 17. Add Input Validation

**Status: ✅ RESOLVED** — validation framework integrated into controllers.

**Solution Implemented**:
- `LoginForm` validation wired into `auth_controller::signin_post()` — validates email format and non-empty password
- `UpdateProfileForm` validation wired into `admin_controller::profile_post()` — validates name length and email format
- `#[allow(dead_code)]` attributes removed from validation structs
- Validation errors return appropriate HTTP responses (400 Bad Request for API, redirect with flash message for browser)

---

### 18. Add Proper Logging

**Status: ✅ RESOLVED** — migrated to `tracing` crate throughout codebase.

---

### 19. Environment Variable Validation

**Status: ✅ RESOLVED** — unified `validate_environment()` function added to `main.rs`.

**Solution Implemented**:
- `validate_environment()` checks for all required env vars (`APP_URL`, `SECRET_KEY`, `DATABASE_URL`) at startup
- Logs clear error message listing all missing variables
- Calls `exit(1)` if any are missing — fail-fast before any server initialization
- Called immediately after `dotenv().ok()` in `main()`

---

### 20. Add Health Check Endpoint

**Status: ✅ RESOLVED** — `/health` endpoint added.

**Solution Implemented**:
- `GET /health` returns `200 OK` with JSON: `{"status": "healthy", "version": "0.2.2"}`
- Version pulled from `CARGO_PKG_VERSION` at compile time
- Defined in `src/routes.rs`
- Excluded from global rate limiting

---

### 21. Update `diesel.toml` Path

**Status: ✅ RESOLVED** — both schema path and migrations directory now use relative paths.

**Solution Implemented**:
- Schema: `file = "src/schema.rs"` (was already fixed)
- Migrations: `dir = "migrations"` (was absolute `/var/www/JackedPHP/nineties-test/test-1/migrations`)

---

### 22. Add API Versioning

**Status: ✅ RESOLVED** — API routes available under `/api/v1/` with backwards-compatible `/api/` routes.

**Solution Implemented**:
- New versioned routes: `/api/v1/login`, `/api/v1/protected/profile`
- Legacy routes still active: `/api/login`, `/api/protected/profile`
- Defined in `src/routes.rs`

---

### 23. Missing `updated_at` Auto-Update

**Status: ✅ RESOLVED** — application-level auto-update implemented.

**Solution Implemented**:
- `admin_controller::profile_post()` now sets `updated_at` to current UTC timestamp on profile update
- `admin_controller::profile_password_post()` now sets `updated_at` on password change
- Uses `chrono::Utc::now()` formatted as `"%Y-%m-%d %H:%M:%S"`

---

### 24. Test Isolation Improvements

**Status: ✅ RESOLVED** — test infrastructure supports both file-based and in-memory SQLite.

**Solution Implemented**:
- `TestFinalizer` RAII guard: resets connection pool and cleans up file-based test databases on drop
- `InMemoryTestGuard` RAII guard: supports `:memory:` SQLite databases for true per-test isolation without file I/O
- `reset_pool()` function clears the global pool between tests
- `#[serial]` attribute ensures sequential execution for integration tests that share state
- `test_transaction()` used in model tests for automatic rollback
- Full in-memory isolation will be the default in the Event Sourcing workspace restructuring

---

## Documentation Gaps

### 25. Missing Inline Documentation

**Status: ✅ RESOLVED** — all public functions, structs, traits, and enums now have `///` doc comments.

**Solution Implemented**:
- All controllers: `admin_controller`, `auth_controller`, `api_controller`, `home_controller`
- All middlewares: `AuthMiddleware`, `JwtMiddleware`, `global_rate_limit`
- All helpers: `database`, `session`, `template`, `form`, `jwt`, `csrf`, `config`, `rate_limit`, `general`, `test`
- All commands: `serve`, `develop`, `migrate`, `seed`
- Models: `User`, `NewUser`, `MIGRATIONS`
- Services: `UserValidationResult` enum variants, `validate_user_credentials`, `prepare_password`
- Seeders: `Seeder` trait, `UserSeeder`
- Validation: `LoginForm`, `RegisterForm`, `UpdateProfileForm`, `ChangePasswordForm`
- WebSocket: `WsConnection`, `WsServer`, all message types

---

### 26. Missing `.env.example` Complete Template

**Status: ✅ RESOLVED** — now includes all variables with descriptive comments, including global rate limit configuration.

---

## Performance Improvements

### 27. Static Asset Caching Headers

**Status: ✅ RESOLVED** — custom handler with ETag, conditional requests, immutable cache for hashed assets.

---

### 28. Gzip Compression

**Status: ✅ RESOLVED** — `Compress::default()` middleware added to App builder in `src/commands/serve.rs`.

**Solution Implemented**:
```rust
App::new()
    .wrap(Compress::default())
```

---

## Newly Discovered Issues

### 29. `get_session_user().unwrap()` Panics in Admin Handlers

**Status: ✅ RESOLVED** — all admin handlers now use `match get_session_user()` with redirect to `/signin` on `None`.

**Solution Implemented**:
- `dashboard()`, `settings()`, `profile()`, `profile_post()`, `profile_password_post()` all use:
  ```rust
  let user: User = match get_session_user(&session) {
      Some(u) => u,
      None => return HttpResponse::SeeOther().insert_header(("Location", "/signin")).finish()
  };
  ```

---

### 30. Rate Limit Response Doesn't Redirect Browser

**Status: ✅ RESOLVED** — rate limit on `/signin` POST now returns `303 See Other` with `Location: /signin` instead of `429 Too Many Requests`.

**Solution Implemented**:
- Changed `HttpResponse::TooManyRequests()` to `HttpResponse::SeeOther()` for browser form submissions
- Flash message is properly displayed after redirect
- API endpoint (`/api/login`) correctly returns `429` (API clients handle this properly)

---

### 31. Missing Retry-After Header on Rate Limit Responses

**Status: ✅ RESOLVED** — `Retry-After` header added to all rate limit responses.

**Solution Implemented**:
- `auth_controller::signin_post()`: `.insert_header(("Retry-After", rate_limit_period.to_string()))`
- `api_controller::login()`: `.insert_header(("Retry-After", rate_limit_period.to_string()))`
- Global rate limit middleware: `.insert_header(("Retry-After", rate_limit_period.to_string()))`
- Period derived from `RATE_LIMIT_PERIOD_SECS` / `GLOBAL_RATE_LIMIT_PERIOD_SECS` env vars

---

### 32. diesel.toml Hardcoded Absolute Migrations Path

**Status: ✅ RESOLVED** — changed to `dir = "migrations"`.

---

### 33. Validation Structs Are Dead Code

**Status: ✅ RESOLVED** — validation structs wired into controllers, `#[allow(dead_code)]` removed.

**Solution Implemented**:
- `LoginForm` used in `auth_controller::signin_post()` for email/password validation
- `UpdateProfileForm` used in `admin_controller::profile_post()` for name/email validation
- `#[allow(dead_code)]` removed from `src/validation/mod.rs` and individual structs

---

### 34. CSRF Validation Inconsistency

**Status: ✅ RESOLVED** — all POST endpoints now use `validate_and_regenerate_csrf_token()` (single-use tokens).

**Solution Implemented**:
- `admin_controller.rs`: Changed `validate_csrf_token()` to `validate_and_regenerate_csrf_token()` in both `profile_post()` and `profile_password_post()`
- `auth_controller.rs`: Already used `validate_and_regenerate_csrf_token()` — no change needed
- Consistent single-use CSRF token security across all endpoints

---

### 35. Remaining `eprintln!` Bypasses Structured Logging

**Status: ✅ RESOLVED** — replaced with `tracing` macros.

**Solution Implemented**:
- `src/helpers/template.rs:11`: `eprintln!` → `tracing::error!` (fatal template parse error)
- `src/helpers/rate_limit.rs:32`: `eprintln!` → `tracing::warn!` (rate limiter fallback warning)

---

### 36. `tracing-actix-web` Dependency Unused

**Status: ✅ RESOLVED** — wired as middleware in `src/commands/serve.rs`.

**Solution Implemented**:
```rust
App::new()
    .wrap(tracing_actix_web::TracingLogger::default())
```
Provides structured request/response logging with trace IDs.

---

### 37. Performance: Auth Middleware DB Query on Every Request

**Status: ✅ RESOLVED** — session caching implemented; DB query only on first request per session.

**Solution Implemented** (already in place):
- `is_authenticated()` first checks for cached `user_data` in session
- Falls back to DB query only if `user_id` exists but `user_data` doesn't
- `set_session_user()` stores both `user_id` and `user_data` after login
- Subsequent requests read from session cache — zero DB queries

---

## Summary

| Category | Total | Resolved | Partial | Open |
|----------|-------|----------|---------|------|
| Critical Issues (1-4) | 4 | 4 | 0 | 0 |
| Security Concerns (5-9) | 5 | 5 | 0 | 0 |
| Code Quality (10-16) | 7 | 7 | 0 | 0 |
| Low-Hanging Fruit (17-24) | 8 | 8 | 0 | 0 |
| Documentation (25-26) | 2 | 2 | 0 | 0 |
| Performance (27-28) | 2 | 2 | 0 | 0 |
| Newly Discovered (29-37) | 9 | 9 | 0 | 0 |
| **TOTAL** | **37** | **37** | **0** | **0** |

### All Issues Resolved (37/37)

| Issue | Status |
|-------|--------|
| #1 - Duplicate Password Hashing | ✅ Resolved |
| #2 - Connection Pool Recreation | ✅ Resolved |
| #3 - Session Message Bug | ✅ Resolved |
| #4 - Sensitive Data Logging | ✅ Resolved |
| #5 - Missing CSRF Protection | ✅ Resolved |
| #6 - SQL Injection | ✅ Secure (no action needed) |
| #7 - Rate Limiting | ✅ Resolved — global + login-specific |
| #8 - Weak Session Configuration | ✅ Resolved |
| #9 - Insecure Password Logging | ✅ Resolved |
| #10 - Unused Import | ✅ Resolved |
| #11 - Unused Variable AuthMiddleware | ✅ Resolved |
| #12 - Inconsistent Error Handling | ✅ Resolved |
| #13 - Hardcoded Values | ✅ Resolved |
| #14 - Manual Form Parsing | ✅ Resolved |
| #15 - Unused `_req` Parameters | ✅ Resolved |
| #16 - Template Reinitialization | ✅ Resolved |
| #17 - Input Validation | ✅ Resolved |
| #18 - Add Proper Logging | ✅ Resolved |
| #19 - Env Validation | ✅ Resolved |
| #20 - Health Check Endpoint | ✅ Resolved |
| #21 - diesel.toml Path | ✅ Resolved |
| #22 - API Versioning | ✅ Resolved |
| #23 - updated_at Auto-Update | ✅ Resolved |
| #24 - Test Isolation | ✅ Resolved |
| #25 - Inline Documentation | ✅ Resolved |
| #26 - .env.example Template | ✅ Resolved |
| #27 - Static Asset Caching | ✅ Resolved |
| #28 - Gzip Compression | ✅ Resolved |
| #29 - Session User Panics | ✅ Resolved |
| #30 - Rate Limit Redirect | ✅ Resolved |
| #31 - Retry-After Header | ✅ Resolved |
| #32 - diesel.toml Migrations Path | ✅ Resolved |
| #33 - Validation Dead Code | ✅ Resolved |
| #34 - CSRF Inconsistency | ✅ Resolved |
| #35 - eprintln! Bypass | ✅ Resolved |
| #36 - tracing-actix-web Unused | ✅ Resolved |
| #37 - Auth Middleware DB Query | ✅ Resolved |

---

## Event Sourcing Migration Impact

Issues in this document have been assessed against the planned Event Sourcing migration (see docs/09-event-sourcing-architecture.md).

**Previously blocking issues — now resolved:**
- ~~#12 (Inconsistent error handling)~~ ✅ — no longer blocks Result-based command/event pipeline
- ~~#21 (diesel.toml absolute path)~~ ✅ — no longer breaks workspace restructuring
- ~~#2 (Connection pool strategy)~~ ✅ — centralized in config.rs, ready for pluggable multi-store design
- ~~#24 (Test isolation)~~ ✅ — in-memory SQLite guard added; full isolation in ES Phase 4

**Issues that become irrelevant after ES migration:**
- #1, #3, #10, #11, #14, #15, #22, #23 — code will be replaced by ES architecture

**Contradictions with ES architecture:**
- #8 suggests Redis sessions; ES architecture prefers local-first storage
- #17 targets form structs; ES needs validation on command types (both patterns now implemented)
- #6 says "never use sql_query"; ES projections may need it

---

## HIPAA & Government Compliance Considerations

The following items should be prioritized for organizations requiring HIPAA compliance or government-grade security:

### Audit Trail (Critical for HIPAA)
- **Event sourcing provides immutable audit logs** — every state change is recorded as an event with timestamp, actor, and payload
- The `tracing` framework now captures structured logs suitable for security auditing
- **Recommendation**: Add audit event types for login, logout, data access, and data modification (roadmap Phase 1)

### Access Controls
- Authentication middleware caches sessions securely (no DB query per request)
- CSRF protection with single-use tokens on all forms
- Rate limiting on all endpoints prevents brute-force attacks
- **Recommendation**: Add role-based access control (RBAC) — roadmap Phase 2

### Data Protection
- Passwords hashed with Argon2 (OWASP-recommended)
- No sensitive data logging (password logging removed)
- `#[serde(skip_serializing)]` on password field prevents accidental exposure
- **Recommendation**: Add encryption at rest for PII fields, data retention policies — roadmap Phase 3

### Network Security
- Session cookies: HttpOnly, Secure (production), SameSite
- Gzip compression reduces attack surface for BREACH-type attacks (mitigated by CSRF tokens)
- **Recommendation**: Add Content-Security-Policy, X-Frame-Options, HSTS headers — roadmap Phase 2

### Monitoring & Incident Response
- Health check endpoint for uptime monitoring
- Structured logging via `tracing` with `tracing-actix-web` request tracing
- **Recommendation**: Add alerting on suspicious patterns (multiple failed logins, unusual data access) — roadmap Phase 5
