-- 0020 backfill equipment_items.approved_at for rows created between
-- migration 0018 (which only stamped pre-migration rows) and the fix
-- in commit 1f2d122 ("stamp approved_at on auto-approved item INSERT").
--
-- During that window, both items_create.rs and the upsert helper
-- inserted rows with status='approved' but left approved_at NULL.
-- The UI's catalog meta card renders such rows with "—" in the
-- APPROVED slot, which looks like missing data instead of "we just
-- forgot to stamp it."
--
-- This UPDATE is idempotent — the WHERE clause excludes rows that
-- already have a timestamp, so re-running the migration on a fresh
-- DB (where 0018 already stamped everything and the post-fix INSERT
-- path always stamps too) is a no-op.
--
-- Prefer created_at when present (most accurate "this row appeared
-- at time X"); fall back to now() so we never leave the column NULL
-- for an approved item.

update equipment_items
   set approved_at = coalesce(created_at, now())
 where status = 'approved'
   and approved_at is null;
