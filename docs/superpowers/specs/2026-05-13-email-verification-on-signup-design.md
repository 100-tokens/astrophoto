# Email verification on signup

**Status:** approved, ready for implementation plan
**Date:** 2026-05-13
**Owner:** Pascal Le Clech

## Goal

Add an email-confirmation step to password-based signup. New accounts
cannot sign in until the user clicks a one-time link sent to the email
address on the signup form. Google OAuth signups are auto-verified.
Existing accounts are grandfathered.

## Non-goals

- Re-verifying emails that change later (the existing email-change flow
  in `auth/email_change.rs` already issues its own token; that path is
  unchanged).
- Throttling beyond the same per-email cooldown / per-hour cap pattern
  already in place for password reset.
- Forcing existing signed-in users to re-confirm (the migration
  backfills every existing row as verified).
- Custom verification UI in the email (plain-text only, identical
  style to the existing password-reset email).

## Decisions captured upfront

Driven by the brainstorming questions, recorded here so the
implementation plan doesn't relitigate them:

- **Gate model:** hard block. Until verified, the account exists but
  cannot sign in, cannot upload, cannot do anything that requires a
  session. Sign-in returns 403 with `email_unverified`.
- **Google OAuth:** auto-verified on first sign-in. Google's flow
  already proves ownership of the email.
- **Backfill:** every existing user row is marked verified at the time
  of migration. No re-confirmation forced.

## Data model

One migration: `backend/migrations/0016_email_verification.sql`.

```sql
-- 0016 Email verification on signup.

-- Column on users; null = unverified.
alter table users
  add column email_verified_at timestamptz;

-- Grandfather every existing row. Uses created_at so the audit trail
-- reads as "verified since account creation" for pre-existing users.
update users set email_verified_at = created_at;

-- Tokens table, shape parallel to password_reset_tokens.
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
```

`email_verified_at` is a timestamp (not a bool) so the audit story
parallels `password_changed_at`. Reading code asks "when?" not just
"yes/no".

## Token policy

- 32 random bytes → 43-char URL-safe-base64.
- Stored as SHA-256 hash, never raw.
- TTL: **24 hours**. Longer than password-reset's 1 hour because
  first-touch email often sits in spam for a while.
- Throttle on resend: 60 s per-user cooldown, 5 / hour per user-or-IP
  cap. Identical to password-reset.

## Backend changes

### `backend/src/auth/signup.rs`

- Keep user-creation logic; **drop** the auto-session creation.
- After creating the user, generate a verification token, store the
  SHA-256 hash, and call `mailer.send_plain` with the template defined
  below.
- Mail-send errors are logged but **do not fail the request**: same
  contract as password-reset. The token row stays in the DB; the user
  can resend from the frontend, or an operator can fish the token out
  of the DB and email it manually if SMTP is misconfigured.
- Return `202 Accepted` with `{ "status": "verification_required",
  "email": "<email>" }`. No `Set-Cookie`.

### `backend/src/auth/email_verify.rs` (new module)

Two routes:

**`POST /api/auth/verify-email`** — body `{ "token": "<base64>" }`.

- Lookup row by `sha256(token)`. Reject (410 Gone) if the row is
  missing, already used, or expired.
- Otherwise, in one transaction:
  - Set `email_verification_tokens.used_at = now()`.
  - Set `users.email_verified_at = now()`.
- Create a session for the user. Return 200 with the same shape as a
  successful signin: `Set-Cookie: __Host-session=…` + JSON body
  containing the `User` DTO.

**`POST /api/auth/resend-verification`** — body `{ "email": "<email>" }`.

- Always returns `204 No Content` (anti-enumeration).
- Internally: if a user with that email exists and is **not yet
  verified**, apply the throttle (per-email cooldown, per-hour cap),
  issue a new token, send mail.
- Throttle query mirrors `password_reset_tokens` exactly: cooldown
  test uses `user_id` only; hour-cap test uses
  `(user_id = $1 or request_ip = $2)`. Tokens issued on the original
  signup also count toward this cap, since they share the table.

### `backend/src/auth/signin.rs`

- After looking up the user and validating the password, check
  `email_verified_at IS NULL`. If unverified, return 403 with body
  `{ "error": "email_unverified" }`. Do not create a session.
- All other paths unchanged.

### `backend/src/auth/oauth_google.rs`

- When creating the user row for the first time on a Google sign-in,
  set `email_verified_at = now()` in the insert.
- Existing Google-linked accounts (created before this migration)
  are already grandfathered.

### `backend/src/mail/templates.rs`

Add `email_verification(display_name: &str, link: &str) -> (String, String)`:

```
Subject: Confirm your Astrophoto account

Hi {display_name},

Click the link below to confirm your email and finish setting up your
Astrophoto account:

{link}

This link expires in 24 hours. If you didn't sign up, you can ignore
this message.
```

Style matches the existing `password_reset` template exactly. Base64
content-transfer-encoding (same reason as password reset: UTF-8 + long
URLs).

### Routing

In `backend/src/http.rs` (wherever routes are wired):

```rust
.route("/api/auth/verify-email", post(email_verify::verify))
.route("/api/auth/resend-verification", post(email_verify::resend))
```

## Frontend changes

### `frontend/src/routes/(public)/signup/+page.server.ts`

Server action: on a successful 202 from the backend, **redirect to**
`/signup/check-email?email=<email>` instead of setting the cookie /
redirecting to `/`. On 4xx, surface the validation error as before.

### `frontend/src/routes/(public)/signup/check-email/+page.svelte` (new)

Static page that reads `email` from the URL search params and shows:

```
Check your email
We sent a confirmation link to {email}.
It can take a minute to arrive. Don't forget to check spam.
[Resend] [Back to signin]
```

The Resend button posts to a thin SvelteKit form action that calls
`/api/auth/resend-verification`. The page does not display whether
the resend actually fired (the backend's 204 is opaque on purpose);
just shows "If your account exists and isn't verified, we sent
another link."

### `frontend/src/routes/(public)/verify/[token]/+page.server.ts` (new)

Server-side `load`: takes `params.token`, POSTs it to
`/api/auth/verify-email`.

- On 200: forward the `Set-Cookie` from the backend response to the
  browser via `setHeaders`, then redirect to `/`.
- On 410: redirect to `/signup/check-email?expired=1`.
- On any other error: throw a 500 (same UX as other broken backend
  routes).

### `frontend/src/routes/(public)/signin/+page.server.ts`

Server action: on a 403 with `{ "error": "email_unverified" }`,
redirect to `/signup/check-email?email=<email>` instead of showing the
"invalid credentials" message. Other failure modes unchanged.

## Cross-cutting

### What stays the same

- `crate::mail::Mailer` plumbing.
- `sessions` table and cookie format.
- Existing routes for password reset, email change, account deletion,
  Google OAuth.
- The signup *form* fields and validation rules.

### Why split the work this way

The verification flow is structurally identical to password reset:
random token, sha256 hash in DB, time-limited, used-once. Copying
that pattern keeps the threat model uniform and the code reviewable
side-by-side.

## Testing

- **Backend unit / integration** (`backend/src/auth/email_verify.rs`
  tests module): token issued on signup, token honoured on verify,
  token rejected when expired / used / unknown, sign-in blocked when
  unverified, sign-in works after verify.
- **Mail dev path:** local dev still uses MailHog (`localhost:8025`);
  the existing dev `.env` doesn't need changes.
- **Prod smoke** (post-deploy): full signup → SES delivers email →
  click link → arrive at signed-in `/`. Confirmed by running it
  manually with a real email under our control.

## Migration safety

- Append-only: new column is nullable, backfilled in the same DDL.
- Migration ships in the same release as the signup-handler change,
  so there is no window where the new column exists but signup writes
  rows without ever marking them verified.
- Rollback story: if the release has to roll back, the column is
  harmless (older code ignores it). The verification-token rows would
  orphan but cascade-delete with the user row anyway.

## Open questions

None blocking. Two judgement calls during implementation:

- Whether to also expose `email_verified_at` in the User DTO. Leaning
  yes — the frontend can then show a small "verified" badge on the
  profile page later, and it's a write-once timestamp so no privacy
  concern.
- Whether the resend page should rate-limit on the frontend side too
  (button disabled for 60s after click) or only rely on backend
  throttling. Defer to frontend implementation; trivial either way.

## References

- `backend/src/auth/password_reset.rs` — pattern this design mirrors.
- `backend/migrations/0003_security_account.sql` — schema patterns
  for short-lived tokens.
- `backend/src/mail/templates.rs` and
  `backend/src/mail/mod.rs` — mailer plumbing.
- `2026-05-11-prod-deploy-design.md` — the SES setup that this
  feature finally exercises.
