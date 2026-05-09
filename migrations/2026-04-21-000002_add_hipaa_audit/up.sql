-- HIPAA-1: every event carries typed audit metadata. §164.312(b).
--
-- Adds seven audit columns to the events table, indices for the two columns
-- audit queries hit most ('who?' and 'what request chain?'), and backfills
-- pre-existing rows with the 'legacy-pre-hipaa' sentinel actor.

ALTER TABLE events ADD COLUMN actor_id TEXT NOT NULL DEFAULT 'legacy-pre-hipaa';
ALTER TABLE events ADD COLUMN actor_session_id TEXT;
ALTER TABLE events ADD COLUMN source_ip TEXT;
ALTER TABLE events ADD COLUMN user_agent TEXT;
ALTER TABLE events ADD COLUMN timestamp_utc_us BIGINT NOT NULL DEFAULT 0;
ALTER TABLE events ADD COLUMN causation_id TEXT;
ALTER TABLE events ADD COLUMN correlation_id TEXT NOT NULL DEFAULT '00000000-0000-0000-0000-000000000000';

-- Backfill audit timestamp from the legacy wall-clock column (seconds -> microseconds)
UPDATE events SET timestamp_utc_us = timestamp * 1000000 WHERE timestamp_utc_us = 0;

CREATE INDEX idx_events_actor_id       ON events(actor_id);
CREATE INDEX idx_events_correlation_id ON events(correlation_id);

-- Free-form metadata column is replaced by the typed audit columns above.
ALTER TABLE events DROP COLUMN metadata;
