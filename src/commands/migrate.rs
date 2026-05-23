use crate::database::backend::{database_url, reset_database_storage, DbPooledConnection};
use crate::database::seeders::create_users::UserSeeder;
use crate::database::seeders::traits::seeder::Seeder;
use crate::helpers::database::get_connection;
use crate::models::user::MIGRATIONS;

use diesel_migrations::MigrationHarness;
use std::io;
use tracing::info;

pub fn run(args: &[String]) -> io::Result<()> {
    info!("Starting migration procedure");

    if args.contains(&"--fresh".to_string()) {
        info!("Reverting all migrations");
        let database = database_url();
        reset_database_storage(&database).expect("Failed to reset database storage");
        info!("Removed database file: {}", database);
    }

    info!("Running pending migrations");
    let mut conn: DbPooledConnection = get_connection();
    conn.run_pending_migrations(MIGRATIONS)
        .expect("Failed to run migrations");
    info!("Migrations completed successfully");

    if args.contains(&"--seed".to_string()) {
        info!("Running seeders");
        UserSeeder::execute(&mut get_connection()).expect("Failed to seed users table");
        info!("Seeders completed successfully");
    }

    Ok(())
}
