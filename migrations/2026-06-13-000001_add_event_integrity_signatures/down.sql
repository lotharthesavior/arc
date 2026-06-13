DROP INDEX IF EXISTS idx_events_integrity_key_id;
ALTER TABLE events DROP COLUMN integrity_key_id;
ALTER TABLE events DROP COLUMN integrity_signature;
