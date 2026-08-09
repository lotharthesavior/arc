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
    sync::Arc,
};

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
            let _ = session.insert(IDENTITY_SESSION_KEY, user);
            HttpResponse::SeeOther()
                .insert_header(("Location", "/admin"))
                .finish()
        }
        Err(_) => HttpResponse::Unauthorized()
            .content_type("text/html")
            .body(signin_html(
                &arc_web::helpers::csrf::get_csrf_token(&session),
                Some("Email or password was not recognized."),
            )),
    }
}
async fn signin_page(session: Session) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(signin_html(
            &arc_web::helpers::csrf::get_csrf_token(&session),
            None,
        ))
}
fn signin_html(csrf: &str, error: Option<&str>) -> String {
    format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Sign in</title><link rel="stylesheet" href="/public/styles.css"></head><body><main class="focused-shell"><a class="brand" href="/"><span class="brand__mark">A</span><span>Arc</span></a><section class="focused-panel"><p class="eyebrow">Authorized operators</p><h1>Sign in</h1>{}<form method="post" action="/signin"><input type="hidden" name="csrf_token" value="{}"><label>Email<input type="email" name="email" autocomplete="username"></label><label>Password<input type="password" name="password" autocomplete="current-password"></label><button class="button button--primary button--wide">Sign in</button></form></section><p class="build-mark">ARC / INSTRUMENT PANEL</p></main></body></html>"#,
        error
            .map(|e| format!("<p role=alert>{e}</p>"))
            .unwrap_or_default(),
        csrf
    )
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
    HttpResponse::SeeOther()
        .insert_header(("Location", "/"))
        .finish()
}

fn csrf_ok(session: &Session, token: &str) -> bool {
    arc_web::helpers::csrf::validate_and_regenerate_csrf_token(session, token)
}
fn page(title: &str, body: String) -> HttpResponse {
    HttpResponse::Ok().content_type("text/html").body(format!("<!doctype html><html lang=en><head><meta charset=utf-8><meta name=viewport content=\"width=device-width,initial-scale=1\"><title>{title}</title><link rel=stylesheet href=/public/styles.css></head><body><main class=focused-shell><nav><a href=/admin>Admin</a> <a href=/profile>Profile</a> <a href=/admin/users>Users</a></nav><section class=focused-panel><h1>{title}</h1>{body}</section></main></body></html>"))
}

#[get("/profile")]
async fn profile(session: Session) -> HttpResponse {
    let Some(user) = identity(&session) else {
        return HttpResponse::Found()
            .insert_header(("Location", "/signin"))
            .finish();
    };
    let token = arc_web::helpers::csrf::get_csrf_token(&session);
    page(
        "Profile",
        format!(
            r#"<form method=post><input type=hidden name=csrf_token value="{token}"><label>Name<input name=name value="{}"></label><label>Email<input type=email name=email value="{}"></label><button>Save</button></form><form method=post action=/profile/password><input type=hidden name=csrf_token value="{token}"><label>Current password<input type=password name=current_password></label><label>New password<input type=password name=new_password></label><button>Change password</button></form>"#,
            user.name, user.email
        ),
    )
}
#[derive(Deserialize)]
struct ProfileForm {
    name: String,
    email: String,
    csrf_token: String,
}
#[post("/profile")]
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
            let _ = session.insert(IDENTITY_SESSION_KEY, updated);
            HttpResponse::SeeOther()
                .insert_header(("Location", "/profile"))
                .finish()
        }
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}
#[derive(Deserialize)]
struct PasswordForm {
    current_password: String,
    new_password: String,
    csrf_token: String,
}
#[post("/profile/password")]
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
        return HttpResponse::Unauthorized().body("current password is incorrect");
    }
    match store.change_password(&user.id, &form.new_password).await {
        Ok(()) => HttpResponse::SeeOther()
            .insert_header(("Location", "/profile"))
            .finish(),
        Err(e) => HttpResponse::BadRequest().body(e.to_string()),
    }
}

#[get("/admin/users")]
async fn users(session: Session, store: web::Data<dyn IdentityStore>) -> HttpResponse {
    let Some(actor) = identity(&session) else {
        return HttpResponse::Found()
            .insert_header(("Location", "/signin"))
            .finish();
    };
    if !actor.has_role("admin") {
        return HttpResponse::Forbidden().finish();
    }
    let token = arc_web::helpers::csrf::get_csrf_token(&session);
    match store.list().await {Ok(users)=>page("Users",users.into_iter().map(|u|format!(r#"<section><strong>{}</strong> &lt;{}&gt; roles: {}<form method=post action="/admin/users/{}/roles"><input type=hidden name=csrf_token value="{}"><input name=roles value="{}"><button>Set roles</button></form></section>"#,u.name,u.email,u.roles.join(", "),u.id,token,u.roles.join(","))).collect()),Err(e)=>HttpResponse::InternalServerError().body(e.to_string())}
}
#[derive(Deserialize)]
struct RolesForm {
    roles: String,
    csrf_token: String,
}
#[post("/admin/users/{id}/roles")]
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
        .service(profile)
        .service(profile_save)
        .service(password_save)
        .service(users)
        .service(roles_save);
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
    fn signin_uses_scaffold_styles() {
        let html = signin_html("csrf", None);
        assert!(html.contains("/public/styles.css"));
        assert!(html.contains("focused-shell"));
    }
}
