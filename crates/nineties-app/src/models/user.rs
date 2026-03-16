use diesel::prelude::*;
use diesel_migrations::{embed_migrations, EmbeddedMigrations};
use serde::{Deserialize, Serialize};

use crate::database::backend::DbBackend;

/// Embedded database migrations, compiled into the binary.
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

/// A user record from the `users` table.
/// The password field is excluded from JSON serialization for security.
#[derive(Queryable, Selectable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(DbBackend))]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// A new user record for insertion into the `users` table.
#[derive(Insertable)]
#[diesel(table_name = crate::schema::users)]
pub struct NewUser<'a> {
    pub name: &'a str,
    pub email: &'a str,
    pub password: &'a str,
}

#[cfg(test)]
mod tests {
    use crate::database::backend::{DbConnection, DbPooledConnection};
    use crate::database::seeders::create_users::UserSeeder;
    use crate::database::seeders::traits::seeder::Seeder;
    use crate::helpers::database::get_connection;
    use crate::helpers::test::InMemoryTestGuard;
    use crate::models::user::{NewUser, User, MIGRATIONS};
    use crate::schema::users::dsl::*;

    use diesel::result::Error;
    use diesel::{Connection, ExpressionMethods, QueryDsl, RunQueryDsl};
    use diesel_migrations::MigrationHarness;
    use serial_test::serial;
    use std::env;

    fn prepare_test_db() -> DbPooledConnection {
        dotenv::from_filename(".env.test").ok();
        env::set_var("DATABASE_URL", "file::memory:?cache=shared");
        let mut conn: DbPooledConnection = get_connection();
        conn.run_pending_migrations(MIGRATIONS)
            .expect("Failed to run migrations");
        conn
    }

    fn seed_users_table(conn: &mut DbConnection) {
        UserSeeder::execute(conn).expect("Failed to seed users table");
    }

    #[serial]
    #[actix_web::test]
    async fn test_can_create_user() {
        let _guard = InMemoryTestGuard;

        let mut conn: DbPooledConnection = prepare_test_db();

        conn.test_transaction::<_, Error, _>(|conn| {
            let expected_email: String = "john@email.com".to_string();

            let _ = diesel::insert_into(users)
                .values(NewUser {
                    name: "John Doe",
                    email: &expected_email,
                    password: "password",
                })
                .execute(conn)
                .unwrap();

            let results: Vec<User> = users
                .filter(email.eq(&expected_email))
                .load::<User>(conn)
                .unwrap();

            assert!(!results.is_empty());

            Ok(())
        });
    }

    #[serial]
    #[actix_web::test]
    async fn test_can_delete_user() {
        let _guard = InMemoryTestGuard;

        let mut conn: DbPooledConnection = prepare_test_db();

        conn.test_transaction::<_, Error, _>(|conn| {
            seed_users_table(conn);

            let expected_email: &str = "jekyll@example.com";

            let results: Vec<User> = users
                .filter(email.eq(expected_email))
                .get_results(conn)
                .unwrap();
            assert!(!results.is_empty());

            let _ = diesel::delete(users.filter(email.eq(expected_email)))
                .execute(conn)
                .unwrap();

            let results2: Vec<User> = users
                .filter(email.eq(expected_email))
                .get_results(conn)
                .unwrap();
            assert!(results2.is_empty());

            Ok(())
        });
    }

    #[serial]
    #[actix_web::test]
    async fn test_can_retrieve_user_by_id() {
        let _guard = InMemoryTestGuard;

        let mut conn: DbPooledConnection = prepare_test_db();

        conn.test_transaction::<_, Error, _>(|conn| {
            seed_users_table(conn);

            let all_users: Vec<i32> = users.select(id).load::<i32>(conn).unwrap();
            let user_id: i32 = all_users[0];

            let user: Result<User, Error> = users.find(user_id).first::<User>(conn);
            let user2: Result<User, Error> = users.find(99).first::<User>(conn);

            assert!(user.is_ok());
            assert!(user2.is_err());

            Ok(())
        });
    }

    #[serial]
    #[actix_web::test]
    async fn test_can_update_user() {
        let _guard = InMemoryTestGuard;

        let mut conn: DbPooledConnection = prepare_test_db();

        conn.test_transaction::<_, Error, _>(|conn| {
            seed_users_table(conn);

            let all_users: Vec<i32> = users.select(id).load::<i32>(conn).unwrap();
            let user_id: i32 = all_users[0];

            let user: Result<User, Error> = users.find(user_id).first::<User>(conn);
            assert!(user.is_ok());

            let new_email: &str = "newemail@example.com";

            let result = diesel::update(users.find(user_id))
                .set(email.eq(new_email))
                .execute(conn)
                .unwrap();
            assert_eq!(result, 1);

            let user: Result<User, Error> = users.filter(email.eq(new_email)).first::<User>(conn);
            assert!(user.is_ok());
            assert_eq!(user.unwrap().email, new_email);

            Ok(())
        });
    }
}
