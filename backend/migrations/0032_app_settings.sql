-- Runtime-editable, app-wide settings (managed by super-admins).
--
-- Single-row table (the `id = 1` CHECK enforces the singleton): the whole app
-- has exactly one settings row. Readers MUST be fail-safe — if the row is
-- somehow missing or a read errors, callers fall back to the compile-time
-- defaults below, never breaking signup or upload. Columns mirror constants
-- that were previously hardcoded:
--   * signups_enabled         — gate on POST /api/auth/signup (maintenance switch)
--   * free_upload_max_mb       — FREE_MAX in photos/upload_init.rs (was 50 MiB)
--   * subscriber_upload_max_mb — SUBSCRIBER_MAX in photos/upload_init.rs (was 200 MiB)
create table app_settings (
    id                        smallint primary key default 1 check (id = 1),
    signups_enabled           boolean  not null default true,
    free_upload_max_mb        integer  not null default 50   check (free_upload_max_mb between 1 and 100000),
    subscriber_upload_max_mb  integer  not null default 200  check (subscriber_upload_max_mb between 1 and 100000),
    updated_at                timestamptz not null default now(),
    updated_by                uuid references users(id) on delete set null
);

-- Seed the singleton with the historical compile-time defaults.
insert into app_settings (id) values (1);
