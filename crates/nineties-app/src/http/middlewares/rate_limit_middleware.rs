use crate::helpers::rate_limit::RateLimiter;
use actix_web::{
    body::MessageBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    web, Error,
};
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};

/// Middleware factory for global rate limiting
pub struct GlobalRateLimit;

impl<S, B> Transform<S, ServiceRequest> for GlobalRateLimit
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = GlobalRateLimitMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(GlobalRateLimitMiddleware { service }))
    }
}

pub struct GlobalRateLimitMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for GlobalRateLimitMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let path = req.path().to_string();

        // Skip rate limiting for health check and static files
        if path == "/health" || path.starts_with("/public/") {
            let fut = self.service.call(req);
            return Box::pin(async move { fut.await });
        }

        let ip = req
            .connection_info()
            .realip_remote_addr()
            .unwrap_or("unknown")
            .to_string();
        let key = format!("global:{}", ip);

        // Clone the limiter reference
        let limiter = req.app_data::<web::Data<RateLimiter>>().cloned();

        let fut = self.service.call(req);

        Box::pin(async move {
            if let Some(limiter) = limiter {
                match limiter.check(key) {
                    Ok(()) => {
                        // Request allowed
                    }
                    Err(retry_after) => {
                        tracing::warn!(ip = %ip, path = %path, "Global rate limit exceeded");

                        return Err(actix_web::error::ErrorTooManyRequests(format!(
                            "Rate limit exceeded. Retry after {} seconds",
                            retry_after.as_secs()
                        )));
                    }
                }
            }

            fut.await
        })
    }
}
