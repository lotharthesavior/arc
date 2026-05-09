use dotenv::dotenv;
use jsonwebtoken::{
    decode, encode, Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// JWT token claims payload.
///
/// `sub` holds the aggregate UUID; `jti` is the unique token id used by the
/// server-side session registry (HIPAA-4) for revocation.
///
/// `jti` is `Option` for the rollout window — tokens minted before HIPAA-4
/// landed have no `jti`. Acceptance of those is governed by the
/// `JWT_GRANDFATHER_LEGACY` env flag, enforced in the JWT middleware.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject: aggregate UUID.
    pub sub: String,
    /// Expiration timestamp (seconds since UNIX epoch).
    pub exp: usize,
    /// JWT id (HIPAA-4 revocation key). `None` only for pre-HIPAA-4 tokens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jti: Option<Uuid>,
}

static JWT_SECRET: Lazy<Vec<u8>> = Lazy::new(|| {
    dotenv().ok();
    env::var("JWT_SECRET")
        .expect("JWT_SECRET must be set")
        .into_bytes()
});

static JWT_EXPIRY_HOURS: Lazy<u64> = Lazy::new(|| {
    dotenv().ok();
    env::var("JWT_EXPIRY_HOURS")
        .unwrap_or_else(|_| "24".to_string())
        .parse()
        .expect("Invalid JWT_EXPIRY_HOURS")
});

pub fn get_jwt_secret() -> &'static [u8] {
    &JWT_SECRET
}

pub fn get_jwt_expiry() -> u64 {
    *JWT_EXPIRY_HOURS
}

/// Mint a signed JWT for the given aggregate UUID. Returns the token and the
/// `jti` so the caller can record the session in the server-side store
/// before handing the token to the client.
pub fn create_token(aggregate_id: &str) -> Result<(String, Uuid), jsonwebtoken::errors::Error> {
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;
    let exp = now_secs + (get_jwt_expiry() * 3600) as usize;
    let jti = Uuid::new_v4();
    let claims = Claims {
        sub: aggregate_id.to_string(),
        exp,
        jti: Some(jti),
    };
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(get_jwt_secret()),
    )?;
    Ok((token, jti))
}

/// Decode and signature-verify a JWT, returning the full claims object so
/// downstream code can apply HIPAA-4 jti policy.
pub fn decode_token(token: &str) -> Result<Claims, Box<dyn Error + Send + Sync>> {
    let validation = Validation::new(Algorithm::HS256);
    let data: TokenData<Claims> = decode(
        token,
        &DecodingKey::from_secret(get_jwt_secret()),
        &validation,
    )?;
    Ok(data.claims)
}

/// Backwards-compat shim used by tests. Returns just the aggregate UUID. New
/// code should call [`decode_token`] and route through the session-aware
/// middleware.
#[allow(dead_code)]
pub fn validate_token(token: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    Ok(decode_token(token)?.sub)
}
