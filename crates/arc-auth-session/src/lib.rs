use actix_session::{Session, SessionExt};
use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    get, post, web, Error, HttpResponse,
};
use arc_auth_core::{Identity, IdentityStore};
use arc_web::{ArcAppBuilder, ArcPlugin};
use futures_util::future::LocalBoxFuture;
use serde::Deserialize;
use std::{
    future::{ready, Ready},
    sync::{Arc, LazyLock},
};
use tera::{Context, Tera};

static TEMPLATES: LazyLock<Tera> = LazyLock::new(|| {
    let mut tera = Tera::default();
    tera.add_raw_templates([
        ("layout.html", include_str!("../templates/layout.html")),
        (
            "admin_layout.html",
            include_str!("../templates/admin_layout.html"),
        ),
        ("signin.html", include_str!("../templates/signin.html")),
        ("profile.html", include_str!("../templates/profile.html")),
        ("users.html", include_str!("../templates/users.html")),
    ])
    .expect("arc-auth-session templates must be valid");
    tera
});

fn render(name: &str, context: &Context, status: actix_web::http::StatusCode) -> HttpResponse {
    match TEMPLATES.render(name, context) {
        Ok(body) => HttpResponse::build(status)
            .content_type("text/html; charset=utf-8")
            .body(body),
        Err(error) => HttpResponse::InternalServerError().body(error.to_string()),
    }
}

fn signin_response(
    session: &Session,
    email: &str,
    error: Option<&str>,
    status: actix_web::http::StatusCode,
) -> HttpResponse {
    let mut context = Context::new();
    context.insert(
        "csrf_token",
        &arc_web::helpers::csrf::get_csrf_token(session),
    );
    context.insert("email", email);
    context.insert("error", &error);
    render("signin.html", &context, status)
}

pub const IDENTITY_SESSION_KEY: &str = "arc_auth_identity";
pub fn identity(session: &Session) -> Option<Identity> {
    session.get(IDENTITY_SESSION_KEY).ok().flatten()
}

#[derive(Deserialize)]
struct SignIn {
    email: String,
    password: String,
    csrf_token: String,
}
#[post("/signin")]
async fn signin(
    form: web::Form<SignIn>,
    session: Session,
    store: web::Data<dyn IdentityStore>,
) -> HttpResponse {
    if !arc_web::helpers::csrf::validate_and_regenerate_csrf_token(&session, &form.csrf_token) {
        return HttpResponse::Forbidden().finish();
    }
    match store.authenticate(&form.email, &form.password).await {
        Ok(user) => {
            cache_identity(&session, &user);
            HttpResponse::SeeOther()
                .insert_header(("Location", "/admin"))
                .finish()
        }
        Err(_) => signin_response(
            &session,
            &form.email,
            Some("Email or password was not recognized."),
            actix_web::http::StatusCode::UNAUTHORIZED,
        ),
    }
}
async fn signin_page(session: Session) -> HttpResponse {
    signin_response(&session, "", None, actix_web::http::StatusCode::OK)
}
#[derive(Deserialize)]
struct SignOut {
    csrf_token: String,
}
#[post("/signout")]
async fn signout(form: web::Form<SignOut>, session: Session) -> HttpResponse {
    if !arc_web::helpers::csrf::validate_and_regenerate_csrf_token(&session, &form.csrf_token) {
        return HttpResponse::Forbidden().finish();
    }
    session.remove(IDENTITY_SESSION_KEY);
    arc_web::helpers::session::clear_session_user(&session);
    HttpResponse::SeeOther()
        .insert_header(("Location", "/"))
        .finish()
}

fn csrf_ok(session: &Session, token: &str) -> bool {
    arc_web::helpers::csrf::validate_and_regenerate_csrf_token(session, token)
}

fn cache_identity(session: &Session, identity: &Identity) {
    let _ = session.insert(IDENTITY_SESSION_KEY, identity);
    arc_web::helpers::session::set_session_user(
        session,
        &arc_web::helpers::session::SessionUser {
            id: identity.id.clone(),
            name: identity.name.clone(),
            email: identity.email.clone(),
        },
    );
}
#[get("")]
async fn profile(session: Session) -> HttpResponse {
    let Some(user) = identity(&session) else {
        return HttpResponse::Found()
            .insert_header(("Location", "/signin"))
            .finish();
    };
    profile_response(&session, &user, None, actix_web::http::StatusCode::OK)
}

fn profile_response(
    session: &Session,
    user: &Identity,
    error: Option<&str>,
    status: actix_web::http::StatusCode,
) -> HttpResponse {
    let mut context = Context::new();
    context.insert(
        "csrf_token",
        &arc_web::helpers::csrf::get_csrf_token(session),
    );
    context.insert("user", user);
    context.insert("error", &error);
    render("profile.html", &context, status)
}
#[derive(Deserialize)]
struct ProfileForm {
    name: String,
    email: String,
    csrf_token: String,
}
#[post("")]
async fn profile_save(
    form: web::Form<ProfileForm>,
    session: Session,
    store: web::Data<dyn IdentityStore>,
) -> HttpResponse {
    if !csrf_ok(&session, &form.csrf_token) {
        return HttpResponse::Forbidden().finish();
    }
    let Some(user) = identity(&session) else {
        return HttpResponse::Unauthorized().finish();
    };
    match store
        .update_profile(&user.id, &form.name, &form.email)
        .await
    {
        Ok(updated) => {
            cache_identity(&session, &updated);
            HttpResponse::SeeOther()
                .insert_header(("Location", "/admin/profile"))
                .finish()
        }
        Err(error) => {
            let message = error.to_string();
            profile_response(
                &session,
                &user,
                Some(&message),
                actix_web::http::StatusCode::UNPROCESSABLE_ENTITY,
            )
        }
    }
}
#[derive(Deserialize)]
struct PasswordForm {
    current_password: String,
    new_password: String,
    csrf_token: String,
}
#[post("/password")]
async fn password_save(
    form: web::Form<PasswordForm>,
    session: Session,
    store: web::Data<dyn IdentityStore>,
) -> HttpResponse {
    if !csrf_ok(&session, &form.csrf_token) {
        return HttpResponse::Forbidden().finish();
    }
    let Some(user) = identity(&session) else {
        return HttpResponse::Unauthorized().finish();
    };
    if store
        .authenticate(&user.email, &form.current_password)
        .await
        .is_err()
    {
        return profile_response(
            &session,
            &user,
            Some("Current password is incorrect."),
            actix_web::http::StatusCode::UNAUTHORIZED,
        );
    }
    match store.change_password(&user.id, &form.new_password).await {
        Ok(()) => HttpResponse::SeeOther()
            .insert_header(("Location", "/admin/profile"))
            .finish(),
        Err(error) => {
            let message = error.to_string();
            profile_response(
                &session,
                &user,
                Some(&message),
                actix_web::http::StatusCode::UNPROCESSABLE_ENTITY,
            )
        }
    }
}

#[get("")]
async fn users(session: Session, store: web::Data<dyn IdentityStore>) -> HttpResponse {
    let Some(actor) = identity(&session) else {
        return HttpResponse::Found()
            .insert_header(("Location", "/signin"))
            .finish();
    };
    if !actor.has_role("admin") {
        return HttpResponse::Forbidden().finish();
    }
    match store.list().await {
        Ok(identities) => {
            users_response(&session, &identities, None, actix_web::http::StatusCode::OK)
        }
        Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

fn users_response(
    session: &Session,
    identities: &[Identity],
    error: Option<&str>,
    status: actix_web::http::StatusCode,
) -> HttpResponse {
    let mut context = Context::new();
    context.insert("users", identities);
    context.insert(
        "csrf_token",
        &arc_web::helpers::csrf::get_csrf_token(session),
    );
    context.insert("error", &error);
    render("users.html", &context, status)
}

#[derive(Deserialize)]
struct CreateUserForm {
    name: String,
    email: String,
    password: String,
    roles: String,
    csrf_token: String,
}

#[post("")]
async fn user_create(
    form: web::Form<CreateUserForm>,
    session: Session,
    store: web::Data<dyn IdentityStore>,
) -> HttpResponse {
    if !csrf_ok(&session, &form.csrf_token) {
        return HttpResponse::Forbidden().finish();
    }
    let Some(actor) = identity(&session) else {
        return HttpResponse::Unauthorized().finish();
    };
    if !actor.has_role("admin") {
        return HttpResponse::Forbidden().finish();
    }
    let roles = form
        .roles
        .split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    match store
        .create_user(&form.name, &form.email, &form.password, &roles)
        .await
    {
        Ok(_) => HttpResponse::SeeOther()
            .insert_header(("Location", "/admin/users"))
            .finish(),
        Err(error) => match store.list().await {
            Ok(identities) => {
                let message = error.to_string();
                users_response(
                    &session,
                    &identities,
                    Some(&message),
                    actix_web::http::StatusCode::UNPROCESSABLE_ENTITY,
                )
            }
            Err(_) => HttpResponse::InternalServerError().finish(),
        },
    }
}
#[derive(Deserialize)]
struct RolesForm {
    roles: String,
    csrf_token: String,
}
#[post("/{id}/roles")]
async fn roles_save(
    id: web::Path<String>,
    form: web::Form<RolesForm>,
    session: Session,
    store: web::Data<dyn IdentityStore>,
) -> HttpResponse {
    if !csrf_ok(&session, &form.csrf_token) {
        return HttpResponse::Forbidden().finish();
    }
    let Some(actor) = identity(&session) else {
        return HttpResponse::Unauthorized().finish();
    };
    if !actor.has_role("admin") {
        return HttpResponse::Forbidden().finish();
    }
    let roles = form
        .roles
        .split(',')
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    match store.set_roles(&id, &roles).await {
        Ok(_) => HttpResponse::SeeOther()
            .insert_header(("Location", "/admin/users"))
            .finish(),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

pub fn routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/signin", web::get().to(signin_page))
        .service(signin)
        .service(signout)
        .service(
            web::scope("/admin/profile")
                .wrap(RequireSession)
                .wrap(
                    arc_web::http::middlewares::idle_timeout_middleware::IdleTimeoutMiddleware::from_env(),
                )
                .service(profile)
                .service(profile_save)
                .service(password_save),
        )
        .service(
            web::scope("/admin/users")
                .wrap(RequireSession)
                .wrap(
                    arc_web::http::middlewares::idle_timeout_middleware::IdleTimeoutMiddleware::from_env(),
                )
                .service(users)
                .service(user_create)
                .service(roles_save),
        );
}

pub struct SessionAuthPlugin;
#[async_trait::async_trait]
impl ArcPlugin for SessionAuthPlugin {
    fn name(&self) -> &'static str {
        "auth-session"
    }
    fn register(&self, builder: ArcAppBuilder) -> ArcAppBuilder {
        builder.register_routes(routes)
    }
}

/// Explicit browser-resource opt-in middleware.
pub struct RequireSession;
impl<S, B> Transform<S, ServiceRequest> for RequireSession
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = SessionCheck<S>;
    type Future = Ready<Result<Self::Transform, ()>>;
    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(SessionCheck {
            service: Arc::new(service),
        }))
    }
}
pub struct SessionCheck<S> {
    service: Arc<S>,
}
impl<S, B> Service<ServiceRequest> for SessionCheck<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Error>>;
    forward_ready!(service);
    fn call(&self, req: ServiceRequest) -> Self::Future {
        if req
            .get_session()
            .get::<Identity>(IDENTITY_SESSION_KEY)
            .ok()
            .flatten()
            .is_none()
        {
            return Box::pin(async move {
                Ok(req.into_response(
                    HttpResponse::Found()
                        .insert_header(("Location", "/signin"))
                        .finish()
                        .map_into_right_body(),
                ))
            });
        }
        let fut = self.service.call(req);
        Box::pin(async move { fut.await.map(ServiceResponse::map_into_left_body) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signin_uses_accessible_scaffold_fields_and_escapes_values() {
        let mut context = Context::new();
        context.insert("csrf_token", "csrf");
        context.insert("email", "<script>alert(1)</script>");
        context.insert("error", &Option::<String>::None);
        let html = TEMPLATES.render("signin.html", &context).unwrap();
        assert!(html.contains("/public/styles.css"));
        assert!(html.contains("focused-shell"));
        assert!(html.contains("class=\"field\""));
        assert!(html.contains("for=\"signin-email\""));
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn profile_is_an_admin_route_with_admin_navigation() {
        let user = Identity {
            id: "1".into(),
            name: "Admin".into(),
            email: "admin@example.com".into(),
            active: true,
            roles: vec!["admin".into()],
        };
        let mut context = Context::new();
        context.insert("csrf_token", "csrf");
        context.insert("user", &user);
        context.insert("users", &vec![user]);
        context.insert("error", &Option::<String>::None);
        let profile_html = TEMPLATES.render("profile.html", &context).unwrap();
        let users_html = TEMPLATES.render("users.html", &context).unwrap();
        assert!(profile_html.contains("href=\"/admin/profile\""));
        assert!(profile_html.contains("class=\"workbench\""));
        assert!(profile_html.contains("class=\"rail\""));
        assert!(profile_html.contains("action=\"/admin/profile\""));
        assert!(profile_html.contains("action=\"/admin/profile/password\""));
        assert!(users_html.contains("href=\"/admin/profile\""));
    }
}
