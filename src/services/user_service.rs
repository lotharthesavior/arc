use crate::helpers::database::get_connection;
use crate::models::user::User;
use crate::schema::users::dsl::*;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use tracing::warn;

#[derive(PartialEq, Debug)]
pub enum UserValidationResult {
    InvalidEmail,
    InvalidPasswordHash,
    Invalid,
    Valid,
}

// This validates the user credentials.
pub fn validate_user_credentials(user_email: &str, user_password: &str) -> UserValidationResult {
    let conn = &mut get_connection();

    let user = users
        .filter(email.eq(&user_email))
        .load::<User>(conn)
        .expect("Failed to load users");

    if user.is_empty() {
        return UserValidationResult::InvalidEmail;
    }

    let user: &User = user.first().unwrap();
    let parsed_hash = PasswordHash::new(&user.password);
    if parsed_hash.is_err() {
        warn!("Failed to parse password hash for user ID: {}", user.id);
        return UserValidationResult::InvalidPasswordHash;
    }

    let password_verified: bool = Argon2::default()
        .verify_password(user_password.as_ref(), &parsed_hash.unwrap())
        .is_ok();

    if password_verified {
        return UserValidationResult::Valid;
    }

    UserValidationResult::Invalid
}

// This prepares the password for the database with a salt.
pub fn prepare_password(password_string: &str) -> String {
    let password_bytes = password_string.as_bytes();
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(password_bytes, &salt)
        .unwrap()
        .to_string()
}
