use std::env;

/// Default database file path used when DATABASE_URL is not set
pub const DEFAULT_DATABASE_URL: &str = "database/database.sqlite";

/// Default SQLite file backing the JWT session-revocation registry when the
/// primary driver is not SQLite.
pub const DEFAULT_SESSION_DATABASE_URL: &str = "database/sessions.sqlite";

/// Default database connection pool size
pub const DEFAULT_POOL_LIMIT: u32 = 10;

/// Default number of new aggregate events between best-effort user snapshots.
pub const DEFAULT_USER_SNAPSHOT_INTERVAL_EVENTS: i64 = 50;

/// Storage backend selected at startup behind the `EventStore` /
/// `ReadModelStore` traits. Parsing is always available; constructing the
/// Postgres stores additionally requires the `postgres` cargo feature.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatabaseDriver {
    Sqlite,
    Postgres,
}

impl DatabaseDriver {
    /// Read `DATABASE_DRIVER` from the environment, defaulting to SQLite.
    pub fn from_env() -> Self {
        Self::parse(&env::var("DATABASE_DRIVER").unwrap_or_else(|_| "sqlite".to_string()))
    }

    /// Parse a driver name. Anything other than a recognized Postgres alias
    /// falls back to SQLite, matching the historical default.
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "postgres" | "postgresql" | "pg" => DatabaseDriver::Postgres,
            _ => DatabaseDriver::Sqlite,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            DatabaseDriver::Sqlite => "sqlite",
            DatabaseDriver::Postgres => "postgres",
        }
    }

    /// Whether `DATABASE_URL` names a local filesystem path the startup health
    /// check can stat. Postgres uses a connection string, not a file.
    pub fn is_file_backed(self) -> bool {
        matches!(self, DatabaseDriver::Sqlite)
    }
}

/// Get the database URL from environment or use default
pub fn database_url() -> String {
    env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string())
}

/// SQLite URL backing the JWT session-revocation registry. The registry is
/// SQLite-only today; under a non-SQLite primary driver it keeps its own local
/// SQLite database (overridable via `SESSION_DATABASE_URL`) so session
/// revocation still functions.
pub fn session_store_url(driver: DatabaseDriver) -> String {
    match driver {
        DatabaseDriver::Sqlite => database_url(),
        DatabaseDriver::Postgres => env::var("SESSION_DATABASE_URL")
            .unwrap_or_else(|_| DEFAULT_SESSION_DATABASE_URL.to_string()),
    }
}

/// Optional HMAC key enabling HIPAA-5 event integrity signing/enforcement.
/// The storage layer validates length and rejects keys shorter than 32 bytes.
pub fn event_integrity_key() -> Option<Vec<u8>> {
    env::var("EVENT_INTEGRITY_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.into_bytes())
}

/// Identifier stored with each event signature so future key rotation can
/// distinguish which key signed a row.
pub fn event_integrity_key_id() -> String {
    env::var("EVENT_INTEGRITY_KEY_ID").unwrap_or_else(|_| "default".to_string())
}

/// Number of new events between best-effort aggregate snapshots.
/// Values <= 0 disable user snapshots without changing command correctness.
pub fn user_snapshot_interval_events() -> Option<i64> {
    let interval = env::var("USER_SNAPSHOT_INTERVAL_EVENTS")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(|value| {
            value
                .parse()
                .expect("USER_SNAPSHOT_INTERVAL_EVENTS must be a number")
        })
        .unwrap_or(DEFAULT_USER_SNAPSHOT_INTERVAL_EVENTS);

    (interval > 0).then_some(interval)
}

/// Get the database pool limit from environment or use default
pub fn database_pool_limit() -> u32 {
    env::var("DATABASE_POOL_LIMIT")
        .unwrap_or_else(|_| DEFAULT_POOL_LIMIT.to_string())
        .parse()
        .expect("DATABASE_POOL_LIMIT must be a number")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn user_snapshot_interval_defaults_to_50() {
        env::remove_var("USER_SNAPSHOT_INTERVAL_EVENTS");
        assert_eq!(
            user_snapshot_interval_events(),
            Some(DEFAULT_USER_SNAPSHOT_INTERVAL_EVENTS)
        );
    }

    #[test]
    #[serial]
    fn user_snapshot_interval_reads_env_override() {
        env::set_var("USER_SNAPSHOT_INTERVAL_EVENTS", "25");
        assert_eq!(user_snapshot_interval_events(), Some(25));
        env::remove_var("USER_SNAPSHOT_INTERVAL_EVENTS");
    }

    #[test]
    #[serial]
    fn user_snapshot_interval_non_positive_disables_snapshots() {
        env::set_var("USER_SNAPSHOT_INTERVAL_EVENTS", "0");
        assert_eq!(user_snapshot_interval_events(), None);
        env::remove_var("USER_SNAPSHOT_INTERVAL_EVENTS");
    }
}
