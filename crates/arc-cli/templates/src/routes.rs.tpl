use actix_web::{get, post, web, HttpResponse, Responder};
use arc_core::{
    read_model_store::ReadModelStore,
    session::{SessionRecord, SessionStore},
};
use arc_web::http::middlewares::jwt_middleware::JwtMiddleware;
use serde::Deserialize;

#[get("/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({"status":"healthy","application":env!("CARGO_PKG_NAME"),"version":env!("CARGO_PKG_VERSION")}))
}

#[derive(Deserialize)]
struct ApiSignIn {
    email: String,
    password: String,
}

#[post("/api/session")]
async fn api_signin(
    body: web::Json<ApiSignIn>,
    read_models: web::Data<dyn ReadModelStore>,
    sessions: web::Data<dyn SessionStore>,
) -> impl Responder {
    let Some(user) =
        crate::auth::authenticate(read_models.get_ref(), &body.email, &body.password).await
    else {
        return HttpResponse::Unauthorized()
            .json(serde_json::json!({"error":"invalid credentials"}));
    };
    let Ok((token, jti)) = arc_web::helpers::jwt::create_token(&user.id) else {
        return HttpResponse::InternalServerError().finish();
    };
    let now_us = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as i64;
    let expires_at_us = now_us + (arc_web::helpers::jwt::get_jwt_expiry() as i64 * 3_600_000_000);
    if sessions
        .record_session(SessionRecord {
            jti,
            actor_id: user.id,
            created_at_us: now_us,
            expires_at_us,
            revoked_at_us: None,
        })
        .await
        .is_err()
    {
        return HttpResponse::ServiceUnavailable().finish();
    }
    HttpResponse::Ok().json(serde_json::json!({"token":token,"token_type":"Bearer"}))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    // arc:browser-resource-routes
    cfg.service(health)
        .service(api_signin)
        // {{ui-routes}}
        .service(web::scope("/api").wrap(JwtMiddleware).configure(api_config));
}
fn api_config(_cfg: &mut web::ServiceConfig) {
    // arc:api-routes
}
