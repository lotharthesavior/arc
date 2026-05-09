//! Shared assembly of the event-sourced stack — `EventStore`, in-process
//! `EventBus`, `ReadModelStore`, `ProjectionEngine`, and `CommandBus`.
//!
//! Used by the runtime server (`commands::serve`), CLI utilities
//! (`commands::migrate`, `commands::seed`), and integration tests so the
//! exact same wiring drives every entry point.

use crate::domain::user::aggregate::UserAggregate;
use crate::domain::user::projector::{UserProjector, USERS_VIEW};
use nineties_core::command_bus::CommandBus;
use nineties_core::event_bus::{EventBus, InProcessEventBus};
use nineties_core::projection::{ProjectionEngine, ProjectionEngineHandler};
use nineties_core::read_model_store::ReadModelStore;
use nineties_es_sqlite::{SqliteEventStore, SqliteReadModelStore};
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

/// Build the production stack against a SQLite database URL. Subscribes the
/// projector to the in-process bus so writes drive `users_view` synchronously.
pub async fn build(database_url: &str) -> Result<EsStack, Box<dyn std::error::Error>> {
    let event_store = SqliteEventStore::new(database_url).await?;
    let read_model_store: Arc<dyn ReadModelStore> =
        Arc::new(SqliteReadModelStore::new(database_url).await?);

    let mut engine = ProjectionEngine::new(Box::new(event_store.clone()));
    engine.register_projector(
        Box::new(UserProjector::new()),
        read_model_store.clone(),
        USERS_VIEW,
    );
    let engine = Arc::new(engine);

    let mut bus = InProcessEventBus::new();
    bus.subscribe(Box::new(ProjectionEngineHandler::new(engine.clone())))
        .await?;

    let command_bus = CommandBus::<UserAggregate>::new(Box::new(event_store), Box::new(bus));

    Ok(EsStack {
        command_bus,
        read_model_store,
        projection_engine: engine,
    })
}
