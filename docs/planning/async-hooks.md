# Async and Multi-Process Plan for `the-hook`

## 1. Objective

This document outlines a plan to evolve the `the-hook` library to support `async` operations and to clarify its behavior in a multi-process environment, making it suitable for modern, high-performance Rust web applications.

---

## 2. Part 1: Implementing Full Async Support

The current synchronous design of `the-hook` is a significant limitation in an async runtime like Actix or Tokio. A filter that performs database I/O would block an executor thread, severely impacting application performance.

### 2.1 The Problem: Synchronous Callbacks

A synchronous filter forces I/O operations to be blocking:

```rust
// Current synchronous (blocking) filter
add_filter("user:created", 10, |user_id: Uuid| {
    // This blocks the entire thread!
    let db_conn = establish_connection(); 
    send_welcome_email(db_conn, user_id);
    user_id
});
```

### 2.2 Proposed Solution: Async-native Hooks

The solution is to change the core traits and function signatures to be `async`-native.

#### New Callback Signature

The internal storage will need to hold futures. The callback signature for `add_filter` would change from `Fn` to a boxed `Future`.

```rust
// Using `async-trait` or a manual future-pinning approach
pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

// The new trait for a filter callback
pub trait AsyncFilter<T>: Send + Sync {
    fn call(&self, value: T) -> BoxedFuture<T>;
}
```

#### New API Functions

The user-facing API must also become `async`.

```rust
// Before
pub fn apply_filters<T: 'static>(hook: &str, value: T) -> T;

// After
pub async fn apply_filters<T: 'static + Send>(hook: &str, value: T) -> T;
```

#### Example Usage (After)

With these changes, plugins can now define non-blocking filters.

```rust
// New non-blocking filter
add_filter("user:created", 10, |user_id: Uuid| {
    Box::pin(async move {
        let db_pool = get_db_pool().await; // Non-blocking
        send_welcome_email(db_pool, user_id).await; // Non-blocking
        user_id
    })
});

// Applying filters becomes an awaitable operation
let final_user_id = apply_filters("user:created", new_user.id).await;
```

### 2.3 Internal Implementation

- **Mutex:** The `std::sync::Mutex` should be replaced with `tokio::sync::Mutex` or `tokio::sync::RwLock`. The Tokio mutex is designed to work with the async runtime and yields control back to the scheduler when locked, preventing it from blocking a thread.
- **Callback Storage:** The global `HashMap` will now store `Box<dyn AsyncFilter<T>>` instances.

---

## 3. Part 2: Understanding Multi-Process Behavior

The term "multi-process" can be ambiguous. It's crucial to distinguish between multi-threading (within one process) and multiple processes (e.g., running several application instances).

### 3.1 The In-Process Model (Current State)

A library like `the-hook` that uses `static` variables for storage operates **within a single OS process**.

- **Shared Memory:** All threads within a single process share the same memory space. The hook registry is accessible to all threads.
- **No Cross-Process Communication:** If you run two separate instances of the Arc server, **they do not share memory**. The hooks registered in Process A are completely invisible to Process B.

This is the standard and expected behavior. The library is "multi-process capable" in that you can run multiple processes, but it does not provide any magic communication *between* them.

### 3.2 Recommendation: Process-Local Hooks

**The recommendation is to embrace this process-local behavior.** It is simple, robust, and predictable.

In a typical production deployment, you would have multiple instances of the Arc application running behind a load balancer. Each instance would be a separate process. When each process starts, it loads all plugins and registers their hooks independently.

This means that a request handled by Process A will trigger the hooks registered within Process A. This is a standard and scalable stateless architecture.

---

## 4. Part 3: Advanced Alternative (A Distributed Hook System)

If true cross-process communication is ever a requirement, `the-hook` would need to be fundamentally re-architected to use an external backend for communication. This transforms it from a simple in-process library into a distributed messaging system.

### 4.1 Conceptual Design (Redis Backend)

- **`add_filter(hook, callback)`:** This would no longer just store the callback locally. It would also register the hook with a central system like Redis, perhaps by adding the `hook` name to a `SET`.
- **`apply_filters(hook, data)`:** This function would become much more complex:
    1.  Generate a unique ID for this specific operation.
    2.  Publish the `data` to a Redis Pub/Sub channel (e.g., `hooks:my_hook`).
    3.  All application processes (listening to that channel) would execute their local callback for `my_hook`.
    4.  Each process would publish the result to a reply channel (e.g., `hooks:reply:<unique_id>`).
    5.  The original `apply_filters` call would wait for and aggregate the responses from the reply channel before returning the final value.

### 4.2 Trade-offs

- **Pros:** Hooks could be triggered and modified across an entire cluster of servers.
- **Cons:**
    - **Massive Complexity:** This is a distributed system with all its associated challenges (network latency, serialization, error handling, discovery).
    - **External Dependency:** Requires a running Redis cluster (or similar).
    - **Performance:** Network communication is orders of magnitude slower than in-process function calls.

This approach is not recommended for the core library but could exist as an optional, advanced "distributed" feature flag.

---

## 5. Recommended Roadmap

1.  **Implement Async Support (Part 1):** Refactor `the-hook` to be fully `async`. This is the highest-priority item and provides the most immediate value.
2.  **Update Documentation (Part 2):** Clearly document that hooks are **process-local** to avoid confusion about multi-process behavior.
3.  **Defer Distributed Hooks (Part 3):** The distributed model is a separate, highly complex project. It should be considered a long-term, optional extension, not part of the core library.
