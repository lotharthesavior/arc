use crate::helpers::csrf::{get_csrf_token, validate_csrf_token};
use crate::helpers::database::get_connection;
use crate::helpers::general::gravatar_url;
use crate::helpers::session::get_session_user;
use crate::helpers::template::load_template;
use crate::models::user::User;
use crate::schema::users::dsl::users;
use crate::schema::users::{email, name, password};
use crate::services::user_service::{
    prepare_password, validate_user_credentials, UserValidationResult,
};
use crate::validation::user_validation::{ChangePasswordForm, UpdateProfileForm};
use crate::validation::ValidationError;
use crate::AppState;
use actix_session::Session;
use actix_web::{get, post, web, HttpRequest, HttpResponse, Responder, ResponseError};
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[get("")] // /admin - The Dashboard
pub async fn dashboard(data: web::Data<AppState>, session: Session) -> HttpResponse {
    let user: User = get_session_user(&session).unwrap();
    let app_name = &data.app_name.lock().unwrap();
    let user_avatar = gravatar_url(&user.email);

    HttpResponse::Ok().body(load_template(
        "admin/pages/dashboard.html",
        vec![
            ("name", app_name),
            ("user_name", &user.name),
            ("user_avatar", &user_avatar),
        ],
        None,
    ))
}

#[get("/settings")]
pub async fn settings(
    _req: HttpRequest,
    data: web::Data<AppState>,
    session: Session,
) -> impl Responder {
    let user: User = get_session_user(&session).unwrap();
    let app_name = &data.app_name.lock().unwrap();
    let user_avatar = gravatar_url(&user.email);

    HttpResponse::Ok().body(load_template(
        "admin/pages/settings.html",
        vec![
            ("name", app_name),
            ("user_name", &user.name),
            ("user_avatar", &user_avatar),
        ],
        None,
    ))
}

#[get("/profile")]
pub async fn profile(
    _req: HttpRequest,
    data: web::Data<AppState>,
    session: Session,
) -> impl Responder {
    let user: User = get_session_user(&session).unwrap();
    let app_name = &data.app_name.lock().unwrap();
    let user_name: String = user.name;
    let user_email: String = user.email;
    let user_avatar = gravatar_url(&user_email);
    let csrf_token = get_csrf_token(&session);

    HttpResponse::Ok().body(load_template(
        "admin/pages/profile.html",
        vec![
            ("name", app_name),
            ("user_name", &user_name),
            ("user_email", &user_email),
            ("user_avatar", &user_avatar),
            ("csrf_token", &csrf_token),
        ],
        None,
    ))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserForm {
    csrf_token: String,
    name: String,
    email: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PasswordForm {
    csrf_token: String,
    current_email: String,
    old_password: String,
    new_password: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserResponseData {
    name: String,
    email: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProfileResponse {
    data: UserResponseData,
}

#[post("/profile")]
pub async fn profile_post(form: web::Form<UserForm>, session: Session) -> impl Responder {
    // Validate CSRF token
    if !validate_csrf_token(&session, &form.csrf_token) {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"errors": {"csrf": "Invalid request. Please refresh and try again."}}));
    }

    let profile_form = UpdateProfileForm {
        name: form.name.clone(),
        email: form.email.clone(),
    };

    if let Err(errors) = profile_form.validate() {
        return ValidationError::from(errors).error_response();
    }

    let user: User = get_session_user(&session).unwrap();

    let user_name: String = form.name.clone();
    let user_email: String = form.email.clone();

    let result = diesel::update(users.find(user.id))
        .set((email.eq(user_email.clone()), name.eq(user_name.clone())))
        .execute(&mut get_connection())
        .unwrap();

    if result == 0 {
        return HttpResponse::InternalServerError()
            .json(serde_json::json!({"errors": {"server_error": "Failed to update user"}}));
    }

    let obj = ProfileResponse {
        data: UserResponseData {
            name: user_name.to_string(),
            email: user_email.to_string(),
        },
    };

    HttpResponse::Ok().json(obj)
}

#[post("/profile-password")]
pub async fn profile_password_post(
    form: web::Form<PasswordForm>,
    session: Session,
) -> impl Responder {
    // Validate CSRF token
    if !validate_csrf_token(&session, &form.csrf_token) {
        return HttpResponse::Forbidden()
            .json(serde_json::json!({"errors": {"csrf": "Invalid request. Please refresh and try again."}}));
    }

    let password_form = ChangePasswordForm {
        current_password: form.old_password.clone(),
        new_password: form.new_password.clone(),
        confirm_password: form.new_password.clone(),
    };

    if let Err(errors) = password_form.validate() {
        return ValidationError::from(errors).error_response();
    }

    let user: User = get_session_user(&session).unwrap();

    let current_email: String = form.current_email.clone();
    let old_password: String = form.old_password.clone();
    let new_password: String = form.new_password.clone();

    let user_validation_result = validate_user_credentials(&current_email, &old_password);

    if user_validation_result != UserValidationResult::Valid {
        return HttpResponse::InternalServerError()
            .json(serde_json::json!({"errors": {"server_error": "Invalid credentials"}}));
    }

    let result = diesel::update(users.find(user.id))
        .set(password.eq(prepare_password(&new_password)))
        .execute(&mut get_connection())
        .unwrap();

    if result == 0 {
        return HttpResponse::InternalServerError()
            .json(serde_json::json!({"errors": {"server_error": "Failed to update user"}}));
    }

    HttpResponse::Ok().json(serde_json::json!({"success": "Password updated"}))
}

#[cfg(test)]
mod tests {
    use crate::database::backend::DbPooledConnection;
    use crate::database::seeders::create_users::UserSeeder;
    use crate::database::seeders::traits::seeder::Seeder;
    use crate::helpers::database::get_connection;
    use crate::helpers::test::TestFinalizer;
    use crate::http::controllers::{admin_controller, auth_controller};
    use crate::http::middlewares::auth_middleware::AuthMiddleware;
    use crate::models::user::{User, MIGRATIONS};
    use crate::schema::users::dsl::*;
    use crate::services::user_service::{validate_user_credentials, UserValidationResult};
    use crate::AppState;
    use actix_session::storage::CookieSessionStore;
    use actix_session::SessionMiddleware;
    use actix_web::cookie::{Cookie, Key};
    use actix_web::{http, test, web, App};
    use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
    use diesel_migrations::MigrationHarness;
    use serial_test::serial;
    use std::env;
    use std::sync::Mutex;

    fn prepare_test_db() -> DbPooledConnection {
        dotenv::from_filename(".env.test").ok();
        let mut conn: DbPooledConnection = get_connection();
        conn.revert_all_migrations(MIGRATIONS)
            .expect("Failed to revert migrations");
        conn.run_pending_migrations(MIGRATIONS)
            .expect("Failed to run migrations");

        conn
    }

    fn seed_users_table() {
        let mut conn: DbPooledConnection = prepare_test_db();
        UserSeeder::execute(&mut conn).expect("Failed to seed users table");
    }

    /// Helper to extract CSRF token from HTML response body
    fn extract_csrf_token(body_str: &str) -> String {
        body_str
            .split("name=\"csrf_token\" value=\"")
            .nth(1)
            .unwrap()
            .split("\"")
            .next()
            .unwrap()
            .to_string()
    }

    #[serial]
    #[actix_web::test]
    async fn test_dashboard() {
        let _finalizer = TestFinalizer;

        seed_users_table();

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
                .service(
                    web::scope("/admin")
                        .service(admin_controller::dashboard)
                        .service(admin_controller::settings)
                        .wrap(AuthMiddleware),
                ),
        )
        .await;

        // Test that unauthenticated access redirects
        let req1 = test::TestRequest::get().uri("/admin").to_request();
        let resp1 = test::call_service(&app, req1).await;
        assert_eq!(resp1.status(), http::StatusCode::FOUND);

        // Get signin page to obtain CSRF token
        let req_signin = test::TestRequest::get().uri("/signin").to_request();
        let resp_signin = test::call_service(&app, req_signin).await;
        let headers_signin = resp_signin.headers().clone();
        let cookie_header_signin = headers_signin.get("set-cookie").unwrap().to_str().unwrap();
        let session_cookie = Cookie::parse_encoded(cookie_header_signin)
            .unwrap()
            .into_owned();
        let body = test::read_body(resp_signin).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let csrf_token = extract_csrf_token(&body_str);

        // Login with CSRF token
        let req_login = test::TestRequest::post()
            .uri("/signin")
            .cookie(session_cookie.clone())
            .set_form([
                ("csrf_token", csrf_token.as_str()),
                ("email", "jekyll@example.com"),
                ("password", "password"),
            ])
            .to_request();
        let resp_login = test::call_service(&app, req_login).await;
        let headers = resp_login.headers().clone();
        let cookie_header = headers.get("set-cookie").unwrap().to_str().unwrap();
        let parsed_cookie = Cookie::parse_encoded(cookie_header).unwrap().into_owned();

        // Now access dashboard with session
        let req3 = test::TestRequest::get()
            .cookie(parsed_cookie)
            .uri("/admin")
            .to_request();
        let resp3 = test::call_service(&app, req3).await;
        assert_eq!(resp3.status(), http::StatusCode::OK);
    }

    #[serial]
    #[actix_web::test]
    async fn test_settings() {
        let _finalizer = TestFinalizer;

        seed_users_table();

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
                .service(
                    web::scope("/admin")
                        .service(admin_controller::dashboard)
                        .service(admin_controller::settings)
                        .wrap(AuthMiddleware),
                ),
        )
        .await;

        // Test that unauthenticated access redirects
        let req1 = test::TestRequest::get().uri("/admin/settings").to_request();
        let resp1 = test::call_service(&app, req1).await;
        assert_eq!(resp1.status(), http::StatusCode::FOUND);

        // Get signin page to obtain CSRF token
        let req_signin = test::TestRequest::get().uri("/signin").to_request();
        let resp_signin = test::call_service(&app, req_signin).await;
        let headers_signin = resp_signin.headers().clone();
        let cookie_header_signin = headers_signin.get("set-cookie").unwrap().to_str().unwrap();
        let session_cookie = Cookie::parse_encoded(cookie_header_signin)
            .unwrap()
            .into_owned();
        let body = test::read_body(resp_signin).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let csrf_token = extract_csrf_token(&body_str);

        // Login with CSRF token
        let req_login = test::TestRequest::post()
            .uri("/signin")
            .cookie(session_cookie.clone())
            .set_form([
                ("csrf_token", csrf_token.as_str()),
                ("email", "jekyll@example.com"),
                ("password", "password"),
            ])
            .to_request();
        let resp_login = test::call_service(&app, req_login).await;
        let headers = resp_login.headers().clone();
        let cookie_header = headers.get("set-cookie").unwrap().to_str().unwrap();
        let parsed_cookie = Cookie::parse_encoded(cookie_header).unwrap().into_owned();

        // Now access settings with session
        let req3 = test::TestRequest::get()
            .cookie(parsed_cookie)
            .uri("/admin/settings")
            .to_request();
        let resp3 = test::call_service(&app, req3).await;
        assert_eq!(resp3.status(), http::StatusCode::OK);
    }

    #[serial]
    #[actix_web::test]
    async fn test_profile_data() {
        let _finalizer = TestFinalizer;

        seed_users_table();

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
                .service(
                    web::scope("/admin")
                        .service(admin_controller::profile)
                        .service(admin_controller::profile_post)
                        .wrap(AuthMiddleware),
                ),
        )
        .await;

        // Get signin page to obtain CSRF token
        let req_signin = test::TestRequest::get().uri("/signin").to_request();
        let resp_signin = test::call_service(&app, req_signin).await;
        let headers_signin = resp_signin.headers().clone();
        let cookie_header_signin = headers_signin.get("set-cookie").unwrap().to_str().unwrap();
        let session_cookie = Cookie::parse_encoded(cookie_header_signin)
            .unwrap()
            .into_owned();
        let body = test::read_body(resp_signin).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let csrf_token = extract_csrf_token(&body_str);

        // Login with CSRF token
        let req_login = test::TestRequest::post()
            .uri("/signin")
            .cookie(session_cookie.clone())
            .set_form([
                ("csrf_token", csrf_token.as_str()),
                ("email", "jekyll@example.com"),
                ("password", "password"),
            ])
            .to_request();
        let resp_login = test::call_service(&app, req_login).await;
        let headers = resp_login.headers().clone();
        let cookie_header = headers.get("set-cookie").unwrap().to_str().unwrap();
        let parsed_cookie = Cookie::parse_encoded(cookie_header).unwrap().into_owned();

        // Get profile page to obtain CSRF token
        let req3 = test::TestRequest::get()
            .cookie(parsed_cookie.clone())
            .uri("/admin/profile")
            .to_request();
        let resp3 = test::call_service(&app, req3).await;
        assert_eq!(resp3.status(), http::StatusCode::OK);

        // Extract CSRF token from profile page
        let body = test::read_body(resp3).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let csrf_token = body_str
            .split("name=\"csrf_token\" value=\"")
            .nth(1)
            .unwrap()
            .split("\"")
            .next()
            .unwrap();

        let new_email = "hyde@example.com";

        // Submit profile update with CSRF token
        let req4 = test::TestRequest::post()
            .cookie(parsed_cookie)
            .uri("/admin/profile")
            .set_form([
                ("csrf_token", csrf_token),
                ("name", "Hyde"),
                ("email", new_email),
            ])
            .to_request();
        let resp4 = test::call_service(&app, req4).await;
        assert_eq!(resp4.status(), http::StatusCode::OK);

        let user = users
            .filter(email.eq(new_email))
            .load::<User>(&mut get_connection())
            .expect("Failed to load users");

        assert_eq!(user.len(), 1);
    }

    #[serial]
    #[actix_web::test]
    async fn test_profile_password() {
        let _finalizer = TestFinalizer;

        seed_users_table();

        let user_email = "jekyll@example.com";

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
                .service(
                    web::scope("/admin")
                        .service(admin_controller::profile)
                        .service(admin_controller::profile_password_post)
                        .wrap(AuthMiddleware),
                ),
        )
        .await;

        // Get signin page to obtain CSRF token
        let req_signin = test::TestRequest::get().uri("/signin").to_request();
        let resp_signin = test::call_service(&app, req_signin).await;
        let headers_signin = resp_signin.headers().clone();
        let cookie_header_signin = headers_signin.get("set-cookie").unwrap().to_str().unwrap();
        let session_cookie = Cookie::parse_encoded(cookie_header_signin)
            .unwrap()
            .into_owned();
        let body = test::read_body(resp_signin).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let csrf_token = extract_csrf_token(&body_str);

        // Login with CSRF token
        let req_login = test::TestRequest::post()
            .uri("/signin")
            .cookie(session_cookie.clone())
            .set_form([
                ("csrf_token", csrf_token.as_str()),
                ("email", "jekyll@example.com"),
                ("password", "password"),
            ])
            .to_request();
        let resp_login = test::call_service(&app, req_login).await;
        let headers = resp_login.headers().clone();
        let cookie_header = headers.get("set-cookie").unwrap().to_str().unwrap();
        let parsed_cookie = Cookie::parse_encoded(cookie_header).unwrap().into_owned();

        // Get profile page to obtain CSRF token
        let req3 = test::TestRequest::get()
            .cookie(parsed_cookie.clone())
            .uri("/admin/profile")
            .to_request();
        let resp3 = test::call_service(&app, req3).await;
        assert_eq!(resp3.status(), http::StatusCode::OK);

        // Extract CSRF token from profile page
        let body = test::read_body(resp3).await;
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        let csrf_token = body_str
            .split("name=\"csrf_token\" value=\"")
            .nth(1)
            .unwrap()
            .split("\"")
            .next()
            .unwrap();

        // Submit password change with CSRF token
        let req4 = test::TestRequest::post()
            .cookie(parsed_cookie)
            .uri("/admin/profile-password")
            .set_form([
                ("csrf_token", csrf_token),
                ("current_email", user_email),
                ("old_password", "password"),
                ("new_password", "SecurePass123"),
            ])
            .to_request();
        let resp4 = test::call_service(&app, req4).await;
        assert_eq!(resp4.status(), http::StatusCode::OK);
        assert_eq!(
            validate_user_credentials(user_email, "SecurePass123"),
            UserValidationResult::Valid
        );
    }
}
