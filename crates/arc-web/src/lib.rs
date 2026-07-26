//! Arc framework runtime.
//!
//! Holds the reusable web/runtime machinery — Actix middleware, Tera helpers,
//! the event-sourced stack wiring, the websocket transport, and the server
//! bootstrap — behind a builder seam. Applications depend on this crate by
//! version and plug their own aggregate, projectors, and routes in through
//! [`ArcApp::builder`]. No concrete domain type lives here: the runtime is
//! generic over [`arc_core::aggregate::Aggregate`].

use std::marker::PhantomData;
use std::path::PathBuf;
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

/// Entry point to the framework. `ArcApp::builder::<MyAggregate>()` yields a
/// builder the application configures with its aggregate, projectors, and
/// routes, then drives with [`ArcAppBuilder::serve`].
pub struct ArcApp;

impl ArcApp {
    pub fn builder<A: Aggregate + 'static>() -> ArcAppBuilder<A> {
        ArcAppBuilder {
            projectors: Vec::new(),
            routes: None,
            snapshot_policy: None,
            _marker: PhantomData,
        }
    }
}

type RoutesFn = dyn Fn(&mut ServiceConfig) + Send + Sync + 'static;

/// Application builder, generic over the domain aggregate `A`. The concrete
/// `A` (e.g. an application's `UserAggregate`) is supplied by the application
/// crate; the framework never names it.
pub struct ArcAppBuilder<A: Aggregate + 'static> {
    projectors: Vec<ProjectorReg>,
    routes: Option<Arc<RoutesFn>>,
    snapshot_policy: Option<SnapshotPolicy>,
    _marker: PhantomData<A>,
}

impl<A: Aggregate + 'static> ArcAppBuilder<A> {
    /// Register the aggregate's read-model projectors. They are subscribed to
    /// the in-process event bus so writes update their views synchronously.
    pub fn register_aggregate(mut self, projectors: Vec<ProjectorReg>) -> Self {
        self.projectors = projectors;
        self
    }

    /// Register a single projector (chainable alternative to
    /// [`register_aggregate`]).
    pub fn register_projector(
        mut self,
        projector: impl Projector + 'static,
        view: impl Into<String>,
    ) -> Self {
        self.projectors.push(ProjectorReg::new(projector, view));
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

    /// Snapshot policy for the aggregate's command bus (e.g. every N events).
    pub fn snapshot_policy(mut self, policy: Option<SnapshotPolicy>) -> Self {
        self.snapshot_policy = policy;
        self
    }

    /// Boot the HTTP server with the configured aggregate, projectors, and
    /// routes. Equivalent to the old `commands::serve::run`, now generic.
    pub async fn serve(self, app_url: String, app_port: u16) -> std::io::Result<()> {
        let routes = self
            .routes
            .expect("ArcAppBuilder::serve requires register_routes(..)");
        commands::serve::run::<A>(
            app_url,
            app_port,
            self.projectors,
            self.snapshot_policy,
            routes,
        )
        .await
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
