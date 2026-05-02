# Phase 8a — Security & Account Design

**Date:** 2026-05-02
**Status:** Approved (sections 1–6) — pending written-spec review
**Author:** Pascal (with Claude)

## Goal

Land the security and account-management surface from the Phase 8 design
handoff, **excluding 2FA** (deferred — see Out of scope). Phase 8a covers:
the `Settings` shell with five active sections (Profile, Email & Security,
Appearance, Sessions, Delete account), the public password-reset flow, the
in-settings password-change dialog, the email-change-by-link flow, the
account deletion flow with a 7-day grace + cron-driven purge worker, the
RGPD-minimum data export, and the supporting mailer infrastructure.

Phase 8b (Photos & polish — drafts surfaced, replace image, polish 8.5)
will be brainstormed and shipped separately.

## Decisions

| #  | Topic                              | Choice                                                                       |
|----|------------------------------------|------------------------------------------------------------------------------|
| 1  | Phase split                        | 8a (security/account) and 8b (photos/polish) shipped as separate specs       |
| 2  | Settings sections built            | Profile, Email & Security, Appearance, Sessions, Delete                      |
| 3  | Settings sections deferred         | Equipment + Notifications shown as disabled "SOON" entries in left rail      |
| 4  | 2FA                                | Out of scope                                                                 |
| 5  | Mailer                             | `lettre` over SMTP — MailHog in dev, AWS SES SMTP in prod                    |
| 6  | Email change                       | Verify-by-link on new address; old address notified at swap                  |
| 7  | Password reset for OAuth-only user | Bienveillant — reset flow doubles as "set initial password"                  |
| 8  | Sessions device label              | `woothee` parses `user_agent` at render time; no GeoIP                        |
| 9  | `last_used_at` write frequency     | Conditional update with 5-minute threshold (`WHERE last_used_at < now()-5m`)  |
| 10 | Account deletion grace             | 7 days; `pending_deletion_at` column; cancel restores                        |
| 11 | Deletion worker                    | In-process `tokio::spawn` interval (1 h), single binary                       |
| 12 | S3 cleanup at purge                | Worker collects keys, batch `DeleteObjects`, *then* SQL `DELETE FROM users`   |
| 13 | Comments on account deletion       | Pseudonymise (`author_id` → SET NULL), keep body under others' threads        |
| 14 | Appreciations on account deletion  | Hard-delete via existing CASCADE                                             |
| 15 | RGPD data export                   | Stub minimal — `GET /api/me/export.json` with 7-day signed S3 URLs           |
| 16 | Theme & density                    | SSR cookies + DB persistence; `transformPageChunk` rewrite for SSR safety    |
| 17 | Autosave                           | Profile + Appearance autosave on debounce; Email/Password/Deletion explicit  |
| 18 | Modal component                    | New shared `<Modal>` in `frontend/src/lib/components/`                       |
| 19 | Handle reservation post-deletion   | None — `display_name` released immediately at purge                          |

## Module map

```
backend/src/
├─ auth/
│  ├─ password.rs              (existing — extended: verify for change/delete)
│  ├─ session.rs               (existing — extended: last_used_at update)
│  ├─ password_reset.rs        (NEW — request/consume tokens)
│  ├─ email_change.rs          (NEW — request/confirm tokens)
│  └─ middleware.rs            (existing — extended: grace banner data + last_used_at)
├─ mail/                       (NEW)
│  ├─ mod.rs                   (Mailer struct, configuration from env)
│  └─ templates.rs             (4 plain-text templates)
├─ users/
│  ├─ profile.rs               (NEW — GET/PUT /api/me/profile)
│  ├─ deletion.rs              (NEW — request/cancel)
│  ├─ export.rs                (NEW — GET /api/me/export.json)
│  └─ preferences.rs           (NEW — theme/density via /api/me/preferences)
└─ jobs/                       (NEW)
   └─ purge_deletions.rs       (tokio interval task, started from main.rs)

frontend/src/
├─ lib/components/
│  ├─ Modal.svelte                       (NEW — shared focus-trapping dialog)
│  └─ settings/
│     ├─ Section.svelte                  (NEW — h2 italic + description + slot)
│     ├─ Row.svelte                      (NEW — label/field grid)
│     └─ AutosaveField.svelte            (NEW — debounce + saved indicator)
└─ routes/
   ├─ +layout.svelte                     (existing — extended to host the grace banner)
   ├─ reset/
   │  ├─ +page.svelte                    (Step 01 — request)
   │  ├─ sent/+page.svelte               (Step 02 — check email visualiser)
   │  └─ [token]/+page.svelte            (Step 03 — set new password)
   ├─ email-change/[token]/+page.server.ts   (consume + redirect)
   └─ settings/
      ├─ +layout.svelte                  (sticky left rail + content slot)
      ├─ +page.server.ts                 (redirect → /settings/profile)
      ├─ profile/+page.svelte
      ├─ email/+page.svelte              (request email change, dialog confirm)
      ├─ password/+page.svelte           (dialog change password)
      ├─ appearance/+page.svelte         (theme + density chip groups)
      ├─ sessions/+page.svelte           (list + revoke + sign-out-others)
      └─ delete/+page.svelte             (default + grace state variants)
```

Routing convention: the project uses flat routes, **not** SvelteKit
route groups (`(auth)` / `(public)`). The grace banner lives in the
root `+layout.svelte` and is rendered when `locals.user?.pending_deletion_at`
is set. Each `/settings/*` page enforces auth in its `+page.server.ts`
load (returning a `redirect(303, '/signin?next=/settings/...')` if no
user), matching the pattern already used in `/upload` and `/account/logout`.

No new trait. The mailer is a concrete struct with one production path
(SMTP) and a test constructor (`Mailer::for_test()`) that returns an
in-memory transport plus a shared `Vec<SentMail>` for assertions.

## Migration `0003_security_account.sql`

```sql
-- Sessions: track last activity (label is derived at render time, not stored).
alter table sessions
  add column last_used_at timestamptz not null default now();
create index sessions_last_used_at_idx on sessions (user_id, last_used_at desc);

-- Short-lived tokens (password reset + email change). Hash stored, never raw.
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

-- Backfill password_changed_at for existing password users so the UI
-- "LAST CHANGED" label is always meaningful (vs NULL for pre-migration
-- accounts).
update users set password_changed_at = created_at
 where password_hash is not null and password_changed_at is null;

create index users_pending_deletion_idx on users (pending_deletion_at)
  where pending_deletion_at is not null;

-- Pseudonymise comments when an account is purged: keep body, drop author identity.
alter table comments
  alter column author_id drop not null;
alter table comments
  drop constraint comments_author_id_fkey,
  add constraint comments_author_id_fkey
    foreign key (author_id) references users(id) on delete set null;
```

## API surface

| Method | Route                                        | Auth          | Effect                                              |
|--------|----------------------------------------------|---------------|-----------------------------------------------------|
| POST   | `/api/auth/password-reset/request`           | none          | Issue token, send mail; throttle silently           |
| POST   | `/api/auth/password-reset/confirm`           | token in body | Set password, kill all sessions, auto-login          |
| GET    | `/api/me/profile`                            | session       | Read profile fields                                 |
| PUT    | `/api/me/profile`                            | session       | Partial update (display_name, bio)                  |
| GET    | `/api/me/preferences`                        | session       | Read theme + density                                |
| PUT    | `/api/me/preferences`                        | session       | Update theme + density                              |
| GET    | `/api/me/sessions`                           | session       | List active sessions with derived label             |
| DELETE | `/api/me/sessions/:id`                       | session       | Revoke specified session (not the current one)      |
| POST   | `/api/me/sessions/sign-out-others`           | session       | DELETE all sessions other than current              |
| POST   | `/api/me/email-change/request`               | session +pwd  | Issue token, send mail to new address               |
| POST   | `/api/auth/email-change/confirm`               | none (token)  | Swap email, notify old address                      |
| POST   | `/api/me/password-change`                    | session +pwd  | Update + sign-out-others + rotate current cookie    |
| POST   | `/api/me/delete-request`                     | session +pwd  | Set `pending_deletion_at = now() + 7 days`          |
| POST   | `/api/me/delete-cancel`                      | session       | Clear `pending_deletion_at`                         |
| GET    | `/api/me/export.json`                        | session       | RGPD-minimum export with 7-day signed S3 URLs       |

## Mailer

`backend/src/mail/mod.rs` exposes a `Mailer` struct backed by
`lettre::AsyncSmtpTransport<Tokio1Executor>` with features `tokio1`,
`smtp-transport`, `rustls-tls` (no OpenSSL dependency). Mails are
plain-text only — consistent with the JetBrains-Mono "EMAIL PREVIEW"
panel in the public reset Step 02 design.

Configuration via env (see updated `.env.example`):

```
APP_SMTP_HOST=localhost
APP_SMTP_PORT=1025
APP_SMTP_TLS=false
APP_SMTP_USER=
APP_SMTP_PASS=
APP_MAIL_FROM=Astrophoto <noreply@astrophoto.local>
```

`APP_SMTP_TLS` selects between `starttls_relay()` (true, used in prod for AWS SES on port 587) and `builder_dangerous()` (false, used in dev for MailHog on port 1025).

Dev: a `mailhog` service is added to `compose.yml` exposing the SMTP port
on `1025` and the UI on `8025`. Prod: AWS SES SMTP credentials —
`APP_MAIL_FROM` must point at a verified sender.

Templates are five pure functions in `templates.rs` returning
`(subject, body)` tuples; no templating engine. `format!` is enough for
~10–15-line plain-text bodies:

1. `password_reset` — also renders the "set a password" variant when the
   target user has `password_hash IS NULL`.
2. `email_change_request` — sent to the *new* address with the
   confirmation link.
3. `email_change_notification` — sent to the *old* address at the moment
   the swap commits.
4. `account_deletion_scheduled` — sent at delete-request, includes
   instructions for cancelling.
5. `account_deletion_cancelled` — sent at delete-cancel.

Sends are synchronous within the request handler — SMTP latency is
acceptable, and a transient SES failure surfaces as a 500 the user can
retry. No queue.

## Password reset (3-step public flow)

### Step 01 — request

Endpoint: `POST /api/auth/password-reset/request { email }`. Returns
**204 No Content unconditionally** to prevent account enumeration.

Server logic:

1. Lookup user by email (`citext`, case-insensitive).
2. Throttle: skip silently if the most recent token for this email was
   created less than 60 seconds ago, or if more than 5 tokens have been
   created in the last hour for this email or this IP.
3. If user found:
   - Generate a 32-byte random token, URL-safe base64-encoded.
   - Store `sha256(token)` with `expires_at = now() + 1 hour` and the
     request IP.
   - Render the appropriate template (`password_reset` — set-vs-reset
     branch on `password_hash IS NULL`).
   - Send via `mailer.send_plain`.

If the user is not found, the response is the same 204 with no work done.
Timing is *not* constant — acceptable for MVP.

### Step 02 — visualise sent state (`/reset/sent`)

Pure UI page. Reproduces the design mockup: eyebrow, italic h1 with the
recipient email, the "EMAIL PREVIEW · PLAIN TEXT" `<pre>` block (an
illustrative copy of what was sent — useful when the real mail does not
arrive). The "Resend in 0:42" countdown is computed client-side from a
search-param timestamp; clicking re-issues the request and is
re-throttled by the server.

### Step 03 — set new password

Frontend: `/reset/[token]/+page.svelte` with a `+page.server.ts`
that pre-validates the token (`SELECT … WHERE token_hash = $1 AND
used_at IS NULL AND expires_at > now()`) without consuming it. If
invalid: render the expired panel.

Form action submits to `POST /api/auth/password-reset/confirm
{ token, new_password }`:

1. Look up by `sha256(token)`. If missing/used/expired → 410 Gone.
2. Validate password: length ≥ 12, not in the embedded common-password
   list (~1,000 words; one `.txt` file in `assets/`). No zxcvbn.
3. Hash with Argon2 inside `tokio::task::spawn_blocking`.
4. Transaction:
   - `UPDATE users SET password_hash = ?, password_changed_at = now()`
   - `UPDATE password_reset_tokens SET used_at = now()`
   - `DELETE FROM sessions WHERE user_id = ?` (all of them; the user
     has no active reset session).
5. Issue a fresh session cookie and return 204.

The strength meter on the form is computed client-side (4 buckets:
length-based + dictionary check). The "200 years to crack" caption from
the design is decorative — labelled WEAK / FAIR / GOOD / STRONG.

## Email change

### Request — `POST /api/me/email-change/request`

`{ new_email, current_password }`, session-authenticated.

1. Verify `current_password` (Argon2, `spawn_blocking`). 401 if wrong.
   If `password_hash IS NULL`: 400 `no_password_set` (the user must first
   set a password via the bienveillant reset flow).
2. Validate `new_email` (format + lowercase + ≠ current email).
3. Invalidate any prior pending token for this user
   (`UPDATE … SET used_at = now() WHERE user_id = ? AND used_at IS NULL`),
   then insert a new one with `expires_at = now() + 1 hour`.
4. Send the confirmation mail to `new_email` with link
   `{BASE_URL}/email-change/{token}`. The mail body uses `mask_email(current_email)` so an
   attacker controlling the new inbox cannot learn the victim's current address.

### Confirm — `/email-change/[token]/+page.server.ts`

A SvelteKit route, not an API endpoint, because the user clicks from
their inbox and may or may not be signed in on this device. The `load`
posts to `POST /api/auth/email-change/confirm { token }` (auth optional —
the token is sufficient).

Server:

1. `token_hash = sha256(token)`. Lookup row.
2. If missing / used / expired → return `{ status: "expired" }`.
3. Transaction:
   - `SELECT old email FROM users WHERE id = user_id`.
   - `UPDATE users SET email = new_email WHERE id = user_id`. If this
     fails the unique constraint → `{ status: "taken" }`.
   - `UPDATE email_change_tokens SET used_at = now()`.
4. Send the notification mail to the **old** email address — the only
   recovery channel if the new address was set fraudulently.

The page redirects to `/settings/email` with a flash on success, or
renders an error panel on `expired` / `taken`.

### UI

`/settings/email/+page.svelte` shows the current email plus a
verified meta line ("LAST SIGN-IN · 12 JAN 2026 · 192.0.2.10" — derived
from sessions; no city since GeoIP is out). Click "Change…" opens the
shared `<Modal>` with the new-email + current-password form.

If a token is already pending (looked up in `+page.server.ts`), the page
shows a banner above the input:

```
● PENDING : a confirmation link was sent to nouveau@example.fr 12 minutes ago.
[ Resend ]   [ Cancel pending change ]
```

## Password change in settings

`POST /api/me/password-change { current_password?, new_password }`,
session-authenticated.

- If `password_hash IS NOT NULL`: `current_password` is required and
  verified.
- If `password_hash IS NULL`: this is "set a password" — `current_password`
  is optional and ignored.
- Validate `new_password` (≥ 12, not in common list).
- Argon2 in `spawn_blocking`.
- Transaction:
  - `UPDATE users SET password_hash = ?, password_changed_at = now()`.
  - `DELETE FROM sessions WHERE user_id = ?` (every row, including the
    current one — keeping the current row would leave the old cookie
    valid against it).
  - `INSERT` a fresh session row.
- `Set-Cookie` for the new session, return 204. The browser keeps
  working through the new cookie; other devices are signed out.

UI: `/settings/password/+page.svelte` rendered as a Modal-overlay
route. Three field sets depending on user state:

| User state                            | Fields                                       | Secondary link             |
|---------------------------------------|----------------------------------------------|----------------------------|
| Has password                          | Current / New (+strength) / Confirm           | "I don't remember it →"    |
| OAuth-only, no password               | New (+strength) / Confirm                     | (none)                     |
| OAuth user who already set a password | Current / New (+strength) / Confirm           | "I don't remember it →"    |

The "ALL OTHER SESSIONS WILL BE SIGNED OUT" warning from the mockup is
rendered only when `other_sessions_count > 0`.

The Email & Security row also displays:

- `password_hash IS NULL` → "PASSWORD : not set · *Sign in with Google*"
  + button "Set a password…", with hint "SETTING A PASSWORD ENABLES
  SIGN-IN WITHOUT GOOGLE."
- `password_hash IS NOT NULL` → "PASSWORD : ●●●●●●●●●●●●" + button
  "Change…", with hint "LAST CHANGED · {password_changed_at | DD MMM YYYY}".

## Sessions

### `last_used_at` tracking

`auth/middleware.rs`, after resolving a valid session, runs:

```sql
UPDATE sessions SET last_used_at = now()
 WHERE id = $1 AND last_used_at < now() - interval '5 minutes'
```

The `WHERE` filters server-side so 99% of consecutive requests perform a
no-op. Failures are logged and ignored — non-critical.

### Label derivation

`woothee` parses `user_agent` at render time. Stored fields are unchanged
(`user_agent text`, `ip inet`). Output:

- Title (display italic): `{Os} · {Browser}` — e.g. `macOS · Safari`.
- Meta line 1 (mono): `{Browser} {Version} · {Os} {Os Version}`.
- Meta line 2 (mono): `IP {ip} · {category}`.

### List endpoint

`GET /api/me/sessions` returns:

```sql
SELECT id, user_agent, ip, last_used_at, created_at, expires_at,
       (id = $current_session_id) AS is_current
  FROM sessions
 WHERE user_id = $1 AND expires_at > now()
 ORDER BY is_current DESC, last_used_at DESC
```

Each row in the JSON is enriched with the `woothee`-derived
browser/os/version fields.

### Revoke / sign-out-others

`DELETE /api/me/sessions/:id` rejects the current session id with 400
(`use_logout`) and otherwise issues a single SQL `DELETE`. Optimistic
update on the UI; no confirm dialog.

`POST /api/me/sessions/sign-out-others` runs
`DELETE FROM sessions WHERE user_id = $1 AND id != $current`. Inline
confirmation in the UI via a `<details>` toggle, not a Modal.

The same operation is triggered silently by:

- Password change → DELETE *all* sessions then INSERT a fresh one for
  the browser that performed the change (Set-Cookie); other devices
  lose their cookies.
- Password reset → DELETE all sessions then INSERT a fresh one (the
  user has just confirmed identity through the token). Same model.

## Account deletion

### Request — `POST /api/me/delete-request`

`{ current_password?, confirmation_phrase }`. Session-authenticated.

1. If `password_hash IS NOT NULL`: verify password (401 if wrong).
   If `password_hash IS NULL`: skip (the literal phrase + active session
   are sufficient — no Google re-auth round trip).
2. Verify `confirmation_phrase == "DELETE MY ACCOUNT"` (literal match,
   case-sensitive).
3. `UPDATE users SET pending_deletion_at = now() + interval '7 days'
    WHERE id = $1 AND pending_deletion_at IS NULL` (idempotent — repeat
   requests do not extend the grace).
4. Send `account_deletion_scheduled` mail.

### Cancel — `POST /api/me/delete-cancel`

```sql
UPDATE users SET pending_deletion_at = NULL
 WHERE id = $1 AND pending_deletion_at IS NOT NULL
RETURNING email
```

Send `account_deletion_cancelled` mail if a row was updated.

### Grace banner

`backend/src/auth/me.rs` is extended to return
`pending_deletion_at: Option<String>` (RFC3339) on the `User` DTO.
`frontend/src/hooks.server.ts` propagates the field onto
`event.locals.user`. The root `+layout.server.ts` adds a single SQL count
(`SELECT count(*) FROM photos WHERE owner_id = $userId`) when
`pending_deletion_at` is set, so the banner can render
`{N} frames will be erased`.

Lives in the root `+layout.svelte`, conditioned on
`locals.user?.pending_deletion_at`. 44 px strip below `<AppHeader>`,
`--bg-danger-tint`, mono 12 px:

```
● ACCOUNT MARKED FOR DELETION  Permanent removal in {countdown} · {N} frames will be erased
                                                                          [ Cancel deletion ]
```

The countdown updates client-side via `$effect` + `setInterval` (1 min
cadence). `{N}` comes from `+layout.server.ts`
(`SELECT count(*) FROM photos WHERE owner_id = $userId`).

### Settings → Delete in grace state

```
[panel --bg-danger-tint, --danger border]
  ● DELETION SCHEDULED
  Your account will be permanently erased in 6 days, 14 hours.

  [ Cancel deletion · keep my account ]   [ Download my archive (JSON) ]
```

The left-rail item "DELETE ACCOUNT" switches from `--danger` to
`--accent` and reads "● DELETION PENDING".

### Purge worker

`backend/src/jobs/purge_deletions.rs` started from `main.rs`:

```rust
pub fn spawn(pool: PgPool, s3: S3Client) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(3600));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            if let Err(e) = purge_once(&pool, &s3).await {
                tracing::error!(error = ?e, "purge_deletions failed");
            }
        }
    });
}

async fn purge_once(pool: &PgPool, s3: &S3Client) -> Result<u64, AppError> {
    // 1. Find users past grace.
    let due: Vec<Uuid> = sqlx::query_scalar!(
        "SELECT id FROM users
          WHERE pending_deletion_at IS NOT NULL
            AND pending_deletion_at < now()"
    ).fetch_all(pool).await?;

    if due.is_empty() { return Ok(0); }

    // 2. For each, gather S3 keys, batch delete, then SQL DELETE the user
    //    (CASCADE handles photos/sessions/follows/appreciations/tokens;
    //     comments are pseudonymised via SET NULL).
    //    Per-user errors are logged and skipped so a single bad account
    //    cannot stall the whole hourly batch.
    let mut deleted = 0u64;
    for user_id in &due {
        match purge_one_user(pool, s3, *user_id).await {
            Ok(()) => deleted += 1,
            Err(e) => tracing::error!(user_id = %user_id, error = ?e,
                                     "purge_one_user failed; skipping"),
        }
    }

    tracing::info!(deleted, total_due = due.len(), "purge cycle done");
    Ok(deleted)
}

async fn purge_one_user(pool: &PgPool, s3: &S3Client, user_id: Uuid) -> Result<(), AppError> {
    let keys = sqlx::query!(
        "SELECT storage_key, thumbnail_key FROM photos WHERE owner_id = $1",
        user_id
    ).fetch_all(pool).await?;

    let to_delete: Vec<String> = keys.iter()
        .flat_map(|r| std::iter::once(r.storage_key.clone()).chain(r.thumbnail_key.clone()))
        .collect();

    if !to_delete.is_empty() {
        s3.delete_objects_batch(&to_delete).await?;
    }
    sqlx::query!("DELETE FROM users WHERE id = $1", user_id).execute(pool).await?;
    Ok(())
}
```

Multi-replica consideration: the `DELETE` is idempotent and Postgres
serialises overlapping transactions. Advisory lock can be added later
if scale demands. Not required for single-binary deploy.

### Cascade & pseudonymisation effects

| Table              | FK behaviour on user delete                     |
|--------------------|-------------------------------------------------|
| `photos`           | CASCADE (owner_id) — rows + S3 keys removed     |
| `sessions`         | CASCADE                                         |
| `oauth_identities` | CASCADE                                         |
| `appreciations`    | CASCADE — count on others' photos decreases     |
| `follows`          | CASCADE on both follower_id and followed_id     |
| `comments`         | **SET NULL** on `author_id` — body preserved    |
| `password_reset_tokens` | CASCADE                                    |
| `email_change_tokens`   | CASCADE                                    |

The `display_name` / `email` are released immediately at purge.
No 90-day handle reservation in this phase.

## RGPD data export

`GET /api/me/export.json` — session-authenticated, accessible at any
time (not only during grace).

Response: `application/json` with `Content-Disposition: attachment;
filename="astrophoto-export-{user_id}-{YYYY-MM-DD}.json"`. Shape:

```jsonc
{
  "exported_at": "2026-05-02T14:32:00Z",
  "user": { "id", "email", "display_name", "created_at" },
  "photos": [
    {
      "id", "title", "caption", "captured_at",
      "exif": { /* full EXIF row */ },
      "original_url": "<S3 signed, 7-day TTL>",
      "thumbnail_url": "<S3 signed, 7-day TTL>"
    }
  ],
  "comments_authored": [{ "id", "photo_id", "body", "created_at" }],
  "appreciations_given": [{ "photo_id", "created_at" }],
  "follows": { "following": ["…uuids…"], "followers": ["…uuids…"] }
}
```

This satisfies RGPD Art. 20 ("structured, commonly used, machine-readable
format"). At purge, the underlying S3 keys are removed and links become
invalid — consistent with the grace window.

## Settings shell IA

Layout `/settings/+layout.svelte`:

```
AppHeader · grace banner (conditional) · page header (eyebrow + h1 italic)
240 px sticky left rail   |   720 px max content column
```

Left rail:

| Slug                   | Label             | State                                     |
|------------------------|-------------------|-------------------------------------------|
| `/settings/profile`    | PROFILE           | active                                    |
| `/settings/equipment`  | EQUIPMENT         | disabled `<span>` + chip "SOON"           |
| `/settings/notifications` | NOTIFICATIONS  | disabled `<span>` + chip "SOON"           |
| `/settings/email`      | EMAIL & SECURITY  | active                                    |
| `/settings/appearance` | APPEARANCE        | active                                    |
| `/settings/sessions`   | SESSIONS          | active                                    |
| `/settings/delete`     | DELETE ACCOUNT    | active (`--danger`, `--accent` in grace)  |

No physical routes for the disabled items — manual visit returns the
standard SvelteKit 404. Footer-of-nav micro-note (10 px mono `--fg-faint`):

```
ALL CHANGES AUTOSAVE
EXCEPT EMAIL · PASSWORD
· DELETION
```

### Autosave

`<AutosaveField>` debounces 600 ms, posts to a form action, and shows
`● Saved` for 2 s on success or a `--danger` dot on failure (with one
silent retry, then a `beforeunload` warning if the user navigates away
with unsaved state).

Backend: `PUT /api/me/profile` and `PUT /api/me/preferences` accept
partial payloads (`{ display_name?, bio? }`, `{ theme?, density? }`)
and update only present fields. Validation: `display_name` 1–60 chars,
`bio` 0–280 chars.

### Theme & density

Cookies `theme={dark|light}` and `density={work|data}`,
`Path=/; Max-Age=31536000; SameSite=Lax`. Not `__Host-` because the
client must read them.

`app.html` uses SvelteKit placeholders:

```html
<html data-theme="%theme%" data-density="%density%">
```

`hooks.server.ts` extends to:

```typescript
const theme   = parseCookie(cookieHeader, 'theme')   ?? user?.theme   ?? 'dark';
const density = parseCookie(cookieHeader, 'density') ?? user?.density ?? 'work';
event.locals.preferences = { theme, density };

return resolve(event, {
  transformPageChunk: ({ html }) =>
    html.replace('%theme%', theme).replace('%density%', density)
});
```

SSR-pure: no flash on first paint. Placeholders are scoped to `app.html`,
so unrelated markup containing the literal `data-theme="dark"` (CSS
content, blog HTML, SVG attributes) cannot be clobbered.

The Appearance page chips POST to a form action that sets the cookie,
calls `PUT /api/me/preferences` to persist DB-side (cross-device), and
returns a redirect that re-renders with the new theme. No client AJAX.

## Error / loading / empty states

- **No tokens, no other sessions** — UI silently hides "Sign out of all
  other sessions" and "Pending email change" affordances.
- **Reset link expired** — `/reset/[token]` renders a `--bg-danger-tint`
  panel "This link has expired or already been used." + primary "Request
  a new link".
- **Email change link expired or target taken** — same fallback panel
  pattern on `/email-change/[token]`.
- **SMTP failure on request** — endpoint returns 500; frontend shows
  generic "Something went wrong sending the email. Please retry in a
  moment."
- **Autosave failure** — dot turns `--danger`, one silent retry,
  `beforeunload` warning if the user navigates with unsaved state.
- **OAuth-only user opens password reset** — same flow runs but the mail
  template uses the "set a password" copy.

## Accessibility (WCAG 2.1 AA)

- `<Modal>` has `role="dialog"`, `aria-modal="true"`, focus trap, ESC,
  restored focus to invoker.
- Sessions list: `<ul role="list">`; each "Revoke" button has
  `aria-label="Revoke session: {label}"`.
- Deletion confirmation: input `aria-describedby` points to the
  instruction; the danger button stays `disabled` until the literal
  phrase matches.
- Strength meter: `<progress max="4" value="…">` with
  `aria-label="Password strength: {bucket}"`.
- Theme/density chip groups: `role="radiogroup"` + `role="radio"` +
  `aria-checked`.
- All form actions submit on Enter; secondary actions reachable via Tab.

## Tests

Backend integration tests (testcontainers, real DB, in-memory mailer):

```
backend/tests/security_account.rs

  password_reset_full_happy_path
  password_reset_oauth_user_gets_set_password_template
  password_reset_request_unknown_email_returns_204_silent
  password_reset_throttle_60s_per_email
  password_reset_throttle_5_per_hour_per_ip
  password_reset_confirm_kills_all_sessions
  password_reset_token_expires_after_1h
  password_reset_token_single_use

  email_change_full_happy_path
  email_change_old_address_receives_notification
  email_change_token_expires_1h
  email_change_pending_token_invalidated_on_new_request
  email_change_target_already_taken_returns_error
  email_change_oauth_only_user_blocked_400

  password_change_wrong_current_returns_401
  password_change_oauth_user_no_current_required
  password_change_invalidates_all_sessions_then_creates_fresh
  password_change_old_cookie_no_longer_authenticates

  sessions_list_returns_only_active_sessions
  sessions_list_marks_current_session_first
  sessions_list_returns_parsed_browser_os_from_ua
  revoke_session_succeeds
  revoke_current_session_returns_400
  revoke_other_users_session_returns_404
  sign_out_others_keeps_current_kills_rest
  last_used_at_updates_only_after_5min_threshold

  delete_request_with_correct_password_and_phrase_succeeds
  delete_request_wrong_phrase_returns_400
  delete_request_oauth_user_skips_password_check
  delete_request_idempotent_does_not_extend_grace
  delete_cancel_clears_pending_and_emails
  purge_worker_deletes_users_past_grace
  purge_worker_skips_users_within_grace
  purge_worker_calls_s3_delete_for_owned_keys
  purge_cascades_photos_sessions_appreciations_followers_tokens
  purge_pseudonymises_comments_keeps_body_under_other_photos
  export_json_includes_signed_urls_for_originals_and_thumbnails
  export_signed_urls_become_invalid_after_account_purge
```

Frontend e2e tests (Playwright) — minimum viable:

```
frontend/tests/e2e/security_account.spec.ts

  reset password from sign-in: request → click MailHog link → set new password → land authenticated
  change email from settings → click MailHog link → email swapped + flash visible
  open settings → toggle theme to light → reload → still light
  request deletion → grace banner appears across all auth routes → cancel → banner gone
```

(MailHog is queryable over HTTP at `:8025/api/v2/messages` from
Playwright in dev — useful for asserting on send.)

## Out of scope

Explicitly **not** in Phase 8a (and not implicitly assumed by any UI
in this phase):

- 2FA (TOTP, backup codes, lost-phone recovery flow) — entire family
  deferred. The Email & Security page does not show a 2FA panel.
- Equipment library and Notifications system — left-rail entries are
  visible but disabled with a "SOON" chip; no routes exist.
- GeoIP-derived city/country in the sessions list (MaxMind + 70 MB DB
  not embedded).
- Archive export as a ZIP of binaries (worker + S3 streaming + persisted
  artifact). The JSON stub with 7-day signed URLs is the entire RGPD
  surface in this phase.
- 90-day handle reservation post-deletion — `display_name` (and email)
  are released immediately at purge.
- OAuth account linking UX (a "Connected accounts" section) and OAuth
  re-authentication on destructive actions.
- Session-revoked notification ("your session was ended remotely") —
  silent redirect to `/sign-in` is acceptable for MVP.
- Constant-time response on the password-reset request endpoint — the
  204 is unconditional but timing varies. To be revisited if account
  enumeration via timing becomes a concern.

## References

- Phase 8 design handoff: `~/Downloads/design_handoff_astrophoto 2/`
  (`README - Phase 8.md`, `phase8-settings.jsx`).
- Existing auth surface: `backend/src/auth/`, `backend/migrations/0001_init.sql`.
- Existing engagement schema (touched by the comments pseudonymisation):
  `backend/migrations/0002_engagement.sql`.
- `lettre` SMTP guide: <https://docs.rs/lettre>.
- `woothee` UA parser: <https://docs.rs/woothee>.
- AWS SES SMTP setup: <https://docs.aws.amazon.com/ses/latest/dg/smtp-credentials.html>.
- RGPD Art. 17 (right to erasure) and Art. 20 (data portability):
  <https://gdpr-info.eu/>.
