DROP INDEX IF EXISTS idx_events_actor_id;
DROP INDEX IF EXISTS idx_events_correlation_id;

ALTER TABLE events ADD COLUMN metadata TEXT DEFAULT '{}';

ALTER TABLE events DROP COLUMN actor_id;
ALTER TABLE events DROP COLUMN actor_session_id;
ALTER TABLE events DROP COLUMN source_ip;
ALTER TABLE events DROP COLUMN user_agent;
ALTER TABLE events DROP COLUMN timestamp_utc_us;
ALTER TABLE events DROP COLUMN causation_id;
ALTER TABLE events DROP COLUMN correlation_id;
