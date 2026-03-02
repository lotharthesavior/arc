use crate::helpers::database::get_connection;
use crate::models::user::User;
use crate::schema::users::dsl::*;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use tracing::warn;

/// Result of a user credential validation attempt.
#[derive(PartialEq, Debug)]
pub enum UserValidationResult {
    /// No user found with the given email.
    InvalidEmail,
    /// The stored password hash could not be parsed (data corruption).
    InvalidPasswordHash,
    /// The password did not match.
    Invalid,
    /// Credentials are valid.
    Valid,
}

/// Validates user credentials against the database.
///
/// # Arguments
/// * `user_email` - The email to validate
/// * `user_password` - The plaintext password to check
///
/// # Returns
/// `UserValidationResult` indicating success or failure type
pub fn validate_user_credentials(user_email: &str, user_password: &str) -> UserValidationResult {
    let conn = &mut get_connection();

    let user_results = match users.filter(email.eq(&user_email)).load::<User>(conn) {
        Ok(results) => results,
        Err(e) => {
            tracing::error!("Database error loading user: {}", e);
            return UserValidationResult::Invalid;
        }
    };

    if user_results.is_empty() {
        return UserValidationResult::InvalidEmail;
    }

    let user = &user_results[0];
    let parsed_hash = match PasswordHash::new(&user.password) {
        Ok(hash) => hash,
        Err(_) => {
            warn!("Failed to parse password hash for user ID: {}", user.id);
            return UserValidationResult::InvalidPasswordHash;
        }
    };

    if Argon2::default()
        .verify_password(user_password.as_bytes(), &parsed_hash)
        .is_ok()
    {
        return UserValidationResult::Valid;
    }

    UserValidationResult::Invalid
}

/// Hashes a plaintext password using Argon2 with a random salt.
///
/// # Arguments
/// * `password_string` - The plaintext password to hash
///
/// # Returns
/// The hashed password string suitable for storage
pub fn prepare_password(password_string: &str) -> String {
    let password_bytes = password_string.as_bytes();
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(password_bytes, &salt)
        .expect("Password hashing should not fail with valid inputs")
        .to_string()
}
