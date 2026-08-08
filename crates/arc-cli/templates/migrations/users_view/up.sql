CREATE TABLE users_view (
    id TEXT NOT NULL PRIMARY KEY,
    version BIGINT NOT NULL,
    data TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_users_view_email_active
    ON users_view(lower(json_extract(data, '$.email')))
    WHERE json_extract(data, '$.active') = 1;
