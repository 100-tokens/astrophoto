-- Per-account login brute-force throttle.
--
-- One row per user that has recently failed a password login (upserted in
-- place, NOT one row per attempt — so the table is bounded by user count, not
-- attack volume, and needs no purge job).
--
-- The lock is a FIXED `locked_until` stamped when `failed_count` first crosses
-- the threshold; it is never extended while a lock is active because the login
-- handler short-circuits a locked account before recording any further failure.
-- That makes the lockout a bounded, known-duration speed bump rather than a
-- DoS an attacker can sustain by continuing to guess. Crossing the threshold
-- also resets `failed_count` to 0, so each lock window grants a fresh budget of
-- attempts (a legitimate user is never permanently locked). A successful login
-- deletes the row.
create table login_throttle (
    user_id      uuid        primary key references users(id) on delete cascade,
    failed_count integer     not null default 0,
    locked_until timestamptz,
    updated_at   timestamptz not null default now()
);
