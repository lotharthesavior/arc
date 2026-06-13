-- HIPAA-5 (§164.312(c)(1)): persist tamper-evident event signatures.
--
-- Existing rows are intentionally left unsigned (`NULL`) so operators can run
-- a backfill before enabling runtime enforcement with EVENT_INTEGRITY_KEY.

ALTER TABLE events ADD COLUMN integrity_signature TEXT;
ALTER TABLE events ADD COLUMN integrity_key_id TEXT;

CREATE INDEX idx_events_integrity_key_id ON events(integrity_key_id);
