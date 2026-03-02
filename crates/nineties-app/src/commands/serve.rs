use crate::helpers::rate_limit;
use crate::http::middlewares::rate_limit_middleware::GlobalRateLimit;
use crate::routes;
use crate::websocket::server::WsServer;
use crate::AppState;
use actix::prelude::*;
use actix_session::storage::CookieSessionStore;
use actix_session::{SessionMiddleware, config::PersistentSession};
use actix_web::cookie::{Key, SameSite, time::Duration};
use actix_web::middleware::{Compress, NormalizePath};
use actix_web::{web, App, HttpServer};
use std::env;
use std::io;
use std::sync::Mutex;
use tracing::{info, warn};

/// Starts the Actix-Web HTTP server with all middleware, session management,
/// rate limiting, compression, and route configuration.
pub async fn run(app_url: String, app_port: u16) -> io::Result<()> {
    crate::check_database_health();

    let secret_key = Key::from(
        env::var("SECRET_KEY")
            .expect("SECRET_KEY must be set")
            .as_bytes(),
    );

    // Determine environment and configure session security accordingly
    let app_env = env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
    let is_production = app_env == "production";

    // Configure session cookie domain
    let session_domain = env::var("SESSION_DOMAIN").ok();

    // Configure SameSite policy
    let same_site_str = env::var("SESSION_SAME_SITE").unwrap_or_else(|_| "Lax".to_string());
    let same_site = match same_site_str.as_str() {
        "Strict" => SameSite::Strict,
        "None" => SameSite::None,
        _ => SameSite::Lax,
    };

    info!(
        environment = app_env,
        secure = is_production,
        same_site = ?same_site,
        domain = ?session_domain,
        "Configuring session middleware"
    );

    if !is_production && app_url == "0.0.0.0" {
        warn!(
            "Running in development mode with 0.0.0.0 - sessions will work across network. \
            Ensure APP_ENV=production for production deployments!"
        );
    }

    let ws_server = WsServer::new().start();

    // Create rate limiters - one for login endpoints, one for global middleware
    let login_rate_limiter = rate_limit::create_rate_limiter();         // 5 requests per 60s
    let global_rate_limiter = rate_limit::create_global_rate_limiter(); // 100 requests per 60s

    HttpServer::new(move || {
        // Build session middleware with proper cookie configuration
        let mut session_middleware = SessionMiddleware::builder(
            CookieSessionStore::default(),
            secret_key.clone(),
        )
        .cookie_name("nineties_session".to_string())
        .cookie_http_only(true)
        .cookie_same_site(same_site)
        .session_lifecycle(
            PersistentSession::default()
                .session_ttl(Duration::hours(24))
        );

        // In production, enforce secure cookies (HTTPS only)
        if is_production {
            session_middleware = session_middleware.cookie_secure(true);
        } else {
            // In development, allow non-HTTPS for easier local testing
            session_middleware = session_middleware.cookie_secure(false);
        }

        // Set cookie domain if specified
        if let Some(ref domain) = session_domain {
            session_middleware = session_middleware.cookie_domain(Some(domain.clone()));
        }
        // If no domain is set, cookie will work for any host (good for dev with IP access)

        App::new()
            .wrap(tracing_actix_web::TracingLogger::default())
            .wrap(GlobalRateLimit)
            .wrap(Compress::default())
            .wrap(session_middleware.build())
            .wrap(NormalizePath::trim())
            .app_data(web::Data::new(global_rate_limiter.clone()))  // Global middleware uses this
            .app_data(web::Data::new(login_rate_limiter.clone()))   // Login controllers use this
            .app_data(web::Data::new(AppState {
                app_name: Mutex::from(env::var("APP_NAME").unwrap_or_else(|_| "".to_string())),
                _user_id: Mutex::from(None),
            }))
            .app_data(web::Data::new(ws_server.clone()))
            .configure(routes::config)
    })
    .bind((app_url, app_port))?
    .run()
    .await
}
