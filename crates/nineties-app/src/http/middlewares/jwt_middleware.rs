use crate::helpers::jwt::validate_token;
use actix_web::body::EitherBody;
use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{Error, HttpMessage, HttpResponse};
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};

/// JWT Bearer token authentication middleware for API endpoints.
/// Extracts and validates the token from the `Authorization: Bearer <token>` header,
/// then injects the user ID into request extensions.
pub struct JwtMiddleware;

impl<S, B> Transform<S, ServiceRequest> for JwtMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = JwtCheck<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(JwtCheck { service }))
    }
}

/// Inner service wrapper created by [`JwtMiddleware`].
pub struct JwtCheck<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for JwtCheck<S>
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
        let auth_value = req.headers().get("Authorization");
        let token = if let Some(auth) = auth_value {
            auth.to_str()
                .ok()
                .and_then(|header| header.strip_prefix("Bearer "))
        } else {
            None
        };

        match token {
            Some(t) => match validate_token(t) {
                Ok(user_id) => {
                    req.extensions_mut().insert(user_id);
                    let fut = self.service.call(req);
                    Box::pin(async move { fut.await.map(|r| r.map_into_left_body()) })
                }
                Err(_) => {
                    let resp = req.into_response(
                        HttpResponse::Unauthorized()
                            .content_type("application/json")
                            .body(r#"{"error": "Invalid or expired token"}"#)
                            .map_into_right_body(),
                    );
                    Box::pin(async move { Ok(resp) })
                }
            },
            None => {
                let resp = req.into_response(
                    HttpResponse::Unauthorized()
                        .content_type("application/json")
                        .body(r#"{"error": "Missing or invalid Authorization header. Expected: Bearer <token>"}"#)
                        .map_into_right_body(),
                );
                Box::pin(async move { Ok(resp) })
            }
        }
    }
}
