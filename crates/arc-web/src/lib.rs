//! Arc framework runtime.
//!
//! Holds the reusable web/runtime machinery — Actix middleware, Tera helpers,
//! the event-sourced stack wiring, the websocket transport, and the server
//! bootstrap — behind a builder seam. Applications depend on this crate by
//! version and plug their own aggregate, projectors, and routes in through
//! [`ArcApp::builder`]. No concrete domain type lives here: the runtime is
//! generic over [`arc_core::aggregate::Aggregate`].

use std::any::TypeId;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::{env, fs};

use actix_web::web::ServiceConfig;
use arc_core::aggregate::Aggregate;
use arc_core::command_bus::SnapshotPolicy;
use arc_core::projection::Projector;
use tracing::{debug, error, info};

pub mod helpers {
    pub mod access_log;
    pub mod audit_context;
    pub mod config;
    pub mod csrf;
    pub mod database;
    pub mod es_stack;
    pub mod general;
    pub mod jwt;
    pub mod rate_limit;
    pub mod session;
    pub mod template;
}

pub mod http {
    pub mod middlewares {
        pub mod auth_middleware;
        pub mod idle_timeout_middleware;
        pub mod jwt_middleware;
        pub mod rate_limit_middleware;
    }
    pub mod errors;
}

pub mod commands;
pub mod websocket;

/// Shared application state accessible by request handlers via `web::Data`.
#[derive(Debug)]
pub struct AppState {
    pub app_name: Mutex<String>,
}

/// A read-model projector plus the view (table) it maintains. Applications
/// register these against their aggregate so the framework can drive
/// synchronous, read-after-write projections in single-process mode.
pub struct ProjectorReg {
    pub projector: Box<dyn Projector>,
    pub view: String,
}

impl ProjectorReg {
    pub fn new(projector: impl Projector + 'static, view: impl Into<String>) -> Self {
        Self {
            projector: Box::new(projector),
            view: view.into(),
        }
    }
}

/// Entry point to the framework. Applications register one or more aggregate
/// types, their projectors, and routes, then drive the result with
/// [`ArcAppBuilder::serve`].
pub struct ArcApp;

impl ArcApp {
    pub fn builder() -> ArcAppBuilder {
        ArcAppBuilder {
            aggregates: Vec::new(),
            routes: None,
        }
    }
}

type RoutesFn = dyn Fn(&mut ServiceConfig) + Send + Sync + 'static;
type AggregateBuildFuture = Pin<
    Box<
        dyn Future<Output = std::io::Result<commands::serve::BuiltAggregateRuntime>>
            + Send
            + 'static,
    >,
>;
type AggregateBuildFn =
    dyn FnOnce(Vec<ProjectorReg>, Option<SnapshotPolicy>) -> AggregateBuildFuture + Send + 'static;

struct AggregateRegistration {
    type_id: TypeId,
    aggregate_type: &'static str,
    projectors: Vec<ProjectorReg>,
    snapshot_policy: Option<SnapshotPolicy>,
    build: Box<AggregateBuildFn>,
}

impl AggregateRegistration {
    fn new<A: Aggregate + 'static>() -> Self {
        Self {
            type_id: TypeId::of::<A>(),
            aggregate_type: A::aggregate_type(),
            projectors: Vec::new(),
            snapshot_policy: None,
            build: Box::new(|projectors, snapshot_policy| {
                Box::pin(commands::serve::build_aggregate_runtime::<A>(
                    projectors,
                    snapshot_policy,
                ))
            }),
        }
    }
}

/// Application builder holding type-erased registrations for every aggregate
/// served by the process. Each registration produces its own typed
/// `CommandBus<A>` while sharing the configured storage backend.
pub struct ArcAppBuilder {
    aggregates: Vec<AggregateRegistration>,
    routes: Option<Arc<RoutesFn>>,
}

impl ArcAppBuilder {
    /// Register a writable aggregate type. Arc creates and injects a distinct
    /// `CommandBus<A>` for every call.
    pub fn register_aggregate<A: Aggregate + 'static>(mut self) -> Self {
        assert!(
            !self
                .aggregates
                .iter()
                .any(|registration| registration.type_id == TypeId::of::<A>()),
            "aggregate type {} is already registered",
            A::aggregate_type()
        );
        self.aggregates.push(AggregateRegistration::new::<A>());
        self
    }

    /// Register all projectors for the most recently registered aggregate.
    pub fn register_projectors(mut self, projectors: Vec<ProjectorReg>) -> Self {
        self.current_aggregate_mut().projectors.extend(projectors);
        self
    }

    /// Register a single projector for the most recently registered aggregate.
    pub fn register_projector(
        mut self,
        projector: impl Projector + 'static,
        view: impl Into<String>,
    ) -> Self {
        self.current_aggregate_mut()
            .projectors
            .push(ProjectorReg::new(projector, view));
        self
    }

    /// Mount the application's own routes. Framework handlers (static assets,
    /// websocket, health) are exported from this crate for the closure to
    /// reference; the application owns which routes it mounts.
    pub fn register_routes<F>(mut self, f: F) -> Self
    where
        F: Fn(&mut ServiceConfig) + Send + Sync + 'static,
    {
        self.routes = Some(Arc::new(f));
        self
    }

    /// Snapshot policy for the most recently registered aggregate.
    pub fn snapshot_policy(mut self, policy: Option<SnapshotPolicy>) -> Self {
        self.current_aggregate_mut().snapshot_policy = policy;
        self
    }

    fn current_aggregate_mut(&mut self) -> &mut AggregateRegistration {
        self.aggregates.last_mut().unwrap_or_else(|| {
            panic!(
                "register_aggregate::<A>() must be called before registering projectors or a snapshot policy"
            )
        })
    }

    /// Boot the HTTP server with every registered aggregate and the
    /// application's routes.
    pub async fn serve(self, app_url: String, app_port: u16) -> std::io::Result<()> {
        if self.aggregates.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "ArcAppBuilder::serve requires at least one register_aggregate::<A>() call",
            ));
        }
        let routes = self
            .routes
            .expect("ArcAppBuilder::serve requires register_routes(..)");
        commands::serve::run(app_url, app_port, self.aggregates, routes).await
    }
}

/// Verifies database availability for the configured driver. For file-backed
/// drivers (SQLite) this checks the `DATABASE_URL` file exists and exits with
/// code 1 if missing. Connection-string drivers (Postgres) skip the check.
pub fn check_database_health() {
    info!("Checking database health");
    let driver = helpers::config::DatabaseDriver::from_env();

    if !driver.is_file_backed() {
        debug!(
            driver = driver.as_str(),
            "Database driver uses a connection string; skipping filesystem check"
        );
        return;
    }

    let database: String = helpers::config::database_url();
    if !fs::exists(PathBuf::from(&database)).unwrap() {
        error!("Database file not found at: {}", database);
        error!("Please run `cargo run migrate` to create the database");
        std::process::exit(1);
    }
    debug!("Database file found at: {}", database);
}

/// Copies `.env.example` to `.env` when no `.env` file is present.
pub fn check_app_health() {
    info!("Checking app health");
    if !fs::exists(PathBuf::from(".env")).unwrap() {
        info!("Creating .env file from .env.example");
        fs::copy(PathBuf::from(".env.example"), PathBuf::from(".env"))
            .expect("Failed to copy .env.example to .env");
    }
}

/// Fails fast if required environment variables are missing.
pub fn validate_environment() {
    let required_vars = ["APP_URL", "SECRET_KEY", "DATABASE_URL"];
    let mut missing = Vec::new();
    for var in required_vars {
        if env::var(var).is_err() {
            missing.push(var);
        }
    }
    if !missing.is_empty() {
        error!(
            "Missing required environment variables: {}. Check your .env file.",
            missing.join(", ")
        );
        std::process::exit(1);
    }
    debug!("All required environment variables present");
}
