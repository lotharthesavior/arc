use crate::domain::user::{aggregate::UserAggregate, commands::UserCommand, projector::USERS_VIEW};
use actix_session::Session;
use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder};
use arc_core::{
    command_bus::{CommandBus, CommandContext},
    read_model_store::ReadModelStore,
    session::SessionStore,
};
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
        "admin/dashboard.html",
        include_str!("../resources/views/admin/dashboard.html"),
    ),
    (
        "admin/profile.html",
        include_str!("../resources/views/admin/profile.html"),
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
fn forbidden() -> HttpResponse {
    render(
        "errors/403.html",
        Context::new(),
        actix_web::http::StatusCode::FORBIDDEN,
    )
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
    render("auth/signin.html", context, actix_web::http::StatusCode::OK)
}
#[derive(Deserialize)]
struct SignInForm {
    email: String,
    password: String,
    csrf_token: String,
}
#[post("/signin")]
async fn signin_submit(
    form: web::Form<SignInForm>,
    cookie: Session,
    store: web::Data<dyn ReadModelStore>,
) -> impl Responder {
    if !csrf::validate_and_regenerate_csrf_token(&cookie, &form.csrf_token) {
        return forbidden();
    }
    let Some(user) = crate::auth::authenticate(store.get_ref(), &form.email, &form.password).await
    else {
        let mut context = Context::new();
        context.insert("csrf_token", &csrf::get_csrf_token(&cookie));
        context.insert("email", &form.email);
        context.insert("error", "Email or password was not recognized.");
        return render(
            "auth/signin.html",
            context,
            actix_web::http::StatusCode::UNAUTHORIZED,
        );
    };
    session::set_session_user(
        &cookie,
        &session::SessionUser {
            id: user.id,
            name: user.name,
            email: user.email,
        },
    );
    HttpResponse::SeeOther()
        .insert_header(("Location", "/admin"))
        .finish()
}
#[derive(Deserialize)]
struct SignOutForm {
    csrf_token: String,
}
#[post("/signout")]
async fn signout(form: web::Form<SignOutForm>, cookie: Session) -> impl Responder {
    if !csrf::validate_and_regenerate_csrf_token(&cookie, &form.csrf_token) {
        return forbidden();
    }
    session::clear_session_user(&cookie);
    HttpResponse::SeeOther()
        .insert_header(("Location", "/home"))
        .finish()
}

#[get("")]
async fn dashboard(cookie: Session) -> impl Responder {
    let mut c = Context::new();
    c.insert("session_csrf_token", &csrf::get_csrf_token(&cookie));
    render("admin/dashboard.html", c, actix_web::http::StatusCode::OK)
}
#[get("/profile")]
async fn profile(cookie: Session, store: web::Data<dyn ReadModelStore>) -> impl Responder {
    profile_response(&cookie, store.get_ref(), None, None).await
}

#[derive(Deserialize)]
struct ProfileForm {
    name: String,
    email: String,
    csrf_token: String,
}
#[post("/profile")]
async fn profile_submit(
    form: web::Form<ProfileForm>,
    cookie: Session,
    store: web::Data<dyn ReadModelStore>,
    bus: web::Data<CommandBus<UserAggregate>>,
) -> impl Responder {
    if !csrf::validate_and_regenerate_csrf_token(&cookie, &form.csrf_token) {
        return forbidden();
    }
    let Some(current) = session::get_session_user(&cookie) else {
        return HttpResponse::Unauthorized().finish();
    };
    let context = CommandContext::for_actor(&current.id);
    if let Err(error) = bus
        .dispatch(
            UserCommand::UpdateProfile {
                id: current.id.clone(),
                name: form.name.clone(),
            },
            context.clone(),
        )
        .await
    {
        return profile_response(&cookie, store.get_ref(), Some(error.to_string()), None).await;
    }
    if !current.email.eq_ignore_ascii_case(form.email.trim()) {
        if let Err(error) = bus
            .dispatch(
                UserCommand::ChangeEmail {
                    id: current.id.clone(),
                    email: form.email.clone(),
                },
                context,
            )
            .await
        {
            return profile_response(&cookie, store.get_ref(), Some(error.to_string()), None).await;
        }
    }
    refresh_cookie(&cookie, store.get_ref(), &current.id).await;
    profile_response(&cookie, store.get_ref(), None, Some("Profile updated.")).await
}

#[derive(Deserialize)]
struct PasswordForm {
    current_password: String,
    new_password: String,
    csrf_token: String,
}
#[post("/profile/password")]
async fn password_submit(
    form: web::Form<PasswordForm>,
    cookie: Session,
    store: web::Data<dyn ReadModelStore>,
    bus: web::Data<CommandBus<UserAggregate>>,
    sessions: web::Data<dyn SessionStore>,
) -> impl Responder {
    if !csrf::validate_and_regenerate_csrf_token(&cookie, &form.csrf_token) {
        return forbidden();
    }
    let Some(current) = session::get_session_user(&cookie) else {
        return HttpResponse::Unauthorized().finish();
    };
    if crate::auth::authenticate(store.get_ref(), &current.email, &form.current_password)
        .await
        .is_none()
    {
        return profile_response(
            &cookie,
            store.get_ref(),
            Some("Current password was not recognized.".into()),
            None,
        )
        .await;
    }
    let hash = match crate::auth::hash_password(&form.new_password) {
        Ok(hash) => hash,
        Err(error) => {
            return profile_response(&cookie, store.get_ref(), Some(error.to_string()), None).await
        }
    };
    if let Err(error) = bus
        .dispatch(
            UserCommand::ChangePassword {
                id: current.id.clone(),
                password_hash: hash,
            },
            CommandContext::for_actor(&current.id),
        )
        .await
    {
        return profile_response(&cookie, store.get_ref(), Some(error.to_string()), None).await;
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as i64;
    let _ = sessions.revoke_all_for_actor(&current.id, now).await;
    profile_response(
        &cookie,
        store.get_ref(),
        None,
        Some("Password changed; API sessions were revoked."),
    )
    .await
}

async fn refresh_cookie(cookie: &Session, store: &dyn ReadModelStore, id: &str) {
    if let Some(user) = session::SessionUser::from_projection(store, USERS_VIEW, id).await {
        session::set_session_user(cookie, &user);
    }
}
async fn profile_response(
    cookie: &Session,
    store: &dyn ReadModelStore,
    error: Option<String>,
    notice: Option<&str>,
) -> HttpResponse {
    let Some(cached) = session::get_session_user(cookie) else {
        return HttpResponse::Unauthorized().finish();
    };
    let Some(row) = store.get(USERS_VIEW, &cached.id).await.ok().flatten() else {
        return HttpResponse::ServiceUnavailable().finish();
    };
    let Some(user) = crate::auth::public_user(&row) else {
        return HttpResponse::ServiceUnavailable().finish();
    };
    let mut c = Context::new();
    c.insert("session_csrf_token", &csrf::get_csrf_token(cookie));
    c.insert("user", &user);
    c.insert("error", &error);
    c.insert("notice", &notice);
    render("admin/profile.html", c, actix_web::http::StatusCode::OK)
}

async fn browser_not_found(request: HttpRequest) -> HttpResponse {
    if request.path().starts_with("/api/") {
        HttpResponse::NotFound().json(serde_json::json!({"error":"not found"}))
    } else {
        render(
            "errors/404.html",
            Context::new(),
            actix_web::http::StatusCode::NOT_FOUND,
        )
    }
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
        .service(
            web::scope("/admin")
                .wrap(AuthMiddleware)
                .wrap(IdleTimeoutMiddleware::from_env())
                .service(dashboard)
                .service(profile)
                .service(profile_submit)
                .service(password_submit),
        )
        .default_service(web::route().to(browser_not_found));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generated_templates_compile_without_settings() {
        let mut tera = Tera::default();
        tera.add_raw_templates(TEMPLATES.iter().copied()).unwrap();
        assert!(!TEMPLATES
            .iter()
            .any(|(name, _)| *name == "admin/settings.html"));
    }
}
