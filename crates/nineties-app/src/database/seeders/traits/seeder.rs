use std::error::Error;

use crate::database::backend::DbConnection;

/// Trait for database seeders that populate tables with initial data.
pub trait Seeder {
    /// Executes the seeder against the given database connection.
    fn execute(conn: &mut DbConnection) -> Result<(), Box<dyn Error>>;
}
