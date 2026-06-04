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
use arc_core::access_log::{AccessLogger, NoOpAccessLogger};
use arc_core::command_bus::CommandBus;
#[cfg(feature = "nats")]
use arc_core::event_bus::TwoLaneEventBus;
use arc_core::event_bus::{EventBus, InProcessEventBus};
use arc_core::projection::{ProjectionEngine, ProjectionEngineHandler};
use arc_core::read_model_store::ReadModelStore;
use arc_core::session::SessionStore;
#[cfg(feature = "nats")]
use arc_es_nats::NatsEventBus;
use arc_es_sqlite::{SqliteEventStore, SqliteReadModelStore, SqliteSessionStore};
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

    // Read-model store + projection engine. Inprocess mode keeps projections
    // read-after-write consistent; NATS mode leaves projection ownership to
    // arc-worker after the event is durably published.
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

    let event_bus_mode = event_bus_mode();
    let mut event_bus = build_event_bus(&event_bus_mode).await;
    if event_bus_mode == EventBusMode::InProcess {
        event_bus
            .subscribe(Box::new(ProjectionEngineHandler::new(
                projection_engine.clone(),
            )))
            .await
            .expect("Failed to subscribe ProjectionEngine to event bus");

        // Backfill remains in the writer only for the single-process topology.
        if let Err(e) = projection_engine.rebuild_all().await {
            tracing::error!(error = ?e, "ProjectionEngine.rebuild_all failed at startup");
        } else {
            info!("Projections rebuilt from event store");
        }
    } else {
        info!("EVENT_BUS=nats selected; arc-worker owns projection rebuild and delivery");
    }

    let command_bus =
        CommandBus::<UserAggregate>::new(Box::new(sqlite_event_store.clone()), event_bus);
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
                .cookie_name("arc_session".to_string())
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EventBusMode {
    InProcess,
    Nats,
}

fn event_bus_mode() -> EventBusMode {
    let event_bus = env::var("EVENT_BUS")
        .unwrap_or_else(|_| "inprocess".to_string())
        .to_ascii_lowercase();

    match event_bus.as_str() {
        "inprocess" => EventBusMode::InProcess,
        "nats" => EventBusMode::Nats,
        other => {
            warn!(
                event_bus = other,
                "Unknown EVENT_BUS value; falling back to inprocess"
            );
            EventBusMode::InProcess
        }
    }
}

async fn build_event_bus(mode: &EventBusMode) -> Box<dyn EventBus> {
    match mode {
        EventBusMode::InProcess => Box::new(InProcessEventBus::new()),
        EventBusMode::Nats => build_nats_event_bus().await,
    }
}

#[cfg(feature = "nats")]
async fn build_nats_event_bus() -> Box<dyn EventBus> {
    let nats_url = env::var("NATS_URL").unwrap_or_else(|_| "nats://127.0.0.1:4222".to_string());
    let nats_stream = env::var("NATS_STREAM").unwrap_or_else(|_| "EVENTS".to_string());
    let async_bus = Arc::new(
        NatsEventBus::new(&nats_url, &nats_stream)
            .await
            .expect("Failed to initialize NATS event bus"),
    );

    info!(
        nats_url = nats_url,
        nats_stream = nats_stream,
        "Using NATS JetStream event bus"
    );

    Box::new(TwoLaneEventBus::with_async_bus(async_bus))
}

#[cfg(not(feature = "nats"))]
async fn build_nats_event_bus() -> Box<dyn EventBus> {
    warn!("EVENT_BUS=nats requested without the nats cargo feature; falling back to inprocess");
    Box::new(InProcessEventBus::new())
}
