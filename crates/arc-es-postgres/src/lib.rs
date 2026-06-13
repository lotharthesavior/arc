//! # Arc ES Postgres
//!
//! Postgres implementation of the [`EventStore`] trait from `arc-core`, backed
//! by an [`sqlx::PgPool`]. Mirrors the semantics of `arc-es-sqlite`:
//!
//! - append-only event log with a `UNIQUE(aggregate_id, sequence)` invariant
//! - optimistic concurrency via a max-sequence version check
//! - per-event audit validation before any write (HIPAA defense-in-depth)
//! - snapshot upsert keyed by `aggregate_id`
//!
//! [`AuditMetadata`] is persisted inline on each event row, identically to the
//! SQLite store. `append` calls
//! [`validate_audit_batch`](arc_core::event_store::validate_audit_batch) before
//! touching the database.
//!
//! ## Schema ownership
//!
//! Unlike the SQLite crate — whose DDL lives in the Diesel `migrations/`
//! pipeline — this crate ships its own idempotent schema initialization
//! ([`PostgresEventStore::initialize_schema`] and
//! [`read_model_store::PostgresReadModelStore::initialize_schema`]). That lets
//! tests and early adopters stand up the required tables without converting the
//! migration system to Postgres in this step.

use arc_core::audit::AuditMetadata;
use arc_core::event::Event;
use arc_core::event_store::{
    validate_audit_batch, EventStore, EventStoreError, EventStoreResult, VersionCheck,
};
use arc_core::snapshot::Snapshot;
use async_trait::async_trait;
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Row;
use uuid::Uuid;

// Re-export for convenience, matching arc-es-sqlite.
pub use arc_core::{Deserialize, Serialize};

pub mod read_model_store;
pub use read_model_store::PostgresReadModelStore;

/// DDL for the append-only event log. Idempotent.
const EVENTS_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS events (
    id BIGSERIAL PRIMARY KEY,
    event_id TEXT NOT NULL UNIQUE,
    aggregate_type TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    sequence BIGINT NOT NULL,
    event_type TEXT NOT NULL,
    payload JSONB NOT NULL,
    "timestamp" BIGINT NOT NULL,
    actor_id TEXT NOT NULL DEFAULT 'legacy-pre-hipaa',
    actor_session_id TEXT,
    source_ip TEXT,
    user_agent TEXT,
    timestamp_utc_us BIGINT NOT NULL DEFAULT 0,
    causation_id TEXT,
    correlation_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
    UNIQUE(aggregate_id, sequence)
);
CREATE INDEX IF NOT EXISTS idx_events_aggregate ON events(aggregate_id, sequence);
CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);
CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events("timestamp");
CREATE INDEX IF NOT EXISTS idx_events_actor_id ON events(actor_id);
CREATE INDEX IF NOT EXISTS idx_events_correlation_id ON events(correlation_id);
"#;

/// DDL for the snapshot table. Idempotent.
const SNAPSHOTS_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS snapshots (
    aggregate_id TEXT NOT NULL PRIMARY KEY,
    aggregate_type TEXT NOT NULL,
    version BIGINT NOT NULL,
    state JSONB NOT NULL,
    created_at BIGINT NOT NULL
);
"#;

/// Plain, DB-free representation of an event row. Splitting conversion out from
/// the query layer lets the row<->`Event` mapping be unit-tested without a live
/// Postgres server.
#[derive(Debug, Clone, PartialEq)]
struct EventRow {
    event_id: String,
    aggregate_type: String,
    aggregate_id: String,
    sequence: i64,
    event_type: String,
    payload: serde_json::Value,
    timestamp: i64,
    actor_id: String,
    actor_session_id: Option<String>,
    source_ip: Option<String>,
    user_agent: Option<String>,
    timestamp_utc_us: i64,
    causation_id: Option<String>,
    correlation_id: String,
}

impl EventRow {
    fn from_event(event: &Event) -> EventRow {
        // Stored in seconds to match the SQLite store's `timestamp` column unit.
        let timestamp_seconds: i64 = (event.timestamp / 1000) as i64;
        EventRow {
            event_id: event.event_id.to_string(),
            aggregate_type: event.aggregate_type.clone(),
            aggregate_id: event.aggregate_id.clone(),
            sequence: event.sequence,
            event_type: event.event_type.clone(),
            payload: event.payload.clone(),
            timestamp: timestamp_seconds,
            actor_id: event.audit.actor_id.clone(),
            actor_session_id: event.audit.actor_session_id.clone(),
            source_ip: event.audit.source_ip.clone(),
            user_agent: event.audit.user_agent.clone(),
            timestamp_utc_us: event.audit.timestamp_utc_us,
            causation_id: event.audit.causation_id.map(|u| u.to_string()),
            correlation_id: event.audit.correlation_id.to_string(),
        }
    }

    fn to_event(&self) -> EventStoreResult<Event> {
        let event_id = Uuid::parse_str(&self.event_id)
            .map_err(|e| EventStoreError::serialization(format!("Invalid UUID: {}", e)))?;

        let causation_id = match self.causation_id.as_deref() {
            Some(s) => Some(Uuid::parse_str(s).map_err(|e| {
                EventStoreError::serialization(format!("Invalid causation UUID: {}", e))
            })?),
            None => None,
        };

        let correlation_id = Uuid::parse_str(&self.correlation_id).map_err(|e| {
            EventStoreError::serialization(format!("Invalid correlation UUID: {}", e))
        })?;

        let audit = AuditMetadata {
            actor_id: self.actor_id.clone(),
            actor_session_id: self.actor_session_id.clone(),
            source_ip: self.source_ip.clone(),
            user_agent: self.user_agent.clone(),
            timestamp_utc_us: self.timestamp_utc_us,
            causation_id,
            correlation_id,
        };

        Ok(Event {
            event_id,
            aggregate_type: self.aggregate_type.clone(),
            aggregate_id: self.aggregate_id.clone(),
            sequence: self.sequence,
            event_type: self.event_type.clone(),
            payload: self.payload.clone(),
            audit,
            timestamp: (self.timestamp as u64) * 1000,
        })
    }

    fn from_pg_row(row: &sqlx::postgres::PgRow) -> EventStoreResult<EventRow> {
        let map = |e: sqlx::Error| EventStoreError::database(e.to_string());
        Ok(EventRow {
            event_id: row.try_get("event_id").map_err(map)?,
            aggregate_type: row.try_get("aggregate_type").map_err(map)?,
            aggregate_id: row.try_get("aggregate_id").map_err(map)?,
            sequence: row.try_get("sequence").map_err(map)?,
            event_type: row.try_get("event_type").map_err(map)?,
            payload: row.try_get("payload").map_err(map)?,
            timestamp: row.try_get("timestamp").map_err(map)?,
            actor_id: row.try_get("actor_id").map_err(map)?,
            actor_session_id: row.try_get("actor_session_id").map_err(map)?,
            source_ip: row.try_get("source_ip").map_err(map)?,
            user_agent: row.try_get("user_agent").map_err(map)?,
            timestamp_utc_us: row.try_get("timestamp_utc_us").map_err(map)?,
            causation_id: row.try_get("causation_id").map_err(map)?,
            correlation_id: row.try_get("correlation_id").map_err(map)?,
        })
    }
}

/// Postgres implementation of [`EventStore`].
#[derive(Clone)]
pub struct PostgresEventStore {
    pool: PgPool,
}

impl PostgresEventStore {
    /// Build a store from a Postgres connection URL, creating a small pool.
    pub async fn new(database_url: &str) -> EventStoreResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await
            .map_err(|e| EventStoreError::database(format!("Failed to create pool: {}", e)))?;
        Ok(PostgresEventStore { pool })
    }

    /// Build a store from an existing pool. Lets tests share one pool with the
    /// read-model store against the same database.
    pub fn with_pool(pool: PgPool) -> Self {
        PostgresEventStore { pool }
    }

    /// Borrow the underlying pool (e.g. to construct a read-model store that
    /// shares the same connections).
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Create the `events` and `snapshots` tables and their indexes if absent.
    /// Idempotent; safe to call on every startup.
    pub async fn initialize_schema(&self) -> EventStoreResult<()> {
        sqlx::raw_sql(EVENTS_SCHEMA)
            .execute(&self.pool)
            .await
            .map_err(|e| EventStoreError::database(e.to_string()))?;
        sqlx::raw_sql(SNAPSHOTS_SCHEMA)
            .execute(&self.pool)
            .await
            .map_err(|e| EventStoreError::database(e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl EventStore for PostgresEventStore {
    async fn append(
        &self,
        aggregate_id: &str,
        version_check: VersionCheck,
        new_events: Vec<Event>,
    ) -> EventStoreResult<()> {
        if new_events.is_empty() {
            return Ok(());
        }

        // Defense-in-depth: reject any event with invalid audit before touching the DB.
        validate_audit_batch(aggregate_id, &new_events)?;

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| EventStoreError::database(e.to_string()))?;

        let current_version: i64 = sqlx::query(
            "SELECT COALESCE(MAX(sequence), 0) AS v FROM events WHERE aggregate_id = $1",
        )
        .bind(aggregate_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| EventStoreError::database(e.to_string()))?
        .try_get("v")
        .map_err(|e| EventStoreError::database(e.to_string()))?;

        if let Some(expected) = version_check.version() {
            if current_version != expected {
                return Err(EventStoreError::ConcurrencyConflict {
                    aggregate_id: aggregate_id.to_string(),
                    expected,
                    actual: current_version,
                });
            }
        }

        for (expected_sequence, event) in (current_version + 1..).zip(new_events.iter()) {
            if event.sequence != expected_sequence {
                return Err(EventStoreError::InvalidSequence {
                    aggregate_id: aggregate_id.to_string(),
                    expected: expected_sequence,
                    actual: event.sequence,
                });
            }
        }

        for event in &new_events {
            let row = EventRow::from_event(event);
            sqlx::query(
                r#"INSERT INTO events
                    (event_id, aggregate_type, aggregate_id, sequence, event_type, payload,
                     "timestamp", actor_id, actor_session_id, source_ip, user_agent,
                     timestamp_utc_us, causation_id, correlation_id)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)"#,
            )
            .bind(&row.event_id)
            .bind(&row.aggregate_type)
            .bind(&row.aggregate_id)
            .bind(row.sequence)
            .bind(&row.event_type)
            .bind(&row.payload)
            .bind(row.timestamp)
            .bind(&row.actor_id)
            .bind(&row.actor_session_id)
            .bind(&row.source_ip)
            .bind(&row.user_agent)
            .bind(row.timestamp_utc_us)
            .bind(&row.causation_id)
            .bind(&row.correlation_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| EventStoreError::database(e.to_string()))?;
        }

        tx.commit()
            .await
            .map_err(|e| EventStoreError::database(e.to_string()))?;
        Ok(())
    }

    async fn load(&self, aggregate_id: &str) -> EventStoreResult<Vec<Event>> {
        self.load_from(aggregate_id, 1).await
    }

    async fn load_from(
        &self,
        aggregate_id: &str,
        from_sequence: i64,
    ) -> EventStoreResult<Vec<Event>> {
        let rows = sqlx::query(
            "SELECT * FROM events WHERE aggregate_id = $1 AND sequence >= $2 ORDER BY sequence ASC",
        )
        .bind(aggregate_id)
        .bind(from_sequence)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| EventStoreError::database(e.to_string()))?;

        rows.iter()
            .map(|r| EventRow::from_pg_row(r)?.to_event())
            .collect()
    }

    async fn stream_all(&self, from_position: i64) -> EventStoreResult<Vec<Event>> {
        let rows = sqlx::query("SELECT * FROM events WHERE id >= $1 ORDER BY id ASC")
            .bind(from_position)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| EventStoreError::database(e.to_string()))?;

        rows.iter()
            .map(|r| EventRow::from_pg_row(r)?.to_event())
            .collect()
    }

    async fn get_version(&self, aggregate_id: &str) -> EventStoreResult<i64> {
        let version: i64 = sqlx::query(
            "SELECT COALESCE(MAX(sequence), 0) AS v FROM events WHERE aggregate_id = $1",
        )
        .bind(aggregate_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| EventStoreError::database(e.to_string()))?
        .try_get("v")
        .map_err(|e| EventStoreError::database(e.to_string()))?;
        Ok(version)
    }

    async fn save_snapshot(&self, snapshot: &Snapshot) -> EventStoreResult<()> {
        // One snapshot per aggregate: replace in place rather than accumulating
        // stale versions.
        sqlx::query(
            r#"INSERT INTO snapshots (aggregate_id, aggregate_type, version, state, created_at)
               VALUES ($1, $2, $3, $4, $5)
               ON CONFLICT (aggregate_id) DO UPDATE
                 SET aggregate_type = EXCLUDED.aggregate_type,
                     version = EXCLUDED.version,
                     state = EXCLUDED.state,
                     created_at = EXCLUDED.created_at"#,
        )
        .bind(&snapshot.aggregate_id)
        .bind(&snapshot.aggregate_type)
        .bind(snapshot.version)
        .bind(&snapshot.state)
        .bind(snapshot.created_at as i64)
        .execute(&self.pool)
        .await
        .map_err(|e| EventStoreError::database(e.to_string()))?;
        Ok(())
    }

    async fn load_snapshot(&self, aggregate_id: &str) -> EventStoreResult<Option<Snapshot>> {
        let row = sqlx::query(
            "SELECT aggregate_id, aggregate_type, version, state, created_at \
             FROM snapshots WHERE aggregate_id = $1",
        )
        .bind(aggregate_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| EventStoreError::database(e.to_string()))?;

        let row = match row {
            Some(r) => r,
            None => return Ok(None),
        };

        let map = |e: sqlx::Error| EventStoreError::database(e.to_string());
        let created_at: i64 = row.try_get("created_at").map_err(map)?;
        Ok(Some(Snapshot {
            aggregate_id: row.try_get("aggregate_id").map_err(map)?,
            aggregate_type: row.try_get("aggregate_type").map_err(map)?,
            version: row.try_get("version").map_err(map)?,
            state: row.try_get("state").map_err(map)?,
            created_at: created_at as u64,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_event() -> Event {
        let mut audit = AuditMetadata::test_default();
        audit.actor_id = "actor-1".to_string();
        audit.actor_session_id = Some("sess-1".to_string());
        audit.source_ip = Some("10.0.0.1".to_string());
        audit.user_agent = Some("agent/1.0".to_string());
        audit.causation_id = Some(Uuid::new_v4());
        Event::new("User", "u1", 1, "UserCreated", json!({ "name": "Alice" })).with_audit(audit)
    }

    #[test]
    fn test_event_row_roundtrips_all_fields() {
        let event = sample_event();
        let row = EventRow::from_event(&event);
        let back = row.to_event().expect("conversion back to event");

        assert_eq!(back.event_id, event.event_id);
        assert_eq!(back.aggregate_type, event.aggregate_type);
        assert_eq!(back.aggregate_id, event.aggregate_id);
        assert_eq!(back.sequence, event.sequence);
        assert_eq!(back.event_type, event.event_type);
        assert_eq!(back.payload, event.payload);
        assert_eq!(back.audit.actor_id, event.audit.actor_id);
        assert_eq!(back.audit.actor_session_id, event.audit.actor_session_id);
        assert_eq!(back.audit.source_ip, event.audit.source_ip);
        assert_eq!(back.audit.user_agent, event.audit.user_agent);
        assert_eq!(back.audit.timestamp_utc_us, event.audit.timestamp_utc_us);
        assert_eq!(back.audit.causation_id, event.audit.causation_id);
        assert_eq!(back.audit.correlation_id, event.audit.correlation_id);
    }

    #[test]
    fn test_event_row_timestamp_stored_in_seconds() {
        let mut event = sample_event();
        event.timestamp = 1_700_000_123_456; // ms
        let row = EventRow::from_event(&event);
        assert_eq!(row.timestamp, 1_700_000_123); // seconds
        assert_eq!(row.to_event().unwrap().timestamp, 1_700_000_123_000);
    }

    #[test]
    fn test_event_row_handles_nil_optional_audit_fields() {
        let event =
            Event::new("User", "u2", 1, "X", json!({})).with_audit(AuditMetadata::test_default());
        let row = EventRow::from_event(&event);
        assert!(row.causation_id.is_none());
        let back = row.to_event().unwrap();
        assert!(back.audit.causation_id.is_none());
    }

    #[test]
    fn test_event_row_rejects_malformed_correlation_uuid() {
        let mut row = EventRow::from_event(&sample_event());
        row.correlation_id = "not-a-uuid".to_string();
        let err = row.to_event().unwrap_err();
        assert!(
            matches!(err, EventStoreError::SerializationError { ref message } if message.contains("Invalid correlation UUID")),
            "got {err:?}"
        );
    }

    #[test]
    fn test_event_row_accepts_nil_correlation_uuid() {
        let mut row = EventRow::from_event(&sample_event());
        row.correlation_id = "00000000-0000-0000-0000-000000000000".to_string();
        let back = row.to_event().unwrap();
        assert_eq!(back.audit.correlation_id, Uuid::nil());
    }

    // ── Live-database tests ──────────────────────────────────────────────────
    // Gated behind ARC_POSTGRES_TEST_DATABASE_URL so the default `cargo test`
    // run requires no Postgres server. Set it to a throwaway database, e.g.
    //   ARC_POSTGRES_TEST_DATABASE_URL=postgres://localhost/arc_test cargo test

    fn test_db_url() -> Option<String> {
        std::env::var("ARC_POSTGRES_TEST_DATABASE_URL").ok()
    }

    async fn live_store() -> Option<PostgresEventStore> {
        let url = test_db_url()?;
        let store = PostgresEventStore::new(&url).await.expect("connect");
        store.initialize_schema().await.expect("schema");
        // Isolate each run from prior data.
        sqlx::raw_sql("TRUNCATE events RESTART IDENTITY; TRUNCATE snapshots;")
            .execute(store.pool())
            .await
            .expect("truncate");
        Some(store)
    }

    fn stamped(seq: i64, t: &str) -> Event {
        Event::new("User", "u1", seq, t, json!({ "seq": seq }))
            .with_audit(AuditMetadata::test_default())
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_live_append_and_load() {
        let Some(store) = live_store().await else {
            return;
        };
        store
            .append("u1", VersionCheck::New, vec![stamped(1, "Created")])
            .await
            .unwrap();
        let loaded = store.load("u1").await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].sequence, 1);
        assert_eq!(loaded[0].payload["seq"], 1);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_live_optimistic_concurrency() {
        let Some(store) = live_store().await else {
            return;
        };
        store
            .append("u1", VersionCheck::New, vec![stamped(1, "Created")])
            .await
            .unwrap();
        let err = store
            .append("u1", VersionCheck::New, vec![stamped(2, "Updated")])
            .await
            .unwrap_err();
        assert!(matches!(err, EventStoreError::ConcurrencyConflict { .. }));
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_live_rejects_pending_audit() {
        let Some(store) = live_store().await else {
            return;
        };
        let pending = Event::new("User", "u1", 1, "X", json!({}));
        let err = store
            .append("u1", VersionCheck::New, vec![pending])
            .await
            .unwrap_err();
        assert!(matches!(err, EventStoreError::InvalidAudit { .. }));
        assert_eq!(store.load("u1").await.unwrap().len(), 0);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn test_live_snapshot_save_then_load() {
        let Some(store) = live_store().await else {
            return;
        };
        let snap = Snapshot::new("u1", "User", 5, json!({ "name": "Alice" }));
        store.save_snapshot(&snap).await.unwrap();
        assert_eq!(store.load_snapshot("u1").await.unwrap(), Some(snap));
        assert_eq!(store.load_snapshot("missing").await.unwrap(), None);
    }
}
