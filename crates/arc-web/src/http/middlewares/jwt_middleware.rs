//! JWT bearer auth (HIPAA-4 aware).
//!
//! On every request:
//! 1. Decode and signature-verify the bearer token.
//! 2. Check `jti` against the server-side [`SessionStore`]. Revoked / unknown
//!    → 401. Store unavailable → **fail closed** with 503.
//! 3. Insert `(actor_id, jti)` into request extensions for handlers.
//!
//! Tokens minted before HIPAA-4 landed have no `jti`. Set
//! `JWT_GRANDFATHER_LEGACY=true` to accept them during rollout; defaults to
//! refusing such tokens.

use crate::helpers::jwt::decode_token;
use actix_web::body::EitherBody;
use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{Error, HttpMessage, HttpResponse};
use arc_core::session::{SessionStore, SessionStoreError};
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub struct JwtMiddleware;

impl<S, B> Transform<S, ServiceRequest> for JwtMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = JwtCheck<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(JwtCheck {
            service: Rc::new(service),
        }))
    }
}

pub struct JwtCheck<S> {
    service: Rc<S>,
}

fn now_us() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_micros() as i64)
        .unwrap_or(0)
}

fn legacy_grandfather_enabled() -> bool {
    std::env::var("JWT_GRANDFATHER_LEGACY")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false)
}

fn unauthorized<B>(req: ServiceRequest, msg: &str) -> ServiceResponse<EitherBody<B>>
where
    B: 'static,
{
    let body = format!(r#"{{"error": "{}"}}"#, msg);
    req.into_response(
        HttpResponse::Unauthorized()
            .content_type("application/json")
            .body(body)
            .map_into_right_body(),
    )
}

fn service_unavailable<B>(req: ServiceRequest) -> ServiceResponse<EitherBody<B>>
where
    B: 'static,
{
    req.into_response(
        HttpResponse::ServiceUnavailable()
            .content_type("application/json")
            .body(r#"{"error": "Authentication backend unavailable"}"#)
            .map_into_right_body(),
    )
}

impl<S, B> Service<ServiceRequest> for JwtCheck<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let auth_value = req.headers().get("Authorization");
        let token = if let Some(auth) = auth_value {
            auth.to_str()
                .ok()
                .and_then(|header| header.strip_prefix("Bearer "))
                .map(str::to_string)
        } else {
            None
        };

        let token = match token {
            Some(t) => t,
            None => {
                let resp = req.into_response(
                    HttpResponse::Unauthorized()
                        .content_type("application/json")
                        .body(r#"{"error": "Missing or invalid Authorization header. Expected: Bearer <token>"}"#)
                        .map_into_right_body(),
                );
                return Box::pin(async move { Ok(resp) });
            }
        };

        let claims = match decode_token(&token) {
            Ok(c) => c,
            Err(_) => {
                let resp = unauthorized(req, "Invalid or expired token");
                return Box::pin(async move { Ok(resp) });
            }
        };

        let Some(store) = req
            .app_data::<actix_web::web::Data<dyn SessionStore>>()
            .cloned()
        else {
            tracing::error!("JWT revocation store is not configured");
            let response = service_unavailable(req);
            return Box::pin(async move { Ok(response) });
        };
        if claims.jti.is_none() && !legacy_grandfather_enabled() {
            let response = unauthorized(req, "Token has no session identifier");
            return Box::pin(async move { Ok(response) });
        }
        let service = self.service.clone();
        Box::pin(async move {
            if let Some(jti) = claims.jti {
                match store.is_valid(jti, now_us()).await {
                    Ok(true) => {}
                    Ok(false) => return Ok(unauthorized(req, "Session revoked")),
                    Err(SessionStoreError::Sink(_)) => {
                        tracing::error!(category = "sink_unavailable", "session store unavailable");
                        return Ok(service_unavailable(req));
                    }
                    Err(SessionStoreError::NotFound(_)) => {
                        tracing::error!(category = "record_not_found", "session store error");
                        return Ok(service_unavailable(req));
                    }
                    Err(SessionStoreError::Validation(_)) => {
                        tracing::error!(category = "validation", "session store error");
                        return Ok(service_unavailable(req));
                    }
                }
            } else {
                tracing::warn!(reason = "legacy_jwt_no_jti", "accepting jwt without jti");
            }
            service
                .call(req_with_extensions(req, claims.sub, claims.jti))
                .await
                .map(ServiceResponse::map_into_left_body)
        })
    }
}

/// Insert actor_id and jti into request extensions before forwarding.
fn req_with_extensions(req: ServiceRequest, actor_id: String, jti: Option<Uuid>) -> ServiceRequest {
    {
        let mut ext = req.extensions_mut();
        ext.insert(actor_id);
        if let Some(j) = jti {
            ext.insert(j);
        }
    }
    req
}
