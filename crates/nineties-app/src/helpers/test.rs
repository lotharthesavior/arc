use std::env;

#[cfg(test)]
use crate::helpers::database::reset_pool;

/// RAII guard for test cleanup. Resets the connection pool and removes the
/// test database file when dropped. Place `let _finalizer = TestFinalizer;`
/// at the top of each integration test to ensure automatic cleanup.
#[allow(dead_code)]
pub struct TestFinalizer;
impl Drop for TestFinalizer {
    fn drop(&mut self) {
        // Reset the connection pool first to release all connections
        #[cfg(test)]
        reset_pool();

        let database: String =
            env::var("DATABASE_URL").unwrap_or_else(|_| "database/database.sqlite".to_string());

        // Only delete file-based databases; skip in-memory databases
        if database != ":memory:" && !database.contains(":memory:") {
            let _ = std::fs::remove_file(database);
        }
    }
}

/// RAII guard for in-memory test isolation. Resets the connection pool on drop
/// but does not attempt to delete any database file. Use this for tests that
/// configure `DATABASE_URL=:memory:` for true per-test isolation.
#[allow(dead_code)]
pub struct InMemoryTestGuard;
impl Drop for InMemoryTestGuard {
    fn drop(&mut self) {
        #[cfg(test)]
        reset_pool();
    }
}

#[cfg(test)]
pub mod es {
    //! Shared event-sourced test scaffolding. Builds a `CommandBus +
    //! ReadModelStore` pair backed by SQLite in-memory + the in-process
    //! event bus, with `UserProjector` synchronously subscribed so seeded
    //! commands land in `users_view` before `await` returns.

    use crate::database::seeders::create_users::seed_default_user;
    use crate::domain::user::aggregate::UserAggregate;
    use crate::domain::user::projector::{UserProjector, USERS_VIEW};
    use crate::helpers::database::{get_connection, MIGRATIONS};
    use actix_web::web;
    use diesel_migrations::MigrationHarness;
    use nineties_core::command_bus::CommandBus;
    use nineties_core::event_bus::{EventBus, InProcessEventBus};
    use nineties_core::projection::{ProjectionEngine, ProjectionEngineHandler};
    use nineties_core::read_model_store::{InMemoryReadModelStore, ReadModelStore};
    use nineties_es_sqlite::SqliteEventStore;
    use std::env;
    use std::sync::Arc;

    pub struct EsTestStack {
        pub command_bus: web::Data<CommandBus<UserAggregate>>,
        pub read_model_store: web::Data<dyn ReadModelStore>,
        pub seeded_user_id: Option<String>,
    }

    /// Build the ES stack against a shared in-memory SQLite. Runs migrations
    /// once. Does not seed the default user — call [`seed_default`] when a
    /// fixture is needed.
    pub async fn build_stack() -> EsTestStack {
        env::set_var("DATABASE_URL", "file::memory:?cache=shared");
        env::set_var("APP_NAME", env::var("APP_NAME").unwrap_or_default());
        // Actix `cookie::Key` requires at least 64 bytes.
        env::set_var(
            "SECRET_KEY",
            env::var("SECRET_KEY").unwrap_or_else(|_| {
                "test-secret-key-must-be-at-least-sixty-four-bytes-long-for-actix-cookie-signing".into()
            }),
        );
        env::set_var(
            "JWT_SECRET",
            env::var("JWT_SECRET").unwrap_or_else(|_| "test-jwt-secret".into()),
        );
        env::set_var(
            "JWT_EXPIRY_HOURS",
            env::var("JWT_EXPIRY_HOURS").unwrap_or_else(|_| "24".into()),
        );

        let mut conn = get_connection();
        conn.run_pending_migrations(MIGRATIONS).expect("migrations");
        drop(conn);

        let event_store = SqliteEventStore::new("file::memory:?cache=shared")
            .await
            .expect("event store");
        let read_model_store: Arc<dyn ReadModelStore> = Arc::new(InMemoryReadModelStore::new());

        let mut engine = ProjectionEngine::new(Box::new(event_store.clone()));
        engine.register_projector(
            Box::new(UserProjector::new()),
            read_model_store.clone(),
            USERS_VIEW,
        );
        let engine = Arc::new(engine);

        let mut bus = InProcessEventBus::new();
        bus.subscribe(Box::new(ProjectionEngineHandler::new(engine.clone())))
            .await
            .expect("subscribe");

        let command_bus = CommandBus::<UserAggregate>::new(Box::new(event_store), Box::new(bus));

        EsTestStack {
            command_bus: web::Data::new(command_bus),
            read_model_store: web::Data::from(read_model_store),
            seeded_user_id: None,
        }
    }

    /// Build a stack and seed the default user. Returns the stack with
    /// `seeded_user_id` populated for tests that need to assert against it.
    pub async fn build_stack_with_default_user() -> EsTestStack {
        let mut stack = build_stack().await;
        let id = seed_default_user(&stack.command_bus, stack.read_model_store.as_ref())
            .await
            .expect("seed default user");
        stack.seeded_user_id = Some(id);
        stack
    }
}
