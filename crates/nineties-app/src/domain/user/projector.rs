//! `UserProjector` — builds the `users_view` read model from `User` events.
//!
//! Pure event handler: receives an [`Event`] + a [`ReadModelStore`] and
//! emits a single version-gated upsert (or delete on `UserDeleted`). Holds no
//! mutable state. Idempotency comes from the store's version gate, not from
//! the projector — replaying a known event sequence twice yields the same
//! `users_view` because each upsert with a given `version` is rejected the
//! second time.
//!
//! Wired into [`InProcessEventBus`](nineties_core::event_bus::InProcessEventBus)
//! at startup via a thin [`EventHandler`] adapter that calls
//! [`ProjectionEngine::process`](nineties_core::projection::ProjectionEngine::process).

use async_trait::async_trait;
use nineties_core::event::Event;
use nineties_core::projection::{ProjectionError, ProjectionResult, Projector};
use nineties_core::read_model_store::{ReadModelStore, Upsert};
use serde_json::{json, Value};

/// The shared read-model table name. Other modules (controllers, login lookup)
/// reference this constant rather than hard-coding the string.
pub const USERS_VIEW: &str = "users_view";

pub struct UserProjector;

impl UserProjector {
    pub fn new() -> Self {
        Self
    }
}

impl Default for UserProjector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Projector for UserProjector {
    fn name(&self) -> &str {
        "UserProjector"
    }

    fn handles(&self) -> Vec<String> {
        vec![
            "UserRegistered".to_string(),
            "ProfileUpdated".to_string(),
            "EmailChanged".to_string(),
            "PasswordChanged".to_string(),
            "UserDeleted".to_string(),
        ]
    }

    async fn apply(&self, event: &Event, store: &dyn ReadModelStore) -> ProjectionResult<()> {
        let id = &event.aggregate_id;

        // For partial-update events we need the prior row so we can carry
        // forward fields the event doesn't touch. UserRegistered seeds the
        // row outright, so the lookup is skipped.
        match event.event_type.as_str() {
            "UserRegistered" => {
                let row = json!({
                    "id": id,
                    "name": payload_str(&event.payload, "name")?,
                    "email": payload_str(&event.payload, "email")?,
                    "password_hash": payload_str(&event.payload, "password_hash")?,
                    "version": event.sequence,
                });
                store
                    .upsert(Upsert::new(USERS_VIEW, id, row))
                    .await
                    .map_err(|e| project_err(self, event, e.to_string()))?;
            }

            "UserDeleted" => {
                store
                    .delete(USERS_VIEW, id)
                    .await
                    .map_err(|e| project_err(self, event, e.to_string()))?;
            }

            "ProfileUpdated" | "EmailChanged" | "PasswordChanged" => {
                let existing = store
                    .get(USERS_VIEW, id)
                    .await
                    .map_err(|e| project_err(self, event, e.to_string()))?;

                // No prior row to mutate. Either the row was already deleted
                // or this projector is being driven without seeing the
                // upstream UserRegistered event. Skip rather than guess.
                let Some(mut row) = existing else {
                    tracing::warn!(
                        event_type = event.event_type,
                        aggregate_id = id,
                        "UserProjector: skipping update for unknown user"
                    );
                    return Ok(());
                };

                match event.event_type.as_str() {
                    "ProfileUpdated" => {
                        row["name"] = json!(payload_str(&event.payload, "name")?);
                    }
                    "EmailChanged" => {
                        row["email"] = json!(payload_str(&event.payload, "email")?);
                    }
                    "PasswordChanged" => {
                        row["password_hash"] = json!(payload_str(&event.payload, "password_hash")?);
                    }
                    _ => unreachable!(),
                }
                row["version"] = json!(event.sequence);

                store
                    .upsert(Upsert::new(USERS_VIEW, id, row))
                    .await
                    .map_err(|e| project_err(self, event, e.to_string()))?;
            }

            _ => {}
        }

        Ok(())
    }
}

fn payload_str<'a>(payload: &'a Value, field: &str) -> ProjectionResult<&'a str> {
    payload.get(field).and_then(Value::as_str).ok_or_else(|| {
        ProjectionError::other(format!("event payload missing string field '{field}'"))
    })
}

fn project_err(p: &UserProjector, event: &Event, message: impl Into<String>) -> ProjectionError {
    ProjectionError::handle_failed(
        p.name(),
        &event.event_type,
        event.event_id.to_string(),
        message,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use nineties_core::audit::AuditMetadata;
    use nineties_core::read_model_store::InMemoryReadModelStore;

    fn ev(agg_id: &str, seq: i64, ty: &str, payload: Value) -> Event {
        Event::new("User", agg_id, seq, ty, payload).with_audit(AuditMetadata::test_default())
    }

    #[tokio::test]
    async fn registered_then_profile_updated_carries_email_forward() {
        let store = InMemoryReadModelStore::new();
        let p = UserProjector::new();

        p.apply(
            &ev(
                "u1",
                1,
                "UserRegistered",
                json!({"id":"u1","name":"Alice","email":"a@b.c","password_hash":"$argon2$x"}),
            ),
            &store,
        )
        .await
        .unwrap();

        p.apply(
            &ev("u1", 2, "ProfileUpdated", json!({"name":"Alice2"})),
            &store,
        )
        .await
        .unwrap();

        let row = store.get(USERS_VIEW, "u1").await.unwrap().unwrap();
        assert_eq!(row["name"], "Alice2");
        assert_eq!(row["email"], "a@b.c");
        assert_eq!(row["version"], 2);
    }

    #[tokio::test]
    async fn deleted_removes_row() {
        let store = InMemoryReadModelStore::new();
        let p = UserProjector::new();

        p.apply(
            &ev(
                "u1",
                1,
                "UserRegistered",
                json!({"id":"u1","name":"Alice","email":"a@b.c","password_hash":"$argon2$x"}),
            ),
            &store,
        )
        .await
        .unwrap();
        p.apply(&ev("u1", 2, "UserDeleted", json!({})), &store)
            .await
            .unwrap();

        assert!(store.get(USERS_VIEW, "u1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn duplicate_event_delivery_is_idempotent() {
        // Pinned: the framework promises at-least-once semantics on the bus
        // and replay-from-zero on rebuild. Re-applying the same event must
        // not corrupt state.
        let store = InMemoryReadModelStore::new();
        let p = UserProjector::new();
        let registered = ev(
            "u1",
            1,
            "UserRegistered",
            json!({"id":"u1","name":"Alice","email":"a@b.c","password_hash":"$argon2$x"}),
        );
        let updated = ev("u1", 2, "ProfileUpdated", json!({"name":"Alice2"}));

        for _ in 0..3 {
            p.apply(&registered, &store).await.unwrap();
            p.apply(&updated, &store).await.unwrap();
        }

        let row = store.get(USERS_VIEW, "u1").await.unwrap().unwrap();
        assert_eq!(row["name"], "Alice2");
        assert_eq!(row["version"], 2);
    }

    #[tokio::test]
    async fn out_of_order_replay_does_not_regress() {
        let store = InMemoryReadModelStore::new();
        let p = UserProjector::new();

        p.apply(
            &ev(
                "u1",
                1,
                "UserRegistered",
                json!({"id":"u1","name":"Alice","email":"a@b.c","password_hash":"$argon2$x"}),
            ),
            &store,
        )
        .await
        .unwrap();
        // sequence 3 arrives first
        p.apply(
            &ev("u1", 3, "EmailChanged", json!({"email":"new@b.c"})),
            &store,
        )
        .await
        .unwrap();
        // sequence 2 arrives later — must not stomp newer email
        p.apply(
            &ev("u1", 2, "ProfileUpdated", json!({"name":"Alice2"})),
            &store,
        )
        .await
        .unwrap();

        let row = store.get(USERS_VIEW, "u1").await.unwrap().unwrap();
        assert_eq!(row["email"], "new@b.c");
        assert_eq!(row["version"], 3);
    }

    #[tokio::test]
    async fn update_without_prior_row_is_a_warn_skip() {
        let store = InMemoryReadModelStore::new();
        let p = UserProjector::new();
        p.apply(
            &ev("u-orphan", 5, "ProfileUpdated", json!({"name":"X"})),
            &store,
        )
        .await
        .unwrap();
        assert!(store.get(USERS_VIEW, "u-orphan").await.unwrap().is_none());
    }
}

// ---------------------------------------------------------------------------
// Integration test — replay-from-zero against the real SQLite stack.
//
// This exercises the full Step 2 pipeline end-to-end without HTTP: writes
// land in `SqliteEventStore`, `ProjectionEngine::rebuild_all()` truncates
// `users_view` and replays the entire event log through `UserProjector`
// against `SqliteReadModelStore`. Asserts deterministic convergence — the
// promise that lets operators recover from a corrupted read model with
// `rebuild_all`.
#[cfg(test)]
mod replay_from_zero {
    use super::{UserProjector, USERS_VIEW};
    use crate::helpers::database::get_connection;
    use crate::helpers::database::MIGRATIONS;
    use crate::helpers::test::InMemoryTestGuard;
    use diesel_migrations::MigrationHarness;
    use nineties_core::audit::AuditMetadata;
    use nineties_core::event::Event;
    use nineties_core::event_store::{EventStore, VersionCheck};
    use nineties_core::projection::ProjectionEngine;
    use nineties_core::read_model_store::ReadModelStore;
    use nineties_es_sqlite::{SqliteEventStore, SqliteReadModelStore};
    use serde_json::json;
    use serial_test::serial;
    use std::env;
    use std::sync::Arc;

    fn ev(agg_id: &str, seq: i64, ty: &str, payload: serde_json::Value) -> Event {
        Event::new("User", agg_id, seq, ty, payload).with_audit(AuditMetadata::test_default())
    }

    #[serial]
    #[tokio::test]
    async fn rebuild_all_converges_to_deterministic_users_view() {
        let _guard = InMemoryTestGuard;
        env::set_var("DATABASE_URL", "file::memory:?cache=shared");

        // Run schema migrations once on the shared in-memory database.
        let mut conn = get_connection();
        conn.run_pending_migrations(MIGRATIONS).expect("migrations");
        drop(conn);

        let event_store = SqliteEventStore::new("file::memory:?cache=shared")
            .await
            .expect("event store");
        let rm_store: Arc<dyn ReadModelStore> = Arc::new(
            SqliteReadModelStore::new("file::memory:?cache=shared")
                .await
                .expect("rm store"),
        );

        // u1: register → update profile → change email → delete.
        // u2: register only — survives.
        event_store
            .append(
                "u1",
                VersionCheck::New,
                vec![
                    ev(
                        "u1",
                        1,
                        "UserRegistered",
                        json!({"id":"u1","name":"Alice","email":"a@b.c","password_hash":"$argon$x"}),
                    ),
                    ev("u1", 2, "ProfileUpdated", json!({"name":"Alice2"})),
                    ev("u1", 3, "EmailChanged", json!({"email":"a2@b.c"})),
                    ev("u1", 4, "UserDeleted", json!({})),
                ],
            )
            .await
            .unwrap();
        event_store
            .append(
                "u2",
                VersionCheck::New,
                vec![ev(
                    "u2",
                    1,
                    "UserRegistered",
                    json!({"id":"u2","name":"Bob","email":"b@b.c","password_hash":"$argon$y"}),
                )],
            )
            .await
            .unwrap();

        // Build engine, register projector, rebuild from zero.
        let mut engine = ProjectionEngine::new(Box::new(event_store));
        engine.register_projector(Box::new(UserProjector::new()), rm_store.clone(), USERS_VIEW);
        engine.rebuild_all().await.expect("rebuild_all");

        // u1 should be absent (deleted), u2 present with correct fields.
        assert!(rm_store.get(USERS_VIEW, "u1").await.unwrap().is_none());
        let u2 = rm_store.get(USERS_VIEW, "u2").await.unwrap().unwrap();
        assert_eq!(u2["name"], "Bob");
        assert_eq!(u2["email"], "b@b.c");
        assert_eq!(u2["version"], 1);

        // Email lookup hits the JSON-extract index.
        let by_email = rm_store
            .find_by(USERS_VIEW, "email", &json!("b@b.c"))
            .await
            .unwrap();
        assert_eq!(by_email.len(), 1);
        assert_eq!(by_email[0]["id"], "u2");

        // Idempotency: a second rebuild produces the same final state.
        engine.rebuild_all().await.expect("rebuild_all again");
        assert!(rm_store.get(USERS_VIEW, "u1").await.unwrap().is_none());
        assert_eq!(
            rm_store.get(USERS_VIEW, "u2").await.unwrap().unwrap()["version"],
            1
        );
    }
}
