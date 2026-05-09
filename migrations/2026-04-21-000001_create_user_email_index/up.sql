CREATE TABLE user_email_index (
    email TEXT NOT NULL PRIMARY KEY,
    aggregate_id TEXT NOT NULL UNIQUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_user_email_index_aggregate_id ON user_email_index(aggregate_id);
