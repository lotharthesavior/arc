-- Widen events.sequence and events.timestamp from INTEGER (i32, treated by
-- Diesel as 4-byte) to BIGINT (i64). The previous schema silently truncated
-- timestamps after 2038 and sequences past 2.1B (production blocker per
-- docs/ark/refactor-plan.md production risks).
--
-- SQLite's INTEGER type stores up to 8 bytes when the value requires it, so
-- existing rows do not lose data. The change is purely at the Diesel/Rust
-- type layer, which now reads/writes BigInt.

-- SQLite has no ALTER COLUMN TYPE — but it accepts widening in place via
-- type affinity since INTEGER and BIGINT both have INTEGER affinity. We
-- recreate the table to make the schema explicit and portable, then copy
-- the data forward.

CREATE TABLE events_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id TEXT NOT NULL UNIQUE,
    aggregate_type TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    sequence BIGINT NOT NULL,
    event_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    timestamp BIGINT NOT NULL,
    actor_id TEXT NOT NULL DEFAULT 'legacy-pre-hipaa',
    actor_session_id TEXT,
    source_ip TEXT,
    user_agent TEXT,
    timestamp_utc_us BIGINT NOT NULL DEFAULT 0,
    causation_id TEXT,
    correlation_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000',
    UNIQUE(aggregate_id, sequence)
);

INSERT INTO events_new (
    id, event_id, aggregate_type, aggregate_id, sequence, event_type,
    payload, timestamp, actor_id, actor_session_id, source_ip, user_agent,
    timestamp_utc_us, causation_id, correlation_id
)
SELECT
    id, event_id, aggregate_type, aggregate_id, sequence, event_type,
    payload, timestamp, actor_id, actor_session_id, source_ip, user_agent,
    timestamp_utc_us, causation_id, correlation_id
FROM events;

DROP TABLE events;
ALTER TABLE events_new RENAME TO events;

CREATE INDEX idx_events_aggregate ON events(aggregate_id, sequence);
CREATE INDEX idx_events_type ON events(event_type);
CREATE INDEX idx_events_timestamp ON events(timestamp);
CREATE INDEX idx_events_id ON events(id);
CREATE INDEX idx_events_actor_id ON events(actor_id);
CREATE INDEX idx_events_correlation_id ON events(correlation_id);
