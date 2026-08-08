use actix_web::{post, web, HttpResponse};
use arc_auth_core::IdentityStore;
use arc_core::session::{SessionRecord, SessionStore};
use arc_web::{ArcAppBuilder, ArcPlugin};
use serde::Deserialize;

pub use arc_web::http::middlewares::jwt_middleware::JwtMiddleware as RequireJwt;

#[derive(Deserialize)]
struct SignIn {
    email: String,
    password: String,
}
#[post("/api/session")]
async fn signin(
    body: web::Json<SignIn>,
    identities: web::Data<dyn IdentityStore>,
    sessions: web::Data<dyn SessionStore>,
) -> HttpResponse {
    let user = match identities.authenticate(&body.email, &body.password).await {
        Ok(user) => user,
        Err(_) => {
            return HttpResponse::Unauthorized()
                .json(serde_json::json!({"error":"invalid credentials"}))
        }
    };
    let (token, jti) = match arc_web::helpers::jwt::create_token(&user.id) {
        Ok(pair) => pair,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as i64;
    let expires = now + (arc_web::helpers::jwt::get_jwt_expiry() as i64 * 3_600_000_000);
    if sessions
        .record_session(SessionRecord {
            jti,
            actor_id: user.id,
            created_at_us: now,
            expires_at_us: expires,
            revoked_at_us: None,
        })
        .await
        .is_err()
    {
        return HttpResponse::ServiceUnavailable().finish();
    }
    HttpResponse::Ok().json(serde_json::json!({"token":token,"token_type":"Bearer"}))
}
fn routes(cfg: &mut web::ServiceConfig) {
    cfg.service(signin);
}
pub struct JwtAuthPlugin;
#[async_trait::async_trait]
impl ArcPlugin for JwtAuthPlugin {
    fn name(&self) -> &'static str {
        "auth-jwt"
    }
    fn register(&self, builder: ArcAppBuilder) -> ArcAppBuilder {
        builder.register_routes(routes)
    }
}
