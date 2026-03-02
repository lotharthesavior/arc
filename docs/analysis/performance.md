# Performance Analysis: Nineties vs Minimal TCP Server

## Executive Summary

The minimal TCP server achieves **~50,000 req/sec** while Nineties achieves only **~172 req/sec** - a **290x performance difference**.

This document analyzes why this gap exists and provides actionable recommendations to improve Nineties' throughput.

---

## Baseline Comparison

| Metric | Minimal TCP Server | Nineties | Difference |
|--------|-------------------|----------|------------|
| Requests/sec | ~50,000 | ~172 | 290x slower |
| Framework | None (raw TCP) | Actix-web | - |
| Template Engine | None | Tera (per-request) | - |
| Database | None | SQLite + Diesel | - |
| Session Management | None | Cookie-based | - |
| Static Files | None | Dynamic serving | - |

---

## Root Cause Analysis

### 1. CRITICAL: Template Compilation on Every Request

**Location**: `src/helpers/template.rs:6-27`

```rust
pub fn load_template(template: &str, params: Vec<(&str, &str)>, components: Option<Vec<(&str, &str)>>) -> String {
    let tera = match Tera::new("src/resources/views/**/*") {  // <-- CREATED EVERY REQUEST
```

**Impact**: This is the single most expensive operation.

- **Filesystem glob** (`**/*`) scans the entire views directory
- **Template parsing** compiles all templates into AST
- **No caching** - work is thrown away after each request
- Estimated cost: **50-200ms per request**

**Why the minimal server is fast**: It has no template engine - just returns a static string.

---

### 2. CRITICAL: Manifest File I/O on Every Request

**Location**: `src/helpers/template.rs:60-85`

```rust
pub fn get_manifest_assets() -> HashMap<String, String> {
    let manifest_content = fs::read_to_string("dist/.vite/manifest.json");  // <-- FILE I/O EVERY REQUEST
    // ...
    let manifest: HashMap<String, Value> = serde_json::from_str();  // <-- JSON PARSING EVERY REQUEST
```

**Impact**:
- Disk I/O on every page render
- JSON deserialization on every request
- Estimated cost: **5-20ms per request**

---

### 3. HIGH: Database Query on Every Authenticated Request

**Location**: `src/http/middlewares/auth_middleware.rs:46-69`

```rust
fn call(&self, req: ServiceRequest) -> Self::Future {
    let session = req.get_session();
    let user_id: i32 = session.get::<i32>("user_id")...;
    let user = users.find(user_id).first::<User>(&mut get_connection());  // <-- DB QUERY EVERY REQUEST
```

**Impact**:
- Every request to `/admin/*` routes triggers a database query
- No user caching in session or memory
- Estimated cost: **5-20ms per request**

---

### 4. HIGH: Database Queries in Session Helpers

**Location**: `src/helpers/session.rs:7-12, 54-63`

```rust
pub fn is_authenticated(session: &Session) -> bool {
    // ...queries database to verify user exists
}

pub fn get_session_user(session: &Session) -> Option<User> {
    // ...queries database to fetch user
}
```

**Impact**:
- Called in multiple controllers (home, auth, admin)
- Each call hits the database
- Some pages make 2-3 of these calls
- Estimated cost: **5-20ms per call**

---

### 5. HIGH: Duplicate Database Queries on Sign-in

**Location**: `src/http/controllers/auth_controller.rs:70-107`

```rust
match validate_user_credentials(&email_param, &password_param) {
    // validate_user_credentials() queries DB for user...
    UserValidationResult::Valid => {
        let user_results = users
            .filter(email.eq(&email_param))
            .load::<User>(&mut get_connection());  // <-- QUERIES DB AGAIN FOR SAME USER
```

**Impact**:
- User is fetched twice during login
- Estimated cost: **10-40ms wasted per login**

---

### 6. MEDIUM: No HTTP Caching Headers on Static Files

**Location**: `src/routes.rs:8-14`

```rust
#[get("/public/{filename:.*}")]
async fn serve_static(path: web::Path<String>) -> impl Responder {
    let file_path = format!("src/resources/public/{}", path.into_inner());
    NamedFile::open(file_path.parse::<PathBuf>().unwrap())
    // No Cache-Control, ETag, or Last-Modified headers
}
```

**Impact**:
- Browsers re-request static assets every page load
- No CDN caching possible
- Increased bandwidth and server load

---

### 7. MEDIUM: Inefficient Manual Form Parsing

**Location**: `src/helpers/form.rs:2-12`

```rust
pub fn get_from_form_body(key: String, body: String) -> String {
    let params = body.split("&");  // Manual parsing instead of web::Form
    for param in params {
        if param.starts_with(&key_to_search) {
            // ...
```

**Impact**:
- Called multiple times per form submission
- Linear search through parameters
- String allocation overhead
- Estimated cost: **2-5ms per request**

---

### 8. MEDIUM: Lock Contention on AppState

**Location**: `src/main.rs:70-73`

```rust
struct AppState {
    app_name: Mutex<String>,      // <-- Mutex for read-only data
    user_id: Mutex<Option<i32>>,  // <-- Mutex for per-request data
}
```

**Impact**:
- Every request locks mutex to read app_name
- Unnecessary synchronization overhead
- Blocks concurrent request handling

---

### 9. MEDIUM: Database Connection Pool Lock Contention

**Location**: `src/helpers/database.rs:11, 17-40`

```rust
static POOL: OnceLock<RwLock<Pool<ConnectionManager<SqliteConnection>>>> = OnceLock::new();

pub fn get_connection() -> PooledConnection<ConnectionManager<SqliteConnection>> {
    let pool_lock = POOL.get_or_init(...);
    let pool = pool_lock.read().expect("Failed to acquire read lock");
    // ...
```

**Impact**:
- Double-locking pattern (OnceLock + RwLock)
- Every database operation acquires read lock
- Contention under high load

---

## Performance Budget Breakdown

For a typical authenticated page request:

| Operation | Estimated Time |
|-----------|---------------|
| Template compilation | 50-200ms |
| Manifest parsing | 5-20ms |
| Auth middleware DB query | 5-20ms |
| Session user DB query | 5-20ms |
| Controller DB queries | 5-20ms |
| Form parsing (if POST) | 2-5ms |
| Lock contention | 1-5ms |
| **Total** | **73-290ms** |

At 100ms average response time: **~10 req/sec theoretical max** (single-threaded)

With Actix workers: **~100-200 req/sec** (matches observed ~172 req/sec)

---

## Recommendations

### Priority 1: Template Caching (Critical)

**Current**: Tera instance created every request
**Solution**: Create Tera instance once at startup

```rust
// In main.rs or a lazy_static
use once_cell::sync::Lazy;

static TEMPLATES: Lazy<Tera> = Lazy::new(|| {
    Tera::new("src/resources/views/**/*").expect("Failed to load templates")
});

// In template.rs
pub fn load_template(...) -> String {
    TEMPLATES.render(template, &context).unwrap()
}
```

**Expected improvement**: 50-200ms saved per request (~10-50x faster)

---

### Priority 2: Manifest Caching (Critical)

**Current**: File read + JSON parse every request
**Solution**: Parse manifest once at startup or use lazy_static

```rust
use once_cell::sync::Lazy;

static MANIFEST: Lazy<HashMap<String, String>> = Lazy::new(|| {
    let content = fs::read_to_string("dist/.vite/manifest.json")
        .expect("Failed to read manifest");
    // Parse and return HashMap
});
```

**Expected improvement**: 5-20ms saved per request

---

### Priority 3: User Session Caching (High)

**Current**: Database query on every authenticated request
**Solution**: Store user data in session cookie or Redis cache

```rust
// Store user in session after login
session.insert("user_data", serde_json::to_string(&user)?)?;

// Retrieve from session instead of DB
pub fn get_session_user(session: &Session) -> Option<User> {
    session.get::<String>("user_data")
        .ok()
        .flatten()
        .and_then(|s| serde_json::from_str(&s).ok())
}
```

**Expected improvement**: 5-20ms saved per authenticated request

---

### Priority 4: Static File Caching Headers (High)

**Current**: No caching headers
**Solution**: Add Cache-Control and ETag headers

```rust
#[get("/public/{filename:.*}")]
async fn serve_static(path: web::Path<String>) -> impl Responder {
    let file = NamedFile::open(file_path)?;
    file.set_content_disposition(ContentDisposition::default())
        .customize()
        .insert_header(("Cache-Control", "public, max-age=31536000, immutable"))
}
```

**Expected improvement**: Reduced server load, faster page loads

---

### Priority 5: Remove Unnecessary Mutexes (Medium)

**Current**: Mutex around read-only app_name
**Solution**: Use Arc<String> or just String

```rust
struct AppState {
    app_name: String,  // Read-only after init, no mutex needed
}
```

**Expected improvement**: Reduced lock contention

---

### Priority 6: Add Response Compression (Medium)

**Current**: No compression
**Solution**: Add actix-web compress middleware

```rust
use actix_web::middleware::Compress;

App::new()
    .wrap(Compress::default())
```

**Expected improvement**: 60-80% bandwidth reduction for text responses

---

### Priority 7: Optimize Database Queries (Medium)

**Current**: Duplicate queries in auth flow
**Solution**: Return user from validation function

```rust
pub fn validate_user_credentials(email: &str, password: &str) -> Result<User, ValidationError> {
    let user = users.filter(email.eq(email)).first::<User>(&mut get_connection())?;
    // Validate password
    Ok(user)  // Return user instead of just Valid/Invalid
}
```

**Expected improvement**: 5-20ms saved per login

---

## Expected Results After Optimization

| Optimization | Current | After | Improvement |
|-------------|---------|-------|-------------|
| Template caching | 172 req/s | ~2,000 req/s | 10x |
| + Manifest caching | - | ~3,000 req/s | 1.5x |
| + User caching | - | ~5,000 req/s | 1.7x |
| + Other optimizations | - | ~8,000-10,000 req/s | 2x |

**Realistic target**: **5,000-10,000 req/sec** (still won't match raw TCP due to framework overhead, but 30-60x improvement)

---

## Why the Minimal Server is Fast

The minimal TCP server has:

1. **Zero framework overhead** - No routing, middleware, extractors
2. **Zero I/O per request** - Response is a constant string
3. **Zero parsing** - Doesn't even parse HTTP headers
4. **Zero database** - No persistence layer
5. **Zero templating** - No string interpolation
6. **Minimal memory allocation** - Reuses buffer

It represents the **theoretical maximum throughput** for the hardware, useful as a benchmark ceiling.

---

## Conclusion

Nineties' poor performance is primarily due to:

1. **Template compilation on every request** (biggest impact)
2. **File I/O for manifest on every request**
3. **Excessive database queries for authentication**

Implementing the Priority 1-3 fixes should yield **20-50x improvement**, bringing throughput to **3,000-8,000 req/sec**.

The remaining gap to the minimal server (~50,000 req/sec) is inherent framework and database overhead - acceptable for a full-featured web application.
