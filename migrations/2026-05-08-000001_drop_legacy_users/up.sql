-- Step 2D: retire the Diesel-owned `users` and `user_email_index` tables.
--
-- Both are superseded by the `users_view` projection (see migration
-- 2026-05-07-000001). Cookie /signin, admin profile reads, and email →
-- aggregate_id lookup all flow through the projection now; the write path
-- went through `CommandBus → EventStore` in Step 1 and admin mutations
-- followed in Step 2. Nothing reads or writes these tables anymore.

DROP INDEX IF EXISTS idx_user_email_index_aggregate_id;
DROP TABLE IF EXISTS user_email_index;
DROP TABLE IF EXISTS users;
