use actix_web::{get, web, HttpResponse, Responder};
use arc_core::session::{SessionRecord, SessionStore};
use arc_web::http::middlewares::jwt_middleware::JwtMiddleware;
use serde::Deserialize;

#[get("/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "application": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION")
    }))
}

#[derive(Deserialize)]
struct ApiSignIn {
    email: String,
    password: String,
}

pub(crate) fn valid_admin_credentials(email: &str, password: &str) -> bool {
    use argon2::{
        password_hash::{PasswordHash, PasswordVerifier},
        Argon2,
    };
    let configured_email = std::env::var("ADMIN_EMAIL").unwrap_or_default();
    if configured_email.is_empty() || email != configured_email {
        return false;
    }
    if let Ok(hash) = std::env::var("ADMIN_PASSWORD_HASH") {
        if !hash.is_empty() {
            return PasswordHash::new(&hash).is_ok_and(|parsed| {
                Argon2::default()
                    .verify_password(password.as_bytes(), &parsed)
                    .is_ok()
            });
        }
    }
    std::env::var("APP_ENV").as_deref() != Ok("production")
        && std::env::var("ADMIN_PASSWORD")
            .is_ok_and(|configured| !configured.is_empty() && password == configured)
}

#[actix_web::post("/api/session")]
async fn api_signin(
    body: web::Json<ApiSignIn>,
    store: web::Data<dyn SessionStore>,
) -> impl Responder {
    if !valid_admin_credentials(&body.email, &body.password) {
        return HttpResponse::Unauthorized()
            .json(serde_json::json!({"error": "invalid credentials"}));
    }
    let Ok((token, jti)) = arc_web::helpers::jwt::create_token("generated-admin") else {
        return HttpResponse::InternalServerError().finish();
    };
    let now_us = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as i64;
    let expires_at_us = now_us + (arc_web::helpers::jwt::get_jwt_expiry() as i64 * 3_600_000_000);
    if store
        .record_session(SessionRecord {
            jti,
            actor_id: "generated-admin".into(),
            created_at_us: now_us,
            expires_at_us,
            revoked_at_us: None,
        })
        .await
        .is_err()
    {
        return HttpResponse::ServiceUnavailable().finish();
    }
    HttpResponse::Ok().json(serde_json::json!({"token": token, "token_type": "Bearer"}))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(health)
        .service(api_signin){{ui-routes}}
        .service(web::scope("/api").wrap(JwtMiddleware).configure(api_config));
}

fn api_config(_cfg: &mut web::ServiceConfig) {
    // arc:api-routes
}
