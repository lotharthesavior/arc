//! Compatible response defaults shared by generated apps and auth plugins.
use actix_web::{
    body::MessageBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::header::{HeaderName, HeaderValue},
    Error, HttpResponse, ResponseError,
};
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};
use std::{env, io};

/// Baseline containment, deliberately not a script/XSS allowlist. See security-headers.md.
pub const DEFAULT_CSP: &str =
    "base-uri 'self'; object-src 'none'; frame-ancestors 'none'; form-action 'self'";

/// Read once before starting workers. Invalid/empty settings fail startup rather
/// than silently disabling protection. Route-specific headers take precedence.
#[derive(Clone, Debug)]
pub struct SecurityHeaders {
    csp: HeaderValue,
    report_only: Option<HeaderValue>,
    hsts: Option<HeaderValue>,
}

impl SecurityHeaders {
    pub fn from_env() -> io::Result<Self> {
        Self::from_settings(
            env::var("SECURITY_CSP").ok().as_deref(),
            env::var("SECURITY_CSP_REPORT_ONLY").ok().as_deref(),
            env::var("SECURITY_HSTS").ok().as_deref(),
        )
    }

    pub fn from_settings(
        csp: Option<&str>,
        report_only: Option<&str>,
        hsts: Option<&str>,
    ) -> io::Result<Self> {
        fn value(name: &str, value: &str) -> io::Result<HeaderValue> {
            if value.trim().is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{name} must not be empty"),
                ));
            }
            HeaderValue::from_str(value).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{name} is not a valid HTTP header value"),
                )
            })
        }
        Ok(Self {
            csp: value("SECURITY_CSP", csp.unwrap_or(DEFAULT_CSP))?,
            report_only: report_only
                .map(|v| value("SECURITY_CSP_REPORT_ONLY", v))
                .transpose()?,
            hsts: hsts.map(|v| value("SECURITY_HSTS", v)).transpose()?,
        })
    }

    pub fn middleware(&self) -> Self {
        self.clone()
    }

    fn headers(&self) -> Vec<(&'static str, HeaderValue)> {
        let mut headers = vec![
            (
                "x-content-type-options",
                HeaderValue::from_static("nosniff"),
            ),
            ("x-frame-options", HeaderValue::from_static("DENY")),
            (
                "referrer-policy",
                HeaderValue::from_static("strict-origin-when-cross-origin"),
            ),
            (
                "permissions-policy",
                HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
            ),
            ("content-security-policy", self.csp.clone()),
        ];
        if let Some(value) = &self.report_only {
            headers.push(("content-security-policy-report-only", value.clone()));
        }
        if let Some(value) = &self.hsts {
            headers.push(("strict-transport-security", value.clone()));
        }
        headers
    }
}

impl<S, B> Transform<S, ServiceRequest> for SecurityHeaders
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = SecurityHeadersService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(SecurityHeadersService {
            service,
            headers: self.headers(),
        }))
    }
}

pub struct SecurityHeadersService<S> {
    service: S,
    headers: Vec<(&'static str, HeaderValue)>,
}

impl<S, B> Service<ServiceRequest> for SecurityHeadersService<S>
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
        let future = self.service.call(req);
        let headers = self.headers.clone();
        Box::pin(async move {
            let mut response = future.await.map_err(|cause| {
                Error::from(SecurityHeaderError {
                    cause,
                    headers: headers.clone(),
                })
            })?;
            add_headers(response.headers_mut(), headers);

            Ok(response)
        })
    }
}

fn add_headers(
    response: &mut actix_web::http::header::HeaderMap,
    headers: Vec<(&'static str, HeaderValue)>,
) {
    for (name, value) in headers {
        if !response.contains_key(name) {
            response.insert(HeaderName::from_static(name), value);
        }
    }
}

// Preserve the error path for upstream logging. Actix renders this at its normal
// boundary, with the original status/body plus defaults. Never clone HttpRequest
// before routing: Actix requires unique ownership while matching resources.
#[derive(Debug)]
struct SecurityHeaderError {
    cause: Error,
    headers: Vec<(&'static str, HeaderValue)>,
}

impl std::fmt::Display for SecurityHeaderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.cause.fmt(formatter)
    }
}

impl ResponseError for SecurityHeaderError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        self.cause.as_response_error().status_code()
    }

    fn error_response(&self) -> HttpResponse {
        let mut response = self.cause.error_response();
        add_headers(response.headers_mut(), self.headers.clone());
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App, HttpResponse};

    #[actix_web::test]
    async fn defaults_cover_success_redirect_errors_and_assets() {
        let settings = SecurityHeaders::from_settings(None, None, None).unwrap();
        let app = test::init_service(
            App::new()
                .wrap(settings.middleware())
                .route(
                    "/",
                    web::get().to(|| async { HttpResponse::Ok().body("home") }),
                )
                .route(
                    "/signin",
                    web::get().to(|| async {
                        HttpResponse::SeeOther()
                            .insert_header(("location", "/admin"))
                            .finish()
                    }),
                )
                .route(
                    "/csrf",
                    web::post().to(|| async { HttpResponse::Forbidden().finish() }),
                )
                .route(
                    "/api",
                    web::get().to(|| async {
                        Err::<HttpResponse, _>(actix_web::error::ErrorUnauthorized("auth required"))
                    }),
                )
                .route(
                    "/public/app.js",
                    web::get().to(|| async {
                        HttpResponse::Ok()
                            .insert_header(("content-type", "text/javascript"))
                            .insert_header(("cache-control", "public, max-age=31536000, immutable"))
                            .body("export {}")
                    }),
                ),
        )
        .await;
        for (path, status) in [
            ("/", 200),
            ("/signin", 303),
            ("/missing", 404),
            ("/api", 401),
            ("/public/app.js", 200),
        ] {
            let response =
                test::call_service(&app, test::TestRequest::get().uri(path).to_request()).await;
            assert_eq!(response.status().as_u16(), status);
            assert_eq!(
                response.headers().get("content-security-policy").unwrap(),
                DEFAULT_CSP
            );
            assert_eq!(
                response.headers().get("x-content-type-options").unwrap(),
                "nosniff"
            );
            assert_eq!(response.headers().get("x-frame-options").unwrap(), "DENY");
            assert!(!response.headers().contains_key("strict-transport-security"));
            assert!(!response
                .headers()
                .contains_key("content-security-policy-report-only"));
            if path.ends_with(".js") {
                assert_eq!(
                    response.headers().get("cache-control").unwrap(),
                    "public, max-age=31536000, immutable"
                );
            }
        }
        let response =
            test::call_service(&app, test::TestRequest::post().uri("/csrf").to_request()).await;
        assert_eq!(response.status(), 403);
        assert!(response.headers().contains_key("content-security-policy"));
    }

    #[actix_web::test]
    async fn middleware_errors_receive_headers() {
        use actix_web::dev::Service;
        let settings = SecurityHeaders::from_settings(None, None, None).unwrap();
        let app = test::init_service(
            App::new()
                .wrap_fn(|req, service| {
                    let future = service.call(req);
                    async move {
                        let _ = future.await?;
                        Err::<ServiceResponse, _>(actix_web::error::ErrorTooManyRequests("limited"))
                    }
                })
                .wrap(settings.middleware()),
        )
        .await;
        let error = test::try_call_service(&app, test::TestRequest::get().to_request())
            .await
            .unwrap_err();
        let response = error.error_response();
        assert_eq!(response.status(), 429);
        assert_eq!(
            response.headers().get("content-security-policy").unwrap(),
            DEFAULT_CSP
        );
        assert_eq!(
            actix_web::body::to_bytes(response.into_body())
                .await
                .unwrap(),
            "limited"
        );
    }

    #[actix_web::test]
    async fn explicit_policies_and_route_overrides_are_preserved() {
        let settings = SecurityHeaders::from_settings(
            Some("default-src 'self'"),
            Some("script-src 'none'"),
            Some("max-age=300"),
        )
        .unwrap();
        let app = test::init_service(App::new().wrap(settings.middleware()).route(
            "/",
            web::get().to(|| async {
                HttpResponse::Ok()
                    .insert_header(("content-security-policy", "default-src 'none'"))
                    .finish()
            }),
        ))
        .await;
        let response = test::call_service(&app, test::TestRequest::get().to_request()).await;
        assert_eq!(
            response.headers().get("content-security-policy").unwrap(),
            "default-src 'none'"
        );
        assert_eq!(
            response
                .headers()
                .get("content-security-policy-report-only")
                .unwrap(),
            "script-src 'none'"
        );
        assert_eq!(
            response.headers().get("strict-transport-security").unwrap(),
            "max-age=300"
        );
    }

    #[actix_web::test]
    async fn invalid_configuration_fails_without_echoing_values() {
        for bad in ["", "  ", "secret\r\ninjected: value"] {
            for args in [
                (Some(bad), None, None),
                (None, Some(bad), None),
                (None, None, Some(bad)),
            ] {
                let error = SecurityHeaders::from_settings(args.0, args.1, args.2).unwrap_err();
                assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
                assert!(!error.to_string().contains("secret"));
            }
        }
    }
}
