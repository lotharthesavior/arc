use actix_session::Session;
use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use arc_web::helpers::{csrf, session};
use serde::Deserialize;
use tera::{Context, Tera};

const TEMPLATES: &[(&str, &str)] = &[
    (
        "layouts/public.html",
        include_str!("../resources/views/layouts/public.html"),
    ),
    (
        "layouts/admin.html",
        include_str!("../resources/views/layouts/admin.html"),
    ),
    (
        "components/ui.html",
        include_str!("../resources/views/components/ui.html"),
    ),
    ("home.html", include_str!("../resources/views/home.html")),
    (
        "auth/signin.html",
        include_str!("../resources/views/auth/signin.html"),
    ),
    (
        "auth/register.html",
        include_str!("../resources/views/auth/register.html"),
    ),
    (
        "auth/forgot_password.html",
        include_str!("../resources/views/auth/forgot_password.html"),
    ),
    (
        "auth/reset_password.html",
        include_str!("../resources/views/auth/reset_password.html"),
    ),
    (
        "admin/dashboard.html",
        include_str!("../resources/views/admin/dashboard.html"),
    ),
    (
        "admin/profile.html",
        include_str!("../resources/views/admin/profile.html"),
    ),
    (
        "admin/settings.html",
        include_str!("../resources/views/admin/settings.html"),
    ),
    (
        "errors/403.html",
        include_str!("../resources/views/errors/403.html"),
    ),
    (
        "errors/404.html",
        include_str!("../resources/views/errors/404.html"),
    ),
    (
        "errors/500.html",
        include_str!("../resources/views/errors/500.html"),
    ),
];

fn render(name: &str, mut context: Context, status: actix_web::http::StatusCode) -> HttpResponse {
    context.insert("app_name", env!("CARGO_PKG_NAME"));
    let mut tera = Tera::default();
    if let Err(error) = tera.add_raw_templates(TEMPLATES.iter().copied()) {
        return HttpResponse::InternalServerError().body(format!("template error: {error}"));
    }
    match tera.render(name, &context) {
        Ok(html) => HttpResponse::build(status)
            .content_type("text/html; charset=utf-8")
            .body(html),
        Err(error) => HttpResponse::InternalServerError().body(format!("template error: {error}")),
    }
}

fn page(name: &str) -> HttpResponse {
    render(name, Context::new(), actix_web::http::StatusCode::OK)
}

#[get("/")]
pub async fn root() -> impl Responder {
    HttpResponse::Found()
        .insert_header(("Location", "/home"))
        .finish()
}

#[get("/home")]
pub async fn home() -> impl Responder {
    page("home.html")
}

#[get("/signin")]
async fn signin(session: Session) -> impl Responder {
    let mut context = Context::new();
    context.insert("csrf_token", &csrf::get_csrf_token(&session));
    page_with("auth/signin.html", context)
}

#[derive(Deserialize)]
struct SignInForm {
    email: String,
    password: String,
    csrf_token: String,
}

#[post("/signin")]
async fn signin_submit(form: web::Form<SignInForm>, session_data: Session) -> impl Responder {
    if !csrf::validate_and_regenerate_csrf_token(&session_data, &form.csrf_token) {
        return render(
            "errors/403.html",
            Context::new(),
            actix_web::http::StatusCode::FORBIDDEN,
        );
    }
    if !crate::routes::valid_admin_credentials(&form.email, &form.password) {
        let mut context = Context::new();
        context.insert("csrf_token", &csrf::get_csrf_token(&session_data));
        context.insert("email", &form.email);
        context.insert("error", "Email or password was not recognized.");
        return render(
            "auth/signin.html",
            context,
            actix_web::http::StatusCode::UNAUTHORIZED,
        );
    }
    session::set_session_user(
        &session_data,
        &session::SessionUser {
            id: "generated-admin".into(),
            name: "Administrator".into(),
            email: form.email.clone(),
        },
    );
    HttpResponse::SeeOther()
        .insert_header(("Location", "/admin"))
        .finish()
}

#[post("/signout")]
async fn signout(form: web::Form<SignOutForm>, session_data: Session) -> impl Responder {
    if !csrf::validate_and_regenerate_csrf_token(&session_data, &form.csrf_token) {
        return HttpResponse::Forbidden().finish();
    }
    session::clear_session_user(&session_data);
    HttpResponse::SeeOther()
        .insert_header(("Location", "/home"))
        .finish()
}

#[derive(Deserialize)]
struct SignOutForm {
    csrf_token: String,
}

#[get("/register")]
async fn register(session_data: Session) -> impl Responder {
    if std::env::var("SELF_REGISTRATION").as_deref() != Ok("true") {
        return not_found();
    }
    let mut context = Context::new();
    context.insert("csrf_token", &csrf::get_csrf_token(&session_data));
    page_with("auth/register.html", context)
}

#[get("/forgot-password")]
async fn forgot_password(session_data: Session) -> impl Responder {
    let mut context = Context::new();
    context.insert("csrf_token", &csrf::get_csrf_token(&session_data));
    page_with("auth/forgot_password.html", context)
}

#[get("/reset-password/{token}")]
async fn reset_password(token: web::Path<String>, session_data: Session) -> impl Responder {
    let mut context = Context::new();
    context.insert("csrf_token", &csrf::get_csrf_token(&session_data));
    context.insert("token", token.as_str());
    context.insert("token_valid", &false);
    page_with("auth/reset_password.html", context)
}

#[get("")]
async fn dashboard(session_data: Session) -> impl Responder {
    admin_page("admin/dashboard.html", &session_data)
}
#[get("/profile")]
async fn profile(session_data: Session) -> impl Responder {
    admin_page("admin/profile.html", &session_data)
}
#[get("/settings")]
async fn settings(session_data: Session) -> impl Responder {
    admin_page("admin/settings.html", &session_data)
}

fn admin_page(name: &str, session_data: &Session) -> HttpResponse {
    let mut context = Context::new();
    context.insert("session_csrf_token", &csrf::get_csrf_token(session_data));
    page_with(name, context)
}

fn page_with(name: &str, context: Context) -> HttpResponse {
    render(name, context, actix_web::http::StatusCode::OK)
}

async fn browser_not_found(request: HttpRequest) -> HttpResponse {
    if request.path().starts_with("/api/") {
        HttpResponse::NotFound().json(serde_json::json!({"error": "not found"}))
    } else {
        not_found()
    }
}

fn not_found() -> HttpResponse {
    render(
        "errors/404.html",
        Context::new(),
        actix_web::http::StatusCode::NOT_FOUND,
    )
}

pub fn config(cfg: &mut web::ServiceConfig) {
    use arc_web::http::middlewares::{
        auth_middleware::AuthMiddleware, idle_timeout_middleware::IdleTimeoutMiddleware,
    };
    cfg.service(root)
        .service(home)
        .service(signin)
        .service(signin_submit)
        .service(signout)
        .service(register)
        .service(forgot_password)
        .service(reset_password)
        .service(
            web::scope("/admin")
                .wrap(AuthMiddleware)
                .wrap(IdleTimeoutMiddleware::from_env())
                .service(dashboard)
                .service(profile)
                .service(settings),
        )
        .default_service(web::route().to(browser_not_found));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_generated_template_compiles() {
        let mut tera = Tera::default();
        tera.add_raw_templates(TEMPLATES.iter().copied()).unwrap();
        let mut context = Context::new();
        context.insert("app_name", "fixture");
        context.insert("csrf_token", "fixture-token");
        context.insert("session_csrf_token", "fixture-token");
        for name in [
            "home.html",
            "auth/signin.html",
            "admin/dashboard.html",
            "errors/404.html",
        ] {
            let rendered = tera.render(name, &context);
            assert!(rendered.is_ok(), "{name}: {:?}", rendered.unwrap_err());
        }
    }
}
