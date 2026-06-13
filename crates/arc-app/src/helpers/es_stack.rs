//! Shared assembly of the event-sourced stack — `EventStore`, in-process
//! `EventBus`, `ReadModelStore`, `ProjectionEngine`, and `CommandBus`.
//!
//! Used by the runtime server (`commands::serve`), CLI utilities
//! (`commands::migrate`, `commands::seed`), and integration tests so the
//! exact same wiring drives every entry point.

use crate::domain::user::aggregate::UserAggregate;
use crate::domain::user::projector::{UserProjector, USERS_VIEW};
use crate::helpers::config;
use crate::helpers::config::DatabaseDriver;
use arc_core::command_bus::CommandBus;
use arc_core::event_bus::{EventBus, InProcessEventBus};
use arc_core::event_store::EventStore;
use arc_core::projection::{ProjectionEngine, ProjectionEngineHandler};
use arc_core::read_model_store::ReadModelStore;
use arc_es_sqlite::{SqliteEventStore, SqliteReadModelStore};
use std::sync::Arc;

/// Bundle of constructed components — the parts external code keeps a
/// handle on after wiring.
pub struct EsStack {
    pub command_bus: CommandBus<UserAggregate>,
    pub read_model_store: Arc<dyn ReadModelStore>,
    /// Held so callers that want to drive `rebuild_all()` can do so. CLI
    /// utilities ignore it; the runtime server keeps a clone.
    #[allow(dead_code)]
    pub projection_engine: Arc<ProjectionEngine>,
}

/// Boxed storage seam shared by every entry point. The command bus and the
/// projection engine each own an independent handle to the same backend so the
/// write path and replay path do not contend on one connection wrapper.
pub struct EsStores {
    pub command_event_store: Box<dyn EventStore>,
    pub projection_event_store: Box<dyn EventStore>,
    pub read_model_store: Arc<dyn ReadModelStore>,
}

/// Construct the event store and read-model store for the selected driver.
/// Domain code never sees the concrete type — only `Box<dyn EventStore>` and
/// `Arc<dyn ReadModelStore>`.
pub async fn build_stores(
    driver: DatabaseDriver,
    database_url: &str,
) -> Result<EsStores, Box<dyn std::error::Error>> {
    match driver {
        DatabaseDriver::Sqlite => build_sqlite_stores(database_url).await,
        DatabaseDriver::Postgres => build_postgres_stores(database_url).await,
    }
}

async fn build_sqlite_stores(database_url: &str) -> Result<EsStores, Box<dyn std::error::Error>> {
    let event_store = build_sqlite_event_store(database_url).await?;
    let read_model_store: Arc<dyn ReadModelStore> =
        Arc::new(SqliteReadModelStore::new(database_url).await?);

    Ok(EsStores {
        command_event_store: Box::new(event_store.clone()),
        projection_event_store: Box::new(event_store),
        read_model_store,
    })
}

async fn build_sqlite_event_store(
    database_url: &str,
) -> Result<SqliteEventStore, Box<dyn std::error::Error>> {
    match config::event_integrity_key() {
        Some(key) => Ok(SqliteEventStore::new_with_integrity_key(
            database_url,
            key,
            config::event_integrity_key_id(),
        )
        .await?),
        None => Ok(SqliteEventStore::new(database_url).await?),
    }
}

#[cfg(feature = "postgres")]
async fn build_postgres_stores(database_url: &str) -> Result<EsStores, Box<dyn std::error::Error>> {
    use arc_es_postgres::{PostgresEventStore, PostgresReadModelStore};

    let event_store = PostgresEventStore::new(database_url).await?;
    event_store.initialize_schema().await?;

    let read_model_store_impl = PostgresReadModelStore::new(database_url).await?;
    read_model_store_impl.initialize_schema().await?;
    let read_model_store: Arc<dyn ReadModelStore> = Arc::new(read_model_store_impl);

    Ok(EsStores {
        command_event_store: Box::new(event_store.clone()),
        projection_event_store: Box::new(event_store),
        read_model_store,
    })
}

#[cfg(not(feature = "postgres"))]
async fn build_postgres_stores(
    _database_url: &str,
) -> Result<EsStores, Box<dyn std::error::Error>> {
    Err("DATABASE_DRIVER=postgres requires building with the `postgres` cargo feature".into())
}

/// Build the in-process stack for the configured driver. Subscribes the
/// projector to the in-process bus so writes drive `users_view` synchronously.
/// Used by CLI utilities (`migrate --seed`, `seed`) and tests.
pub async fn build(database_url: &str) -> Result<EsStack, Box<dyn std::error::Error>> {
    let stores = build_stores(DatabaseDriver::from_env(), database_url).await?;
    let read_model_store = stores.read_model_store.clone();

    let mut engine = ProjectionEngine::new(stores.projection_event_store);
    engine.register_projector(
        Box::new(UserProjector::new()),
        read_model_store.clone(),
        USERS_VIEW,
    );
    let engine = Arc::new(engine);

    let mut bus = InProcessEventBus::new();
    bus.subscribe(Box::new(ProjectionEngineHandler::new(engine.clone())))
        .await?;

    let command_bus = CommandBus::<UserAggregate>::new(stores.command_event_store, Box::new(bus));

    Ok(EsStack {
        command_bus,
        read_model_store,
        projection_engine: engine,
    })
}
