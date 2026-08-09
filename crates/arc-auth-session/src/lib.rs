//! Cookie-session authentication protocol for Arc browser applications.
//! Browser pages are intentionally provided by `arc-auth-admin`.

use actix_session::{Session, SessionExt};
use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    web, Error, HttpResponse,
};
use arc_auth_core::{AuthError, Identity, IdentityStore};
use arc_web::{ArcAppBuilder, ArcPlugin};
use futures_util::future::LocalBoxFuture;
use std::{
    future::{ready, Ready},
    sync::Arc,
};

pub const IDENTITY_SESSION_KEY: &str = "arc_auth_identity";

pub fn identity(session: &Session) -> Option<Identity> {
    session.get(IDENTITY_SESSION_KEY).ok().flatten()
}

pub fn cache_identity(session: &Session, identity: &Identity) {
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

pub async fn authenticate(
    session: &Session,
    store: &dyn IdentityStore,
    email: &str,
    password: &str,
) -> Result<Identity, AuthError> {
    let identity = store.authenticate(email, password).await?;
    cache_identity(session, &identity);
    Ok(identity)
}

pub fn sign_out(session: &Session) {
    session.remove(IDENTITY_SESSION_KEY);
    arc_web::helpers::session::clear_session_user(session);
}

/// Session plugin retained as the stable protocol/middleware registration seam.
pub struct SessionAuthPlugin;
#[async_trait::async_trait]
impl ArcPlugin for SessionAuthPlugin {
    fn name(&self) -> &'static str {
        "auth-session"
    }
    fn register(&self, builder: ArcAppBuilder) -> ArcAppBuilder {
        builder
    }
}

/// Redirect unauthenticated browser requests to the sign-in page.
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
        let future = self.service.call(req);
        Box::pin(async move { future.await.map(ServiceResponse::map_into_left_body) })
    }
}

/// Convenience extractor for handlers that need the configured identity store.
pub type IdentityStoreData = web::Data<dyn IdentityStore>;
