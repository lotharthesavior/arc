use crate::database::seeders::create_users::seed_default_user;
use crate::helpers::config;
use crate::helpers::database::{get_connection, MIGRATIONS};
use crate::helpers::es_stack;

use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::SqliteConnection;
use diesel_migrations::MigrationHarness;
use std::env;
use std::fs;
use std::io;
use tracing::info;

/// Runs pending database migrations. Supports `--fresh` to drop and recreate
/// the database, and `--seed` to populate with default data after migration.
pub async fn run(args: &[String]) -> io::Result<()> {
    info!("Starting migration procedure");

    if args.contains(&"--fresh".to_string()) {
        info!("Reverting all migrations");
        let database: String =
            env::var("DATABASE_URL").unwrap_or_else(|_| "database/database.sqlite".to_string());
        fs::remove_file(&database).expect("Failed to remove database file");
        info!("Removed database file: {}", database);
    }

    info!("Running pending migrations");
    let mut conn: PooledConnection<ConnectionManager<SqliteConnection>> = get_connection();
    conn.run_pending_migrations(MIGRATIONS)
        .expect("Failed to run migrations");
    info!("Migrations completed successfully");

    if args.contains(&"--seed".to_string()) {
        info!("Running seeders");
        let stack = es_stack::build(&config::database_url())
            .await
            .expect("Failed to build ES stack");
        seed_default_user(&stack.command_bus, stack.read_model_store.as_ref())
            .await
            .expect("Failed to seed default user");
        info!("Seeders completed successfully");
    }

    Ok(())
}
