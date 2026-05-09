-- Recreate the legacy tables. Restoring data is not possible here — the
-- canonical source has moved to the event store + `users_view` projection.
-- These DDL statements exist only to allow `diesel migration redo` against
-- a fresh database to round-trip.

CREATE TABLE users (
    id         INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name       TEXT    NOT NULL,
    email      TEXT    NOT NULL UNIQUE,
    password   TEXT    NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE user_email_index (
    email         TEXT NOT NULL PRIMARY KEY,
    aggregate_id  TEXT NOT NULL,
    created_at    TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX idx_user_email_index_aggregate_id ON user_email_index(aggregate_id);
