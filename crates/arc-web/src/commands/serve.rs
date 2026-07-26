use crate::helpers::config::DatabaseDriver;
use crate::helpers::es_stack;
use crate::helpers::rate_limit;
use crate::http::middlewares::rate_limit_middleware::GlobalRateLimit;
use crate::websocket::server::WsServer;
use crate::{AppState, ProjectorReg};
use actix::prelude::*;
use actix_session::storage::CookieSessionStore;
use actix_session::{config::PersistentSession, SessionMiddleware};
use actix_web::cookie::{time::Duration, Key, SameSite};
use actix_web::middleware::{Compress, NormalizePath};
use actix_web::web::ServiceConfig;
use actix_web::{web, App, HttpServer};
use std::env;
use std::io;
use std::sync::Mutex;
use tracing::{info, warn};

use arc_core::access_log::{AccessLogger, NoOpAccessLogger};
use arc_core::aggregate::Aggregate;
use arc_core::command_bus::{CommandBus, SnapshotPolicy};
#[cfg(feature = "nats")]
use arc_core::event_bus::TwoLaneEventBus;
use arc_core::event_bus::{EventBus, InProcessEventBus};
use arc_core::projection::{ProjectionEngine, ProjectionEngineHandler};
use arc_core::session::SessionStore;
#[cfg(feature = "nats")]
use arc_es_nats::NatsEventBus;
use arc_es_sqlite::SqliteSessionStore;
use std::sync::Arc;

type RoutesFn = dyn Fn(&mut ServiceConfig) + Send + Sync + 'static;

/// Starts the Actix-Web HTTP server, generic over the application's aggregate
/// `A`. The application supplies its projectors, snapshot policy, and route
/// configuration; the framework owns the middleware, session, rate limiting,
/// compression, and event-sourced wiring.
pub async fn run<A: Aggregate + 'static>(
    app_url: String,
    app_port: u16,
    projectors: Vec<ProjectorReg>,
    snapshot_policy: Option<SnapshotPolicy>,
    routes: Arc<RoutesFn>,
) -> io::Result<()> {
    crate::check_database_health();

    let secret_key = Key::from(
        env::var("SECRET_KEY")
            .expect("SECRET_KEY must be set")
            .as_bytes(),
    );

    let app_env = env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
    let is_production = app_env == "production";

    let session_domain = env::var("SESSION_DOMAIN").ok();

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

    let login_rate_limiter = rate_limit::create_rate_limiter();
    let global_rate_limiter = rate_limit::create_global_rate_limiter();

    // Set up Event Sourced CQRS behind the configured driver. Domain wiring
    // sees only `Box<dyn EventStore>` / `Arc<dyn ReadModelStore>`.
    let driver = DatabaseDriver::from_env();
    let db_url = crate::helpers::config::database_url();
    info!(driver = driver.as_str(), "Selected database driver");

    let stores = es_stack::build_stores(driver, &db_url)
        .await
        .expect("Failed to init event-sourced stores");
    let read_model_store = stores.read_model_store.clone();

    // Read-model store + projection engine. Inprocess mode keeps projections
    // read-after-write consistent; NATS mode leaves durable projection and
    // event-handler delivery to the Benthos routing layer after the event is
    // durably published (see docs/adr/0001-benthos-only-event-routing.md).
    let mut projection_engine = ProjectionEngine::new(stores.projection_event_store);
    for reg in projectors {
        projection_engine.register_projector(reg.projector, read_model_store.clone(), reg.view);
    }
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

        if let Err(e) = projection_engine.rebuild_all().await {
            tracing::error!(error = ?e, "ProjectionEngine.rebuild_all failed at startup");
        } else {
            info!("Projections rebuilt from event store");
        }
    } else {
        info!(
            "EVENT_BUS=nats selected; Benthos owns durable projection and event-handler delivery"
        );
    }

    let command_bus = es_stack::apply_snapshot_policy(
        CommandBus::<A>::new(stores.command_event_store, event_bus),
        snapshot_policy,
    );
    let command_bus_data = web::Data::new(command_bus);
    let read_model_store_data = web::Data::from(read_model_store);
    let projection_engine_data = web::Data::from(projection_engine);

    // Default to NoOpAccessLogger for non-regulated deployments. Production
    // PHI/PCI deployments swap this for a JetStream- or DB-backed sink.
    let access_logger: Arc<dyn AccessLogger> = Arc::new(NoOpAccessLogger);
    let access_logger_data = web::Data::from(access_logger);

    // Server-side JWT session registry. SQLite-only today; under a non-SQLite
    // primary driver it keeps its own local SQLite database.
    let session_db_url = crate::helpers::config::session_store_url(driver);
    if !driver.is_file_backed() {
        warn!(
            session_db_url = session_db_url,
            "JWT session registry is SQLite-backed; Postgres session store is not yet implemented"
        );
    }
    let session_store_impl = SqliteSessionStore::new(&session_db_url)
        .await
        .expect("Failed to init session store");
    let session_store: Arc<dyn SessionStore> = Arc::new(session_store_impl);
    let session_store_data = web::Data::from(session_store);

    HttpServer::new(move || {
        let mut session_middleware =
            SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                .cookie_name("arc_session".to_string())
                .cookie_http_only(true)
                .cookie_same_site(same_site)
                .session_lifecycle(PersistentSession::default().session_ttl(Duration::hours(24)));

        if is_production {
            session_middleware = session_middleware.cookie_secure(true);
        } else {
            session_middleware = session_middleware.cookie_secure(false);
        }

        if let Some(ref domain) = session_domain {
            session_middleware = session_middleware.cookie_domain(Some(domain.clone()));
        }

        let routes = routes.clone();

        App::new()
            .wrap(tracing_actix_web::TracingLogger::default())
            .wrap(GlobalRateLimit)
            .wrap(Compress::default())
            .wrap(session_middleware.build())
            .wrap(NormalizePath::trim())
            .app_data(web::Data::new(global_rate_limiter.clone()))
            .app_data(web::Data::new(login_rate_limiter.clone()))
            .app_data(web::Data::new(AppState {
                app_name: Mutex::from(env::var("APP_NAME").unwrap_or_else(|_| "".to_string())),
            }))
            .app_data(command_bus_data.clone())
            .app_data(read_model_store_data.clone())
            .app_data(projection_engine_data.clone())
            .app_data(access_logger_data.clone())
            .app_data(session_store_data.clone())
            .app_data(web::Data::new(ws_server.clone()))
            .configure(move |cfg| routes.as_ref()(cfg))
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
