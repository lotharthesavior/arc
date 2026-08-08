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
    // arc:browser-resource-routes
    cfg.service(health){{ui-routes}}
        .service(web::scope("/api").configure(api_config));
}
fn api_config(_cfg: &mut web::ServiceConfig) {
    // arc:api-routes
}
