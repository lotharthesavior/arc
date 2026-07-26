//! Integration test for the framework `AuthMiddleware`, exercised against the
//! application's concrete `UserAggregate` stack. The middleware itself lives in
//! `arc-web`; this test lives here because it needs an application aggregate to
//! seed a session-bound user.

use crate::helpers::session::{set_session_user, SessionUser};
use crate::helpers::test::{es::build_stack_with_default_user, InMemoryTestGuard};
use actix_session::storage::CookieSessionStore;
use actix_session::{Session, SessionMiddleware};
use actix_web::cookie::{Cookie, Key};
use actix_web::{http, test, web, App, HttpRequest, HttpResponse};
use arc_web::http::middlewares::auth_middleware::AuthMiddleware;
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

    let app = test::init_service(
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
            .service(
                web::resource("/check-data")
                    .wrap(AuthMiddleware)
                    .route(web::get().to({
                        let expected_id = agg_id.clone();
                        move |_req: HttpRequest, session: Session| {
                            let expected_id = expected_id.clone();
                            async move {
                                match session.get::<SessionUser>("user").ok().flatten() {
                                    Some(u) if u.id == expected_id => HttpResponse::Ok().finish(),
                                    _ => HttpResponse::BadRequest().finish(),
                                }
                            }
                        }
                    })),
            ),
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
