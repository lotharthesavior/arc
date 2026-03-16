use std::env;

#[cfg(test)]
use crate::helpers::database::reset_pool;

/// RAII guard for test cleanup. Resets the connection pool and removes the
/// test database file when dropped. Place `let _finalizer = TestFinalizer;`
/// at the top of each integration test to ensure automatic cleanup.
#[allow(dead_code)]
pub struct TestFinalizer;
impl Drop for TestFinalizer {
    fn drop(&mut self) {
        // Reset the connection pool first to release all connections
        #[cfg(test)]
        reset_pool();

        let database: String =
            env::var("DATABASE_URL").unwrap_or_else(|_| "database/database.sqlite".to_string());

        // Only delete file-based databases; skip in-memory databases
        if database != ":memory:" && !database.contains(":memory:") {
            let _ = std::fs::remove_file(database);
        }
    }
}

/// RAII guard for in-memory test isolation. Resets the connection pool on drop
/// but does not attempt to delete any database file. Use this for tests that
/// configure `DATABASE_URL=:memory:` for true per-test isolation.
#[allow(dead_code)]
pub struct InMemoryTestGuard;
impl Drop for InMemoryTestGuard {
    fn drop(&mut self) {
        #[cfg(test)]
        reset_pool();
    }
}
