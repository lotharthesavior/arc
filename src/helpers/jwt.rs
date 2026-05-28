use dotenv::dotenv;
use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation,
};
use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i32, // user id
    pub exp: usize,
}

#[derive(Debug)]
pub enum JwtConfigError {
    Disabled,
    MissingSecret,
    SecretTooShort,
    InvalidExpiry,
}

impl fmt::Display for JwtConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JwtConfigError::Disabled => write!(f, "JWT authentication is disabled"),
            JwtConfigError::MissingSecret => {
                write!(f, "JWT_SECRET must be set when JWT auth is enabled")
            }
            JwtConfigError::SecretTooShort => {
                write!(f, "JWT_SECRET must be at least 32 characters long")
            }
            JwtConfigError::InvalidExpiry => {
                write!(f, "JWT_EXPIRY_HOURS must be a valid positive integer")
            }
        }
    }
}

impl Error for JwtConfigError {}

pub fn jwt_auth_enabled() -> bool {
    dotenv().ok();

    env::var("ENABLE_JWT_AUTH")
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn get_jwt_secret() -> Result<Vec<u8>, JwtConfigError> {
    dotenv().ok();

    if !jwt_auth_enabled() {
        return Err(JwtConfigError::Disabled);
    }

    let secret = env::var("JWT_SECRET").map_err(|_| JwtConfigError::MissingSecret)?;
    if secret.len() < 32 {
        return Err(JwtConfigError::SecretTooShort);
    }

    Ok(secret.into_bytes())
}

pub fn get_jwt_expiry() -> Result<u64, JwtConfigError> {
    dotenv().ok();

    env::var("JWT_EXPIRY_HOURS")
        .unwrap_or_else(|_| "24".to_string())
        .parse()
        .map_err(|_| JwtConfigError::InvalidExpiry)
}

pub fn validate_jwt_configuration() -> Result<(), JwtConfigError> {
    if !jwt_auth_enabled() {
        return Ok(());
    }

    let _ = get_jwt_secret()?;
    let _ = get_jwt_expiry()?;
    Ok(())
}

pub fn create_token(user_id: i32) -> Result<String, Box<dyn Error + Send + Sync>> {
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;
    let exp = now_secs + (get_jwt_expiry()? * 3600) as usize;
    let claims = Claims { sub: user_id, exp };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(&get_jwt_secret()?),
    )?;

    Ok(token)
}

pub fn validate_token(token: &str) -> Result<i32, Box<dyn Error + Send + Sync>> {
    let secret = get_jwt_secret()?;
    let validation = Validation::new(Algorithm::HS256);
    let token_data: TokenData<Claims> =
        decode(token, &DecodingKey::from_secret(&secret), &validation)?;
    Ok(token_data.claims.sub)
}
