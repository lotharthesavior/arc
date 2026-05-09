use crate::database::seeders::create_users::seed_default_user;
use crate::helpers::config;
use crate::helpers::es_stack;
use std::io;
use tracing::info;

/// Runs all database seeders to populate the database with default data.
pub async fn run() -> io::Result<()> {
    info!("Running seeders");
    let stack = es_stack::build(&config::database_url())
        .await
        .expect("Failed to build ES stack");
    seed_default_user(&stack.command_bus, stack.read_model_store.as_ref())
        .await
        .expect("Failed to seed default user");
    info!("Seeders completed successfully");
    Ok(())
}
