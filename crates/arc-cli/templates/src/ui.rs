use actix_web::{get, HttpResponse, Responder};
use tera::{Context, Tera};

#[get("/")]
pub async fn home() -> impl Responder {
    let template = include_str!("../resources/views/home.html");
    let mut context = Context::new();
    context.insert("app_name", env!("CARGO_PKG_NAME"));
    match Tera::one_off(template, &context, true) {
        Ok(html) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html),
        Err(error) => HttpResponse::InternalServerError().body(format!("template error: {error}")),
    }
}
