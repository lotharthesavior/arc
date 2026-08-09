use actix_web::{get, web, HttpResponse, Responder};
use std::collections::HashMap;
use tera::{Context, Tera};

fn templates() -> Tera {
    let mut tera = Tera::default();
    tera.add_raw_templates([
        ("home.html", include_str!("../resources/views/home.html")),
        ("layouts/public.html", include_str!("../resources/views/layouts/public.html")),
        ("layouts/admin.html", include_str!("../resources/views/layouts/admin.html")),
        ("components/ui.html", include_str!("../resources/views/components/ui.html")),
        ("admin/dashboard.html", include_str!("../resources/views/admin/dashboard.html")),
    ]).expect("valid bundled templates");
    tera
}

pub fn render(name: &str, mut context: Context, status: actix_web::http::StatusCode) -> HttpResponse {
    context.insert("app_name", env!("CARGO_PKG_NAME"));
    match templates().render(name, &context) {
        Ok(body) => HttpResponse::build(status).content_type("text/html; charset=utf-8").body(body),
        Err(error) => HttpResponse::InternalServerError().body(error.to_string()),
    }
}

#[get("/")]
async fn home() -> impl Responder { render("home.html", Context::new(), actix_web::http::StatusCode::OK) }

#[get("/admin")]
async fn dashboard() -> impl Responder {
    let mut context = Context::new();
    context.insert("stats", &HashMap::from([("events", 0), ("projections", 0)]));
    render("admin/dashboard.html", context, actix_web::http::StatusCode::OK)
}

pub fn config(cfg: &mut web::ServiceConfig) { cfg.service(home).service(dashboard); }
