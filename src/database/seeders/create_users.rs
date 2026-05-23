use crate::database::backend::DbConnection;
use crate::database::seeders::traits::seeder::Seeder;
use crate::models::user::NewUser;
use crate::schema::users::dsl::*;
use crate::services::user_service::prepare_password;
use diesel::{QueryDsl, RunQueryDsl};
use tracing::info;

pub struct UserSeeder;

impl Seeder for UserSeeder {
    fn execute(conn: &mut DbConnection) -> Result<(), Box<dyn std::error::Error>> {
        let expected_email: &str = "jekyll@example.com";

        let all_users: Vec<i32> = users.select(id).load::<i32>(conn).unwrap();
        if !all_users.is_empty() {
            info!(email = expected_email, "User table already seeded");
            return Ok(());
        }

        info!("Creating default user");
        let _ = diesel::insert_into(users)
            .values(NewUser {
                name: "Jekyll",
                email: expected_email,
                password: &prepare_password("password"),
            })
            .execute(conn);

        Ok(())
    }
}
