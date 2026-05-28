use crate::database::seeders::create_users::UserSeeder;
use crate::database::seeders::traits::seeder::Seeder;
use crate::helpers::database::get_connection;
use std::io;
use tracing::info;

pub fn run() -> io::Result<()> {
    info!("Running seeders");
    UserSeeder::execute(&mut get_connection()).expect("Failed to seed users table");
    info!("Seeders completed successfully");
    Ok(())
}
