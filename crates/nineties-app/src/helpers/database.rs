use std::sync::{OnceLock, RwLock};

use crate::database::backend::{database_url, DbConnectionManager, DbPool, DbPooledConnection};
use crate::helpers::config;

/// Internal state for the connection pool singleton.
struct PoolState {
    pool: DbPool,
    database_url: String,
}

static POOL: OnceLock<RwLock<Option<PoolState>>> = OnceLock::new();

/// Get a database connection from the shared connection pool.
/// The pool is created lazily on first call and reused for subsequent calls.
/// If the `DATABASE_URL` changes (e.g., between test runs), the pool is recreated.
pub fn get_connection() -> DbPooledConnection {
    let current_db_url = database_url();

    let pool_state = POOL.get_or_init(|| RwLock::new(Some(create_pool_state(&current_db_url))));

    // Check if we need to recreate the pool (e.g., for tests with different DB or after reset)
    {
        let state = pool_state.read().unwrap();
        if let Some(ref ps) = *state {
            if ps.database_url == current_db_url {
                return ps.pool.get().expect("Failed to get connection from pool");
            }
        }
    }

    // Database URL changed or pool was reset, recreate pool
    {
        let mut state = pool_state.write().unwrap();
        let needs_recreate = match *state {
            None => true,
            Some(ref ps) => ps.database_url != current_db_url,
        };
        if needs_recreate {
            *state = Some(create_pool_state(&current_db_url));
        }
        state
            .as_ref()
            .unwrap()
            .pool
            .get()
            .expect("Failed to get connection from pool")
    }
}

/// Get a clone of the shared connection pool.
/// Useful when you need to pass the pool to a different context.
#[allow(dead_code)]
pub fn get_connection_pool() -> DbPool {
    let current_db_url = database_url();

    let pool_state = POOL.get_or_init(|| RwLock::new(Some(create_pool_state(&current_db_url))));

    {
        let state = pool_state.read().unwrap();
        if let Some(ref ps) = *state {
            if ps.database_url == current_db_url {
                return ps.pool.clone();
            }
        }
    }

    {
        let mut state = pool_state.write().unwrap();
        let needs_recreate = match *state {
            None => true,
            Some(ref ps) => ps.database_url != current_db_url,
        };
        if needs_recreate {
            *state = Some(create_pool_state(&current_db_url));
        }
        state.as_ref().unwrap().pool.clone()
    }
}

/// Resets the connection pool. Call this when the database file is deleted/recreated.
/// This is primarily used in tests after TestFinalizer deletes the database.
#[cfg(test)]
pub fn reset_pool() {
    if let Some(pool_state) = POOL.get() {
        let mut state = pool_state.write().unwrap();
        *state = None;
    }
}

fn create_pool_state(database_url: &str) -> PoolState {
    let pool_limit: u32 = config::database_pool_limit();

    let manager = DbConnectionManager::new(database_url);

    let pool = DbPool::builder()
        .max_size(pool_limit)
        .test_on_check_out(true)
        .build(manager)
        .expect("Could not build connection pool");

    PoolState {
        pool,
        database_url: database_url.to_string(),
    }
}
