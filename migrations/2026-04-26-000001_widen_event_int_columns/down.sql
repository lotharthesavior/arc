-- Reverting widens back to i32 would lose data above 2^31, so we refuse to.
-- Recreate the events table fresh from the original create_events_table
-- migration if a true rollback is needed; this down is a deliberate no-op.
SELECT 1;
