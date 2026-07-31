CREATE TABLE snapshots_single_aggregate (
    aggregate_id TEXT NOT NULL PRIMARY KEY,
    aggregate_type TEXT NOT NULL,
    version BIGINT NOT NULL,
    state TEXT NOT NULL,
    created_at BIGINT NOT NULL
);

INSERT INTO snapshots_single_aggregate
SELECT aggregate_id, aggregate_type, version, state, created_at
FROM snapshots;

DROP TABLE snapshots;
ALTER TABLE snapshots_single_aggregate RENAME TO snapshots;

CREATE TABLE events_single_aggregate (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id TEXT NOT NULL UNIQUE,
    aggregate_type TEXT NOT NULL,
    aggregate_id TEXT NOT NULL,
    sequence BIGINT NOT NULL,
    event_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    timestamp BIGINT NOT NULL,
    actor_id TEXT NOT NULL,
    actor_session_id TEXT,
    source_ip TEXT,
    user_agent TEXT,
    timestamp_utc_us BIGINT NOT NULL,
    causation_id TEXT,
    correlation_id TEXT NOT NULL,
    integrity_signature TEXT,
    integrity_key_id TEXT,
    UNIQUE(aggregate_id, sequence)
);

INSERT INTO events_single_aggregate
SELECT id, event_id, aggregate_type, aggregate_id, sequence, event_type,
       payload, timestamp, actor_id, actor_session_id, source_ip, user_agent,
       timestamp_utc_us, causation_id, correlation_id,
       integrity_signature, integrity_key_id
FROM events;

DROP TABLE events;
ALTER TABLE events_single_aggregate RENAME TO events;

CREATE INDEX idx_events_aggregate ON events(aggregate_id, sequence);
CREATE INDEX idx_events_type ON events(event_type);
CREATE INDEX idx_events_timestamp ON events(timestamp);
CREATE INDEX idx_events_id ON events(id);
CREATE INDEX idx_events_actor_id ON events(actor_id);
CREATE INDEX idx_events_correlation_id ON events(correlation_id);
CREATE INDEX idx_events_integrity_key_id ON events(integrity_key_id);
