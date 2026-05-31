//! # Snapshot Module
//!
//! A snapshot captures an aggregate's fully-folded state at a known version so
//! the next load can resume from there instead of replaying the entire stream.
//! Without it, `CommandBus::dispatch` reads every event for an aggregate on
//! each command — a long-lived aggregate with hundreds of events pays that cost
//! on every operation. The serialized `state` is opaque to the store: only the
//! owning aggregate knows how to read it back via `Aggregate::from_snapshot`.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// A point-in-time capture of an aggregate's state.
///
/// # Fields
///
/// - `aggregate_id`: instance the snapshot belongs to
/// - `aggregate_type`: type name (e.g., "User"), mirrors [`Event::aggregate_type`](crate::event::Event)
/// - `version`: sequence of the last event folded into `state`; a loader replays
///   events from `version + 1` onward
/// - `state`: aggregate state serialized by `Aggregate::to_snapshot`, opaque here
/// - `created_at`: wall-clock capture time (milliseconds since UNIX epoch, same
///   unit as `Event::timestamp`)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub aggregate_id: String,
    pub aggregate_type: String,
    pub version: i64,
    pub state: serde_json::Value,
    pub created_at: u64,
}

impl Snapshot {
    /// Capture a snapshot, stamping `created_at` with the current wall clock.
    pub fn new(
        aggregate_id: impl Into<String>,
        aggregate_type: impl Into<String>,
        version: i64,
        state: serde_json::Value,
    ) -> Self {
        Self {
            aggregate_id: aggregate_id.into(),
            aggregate_type: aggregate_type.into(),
            version,
            state,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_millis() as u64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_snapshot_new_stamps_created_at() {
        let snap = Snapshot::new("user-1", "User", 7, json!({ "name": "Alice" }));
        assert_eq!(snap.aggregate_id, "user-1");
        assert_eq!(snap.aggregate_type, "User");
        assert_eq!(snap.version, 7);
        assert_eq!(snap.state["name"], "Alice");
        assert!(snap.created_at > 0);
    }

    #[test]
    fn test_snapshot_roundtrips_through_json() {
        let snap = Snapshot::new("user-2", "User", 3, json!({ "value": 42 }));
        let encoded = serde_json::to_string(&snap).unwrap();
        let decoded: Snapshot = serde_json::from_str(&encoded).unwrap();
        assert_eq!(snap, decoded);
    }
}
