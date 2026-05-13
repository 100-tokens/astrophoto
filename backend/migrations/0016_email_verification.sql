-- 0016 Email verification on signup.
--
-- Adds `users.email_verified_at` (nullable timestamp; null = unverified)
-- and a short-lived token table parallel to password_reset_tokens.
--
-- Every existing user row is backfilled as verified (using created_at
-- as the verification timestamp) so the launch doesn't lock anyone out.
-- Only NEW signups after this migration go through the confirmation flow.

alter table users
  add column email_verified_at timestamptz;

update users set email_verified_at = created_at;

create table email_verification_tokens (
  token_hash bytea primary key,
  user_id    uuid not null references users(id) on delete cascade,
  expires_at timestamptz not null,
  used_at    timestamptz,
  created_at timestamptz not null default now(),
  request_ip inet
);
create index email_verification_user_idx
  on email_verification_tokens (user_id, created_at desc);
