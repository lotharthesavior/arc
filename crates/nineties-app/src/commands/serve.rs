use crate::helpers::rate_limit;
use crate::http::middlewares::rate_limit_middleware::GlobalRateLimit;
use crate::routes;
use crate::websocket::server::WsServer;
use crate::AppState;
use actix::prelude::*;
use actix_session::storage::CookieSessionStore;
use actix_session::{config::PersistentSession, SessionMiddleware};
use actix_web::cookie::{time::Duration, Key, SameSite};
use actix_web::middleware::{Compress, NormalizePath};
use actix_web::{web, App, HttpServer};
use std::env;
use std::io;
use std::sync::Mutex;
use tracing::{info, warn};

use crate::domain::user::aggregate::UserAggregate;
use crate::domain::user::projector::{UserProjector, USERS_VIEW};
use nineties_core::access_log::{AccessLogger, NoOpAccessLogger};
use nineties_core::command_bus::CommandBus;
use nineties_core::event_bus::{EventBus, InProcessEventBus};
use nineties_core::projection::{ProjectionEngine, ProjectionEngineHandler};
use nineties_core::read_model_store::ReadModelStore;
use nineties_core::session::SessionStore;
use nineties_es_sqlite::{SqliteEventStore, SqliteReadModelStore, SqliteSessionStore};
use std::sync::Arc;

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
    let login_rate_limiter = rate_limit::create_rate_limiter(); // 5 requests per 60s
    let global_rate_limiter = rate_limit::create_global_rate_limiter(); // 100 requests per 60s

    // Set up Event Sourced CQRS
    let db_url = crate::helpers::config::database_url();

    let sqlite_event_store = SqliteEventStore::new(&db_url)
        .await
        .expect("Failed to init event store");

    // Read-model store + projection engine. The engine subscribes to the
    // in-process event bus through a thin adapter so every committed event
    // drives `UserProjector` synchronously into `users_view`. Step 3 will
    // split this lane out as JetStream consumers move into a worker.
    let read_model_store: Arc<dyn ReadModelStore> = Arc::new(
        SqliteReadModelStore::new(&db_url)
            .await
            .expect("Failed to init read-model store"),
    );
    let mut projection_engine = ProjectionEngine::new(Box::new(sqlite_event_store.clone()));
    projection_engine.register_projector(
        Box::new(UserProjector::new()),
        read_model_store.clone(),
        USERS_VIEW,
    );
    let projection_engine = Arc::new(projection_engine);

    let mut event_bus = InProcessEventBus::new();
    event_bus
        .subscribe(Box::new(ProjectionEngineHandler::new(
            projection_engine.clone(),
        )))
        .await
        .expect("Failed to subscribe ProjectionEngine to event bus");

    // Backfill the read model from the event store on every start. Cheap on
    // SQLite, idempotent under the version-gated upsert, and removes the need
    // for a separate one-shot CLI when an operator is recovering from a
    // truncated read model. Step 4's worker will own this for distributed
    // deployments.
    if let Err(e) = projection_engine.rebuild_all().await {
        tracing::error!(error = ?e, "ProjectionEngine.rebuild_all failed at startup");
    } else {
        info!("Projections rebuilt from event store");
    }

    let command_bus =
        CommandBus::<UserAggregate>::new(Box::new(sqlite_event_store.clone()), Box::new(event_bus));
    let command_bus_data = web::Data::new(command_bus);
    let read_model_store_data = web::Data::from(read_model_store);

    // Default to NoOpAccessLogger for non-regulated deployments. Production
    // PHI/PCI deployments swap this for a JetStream- or DB-backed sink (Step 3+).
    let access_logger: Arc<dyn AccessLogger> = Arc::new(NoOpAccessLogger);
    let access_logger_data = web::Data::from(access_logger);

    // HIPAA-4 server-side JWT session registry.
    let session_store_impl = SqliteSessionStore::new(&db_url)
        .await
        .expect("Failed to init session store");
    let session_store: Arc<dyn SessionStore> = Arc::new(session_store_impl);
    let session_store_data = web::Data::from(session_store);

    HttpServer::new(move || {
        // Build session middleware with proper cookie configuration
        let mut session_middleware =
            SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                .cookie_name("nineties_session".to_string())
                .cookie_http_only(true)
                .cookie_same_site(same_site)
                .session_lifecycle(PersistentSession::default().session_ttl(Duration::hours(24)));

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
            .app_data(web::Data::new(global_rate_limiter.clone())) // Global middleware uses this
            .app_data(web::Data::new(login_rate_limiter.clone())) // Login controllers use this
            .app_data(web::Data::new(AppState {
                app_name: Mutex::from(env::var("APP_NAME").unwrap_or_else(|_| "".to_string())),
                _user_id: Mutex::from(None),
            }))
            .app_data(command_bus_data.clone())
            .app_data(read_model_store_data.clone())
            .app_data(access_logger_data.clone())
            .app_data(session_store_data.clone())
            .app_data(web::Data::new(ws_server.clone()))
            .configure(routes::config)
    })
    .bind((app_url, app_port))?
    .run()
    .await
}
