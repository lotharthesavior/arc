use crate::helpers::database::get_connection;
use crate::helpers::jwt::create_token;
use crate::helpers::rate_limit::LoginRateLimiter;
use crate::models::user::User;
use crate::schema::users::dsl::*;
use crate::services::user_service::{validate_user_credentials, UserValidationResult};
use actix_web::{get, post, web, web::Json, HttpMessage, HttpRequest, HttpResponse, Responder};
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use serde::Deserialize;
use serde_json::json;
use tracing::warn;

/// JSON request body for API login.
#[derive(Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
}

/// API login endpoint. Validates credentials and returns a JWT token on success.
/// Rate-limited per IP address.
#[post("/login")]
pub async fn login(
    http_req: HttpRequest,
    req: Json<LoginRequest>,
    limiter: web::Data<LoginRateLimiter>,
) -> impl Responder {
    // Check rate limit using IP address as key
    let ip = http_req
        .connection_info()
        .realip_remote_addr()
        .unwrap_or("unknown")
        .to_string();
    let key = format!("api_login:{}", ip);

    match limiter.check(key.clone()) {
        Ok(()) => {} // Rate limit not exceeded
        Err(retry_after) => {
            warn!(
                ip = ip,
                path = http_req.path(),
                "Rate limit exceeded on API login attempt"
            );

            return HttpResponse::TooManyRequests()
                .insert_header(("Retry-After", retry_after.as_secs().to_string()))
                .json(json!({"error": "Too many login attempts. Please try again later."}));
        }
    }

    match validate_user_credentials(&req.email, &req.password) {
        UserValidationResult::Valid => {
            let conn = &mut get_connection();
            let user_vec: Vec<User> = users
                .filter(email.eq(&req.email))
                .load(conn)
                .expect("Failed to load user");
            if let Some(user) = user_vec.first() {
                match create_token(user.id) {
                    Ok(token) => HttpResponse::Ok().json(json!({"token": token})),
                    Err(_) => HttpResponse::InternalServerError()
                        .json(json!({"error": "Failed to generate token"})),
                }
            } else {
                HttpResponse::Unauthorized().json(json!({"error": "User not found"}))
            }
        }
        _ => HttpResponse::Unauthorized().json(json!({"error": "Invalid credentials"})),
    }
}

/// Returns the authenticated user's profile as JSON. Requires a valid JWT token
/// (user ID is extracted by the JWT middleware).
#[get("/profile")]
pub async fn profile(req: HttpRequest) -> impl Responder {
    if let Some(&user_id) = req.extensions().get::<i32>() {
        let conn = &mut get_connection();
        match users.find(user_id).first::<User>(conn) {
            Ok(user) => HttpResponse::Ok().json(&user),
            Err(_) => HttpResponse::NotFound().json(json!({"error": "User not found"})),
        }
    } else {
        HttpResponse::Unauthorized().json(json!({"error": "No authenticated user"}))
    }
}

#[cfg(test)]
mod tests {
    use crate::database::backend::DbPooledConnection;
    use crate::database::seeders::create_users::UserSeeder;
    use crate::database::seeders::traits::seeder::Seeder;
    use crate::helpers::database::get_connection;
    use crate::helpers::test::TestFinalizer;
    use crate::models::user::MIGRATIONS;
    use crate::routes;
    use actix_web::{http, test, App};
    use diesel_migrations::MigrationHarness;
    use serde_json::Value;
    use serial_test::serial;
    use std::env;

    struct EnvGuard {
        values: Vec<(&'static str, Option<String>)>,
    }

    impl EnvGuard {
        fn set(pairs: &[(&'static str, &str)]) -> Self {
            let mut values = Vec::with_capacity(pairs.len());

            for (key, value) in pairs {
                values.push((*key, env::var(key).ok()));
                env::set_var(key, value);
            }

            EnvGuard { values }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (key, value) in self.values.drain(..) {
                if let Some(value) = value {
                    env::set_var(key, value);
                } else {
                    env::remove_var(key);
                }
            }
        }
    }

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
    async fn test_api_routes_not_registered_when_jwt_disabled() {
        let _finalizer = TestFinalizer;
        let _env = EnvGuard::set(&[
            ("ENABLE_JWT_AUTH", "false"),
            ("DATABASE_URL", "database/database-test.sqlite"),
        ]);

        let app = test::init_service(App::new().configure(routes::config)).await;

        let req = test::TestRequest::post()
            .uri("/api/login")
            .set_json(serde_json::json!({
                "email": "jekyll@example.com",
                "password": "password"
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::NOT_FOUND);
    }

    #[serial]
    #[actix_web::test]
    async fn test_api_login_and_profile_when_jwt_enabled() {
        let _finalizer = TestFinalizer;
        let _env = EnvGuard::set(&[
            ("ENABLE_JWT_AUTH", "true"),
            ("JWT_SECRET", "test-jwt-secret-at-least-32-characters-long"),
            ("JWT_EXPIRY_HOURS", "24"),
            ("DATABASE_URL", "database/database-test.sqlite"),
        ]);

        seed_users_table();

        let rate_limiter = crate::helpers::rate_limit::LoginRateLimiter(
            crate::helpers::rate_limit::RateLimiter::new(100, std::time::Duration::from_secs(60)),
        );
        let app = test::init_service(
            App::new()
                .app_data(actix_web::web::Data::new(rate_limiter))
                .configure(routes::config),
        )
        .await;

        let login_req = test::TestRequest::post()
            .uri("/api/login")
            .set_json(serde_json::json!({
                "email": "jekyll@example.com",
                "password": "password"
            }))
            .to_request();
        let login_resp = test::call_service(&app, login_req).await;

        assert_eq!(login_resp.status(), http::StatusCode::OK);

        let login_body: Value = test::read_body_json(login_resp).await;
        let token = login_body["token"]
            .as_str()
            .expect("JWT token should be returned");
        assert!(!token.is_empty());

        let profile_req = test::TestRequest::get()
            .uri("/api/protected/profile")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .to_request();
        let profile_resp = test::call_service(&app, profile_req).await;

        assert_eq!(profile_resp.status(), http::StatusCode::OK);

        let profile_body: Value = test::read_body_json(profile_resp).await;
        assert_eq!(profile_body["email"], "jekyll@example.com");
        assert!(profile_body.get("password").is_none());
    }

    #[serial]
    #[actix_web::test]
    async fn test_api_profile_rejects_missing_bearer_token() {
        let _finalizer = TestFinalizer;
        let _env = EnvGuard::set(&[
            ("ENABLE_JWT_AUTH", "true"),
            ("JWT_SECRET", "test-jwt-secret-at-least-32-characters-long"),
            ("JWT_EXPIRY_HOURS", "24"),
            ("DATABASE_URL", "database/database-test.sqlite"),
        ]);

        let app = test::init_service(App::new().configure(routes::config)).await;

        let req = test::TestRequest::get()
            .uri("/api/protected/profile")
            .to_request();
        let resp = test::call_service(&app, req).await;

        assert_eq!(resp.status(), http::StatusCode::UNAUTHORIZED);
    }
}
