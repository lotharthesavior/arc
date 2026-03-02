use std::env;
use std::error::Error;
use std::fmt;
use std::io;

use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};

#[cfg(not(feature = "sqlite"))]
compile_error!("At least one database backend feature must be enabled.");

#[cfg(feature = "sqlite")]
mod sqlite;

pub trait BackendSpec {
    type Connection;
    type DieselBackend;

    const DRIVER_NAME: &'static str;
    const DEFAULT_DATABASE_URL: &'static str;

    fn database_exists(database_url: &str) -> bool;
    fn reset_database_storage(database_url: &str) -> io::Result<()>;
}

#[cfg(feature = "sqlite")]
pub type ActiveBackend = sqlite::SqliteBackend;

pub type DbConnection = <ActiveBackend as BackendSpec>::Connection;
#[allow(dead_code)]
pub type DbBackend = <ActiveBackend as BackendSpec>::DieselBackend;
pub type DbConnectionManager = ConnectionManager<DbConnection>;
pub type DbPool = Pool<DbConnectionManager>;
pub type DbPooledConnection = PooledConnection<DbConnectionManager>;

#[derive(Debug)]
pub enum BackendConfigError {
    UnsupportedConfiguredDriver(String),
    DriverFeatureMismatch {
        configured: String,
        compiled: &'static str,
    },
}

impl fmt::Display for BackendConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendConfigError::UnsupportedConfiguredDriver(driver) => {
                write!(f, "Unsupported DATABASE_DRIVER '{}'", driver)
            }
            BackendConfigError::DriverFeatureMismatch {
                configured,
                compiled,
            } => write!(
                f,
                "DATABASE_DRIVER is set to '{}' but this binary was compiled for '{}' support",
                configured, compiled
            ),
        }
    }
}

impl Error for BackendConfigError {}

pub fn driver_name() -> &'static str {
    ActiveBackend::DRIVER_NAME
}

pub fn configured_driver() -> String {
    env::var("DATABASE_DRIVER")
        .unwrap_or_else(|_| driver_name().to_string())
        .trim()
        .to_ascii_lowercase()
}

pub fn default_database_url() -> &'static str {
    ActiveBackend::DEFAULT_DATABASE_URL
}

pub fn database_url() -> String {
    env::var("DATABASE_URL").unwrap_or_else(|_| default_database_url().to_string())
}

pub fn database_exists(database_url: &str) -> bool {
    ActiveBackend::database_exists(database_url)
}

pub fn reset_database_storage(database_url: &str) -> io::Result<()> {
    ActiveBackend::reset_database_storage(database_url)
}

pub fn validate_backend_configuration() -> Result<(), BackendConfigError> {
    let configured = configured_driver();

    match configured.as_str() {
        "sqlite" | "postgres" | "mysql" => {}
        _ => return Err(BackendConfigError::UnsupportedConfiguredDriver(configured)),
    }

    if configured != driver_name() {
        return Err(BackendConfigError::DriverFeatureMismatch {
            configured,
            compiled: driver_name(),
        });
    }

    Ok(())
}
