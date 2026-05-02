-- 0003 Phase 8a: sessions enrichment, short-lived tokens (password reset +
-- email change), user preferences (theme/density), account-deletion grace,
-- and pseudonymisation of comments at account purge.

-- Sessions: track last activity. Label is derived at render time from
-- user_agent (woothee), not stored.
alter table sessions
  add column last_used_at timestamptz not null default now();
create index sessions_last_used_at_idx on sessions (user_id, last_used_at desc);

-- Short-lived tokens. Hash stored, never raw token.
create table password_reset_tokens (
  token_hash bytea primary key,
  user_id    uuid not null references users(id) on delete cascade,
  expires_at timestamptz not null,
  used_at    timestamptz,
  created_at timestamptz not null default now(),
  request_ip inet
);
create index password_reset_user_idx on password_reset_tokens (user_id, created_at desc);

create table email_change_tokens (
  token_hash bytea primary key,
  user_id    uuid not null references users(id) on delete cascade,
  new_email  citext not null,
  expires_at timestamptz not null,
  used_at    timestamptz,
  created_at timestamptz not null default now()
);

-- UI preferences + account state.
alter table users
  add column theme text not null default 'dark'
    check (theme in ('dark','light')),
  add column density text not null default 'work'
    check (density in ('work','data')),
  add column password_changed_at timestamptz,
  add column pending_deletion_at timestamptz;

-- Backfill so the UI "LAST CHANGED" label is meaningful for accounts that
-- already had a password before this migration.
update users set password_changed_at = created_at
 where password_hash is not null and password_changed_at is null;

create index users_pending_deletion_idx on users (pending_deletion_at)
  where pending_deletion_at is not null;

-- Pseudonymise comments when an account is purged: keep body, drop author.
alter table comments
  alter column author_id drop not null;
alter table comments
  drop constraint comments_author_id_fkey,
  add constraint comments_author_id_fkey
    foreign key (author_id) references users(id) on delete set null;
