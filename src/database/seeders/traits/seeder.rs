use std::error::Error;

use crate::database::backend::DbConnection;

pub trait Seeder {
    fn execute(conn: &mut DbConnection) -> Result<(), Box<dyn Error>>;
}
