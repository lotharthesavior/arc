//! # Read Model Store
//!
//! Backend-agnostic persistence layer for projection read models.
//!
//! The `ReadModelStore` trait abstracts over storage backends (SQLite, Postgres,
//! dqlite, in-memory) so projectors can write read models without coupling to
//! a specific database.
//!
//! ## Implementations
//!
//! - `InMemoryReadModelStore` — built-in, for testing and ephemeral projections
//! - `nineties-rm-sqlite` — SQLite backend (separate crate)
//! - `nineties-rm-postgres` — Postgres backend (separate crate, planned)
//! - `nineties-rm-dqlite` — dqlite backend (separate crate, planned)

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Mutex;
use thiserror::Error;

/// A row returned from a read model query.
///
/// Represented as a JSON object for backend-agnostic access.
pub type Row = serde_json::Value;

/// Errors from read model store operations.
#[derive(Debug, Error)]
pub enum ReadModelError {
    /// Write operation failed
    #[error("Read model write failed: {message}")]
    WriteFailed { message: String },

    /// Query operation failed
    #[error("Read model query failed: {message}")]
    QueryFailed { message: String },

    /// Schema or truncate operation failed
    #[error("Read model schema operation failed: {message}")]
    SchemaFailed { message: String },

    /// Other error
    #[error("Read model error: {message}")]
    Other { message: String },
}

impl ReadModelError {
    pub fn write_failed(message: impl Into<String>) -> Self {
        ReadModelError::WriteFailed {
            message: message.into(),
        }
    }

    pub fn query_failed(message: impl Into<String>) -> Self {
        ReadModelError::QueryFailed {
            message: message.into(),
        }
    }

    pub fn schema_failed(message: impl Into<String>) -> Self {
        ReadModelError::SchemaFailed {
            message: message.into(),
        }
    }

    pub fn other(message: impl Into<String>) -> Self {
        ReadModelError::Other {
            message: message.into(),
        }
    }
}

/// Result type for read model store operations.
pub type ReadModelResult<T> = Result<T, ReadModelError>;

/// Backend-agnostic storage for projection read models.
///
/// Provides a uniform interface for projectors to persist and query materialized
/// views. Implementations handle the specifics of each storage backend.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync`. Multiple projectors may share the same
/// store instance (e.g., different tables in the same database). Interior mutability
/// (connection pools, `Mutex`, etc.) is expected.
///
/// # Example
///
/// ```rust,ignore
/// // Projector uses the store to persist read model state
/// store.execute(
///     "INSERT OR REPLACE INTO users_view (id, name) VALUES (?, ?)",
///     vec![json!("user-1"), json!("Alice")],
/// ).await?;
///
/// let rows = store.query(
///     "SELECT * FROM users_view WHERE id = ?",
///     vec![json!("user-1")],
/// ).await?;
/// ```
#[async_trait]
pub trait ReadModelStore: Send + Sync {
    /// Execute a write operation (INSERT, UPDATE, DELETE).
    async fn execute(&self, sql: &str, params: Vec<serde_json::Value>) -> ReadModelResult<()>;

    /// Execute a query and return rows.
    async fn query(&self, sql: &str, params: Vec<serde_json::Value>) -> ReadModelResult<Vec<Row>>;

    /// Truncate/clear a table or collection.
    ///
    /// Used during projection rebuilds to wipe the read model before replay.
    async fn truncate(&self, table: &str) -> ReadModelResult<()>;
}

/// In-memory read model store for testing.
///
/// Stores rows as JSON objects in a `HashMap<String, Vec<Row>>` keyed by table name.
/// Not suitable for production use — no SQL parsing, just simple append/clear.
///
/// # Example
///
/// ```rust,ignore
/// let store = InMemoryReadModelStore::new();
/// store.execute("users_view", vec![json!({"id": "1", "name": "Alice"})]).await?;
/// ```
pub struct InMemoryReadModelStore {
    tables: Mutex<HashMap<String, Vec<Row>>>,
}

impl InMemoryReadModelStore {
    pub fn new() -> Self {
        Self {
            tables: Mutex::new(HashMap::new()),
        }
    }

    /// Get all rows in a table (test helper).
    pub fn get_rows(&self, table: &str) -> Vec<Row> {
        self.tables
            .lock()
            .unwrap()
            .get(table)
            .cloned()
            .unwrap_or_default()
    }

    /// Get total row count across all tables (test helper).
    pub fn total_rows(&self) -> usize {
        self.tables
            .lock()
            .unwrap()
            .values()
            .map(|rows| rows.len())
            .sum()
    }
}

impl Default for InMemoryReadModelStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ReadModelStore for InMemoryReadModelStore {
    async fn execute(&self, sql: &str, params: Vec<serde_json::Value>) -> ReadModelResult<()> {
        // For in-memory store, `sql` is treated as the table name
        // and the first param is the row to insert.
        let table = sql.to_string();
        let row = if params.len() == 1 {
            params.into_iter().next().unwrap()
        } else {
            serde_json::Value::Array(params)
        };

        self.tables
            .lock()
            .unwrap()
            .entry(table)
            .or_default()
            .push(row);

        Ok(())
    }

    async fn query(&self, sql: &str, _params: Vec<serde_json::Value>) -> ReadModelResult<Vec<Row>> {
        // For in-memory store, `sql` is treated as the table name
        Ok(self.get_rows(sql))
    }

    async fn truncate(&self, table: &str) -> ReadModelResult<()> {
        self.tables.lock().unwrap().remove(table);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_store_execute_and_query() {
        let store = InMemoryReadModelStore::new();

        store
            .execute(
                "users",
                vec![serde_json::json!({"id": "1", "name": "Alice"})],
            )
            .await
            .unwrap();

        let rows = store.query("users", vec![]).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["name"], "Alice");
    }

    #[tokio::test]
    async fn test_in_memory_store_truncate() {
        let store = InMemoryReadModelStore::new();

        store
            .execute("users", vec![serde_json::json!({"id": "1"})])
            .await
            .unwrap();
        assert_eq!(store.total_rows(), 1);

        store.truncate("users").await.unwrap();
        assert_eq!(store.total_rows(), 0);
    }

    #[tokio::test]
    async fn test_in_memory_store_multiple_tables() {
        let store = InMemoryReadModelStore::new();

        store
            .execute("users", vec![serde_json::json!({"id": "1"})])
            .await
            .unwrap();
        store
            .execute("orders", vec![serde_json::json!({"id": "100"})])
            .await
            .unwrap();

        assert_eq!(store.get_rows("users").len(), 1);
        assert_eq!(store.get_rows("orders").len(), 1);
        assert_eq!(store.total_rows(), 2);

        // Truncating one table doesn't affect the other
        store.truncate("users").await.unwrap();
        assert_eq!(store.get_rows("users").len(), 0);
        assert_eq!(store.get_rows("orders").len(), 1);
    }
}
