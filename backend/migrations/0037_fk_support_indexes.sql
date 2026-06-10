-- FK-support indexes (DB-11, June 2026 audit).
--
-- comments.author_id was rewired to ON DELETE SET NULL in 0003 and
-- users.cover_photo_id added with ON DELETE SET NULL in 0008, but neither
-- column was ever indexed: every user purge ran `UPDATE comments SET
-- author_id = NULL WHERE author_id = $1` as a sequential scan, and every
-- photo delete probed users.cover_photo_id the same way. Partial indexes
-- suffice — the RI triggers only ever probe non-NULL values — and they
-- exclude pseudonymised comments and users without a cover. Plain CREATE
-- INDEX (not CONCURRENTLY) matches the 0028 precedent: sqlx wraps
-- migrations in a transaction and the tables are small.

create index comments_author_idx on comments (author_id) where author_id is not null;
create index users_cover_photo_idx on users (cover_photo_id) where cover_photo_id is not null;
