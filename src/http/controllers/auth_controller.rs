use crate::helpers::csrf::{get_csrf_token, validate_and_regenerate_csrf_token};
use crate::helpers::database::get_connection;
use crate::helpers::form::get_from_form_body;
use crate::helpers::session::{
    clear_session_user, get_session_message, is_authenticated, set_session_user,
};
use crate::helpers::template::load_template;
use crate::models::user::User;
use crate::schema::users::dsl::*;
use crate::services::user_service::{validate_user_credentials, UserValidationResult};
use crate::validation::user_validation::LoginForm;
use crate::AppState;
use actix_session::Session;
use actix_web::{get, post, web, HttpResponse, Responder};
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use validator::Validate;

#[get("/signin")]
pub async fn signin(data: web::Data<AppState>, session: Session) -> impl Responder {
    let app_name = &data.app_name.lock().unwrap();

    if is_authenticated(&session) {
        return HttpResponse::Found()
            .insert_header(("Location", "/admin"))
            .finish();
    }

    let session_message: (String, String) = get_session_message(&session, true);
    let csrf_token = get_csrf_token(&session);

    HttpResponse::Ok().body(load_template(
        "signin.html",
        vec![
            ("name", app_name),
            ("session_message", &*session_message.1),
            ("csrf_token", &csrf_token),
        ],
        None,
    ))
}

#[get("/signout")]
pub async fn signout(session: Session) -> impl Responder {
    clear_session_user(&session);
    session
        .insert("message", "You have been signed out")
        .unwrap();

    HttpResponse::Found()
        .insert_header(("Location", "/"))
        .finish()
}

#[post("/signin")]
pub async fn signin_post(req_body: String, session: Session) -> impl Responder {
    let csrf_token_param: String = get_from_form_body("csrf_token".to_string(), req_body.clone());

    // Validate CSRF token
    if !validate_and_regenerate_csrf_token(&session, &csrf_token_param) {
        session
            .insert(
                "message",
                serde_json::json!({
                    "error": "Invalid request. Please try again.",
                    "success": ""
                }),
            )
            .unwrap();

        return HttpResponse::SeeOther()
            .insert_header(("Location", "/signin"))
            .finish();
    }

    let email_param: String = get_from_form_body("email".to_string(), req_body.clone());
    let password_param: String = get_from_form_body("password".to_string(), req_body);

    let login_form = LoginForm {
        email: email_param.clone(),
        password: password_param.clone(),
    };

    if login_form.validate().is_err() {
        session
            .insert(
                "message",
                serde_json::json!({
                    "error": "Valid email and password are required",
                    "success": ""
                }),
            )
            .unwrap();

        return HttpResponse::SeeOther()
            .insert_header(("Location", "/signin"))
            .finish();
    }

    match validate_user_credentials(&email_param, &password_param) {
        UserValidationResult::InvalidEmail => {
            session
                .insert(
                    "message",
                    serde_json::json!({
                        "error": "Invalid credentials",
                        "success": ""
                    }),
                )
                .unwrap();

            HttpResponse::SeeOther()
                .insert_header(("Location", "/signin"))
                .finish()
        }
        UserValidationResult::InvalidPasswordHash => {
            // Security: Don't log details to avoid information disclosure
            session
                .insert(
                    "message",
                    serde_json::json!({
                        "error": "Invalid credentials",
                        "success": ""
                    }),
                )
                .unwrap();

            HttpResponse::SeeOther()
                .insert_header(("Location", "/signin"))
                .finish()
        }
        UserValidationResult::Invalid => {
            session
                .insert(
                    "message",
                    serde_json::json!({
                        "error": "Invalid credentials",
                        "success": ""
                    }),
                )
                .unwrap();

            HttpResponse::SeeOther()
                .insert_header(("Location", "/signin"))
                .finish()
        }
        UserValidationResult::Valid => {
            let user_results = users
                .filter(email.eq(&email_param))
                .load::<User>(&mut get_connection())
                .expect("Failed to load users");

            let user = user_results.first().unwrap();

            // Cache user data in session to avoid DB queries on subsequent requests
            set_session_user(&session, user);

            HttpResponse::SeeOther()
                .insert_header(("Location", "/admin"))
                .finish()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::database::backend::DbPooledConnection;
    use crate::database::seeders::create_users::UserSeeder;
    use crate::database::seeders::traits::seeder::Seeder;
    use crate::helpers::database::get_connection;
    use crate::helpers::test::TestFinalizer;
    use crate::http::controllers::auth_controller;
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
    async fn test_signin_route() {
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

        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(AppState {
                    app_name: Mutex::from(env::var("APP_NAME").unwrap_or_else(|_| "".to_string())),
                    _user_id: Mutex::from(None),
                }))
                .wrap(SessionMiddleware::new(
                    CookieSessionStore::default(),
                    secret_key.clone(),
                ))
                .service(auth_controller::signin)
                .service(auth_controller::signin_post)
                .service(auth_controller::signout)
                .service(
                    web::resource("/check-data")
                        .route(web::get().to(
                            move |_req: HttpRequest, session: Session| async move {
                                let session_user_id: i32 = session
                                    .get::<i32>("user_id")
                                    .unwrap_or(Some(0))
                                    .unwrap_or(0);
                                if user_id == session_user_id {
                                    HttpResponse::Ok().finish()
                                } else {
                                    HttpResponse::BadRequest().finish()
                                }
                            },
                        ))
                        .wrap(AuthMiddleware),
                ),
        )
        .await;

        // First, get the signin page to obtain CSRF token and session cookie
        let req1 = test::TestRequest::get().uri("/signin").to_request();
        let resp1 = test::call_service(&app, req1).await;
        assert_eq!(resp1.status(), http::StatusCode::OK);

        // Get session cookie from GET /signin
        let headers1 = resp1.headers().clone();
        let cookie_header1 = headers1.get("set-cookie").unwrap().to_str().unwrap();
        let session_cookie = Cookie::parse_encoded(cookie_header1).unwrap();

        // Extract CSRF token from response body
        let body = test::read_body(resp1).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let csrf_token = body_str
            .split("name=\"csrf_token\" value=\"")
            .nth(1)
            .unwrap()
            .split("\"")
            .next()
            .unwrap();

        // Now submit login with CSRF token and session cookie
        let req2 = test::TestRequest::post()
            .uri("/signin")
            .cookie(session_cookie.clone())
            .set_form([
                ("csrf_token", csrf_token),
                ("email", "jekyll@example.com"),
                ("password", "password"),
            ])
            .to_request();
        let resp2 = test::call_service(&app, req2).await;
        assert_eq!(resp2.status(), http::StatusCode::SEE_OTHER);

        // Let's get the cookie from the last request here and repeat it!
        let headers = resp2.headers().clone();
        let cookie_header = headers.get("set-cookie").unwrap().to_str().unwrap();
        let parsed_cookie = Cookie::parse_encoded(cookie_header).unwrap();

        let req3 = test::TestRequest::get()
            .cookie(parsed_cookie.clone())
            .uri("/check-data")
            .to_request();
        let resp3 = test::call_service(&app, req3).await;
        assert_eq!(resp3.status(), http::StatusCode::OK);

        // Now we logout and check if the session is destroyed
        let req4 = test::TestRequest::get()
            .cookie(parsed_cookie)
            .uri("/signout")
            .to_request();
        let resp4 = test::call_service(&app, req4).await;
        assert_eq!(resp4.status(), http::StatusCode::FOUND);

        // Let's get the cookie from the last request here and repeat it!
        let headers = resp4.headers().clone();
        let cookie_header2 = headers.get("set-cookie").unwrap().to_str().unwrap();
        let parsed_cookie2 = Cookie::parse_encoded(cookie_header2).unwrap();

        let req5 = test::TestRequest::get()
            .cookie(parsed_cookie2)
            .uri("/check-data")
            .to_request();
        let resp5 = test::call_service(&app, req5).await;
        assert_eq!(resp5.status(), http::StatusCode::FOUND);
    }
}
