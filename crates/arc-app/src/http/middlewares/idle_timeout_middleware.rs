//! Idle-timeout middleware (HIPAA-3, §164.312(a)(2)(iii) Automatic Logoff).
//!
//! For cookie-session-protected routes: enforce that the session is dropped
//! when the user has been idle longer than [`IdleTimeoutMiddleware::seconds`].
//! Configurable via `SESSION_IDLE_TIMEOUT_SECS`; defaults to 900s (15min) per
//! HHS OCR guidance for systems handling PHI.
//!
//! ## Behavior
//!
//! On each request:
//! 1. If the session has no `user_id`, pass through unchanged.
//! 2. If `last_active_at` is missing (first authenticated hit), stamp `now`
//!    and continue.
//! 3. If `now - last_active_at > idle_timeout`, purge the session and
//!    redirect to `/signin?reason=idle`. The user must re-authenticate.
//! 4. Otherwise, refresh `last_active_at = now` and continue.
//!
//! Idle timeout is enforced **independently** of the absolute session
//! lifetime configured in [`actix_session`]. A long-lived session that goes
//! idle still gets logged out. A short-lived session that's actively used
//! still expires on its absolute deadline.
//!
//! Stateless API requests (JWT-bearer, no session) are unaffected: they have
//! no session, so step 1 short-circuits.

use actix_session::SessionExt;
use actix_web::body::EitherBody;
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};
use std::time::{SystemTime, UNIX_EPOCH};

/// Default per HHS OCR guidance for PHI-bearing systems.
pub const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 900;

const SESSION_KEY_LAST_ACTIVE: &str = "last_active_at";
const SESSION_KEY_USER_ID: &str = "user_id";

/// Idle-timeout enforcement middleware.
#[derive(Debug, Clone, Copy)]
pub struct IdleTimeoutMiddleware {
    pub seconds: u64,
}

impl IdleTimeoutMiddleware {
    pub fn new(seconds: u64) -> Self {
        Self { seconds }
    }

    /// Construct from `SESSION_IDLE_TIMEOUT_SECS`; falls back to default.
    pub fn from_env() -> Self {
        let secs = std::env::var("SESSION_IDLE_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_IDLE_TIMEOUT_SECS);
        Self::new(secs)
    }
}

impl Default for IdleTimeoutMiddleware {
    fn default() -> Self {
        Self::new(DEFAULT_IDLE_TIMEOUT_SECS)
    }
}

impl<S, B> Transform<S, ServiceRequest> for IdleTimeoutMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = IdleTimeoutCheck<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(IdleTimeoutCheck {
            service,
            seconds: self.seconds,
        }))
    }
}

/// Inner service produced by [`IdleTimeoutMiddleware`].
pub struct IdleTimeoutCheck<S> {
    service: S,
    seconds: u64,
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl<S, B> Service<ServiceRequest> for IdleTimeoutCheck<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let session = req.get_session();
        let limit = self.seconds;

        // No authenticated user → nothing to enforce.
        let user_present = session
            .get::<i32>(SESSION_KEY_USER_ID)
            .ok()
            .flatten()
            .is_some();
        if !user_present {
            let fut = self.service.call(req);
            return Box::pin(async move { fut.await.map(ServiceResponse::map_into_left_body) });
        }

        let now = now_secs();
        let last_active: Option<u64> = session.get(SESSION_KEY_LAST_ACTIVE).ok().flatten();

        match last_active {
            None => {
                let _ = session.insert(SESSION_KEY_LAST_ACTIVE, now);
            }
            Some(prev) if now.saturating_sub(prev) > limit => {
                tracing::info!(
                    idle_for_secs = now.saturating_sub(prev),
                    limit_secs = limit,
                    "session exceeded idle timeout — purging"
                );
                session.purge();
                return Box::pin(async move {
                    Ok(req.into_response(
                        HttpResponse::Found()
                            .insert_header(("Location", "/signin?reason=idle"))
                            .finish()
                            .map_into_right_body(),
                    ))
                });
            }
            Some(_) => {
                let _ = session.insert(SESSION_KEY_LAST_ACTIVE, now);
            }
        }

        let fut = self.service.call(req);
        Box::pin(async move { fut.await.map(ServiceResponse::map_into_left_body) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_session::storage::CookieSessionStore;
    use actix_session::{Session, SessionMiddleware};
    use actix_web::cookie::{Cookie, Key};
    use actix_web::{http, test, web, App, HttpResponse};

    fn key() -> Key {
        Key::from(&[0u8; 64])
    }

    #[actix_web::test]
    async fn test_no_session_passes_through() {
        let app = test::init_service(
            App::new()
                .wrap(IdleTimeoutMiddleware::new(60))
                .wrap(SessionMiddleware::new(CookieSessionStore::default(), key()))
                .route(
                    "/p",
                    web::get().to(|| async { HttpResponse::Ok().finish() }),
                ),
        )
        .await;
        let req = test::TestRequest::get().uri("/p").to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), http::StatusCode::OK);
    }

    #[actix_web::test]
    async fn test_idle_session_redirects_to_signin() {
        let app = test::init_service(
            App::new()
                .wrap(IdleTimeoutMiddleware::new(1))
                .wrap(SessionMiddleware::new(CookieSessionStore::default(), key()))
                .route(
                    "/seed",
                    web::get().to(|s: Session| async move {
                        s.insert(SESSION_KEY_USER_ID, 42i32).unwrap();
                        // last_active_at = 100s ago — already past the 1s limit
                        s.insert(SESSION_KEY_LAST_ACTIVE, now_secs().saturating_sub(100))
                            .unwrap();
                        HttpResponse::Ok().finish()
                    }),
                )
                .route(
                    "/protected",
                    web::get().to(|| async { HttpResponse::Ok().finish() }),
                ),
        )
        .await;

        let seed = test::TestRequest::get().uri("/seed").to_request();
        let seed_resp = test::call_service(&app, seed).await;
        assert_eq!(seed_resp.status(), http::StatusCode::OK);
        let cookie = Cookie::parse_encoded(
            seed_resp
                .headers()
                .get("set-cookie")
                .unwrap()
                .to_str()
                .unwrap(),
        )
        .unwrap();

        let req = test::TestRequest::get()
            .cookie(cookie)
            .uri("/protected")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), http::StatusCode::FOUND);
        assert!(
            resp.headers()
                .get("location")
                .and_then(|v| v.to_str().ok())
                .map(|l| l.contains("/signin?reason=idle"))
                .unwrap_or(false),
            "expected redirect to /signin?reason=idle"
        );
    }

    #[actix_web::test]
    async fn test_active_session_refreshes_last_active() {
        let app = test::init_service(
            App::new()
                .wrap(IdleTimeoutMiddleware::new(3600))
                .wrap(SessionMiddleware::new(CookieSessionStore::default(), key()))
                .route(
                    "/seed",
                    web::get().to(|s: Session| async move {
                        s.insert(SESSION_KEY_USER_ID, 42i32).unwrap();
                        s.insert(SESSION_KEY_LAST_ACTIVE, now_secs().saturating_sub(10))
                            .unwrap();
                        HttpResponse::Ok().finish()
                    }),
                )
                .route(
                    "/protected",
                    web::get().to(|| async { HttpResponse::Ok().finish() }),
                ),
        )
        .await;

        let seed = test::TestRequest::get().uri("/seed").to_request();
        let seed_resp = test::call_service(&app, seed).await;
        let cookie = Cookie::parse_encoded(
            seed_resp
                .headers()
                .get("set-cookie")
                .unwrap()
                .to_str()
                .unwrap(),
        )
        .unwrap();

        let req = test::TestRequest::get()
            .cookie(cookie)
            .uri("/protected")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), http::StatusCode::OK);
    }
}

#[cfg(test)]
mod env_tests {
    use super::{IdleTimeoutMiddleware, DEFAULT_IDLE_TIMEOUT_SECS};
    use std::env;

    #[test]
    fn test_from_env_uses_default_when_unset() {
        env::remove_var("SESSION_IDLE_TIMEOUT_SECS");
        assert_eq!(
            IdleTimeoutMiddleware::from_env().seconds,
            DEFAULT_IDLE_TIMEOUT_SECS
        );
    }

    #[test]
    fn test_from_env_parses_value() {
        env::set_var("SESSION_IDLE_TIMEOUT_SECS", "300");
        assert_eq!(IdleTimeoutMiddleware::from_env().seconds, 300);
        env::remove_var("SESSION_IDLE_TIMEOUT_SECS");
    }
}
