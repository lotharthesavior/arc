use actix_web::{get, web, HttpResponse, Responder};

#[get("/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy",
        "application": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION")
    }))
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(health)
        {{ui-routes}};
}
