use arc_core::read_model_store::ReadModelStore;
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::rngs::OsRng;
use serde::Serialize;

#[derive(Clone, Serialize)]
pub struct AuthenticatedUser {
    pub id: String,
    pub name: String,
    pub email: String,
    pub version: i64,
}

pub async fn authenticate(
    store: &dyn ReadModelStore,
    email: &str,
    password: &str,
) -> Option<AuthenticatedUser> {
    let email = email.trim().to_ascii_lowercase();
    let rows = store
        .list(crate::domain::user::projector::USERS_VIEW)
        .await
        .ok()?;
    let row = rows.into_iter().find(|row| {
        row["active"].as_bool() == Some(true)
            && row["email"]
                .as_str()
                .is_some_and(|value| value.eq_ignore_ascii_case(&email))
    })?;
    let hash = PasswordHash::new(row["password_hash"].as_str()?).ok()?;
    Argon2::default()
        .verify_password(password.as_bytes(), &hash)
        .ok()?;
    public_user(&row)
}

pub fn hash_password(password: &str) -> anyhow::Result<String> {
    if password.len() < 12 {
        anyhow::bail!("password must contain at least 12 characters");
    }
    Argon2::default()
        .hash_password(password.as_bytes(), &SaltString::generate(&mut OsRng))
        .map(|hash| hash.to_string())
        .map_err(|error| anyhow::anyhow!("password hashing failed: {error}"))
}

pub fn public_user(row: &serde_json::Value) -> Option<AuthenticatedUser> {
    Some(AuthenticatedUser {
        id: row["id"].as_str()?.to_owned(),
        name: row["name"].as_str()?.to_owned(),
        email: row["email"].as_str()?.to_owned(),
        version: row["version"].as_i64()?,
    })
}
