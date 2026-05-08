-- Step 2: read-model projection table for the User aggregate.
--
-- Materialized from the event stream by `UserProjector` via
-- `SqliteReadModelStore`. Replaces direct reads from the legacy `users` table
-- on the cookie /signin path and from `user_email_index` for email →
-- aggregate_id lookup.
--
-- Storage shape: every projection table the framework owns has the same
-- columns — primary key, optimistic version, and a JSON `data` blob holding
-- the full row. Backend-specific schema introspection is therefore avoided;
-- the SqliteReadModelStore can upsert/find/list any table without knowing
-- the projector's domain shape. Indexes on JSON fields are added per-table
-- as needed (here: email).
--
-- The projector applies
--   `INSERT … ON CONFLICT(id) DO UPDATE … WHERE users_view.version < excluded.version`
-- so duplicate event delivery and out-of-order replay converge to the same
-- row. UserDeleted removes the row entirely; the projection holds only
-- active users.

CREATE TABLE users_view (
    id      TEXT   NOT NULL PRIMARY KEY,
    version BIGINT NOT NULL,
    data    TEXT   NOT NULL
);

CREATE UNIQUE INDEX idx_users_view_email
    ON users_view(json_extract(data, '$.email'));
