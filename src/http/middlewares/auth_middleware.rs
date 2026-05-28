use crate::helpers::session::is_authenticated;
use actix_session::SessionExt;
use actix_web::body::EitherBody;
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};

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
    use crate::database::backend::DbPooledConnection;
    use crate::database::seeders::create_users::UserSeeder;
    use crate::database::seeders::traits::seeder::Seeder;
    use crate::helpers::database::get_connection;
    use crate::helpers::test::TestFinalizer;
    use crate::http::middlewares::auth_middleware::AuthMiddleware;
    use crate::models::user::MIGRATIONS;
    use crate::schema::users::dsl::users;
    use crate::schema::users::id;
    use crate::AppState;
    use actix_session::storage::CookieSessionStore;
    use actix_session::{Session, SessionMiddleware};
    use actix_web::cookie::{Cookie, Key};
    use actix_web::{http, test, web, App, HttpRequest, HttpResponse};
    use diesel::{QueryDsl, RunQueryDsl};
    use diesel_migrations::MigrationHarness;
    use serial_test::serial;
    use std::env;
    use std::sync::Mutex;

    fn prepare_test_db() -> DbPooledConnection {
        dotenv::from_filename(".env.test").ok();
        let mut conn: DbPooledConnection = get_connection();
        conn.run_pending_migrations(MIGRATIONS)
            .expect("Failed to run migrations");

        conn
    }

    fn seed_users_table() {
        let mut conn: DbPooledConnection = prepare_test_db();
        UserSeeder::execute(&mut conn).expect("Failed to seed users table");
    }

    #[serial]
    #[actix_web::test]
    async fn test_auth_middleware() {
        let _finalizer = TestFinalizer;

        let mut conn: DbPooledConnection = prepare_test_db();
        seed_users_table();
        let all_users: Vec<i32> = users.select(id).load::<i32>(&mut conn).unwrap();
        let user_id: i32 = all_users[0];

        let secret_key = Key::from(
            env::var("SECRET_KEY")
                .expect("SECRET_KEY must be set")
                .as_bytes(),
        );

        let app =
            test::init_service(
                App::new()
                    .app_data(web::Data::new(AppState {
                        app_name: Mutex::from(
                            env::var("APP_NAME").unwrap_or_else(|_| "".to_string()),
                        ),
                        _user_id: Mutex::from(None),
                    }))
                    .wrap(SessionMiddleware::new(
                        CookieSessionStore::default(),
                        secret_key.clone(),
                    ))
                    .service(web::resource("/force-auth").route(web::get().to(
                        move |_req: HttpRequest, session: Session| async move {
                            session.insert("user_id", user_id).unwrap();
                            HttpResponse::Ok().finish()
                        },
                    )))
                    .service(web::resource("/check-data").wrap(AuthMiddleware).route(
                        web::get().to(move |_req: HttpRequest, session: Session| async move {
                            let session_user_id: i32 = session
                                .get::<i32>("user_id")
                                .unwrap_or(Some(0))
                                .unwrap_or(0);
                            if user_id == session_user_id {
                                HttpResponse::Ok().finish()
                            } else {
                                HttpResponse::BadRequest().finish()
                            }
                        }),
                    )),
            )
            .await;

        let req1 = test::TestRequest::get().uri("/force-auth").to_request();
        let resp1 = test::call_service(&app, req1).await;
        assert_eq!(resp1.status(), http::StatusCode::OK);

        // Let's get the cookie from the last request here and repeat it!
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
