use crate::helpers::session::is_authenticated;
use actix_session::SessionExt;
use actix_web::body::EitherBody;
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};

/// Session-based authentication middleware. Redirects unauthenticated requests to `/signin`.
/// Checks for cached user data in the session to avoid database queries on every request.
pub struct AuthMiddleware;

impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthCheck<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthCheck { service }))
    }
}

/// Inner service wrapper created by [`AuthMiddleware`].
pub struct AuthCheck<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for AuthCheck<S>
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

        // Use cached session check (no DB query if user_data exists in session)
        if !is_authenticated(&session) {
            return Box::pin(async move {
                Ok(req.into_response(
                    HttpResponse::Found()
                        .insert_header(("Location", "/signin"))
                        .finish()
                        .map_into_right_body(),
                ))
            });
        }

        let res: <S as Service<ServiceRequest>>::Future = self.service.call(req);
        Box::pin(async move { res.await.map(ServiceResponse::map_into_left_body) })
    }
}

#[cfg(test)]
mod tests {
    use crate::helpers::session::{set_session_user, SessionUser};
    use crate::helpers::test::{es::build_stack_with_default_user, InMemoryTestGuard};
    use crate::http::middlewares::auth_middleware::AuthMiddleware;
    use actix_session::storage::CookieSessionStore;
    use actix_session::{Session, SessionMiddleware};
    use actix_web::cookie::{Cookie, Key};
    use actix_web::{http, test, web, App, HttpRequest, HttpResponse};
    use serial_test::serial;
    use std::env;

    #[serial]
    #[actix_web::test]
    async fn test_auth_middleware() {
        let _guard = InMemoryTestGuard;
        let stack = build_stack_with_default_user().await;
        let agg_id = stack.seeded_user_id.clone().unwrap();

        let secret_key = Key::from(
            env::var("SECRET_KEY")
                .expect("SECRET_KEY must be set")
                .as_bytes(),
        );

        let app =
            test::init_service(
                App::new()
                    .wrap(SessionMiddleware::new(
                        CookieSessionStore::default(),
                        secret_key.clone(),
                    ))
                    .service(web::resource("/force-auth").route(web::get().to({
                        let agg_id = agg_id.clone();
                        move |_req: HttpRequest, session: Session| {
                            let agg_id = agg_id.clone();
                            async move {
                                set_session_user(
                                    &session,
                                    &SessionUser {
                                        id: agg_id,
                                        name: "Jekyll".into(),
                                        email: "jekyll@example.com".into(),
                                    },
                                );
                                HttpResponse::Ok().finish()
                            }
                        }
                    })))
                    .service(web::resource("/check-data").wrap(AuthMiddleware).route(
                        web::get().to({
                            let expected_id = agg_id.clone();
                            move |_req: HttpRequest, session: Session| {
                                let expected_id = expected_id.clone();
                                async move {
                                    match session.get::<SessionUser>("user").ok().flatten() {
                                        Some(u) if u.id == expected_id => {
                                            HttpResponse::Ok().finish()
                                        }
                                        _ => HttpResponse::BadRequest().finish(),
                                    }
                                }
                            }
                        }),
                    )),
            )
            .await;

        let req1 = test::TestRequest::get().uri("/force-auth").to_request();
        let resp1 = test::call_service(&app, req1).await;
        assert_eq!(resp1.status(), http::StatusCode::OK);

        let headers = resp1.headers().clone();
        let cookie_header = headers.get("set-cookie").unwrap().to_str().unwrap();
        let parsed_cookie = Cookie::parse_encoded(cookie_header).unwrap();

        let req2 = test::TestRequest::get()
            .cookie(parsed_cookie)
            .uri("/check-data")
            .to_request();
        let resp2 = test::call_service(&app, req2).await;
        assert_eq!(resp2.status(), http::StatusCode::OK);
    }
}
