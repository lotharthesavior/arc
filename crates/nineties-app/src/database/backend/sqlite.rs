use std::fs;
use std::io;
use std::path::Path;

use super::BackendSpec;

pub struct SqliteBackend;

impl BackendSpec for SqliteBackend {
    type Connection = diesel::SqliteConnection;
    type DieselBackend = diesel::sqlite::Sqlite;

    const DRIVER_NAME: &'static str = "sqlite";
    const DEFAULT_DATABASE_URL: &'static str = "database/database.sqlite";

    fn database_exists(database_url: &str) -> bool {
        fs::exists(Path::new(database_url)).unwrap_or(false)
    }

    fn reset_database_storage(database_url: &str) -> io::Result<()> {
        match fs::remove_file(database_url) {
            Ok(_) => Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err),
        }
    }
}
