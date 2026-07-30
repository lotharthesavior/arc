use diesel::connection::SimpleConnection;
use diesel::SqliteConnection;

pub(crate) fn migrate(connection: &mut SqliteConnection) {
    connection
        .batch_execute(include_str!("../tests/fixtures/schema.sql"))
        .expect("create SQLite test schema");
}
