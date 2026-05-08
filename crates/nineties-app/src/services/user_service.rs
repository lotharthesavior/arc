use crate::domain::user::aggregate::UserAggregate;
use crate::domain::user::commands::UserCommand;
use crate::domain::user::projector::USERS_VIEW;
use crate::helpers::database::get_connection;
use crate::http::errors::AppError;
use crate::models::user::User;
use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use nineties_core::command_bus::{CommandBus, CommandContext};
use nineties_core::read_model_store::ReadModelStore;
use serde_json::json;

/// Result of a user credential validation attempt.
#[derive(PartialEq, Debug)]
pub enum UserValidationResult {
    InvalidEmail,
    InvalidPasswordHash,
    Invalid,
    Valid,
}

/// Validates user credentials against the legacy Diesel `users` table.
/// Used by cookie-based session login (signin_post) during Step 1 transition.
pub fn validate_user_credentials(user_email: &str, user_password: &str) -> UserValidationResult {
    use crate::schema::users::dsl::{email, users};

    let conn = &mut get_connection();

    let user_results = match users.filter(email.eq(user_email)).load::<User>(conn) {
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
            tracing::warn!("Failed to parse password hash for user ID: {}", user.id);
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

/// Validates credentials against the `users_view` projection.
/// Used by the API JWT login path. Returns the aggregate UUID on success.
///
/// Replaces a prior implementation that read the legacy `user_email_index`
/// Diesel table for the email→id mapping and then replayed the event stream
/// to recover the password hash. Both reads are now served from the
/// projector-maintained read model.
pub async fn validate_user_credentials_es(
    read_model_store: &dyn ReadModelStore,
    user_email: &str,
    user_password: &str,
) -> (UserValidationResult, Option<String>) {
    let row = match find_user_by_email(read_model_store, user_email).await {
        Ok(Some(row)) => row,
        Ok(None) => return (UserValidationResult::InvalidEmail, None),
        Err(e) => {
            tracing::error!("users_view lookup failed for {}: {}", user_email, e);
            return (UserValidationResult::Invalid, None);
        }
    };

    let agg_id = match row.get("id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return (UserValidationResult::Invalid, None),
    };

    let stored_hash = match row.get("password_hash").and_then(|v| v.as_str()) {
        Some(h) => h,
        None => return (UserValidationResult::InvalidPasswordHash, None),
    };

    let parsed_hash = match PasswordHash::new(stored_hash) {
        Ok(hash) => hash,
        Err(_) => return (UserValidationResult::InvalidPasswordHash, None),
    };

    if Argon2::default()
        .verify_password(user_password.as_bytes(), &parsed_hash)
        .is_ok()
    {
        (UserValidationResult::Valid, Some(agg_id))
    } else {
        (UserValidationResult::Invalid, None)
    }
}

/// Look up the aggregate UUID for an email via the `users_view` projection.
pub async fn lookup_aggregate_id_by_email_view(
    read_model_store: &dyn ReadModelStore,
    user_email: &str,
) -> Option<String> {
    match find_user_by_email(read_model_store, user_email).await {
        Ok(Some(row)) => row
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        _ => None,
    }
}

async fn find_user_by_email(
    store: &dyn ReadModelStore,
    user_email: &str,
) -> Result<Option<serde_json::Value>, nineties_core::read_model_store::ReadModelError> {
    let mut hits = store
        .find_by(USERS_VIEW, "email", &json!(user_email))
        .await?;
    Ok(hits.pop())
}

/// Hashes a plaintext password using Argon2 with a random salt.
pub fn prepare_password(password_string: &str) -> String {
    let password_bytes = password_string.as_bytes();
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(password_bytes, &salt)
        .expect("Password hashing should not fail with valid inputs")
        .to_string()
}

/// Create a new user by dispatching a registration command.
///
/// Hashes the password once and embeds it into the `UserRegistered` event.
/// The `UserProjector` (subscribed to the event bus) writes `users_view` so
/// subsequent email→id lookups and login attempts find the new user; no
/// secondary email-index table is maintained here anymore.
///
/// Pre-checking for an existing email is best-effort — the authoritative
/// guard is the `UNIQUE` index on `users_view.email`. If the read model is
/// behind, the projector's upsert will fail with a unique-constraint error
/// and surface as a command-handling failure.
pub async fn create_user(
    command_bus: &CommandBus<UserAggregate>,
    read_model_store: &dyn ReadModelStore,
    ctx: CommandContext,
    user_name: String,
    user_email: String,
    user_password: &str,
) -> Result<String, AppError> {
    if lookup_aggregate_id_by_email_view(read_model_store, &user_email)
        .await
        .is_some()
    {
        return Err(AppError::CommandFailed(
            nineties_core::command_bus::CommandBusError::handle_failed(
                "<unassigned>",
                format!("email '{}' is already registered", user_email),
            ),
        ));
    }

    let aggregate_id = uuid::Uuid::new_v4().to_string();
    let password_hash = prepare_password(user_password);

    let cmd = UserCommand::RegisterUser {
        id: aggregate_id.clone(),
        name: user_name,
        email: user_email.clone(),
        password_hash,
    };

    command_bus.dispatch(cmd, ctx).await?;

    Ok(aggregate_id)
}
