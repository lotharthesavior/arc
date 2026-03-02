use std::env;

/// Default database file path used when DATABASE_URL is not set
pub const DEFAULT_DATABASE_URL: &str = "database/database.sqlite";

/// Default database connection pool size
pub const DEFAULT_POOL_LIMIT: u32 = 10;

/// Get the database URL from environment or use default
pub fn database_url() -> String {
    env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string())
}

/// Get the database pool limit from environment or use default
pub fn database_pool_limit() -> u32 {
    env::var("DATABASE_POOL_LIMIT")
        .unwrap_or_else(|_| DEFAULT_POOL_LIMIT.to_string())
        .parse()
        .expect("DATABASE_POOL_LIMIT must be a number")
}
