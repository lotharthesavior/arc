CREATE TABLE snapshots (
    aggregate_id TEXT NOT NULL PRIMARY KEY,
    aggregate_type TEXT NOT NULL,
    version BIGINT NOT NULL,
    state TEXT NOT NULL,
    created_at BIGINT NOT NULL
);
