use crate::database::seeders::create_users::seed_default_user;
use crate::helpers::config;
use crate::helpers::config::DatabaseDriver;
use crate::helpers::database::{get_connection, MIGRATIONS};
use crate::helpers::es_stack;

use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::SqliteConnection;
use diesel_migrations::MigrationHarness;
use std::env;
use std::fs;
use std::io;
use tracing::info;

/// Runs pending database migrations for the configured driver. Supports
/// `--fresh` to drop and recreate the database, and `--seed` to populate with
/// default data after migration.
pub async fn run(args: &[String]) -> io::Result<()> {
    info!("Starting migration procedure");

    match DatabaseDriver::from_env() {
        DatabaseDriver::Sqlite => run_sqlite(args).await,
        DatabaseDriver::Postgres => run_postgres(args).await,
    }
}

async fn run_sqlite(args: &[String]) -> io::Result<()> {
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
        run_seed().await;
    }

    Ok(())
}

#[cfg(feature = "postgres")]
async fn run_postgres(args: &[String]) -> io::Result<()> {
    use arc_es_postgres::{PostgresEventStore, PostgresReadModelStore};

    if args.contains(&"--fresh".to_string()) {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "`migrate --fresh` is not supported for DATABASE_DRIVER=postgres; \
             drop and recreate the Postgres database out of band",
        ));
    }

    let url = config::database_url();
    info!("Initializing Postgres schema");
    let event_store = PostgresEventStore::new(&url)
        .await
        .expect("Failed to connect Postgres event store");
    event_store
        .initialize_schema()
        .await
        .expect("Failed to initialize Postgres event-store schema");
    let read_model_store = PostgresReadModelStore::new(&url)
        .await
        .expect("Failed to connect Postgres read-model store");
    read_model_store
        .initialize_schema()
        .await
        .expect("Failed to initialize Postgres read-model schema");
    info!("Postgres schema initialized");

    if args.contains(&"--seed".to_string()) {
        run_seed().await;
    }

    Ok(())
}

#[cfg(not(feature = "postgres"))]
async fn run_postgres(_args: &[String]) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "DATABASE_DRIVER=postgres requires building with the `postgres` cargo feature",
    ))
}

async fn run_seed() {
    info!("Running seeders");
    let stack = es_stack::build(&config::database_url())
        .await
        .expect("Failed to build ES stack");
    seed_default_user(&stack.command_bus, stack.read_model_store.as_ref())
        .await
        .expect("Failed to seed default user");
    info!("Seeders completed successfully");
}
