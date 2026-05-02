# Phase 8a Security & Account Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the security and account-management surface — settings shell with five active sections (Profile, Email & Security, Appearance, Sessions, Delete), public password reset (3 steps), in-settings password change, email change by-link with old-address notification, account deletion with 7-day grace + in-process purge worker, and an RGPD-minimum JSON export. 2FA is deferred.

**Architecture:** New `mail/` module wraps `lettre` SMTP (MailHog in dev, AWS SES SMTP in prod). Two new auth submodules — `password_reset.rs` and `email_change.rs` — host token issuance/consumption. New top-level `users/{profile,preferences,deletion,export}.rs` and `jobs/purge_deletions.rs` cover self-service and the cron-driven hard-delete. Frontend gains a flat `/settings/*` tree (no SvelteKit route groups), a shared `<Modal>`, three `<Settings*>` primitives, public `/reset/*` and `/email-change/[token]`, plus a grace banner injected into the root layout. Theme/density flow through `%theme%`/`%density%` placeholders in `app.html` with a `transformPageChunk` rewrite — SSR-pure.

**Tech Stack:** Existing — axum 0.7, sqlx 0.8 (compile-time SQL via `.sqlx/`), Svelte 5 runes, ts-rs (Rust → TS codegen), aws-sdk-s3 1, argon2 0.5. New — `lettre 0.11` (SMTP), `woothee 0.13` (UA parsing), `sha2 0.10` (token hashing — already pulled transitively, made explicit).

**Spec reference:** `docs/superpowers/specs/2026-05-02-phase-8a-security-account-design.md` — read it before starting; this plan does not re-derive the design decisions.

**Working directory:** `/Volumes/Pascal4Tb/Projects/astrophoto/` (referred to below as `$ROOT`).

**Branch:** create `feat/phase-8a-security` before Task 1 (`git switch -c feat/phase-8a-security`).

**Live infra prereqs:**
- Postgres on `localhost:5434` (`astrophoto`/`astrophoto`/`astrophoto`)
- MinIO on `localhost:9100`/`9101`
- After Task 3, MailHog on `localhost:1025` (SMTP) / `localhost:8025` (UI)
- For all `cargo` commands compiling SQL macros: `export DATABASE_URL=postgres://astrophoto:astrophoto@localhost:5434/astrophoto`
- After every task with new SQL macros: `cargo sqlx prepare -- --lib --tests --bins` and commit `.sqlx/`
- After every task that changes a `#[derive(TS)]` Rust type or `api_types.rs`: run `just types` and commit `frontend/src/lib/api/types.ts`

---

## Task 1: Migration `0003_security_account.sql`

**Files:**
- Create: `backend/migrations/0003_security_account.sql`

- [ ] **Step 1: Create the migration**

```sql
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
```

- [ ] **Step 2: Apply against the live dev DB**

```bash
cd $ROOT/backend
DATABASE_URL=postgres://astrophoto:astrophoto@localhost:5434/astrophoto sqlx migrate run
```

Expected: `Applied 3/migrate security_account`.

- [ ] **Step 3: Verify schema**

```bash
docker compose exec -T postgres psql -U astrophoto -d astrophoto -c "\d sessions"
docker compose exec -T postgres psql -U astrophoto -d astrophoto -c "\d users"
docker compose exec -T postgres psql -U astrophoto -d astrophoto -c "\dt"
```

Expected:
- `sessions` shows the new `last_used_at timestamptz` column.
- `users` shows `theme`, `density`, `password_changed_at`, `pending_deletion_at`.
- `\dt` lists `password_reset_tokens` and `email_change_tokens` alongside the prior tables.

- [ ] **Step 4: Verify the comments FK is now SET NULL**

```bash
docker compose exec -T postgres psql -U astrophoto -d astrophoto \
  -c "select conname, confdeltype from pg_constraint where conname = 'comments_author_id_fkey';"
```

Expected: `confdeltype | n` (`n` = SET NULL; was `c` = CASCADE).

- [ ] **Step 5: Commit**

```bash
cd $ROOT
git add backend/migrations/0003_security_account.sql
git commit -m "feat(backend): migration 0003 for security & account (sessions, tokens, prefs, deletion grace, comments pseudonym)"
```

---

## Task 2: Add Cargo dependencies (`lettre`, `woothee`, `sha2`)

**Files:**
- Modify: `backend/Cargo.toml`

- [ ] **Step 1: Add the three crates**

Open `backend/Cargo.toml`. In the `[dependencies]` block, add (alphabetical-ish, near the existing entries):

```toml
# Mail
lettre = { version = "0.11", default-features = false, features = [
    "tokio1-rustls-tls", "smtp-transport", "builder"
] }

# UA parsing for sessions
woothee = "0.13"

# Token hashing (sha2 is pulled transitively today; pin explicitly).
sha2 = "0.10"
```

- [ ] **Step 2: Run `cargo check` to lock the new deps**

```bash
cd $ROOT/backend
cargo check
```

Expected: clean compile; `Cargo.lock` updated.

- [ ] **Step 3: Commit**

```bash
cd $ROOT
git add backend/Cargo.toml backend/Cargo.lock
git commit -m "chore(backend): add lettre, woothee, sha2 deps for Phase 8a"
```

---

## Task 3: MailHog in `compose.yml` + `.env.example`

**Files:**
- Modify: `compose.yml`
- Modify: `.env.example`

- [ ] **Step 1: Add the `mailhog` service**

Open `compose.yml`. Below the `minio:` service block, add:

```yaml
  mailhog:
    image: mailhog/mailhog:v1.0.1
    ports:
      # 1025 = SMTP, 8025 = web UI (open http://localhost:8025).
      - "1025:1025"
      - "8025:8025"
    restart: unless-stopped
```

- [ ] **Step 2: Bring it up**

```bash
cd $ROOT
docker compose up -d mailhog
curl -s http://localhost:8025/api/v2/messages | head -c 200
```

Expected: a JSON envelope `{"total":0,"count":0,...}` proving MailHog answers on port 8025.

- [ ] **Step 3: Document mail env in `.env.example`**

Open `.env.example`. Append (or update if SMTP-related entries already exist):

```
# Mail (Phase 8a). Dev: MailHog (no auth). Prod: AWS SES SMTP credentials.
SMTP_HOST=localhost
SMTP_PORT=1025
SMTP_USER=
SMTP_PASS=
MAIL_FROM=Astrophoto <noreply@astrophoto.local>
PUBLIC_BASE_URL=http://localhost:5173
```

(`PUBLIC_BASE_URL` may already be present from earlier phases; if so, leave the existing line untouched.)

- [ ] **Step 4: Commit**

```bash
cd $ROOT
git add compose.yml .env.example
git commit -m "chore(infra): add mailhog service + SMTP env vars for Phase 8a"
```

---

## Task 4: `Mailer` struct and `mail/templates.rs`

**Files:**
- Create: `backend/src/mail/mod.rs`
- Create: `backend/src/mail/templates.rs`
- Modify: `backend/src/lib.rs` (add `pub mod mail;`)
- Modify: `backend/src/config.rs` (add SMTP fields)
- Modify: `backend/src/http/mod.rs` (`AppState` gains `mailer: Arc<Mailer>`)
- Modify: `backend/src/main.rs` (build `Mailer::from_env()`, pass into router)
- Test: `backend/tests/mail.rs`

- [ ] **Step 1: Extend `Config`**

Open `backend/src/config.rs` and add to the `Config` struct (after the existing fields):

```rust
pub smtp_host: String,
pub smtp_port: u16,
pub smtp_user: String,        // empty string in dev (MailHog accepts no auth)
pub smtp_pass: String,
pub mail_from: String,        // "Astrophoto <noreply@example>" parseable by lettre
pub public_base_url: String,  // existing — leave as-is if already present
```

If a `Default` impl or test fixture exists nearby, extend it accordingly. Run `cargo check`; address compile errors by updating call sites that build `Config` literals (notably the integration-test `config_for(...)` helpers).

- [ ] **Step 2: Create `backend/src/mail/mod.rs`**

```rust
//! Mailer wrapping `lettre` over SMTP. Dev points at MailHog (no auth);
//! prod points at AWS SES SMTP. Bodies are plain text only — keep the
//! design's "EMAIL PREVIEW · PLAIN TEXT" promise honest.

use std::sync::{Arc, Mutex};

use lettre::{
    Message, Tokio1Executor,
    message::{Mailbox, header::ContentType},
    transport::smtp::{
        AsyncSmtpTransport,
        authentication::Credentials,
    },
    AsyncTransport,
};

use crate::AppError;

pub mod templates;

#[derive(Clone, Debug)]
pub struct SentMail {
    pub to: String,
    pub subject: String,
    pub body: String,
}

#[derive(Clone)]
pub enum Mailer {
    Smtp {
        transport: Arc<AsyncSmtpTransport<Tokio1Executor>>,
        from: Mailbox,
    },
    Memory {
        from: Mailbox,
        outbox: Arc<Mutex<Vec<SentMail>>>,
    },
}

impl Mailer {
    pub fn from_env(cfg: &crate::config::Config) -> Result<Self, AppError> {
        let from: Mailbox = cfg.mail_from.parse().map_err(|e| {
            AppError::internal(format!("invalid MAIL_FROM '{}': {e}", cfg.mail_from))
        })?;

        let mut builder = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&cfg.smtp_host)
            .port(cfg.smtp_port);
        if !cfg.smtp_user.is_empty() {
            builder = builder.credentials(Credentials::new(cfg.smtp_user.clone(), cfg.smtp_pass.clone()));
        }
        Ok(Mailer::Smtp { transport: Arc::new(builder.build()), from })
    }

    pub fn for_test() -> (Self, Arc<Mutex<Vec<SentMail>>>) {
        let outbox = Arc::new(Mutex::new(Vec::new()));
        let from: Mailbox = "test <test@astrophoto.local>".parse().expect("valid mailbox");
        (Mailer::Memory { from, outbox: outbox.clone() }, outbox)
    }

    pub async fn send_plain(&self, to: &str, subject: &str, body: &str) -> Result<(), AppError> {
        let (from, send_smtp) = match self {
            Mailer::Smtp { transport, from } => (from.clone(), Some(transport.clone())),
            Mailer::Memory { from, outbox } => {
                outbox
                    .lock()
                    .map_err(|_| AppError::internal("mail outbox lock poisoned"))?
                    .push(SentMail { to: to.to_string(), subject: subject.to_string(), body: body.to_string() });
                (from.clone(), None)
            }
        };

        if let Some(transport) = send_smtp {
            let to_mailbox: Mailbox = to.parse().map_err(|e| {
                AppError::bad_request(format!("invalid recipient '{to}': {e}"))
            })?;
            let msg = Message::builder()
                .from(from)
                .to(to_mailbox)
                .subject(subject)
                .header(ContentType::TEXT_PLAIN)
                .body(body.to_string())
                .map_err(|e| AppError::internal(format!("mail build failed: {e}")))?;
            transport.send(msg).await.map_err(|e| AppError::internal(format!("smtp send failed: {e}")))?;
        }
        Ok(())
    }
}
```

If `AppError` does not yet expose `internal(impl Into<String>)` and `bad_request(impl Into<String>)` constructors, add them in `backend/src/error.rs` mirroring the existing helper pattern (each is a 3-line method returning the matching variant).

- [ ] **Step 3: Create `backend/src/mail/templates.rs`**

```rust
//! Plain-text email templates. Each function returns (subject, body).
//! Bodies are short, mono-friendly, and stable copy.

pub fn password_reset(display_name: &str, link: &str, has_password: bool) -> (String, String) {
    let subject = if has_password {
        "Reset your Astrophoto password"
    } else {
        "Set a password for your Astrophoto account"
    };
    let intro = if has_password {
        "We received a request to reset your password."
    } else {
        "You don't have a password yet — set one to sign in without Google."
    };
    let body = format!(
        "Hello {display_name},\n\n\
         {intro} Open this link to continue:\n\n\
         {link}\n\n\
         This link is single-use and expires in one hour. If you didn't request \
         this, you can ignore this message — nothing changes.\n\n\
         Clear skies,\n\
         The Astrophoto archive\n"
    );
    (subject.to_string(), body)
}

pub fn email_change_request(current_email: &str, link: &str) -> (String, String) {
    let subject = "Confirm your new Astrophoto email".to_string();
    let body = format!(
        "Hello,\n\n\
         A request was made to change the Astrophoto account currently registered as \
         {current_email} to this address. Open the link below to confirm:\n\n\
         {link}\n\n\
         This link is single-use and expires in one hour. If you didn't request this, \
         ignore this message — nothing changes until the link is clicked.\n\n\
         Clear skies,\n\
         The Astrophoto archive\n"
    );
    (subject, body)
}

pub fn email_change_notification(masked_new: &str, occurred_at: &str) -> (String, String) {
    let subject = "Your Astrophoto email was changed".to_string();
    let body = format!(
        "Hello,\n\n\
         Your Astrophoto account email was changed to {masked_new} at {occurred_at}.\n\n\
         If this wasn't you, reply immediately or use \"Forgot password\" \
         on the sign-in page to recover access.\n\n\
         Clear skies,\n\
         The Astrophoto archive\n"
    );
    (subject, body)
}

pub fn account_deletion_scheduled(display_name: &str, when_human: &str, cancel_link: &str) -> (String, String) {
    let subject = "Your Astrophoto account is scheduled for deletion".to_string();
    let body = format!(
        "Hello {display_name},\n\n\
         Your Astrophoto account is scheduled for permanent deletion on {when_human}.\n\n\
         If you change your mind, sign in within the next 7 days and click \
         \"Cancel deletion\":\n\n\
         {cancel_link}\n\n\
         After the grace period, your photos and account data are erased and the \
         operation cannot be undone.\n\n\
         Clear skies,\n\
         The Astrophoto archive\n"
    );
    (subject, body)
}

pub fn account_deletion_cancelled(display_name: &str) -> (String, String) {
    let subject = "Your Astrophoto account deletion was cancelled".to_string();
    let body = format!(
        "Hello {display_name},\n\n\
         Your account deletion request has been cancelled. Welcome back.\n\n\
         Clear skies,\n\
         The Astrophoto archive\n"
    );
    (subject, body)
}

/// Mask `marie@example.com` → `mar***@example.com`. Used in the
/// notification-to-old-address template.
pub fn mask_email(email: &str) -> String {
    if let Some((local, domain)) = email.split_once('@') {
        let prefix: String = local.chars().take(3).collect();
        format!("{prefix}***@{domain}")
    } else {
        "***".into()
    }
}
```

- [ ] **Step 4: Wire mailer into `AppState` and `main.rs`**

Open `backend/src/lib.rs` and add `pub mod mail;` next to the other `pub mod` declarations.

Open `backend/src/http/mod.rs`. Extend `AppState`:

```rust
pub struct AppState {
    pub pool: PgPool,
    pub config: Arc<Config>,
    pub storage: Arc<dyn crate::storage::Storage>,
    pub mailer: Arc<crate::mail::Mailer>,
}
```

Update the `router(...)` signature and the inner `AppState { ... }` literal so the mailer is threaded through (signature: `pub fn router(pool: PgPool, config: Config, storage: Arc<dyn ...>, mailer: Arc<crate::mail::Mailer>) -> Router`).

Open `backend/src/main.rs`. After `Config` is loaded and before the router is built, instantiate the mailer:

```rust
let mailer = std::sync::Arc::new(astrophoto::mail::Mailer::from_env(&config)?);
```

Then pass `mailer` into `http::router(...)`.

- [ ] **Step 5: Update existing test harnesses**

Open `backend/tests/auth.rs`, `backend/tests/photos.rs`, `backend/tests/engagement.rs` (and any other integration test that builds the router). Wherever the router is constructed via `http::router(pool, config, storage)`, change to:

```rust
let (mailer, _outbox) = astrophoto::mail::Mailer::for_test();
let app = http::router(pool, config, std::sync::Arc::new(storage), std::sync::Arc::new(mailer));
```

(Discard `_outbox` in tests that don't assert on mail; rebind to `outbox` where they do.)

- [ ] **Step 6: Write the failing mailer test**

Create `backend/tests/mail.rs`:

```rust
//! In-memory mailer assertions. No DB, no SMTP.
#![allow(clippy::unwrap_used, clippy::panic)]

use astrophoto::mail::{Mailer, templates};

#[tokio::test]
async fn memory_mailer_records_sends() {
    let (mailer, outbox) = Mailer::for_test();
    mailer.send_plain("alice@example.com", "Hi", "Body").await.unwrap();
    let sent = outbox.lock().unwrap();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].to, "alice@example.com");
    assert_eq!(sent[0].subject, "Hi");
    assert_eq!(sent[0].body, "Body");
}

#[test]
fn mask_email_keeps_first_three_chars() {
    assert_eq!(templates::mask_email("marie.dubois@example.fr"), "mar***@example.fr");
    assert_eq!(templates::mask_email("ab@x.io"), "ab***@x.io");
    assert_eq!(templates::mask_email("not-an-email"), "***");
}

#[test]
fn password_reset_uses_set_subject_when_no_password() {
    let (subject_set, body_set) = templates::password_reset("Marie", "https://x/r/abc", false);
    assert!(subject_set.contains("Set a password"));
    assert!(body_set.contains("https://x/r/abc"));
    let (subject_reset, _) = templates::password_reset("Marie", "https://x/r/abc", true);
    assert!(subject_reset.contains("Reset"));
}
```

- [ ] **Step 7: Run the test**

```bash
cd $ROOT/backend
cargo test --test mail -- --nocapture
```

Expected: 3 passing tests.

- [ ] **Step 8: Commit**

```bash
cd $ROOT
git add backend/src/mail/ \
        backend/src/lib.rs \
        backend/src/config.rs \
        backend/src/http/mod.rs \
        backend/src/main.rs \
        backend/src/error.rs \
        backend/tests/mail.rs \
        backend/tests/auth.rs backend/tests/photos.rs backend/tests/engagement.rs
git commit -m "feat(backend/mail): add Mailer (lettre+SMTP) with in-memory test variant + templates"
```

---

## Task 5: `password_reset` request endpoint

**Files:**
- Create: `backend/src/auth/password_reset.rs`
- Modify: `backend/src/auth/mod.rs` (add `pub mod password_reset;`)
- Modify: `backend/src/http/mod.rs` (mount `POST /api/auth/password-reset/request`)
- Test: `backend/tests/security_account.rs` (new file)

- [ ] **Step 1: Create `backend/src/auth/password_reset.rs`**

```rust
//! Password reset: 3-step public flow.
//! - request: issue a token, email a link, return 204 unconditionally
//!   (anti-enumeration). Throttled per-email and per-IP.
//! - confirm: set a new password, kill all sessions, auto-login (Task 6).

use axum::{
    Json,
    extract::{ConnectInfo, State},
    http::StatusCode,
    response::IntoResponse,
};
use base64::Engine;
use rand::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::net::SocketAddr;

use crate::AppError;
use crate::http::AppState;
use crate::mail::templates;

#[derive(Deserialize)]
pub struct RequestBody {
    pub email: String,
}

const TTL_HOURS: i64 = 1;
const PER_EMAIL_COOLDOWN_SECS: i64 = 60;
const PER_HOUR_CAP: i64 = 5;

pub async fn request(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(body): Json<RequestBody>,
) -> Result<impl IntoResponse, AppError> {
    let user = sqlx::query!(
        r#"select id, email as "email!: String", display_name, password_hash
             from users where email = $1"#,
        body.email
    )
    .fetch_optional(&state.pool)
    .await?;

    if let Some(u) = user {
        // Throttle: skip silently when the most recent token for this email
        // was issued < cooldown ago, or > cap have been issued in the last hour.
        let cooldown_hit = sqlx::query_scalar!(
            "select exists(
                select 1 from password_reset_tokens
                where user_id = $1
                  and created_at > now() - make_interval(secs => $2)
            )",
            u.id,
            PER_EMAIL_COOLDOWN_SECS as f64
        )
        .fetch_one(&state.pool)
        .await?
        .unwrap_or(false);

        let hour_cap_hit = sqlx::query_scalar!(
            "select count(*) >= $2 from password_reset_tokens
              where (user_id = $1 or request_ip = $3)
                and created_at > now() - interval '1 hour'",
            u.id,
            PER_HOUR_CAP,
            ipnetwork::IpNetwork::from(addr.ip())
        )
        .fetch_one(&state.pool)
        .await?
        .unwrap_or(false);

        if !cooldown_hit && !hour_cap_hit {
            // Issue token.
            let mut raw = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut raw);
            let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw);
            let hash: Vec<u8> = Sha256::digest(token.as_bytes()).to_vec();

            sqlx::query!(
                "insert into password_reset_tokens (token_hash, user_id, expires_at, request_ip)
                  values ($1, $2, now() + make_interval(hours => $3), $4)",
                hash,
                u.id,
                TTL_HOURS as f64,
                ipnetwork::IpNetwork::from(addr.ip())
            )
            .execute(&state.pool)
            .await?;

            // Build the link (frontend handles the page).
            let link = format!("{}/reset/{}", state.config.public_base_url.trim_end_matches('/'), token);
            let (subject, body) =
                templates::password_reset(&u.display_name, &link, u.password_hash.is_some());
            state.mailer.send_plain(&u.email, &subject, &body).await?;
        }
    }

    Ok(StatusCode::NO_CONTENT)
}
```

- [ ] **Step 2: Wire the module + the route**

Open `backend/src/auth/mod.rs` and add:
```rust
pub mod password_reset;
```

Open `backend/src/http/mod.rs`. In the router chain (next to the existing `/api/auth/...` lines), add:
```rust
.route(
    "/api/auth/password-reset/request",
    post(crate::auth::password_reset::request),
)
```

The handler uses `ConnectInfo<SocketAddr>` — confirm that `axum::serve(...)` in `main.rs` is built with `into_make_service_with_connect_info::<SocketAddr>()`. If it currently uses `into_make_service()`, change it; otherwise the route panics at request time.

- [ ] **Step 3: Refresh the sqlx cache**

```bash
cd $ROOT/backend
DATABASE_URL=postgres://astrophoto:astrophoto@localhost:5434/astrophoto \
  cargo sqlx prepare -- --lib --tests --bins
```

- [ ] **Step 4: Create the integration test file with the first test**

Create `backend/tests/security_account.rs`. (This file accumulates Phase 8a tests across many tasks — append, don't replace, in subsequent tasks.)

```rust
//! Integration tests for Phase 8a security & account flows.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use astrophoto::mail::{Mailer, SentMail};
use astrophoto::storage::MemoryStorage;
use astrophoto::{Config, db, http};
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt as _;
use serde_json::json;
use std::sync::Mutex;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PgImage;
use tower::ServiceExt;

fn config_for(url: &str) -> Config {
    Config {
        bind: "127.0.0.1:0".into(),
        log: "info".into(),
        database_url: url.into(),
        session_domain: "localhost".into(),
        session_secure: false,
        public_base_url: "http://localhost:5173".into(),
        smtp_host: "unused-in-tests".into(),
        smtp_port: 1025,
        smtp_user: String::new(),
        smtp_pass: String::new(),
        mail_from: "test <test@astrophoto.local>".into(),
        // Carry over remaining existing fields with their existing test defaults
        // (S3, OAuth, etc.); copy from the existing engagement test config_for.
        ..Default::default()
    }
}

async fn boot() -> (axum::Router, Arc<Mutex<Vec<SentMail>>>) {
    let pg = PgImage::default().start().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@127.0.0.1:{port}/postgres");
    let pool = db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();

    let cfg = config_for(&url);
    let storage = Arc::new(MemoryStorage::default());
    let (mailer, outbox) = Mailer::for_test();
    // Return the bare Router; tests will inject ConnectInfo per-request via
    // `req.extensions_mut().insert(SocketAddr)`. The `into_make_service_with_connect_info`
    // call belongs to `main.rs`, not the tests.
    let app: axum::Router = http::router(pool, cfg, storage, Arc::new(mailer));
    (app, outbox)
}

/// Construct a request whose ConnectInfo is set to a stable test IP.
fn req_with_ip(method: &str, path: &str, body: serde_json::Value) -> Request<Body> {
    let mut r = Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    r.extensions_mut().insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
    r
}

#[tokio::test]
async fn password_reset_request_unknown_email_returns_204_silent() {
    let (app, outbox) = boot().await;
    let resp = app
        .oneshot(req_with_ip("POST", "/api/auth/password-reset/request",
                             json!({"email": "ghost@nowhere.test"})))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    assert!(outbox.lock().unwrap().is_empty(),
        "no mail must be sent for unknown emails");
}
```

If `Default` is not implemented on `Config`, prefer constructing the full struct literal explicitly here (copy the field set from `backend/tests/engagement.rs`'s `config_for`, then add the new SMTP fields).

- [ ] **Step 5: Run the test**

```bash
cd $ROOT/backend
cargo test --test security_account password_reset_request_unknown_email -- --nocapture
```

Expected: PASS. `outbox` is empty.

- [ ] **Step 6: Add the throttle + happy-path tests**

Append to `backend/tests/security_account.rs`:

```rust
async fn signup(app: &axum::Router, email: &str, password: &str) {
    let body = json!({"email": email, "password": password, "display_name": "Marie"});
    let resp = app.clone()
        .oneshot(req_with_ip("POST", "/api/auth/signup", body))
        .await.unwrap();
    assert!(resp.status().is_success(), "signup must succeed (got {})", resp.status());
}

#[tokio::test]
async fn password_reset_request_known_email_sends_one_mail() {
    let (app, outbox) = boot().await;
    signup(&app, "marie@example.com", "longenoughpw1").await;

    let resp = app.clone()
        .oneshot(req_with_ip("POST", "/api/auth/password-reset/request",
                             json!({"email": "marie@example.com"})))
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let sent = outbox.lock().unwrap();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].to, "marie@example.com");
    assert!(sent[0].subject.contains("Reset"));
    assert!(sent[0].body.contains("/reset/"));
}

#[tokio::test]
async fn password_reset_throttle_60s_per_email() {
    let (app, outbox) = boot().await;
    signup(&app, "marie@example.com", "longenoughpw1").await;

    for _ in 0..3 {
        let resp = app.clone()
            .oneshot(req_with_ip("POST", "/api/auth/password-reset/request",
                                 json!({"email": "marie@example.com"})))
            .await.unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    }
    assert_eq!(outbox.lock().unwrap().len(), 1, "only the first request emails");
}

// (The OAuth-only "set a password" template path is exercised in Task 6,
//  once `boot()` exposes `pool` and we can `INSERT INTO users (..., password_hash) VALUES (..., NULL)`
//  directly.)
```

- [ ] **Step 7: Run the new tests**

```bash
cd $ROOT/backend
cargo test --test security_account password_reset -- --nocapture
```

Expected: 3 PASS (`password_reset_request_known_email_sends_one_mail`, `password_reset_throttle_60s_per_email`, plus the silent-204 test from Step 5).

- [ ] **Step 8: Commit**

```bash
cd $ROOT
git add backend/src/auth/password_reset.rs \
        backend/src/auth/mod.rs \
        backend/src/http/mod.rs \
        backend/src/main.rs \
        backend/.sqlx/ \
        backend/tests/security_account.rs
git commit -m "feat(backend/auth): add password-reset request endpoint with anti-enumeration + throttle"
```

---

## Task 6: Password reset confirm + in-settings password change

**Files:**
- Modify: `backend/src/auth/password_reset.rs` (add `confirm` handler)
- Create: `backend/src/auth/password_change.rs`
- Modify: `backend/src/auth/mod.rs` (add `pub mod password_change;`)
- Modify: `backend/src/http/mod.rs` (mount 2 routes)
- Modify: `backend/src/auth/password.rs` (expose a `validate_strength(pwd)` helper if not present)
- Create: `backend/assets/common-passwords.txt` (1,000 most-common passwords, one per line)
- Modify: `backend/tests/security_account.rs`

- [ ] **Step 1: Embed the common-password dictionary**

Download or paste a 1,000-line list (e.g. SecLists `10-million-password-list-top-1000`) into `backend/assets/common-passwords.txt`. One password per line, no header.

```bash
cd $ROOT/backend
mkdir -p assets
# Manually paste content; or fetch and pipe through `head -n 1000 > assets/common-passwords.txt`.
wc -l assets/common-passwords.txt   # expect: 1000
```

- [ ] **Step 2: Add the validator helper**

Open `backend/src/auth/password.rs`. Add (or extend, if a stub exists):

```rust
const COMMON: &str = include_str!("../../assets/common-passwords.txt");

pub fn validate_strength(pwd: &str) -> Result<(), &'static str> {
    if pwd.chars().count() < 12 {
        return Err("password_too_short");
    }
    let lower = pwd.to_ascii_lowercase();
    if COMMON.lines().any(|p| p.trim() == lower) {
        return Err("password_too_common");
    }
    Ok(())
}
```

- [ ] **Step 3: Add the `confirm` handler in `password_reset.rs`**

Append to `backend/src/auth/password_reset.rs`:

```rust
use axum::http::HeaderMap;
use crate::auth::session;
use crate::auth::password::{hash_password, validate_strength};

#[derive(Deserialize)]
pub struct ConfirmBody {
    pub token: String,
    pub new_password: String,
}

pub async fn confirm(
    State(state): State<AppState>,
    Json(body): Json<ConfirmBody>,
) -> Result<impl IntoResponse, AppError> {
    validate_strength(&body.new_password)
        .map_err(|e| AppError::bad_request(e))?;

    let hash: Vec<u8> = Sha256::digest(body.token.as_bytes()).to_vec();
    let row = sqlx::query!(
        r#"select user_id, expires_at, used_at
             from password_reset_tokens
            where token_hash = $1"#,
        hash
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::gone("expired_or_used"))?;

    if row.used_at.is_some() || row.expires_at < chrono::Utc::now() {
        return Err(AppError::gone("expired_or_used"));
    }

    let new_password = body.new_password.clone();
    let pwd_hash = tokio::task::spawn_blocking(move || hash_password(&new_password))
        .await
        .map_err(|e| AppError::internal(format!("argon2 join: {e}")))??;

    let mut tx = state.pool.begin().await?;
    sqlx::query!(
        "update users set password_hash = $1, password_changed_at = now() where id = $2",
        pwd_hash, row.user_id
    ).execute(&mut *tx).await?;
    sqlx::query!(
        "update password_reset_tokens set used_at = now() where token_hash = $1",
        hash
    ).execute(&mut *tx).await?;
    sqlx::query!("delete from sessions where user_id = $1", row.user_id)
        .execute(&mut *tx).await?;
    tx.commit().await?;

    // Auto-login: create a fresh session and return Set-Cookie.
    let cookie = session::create_session(&state, row.user_id).await?;
    let mut headers = HeaderMap::new();
    headers.insert(header::SET_COOKIE, cookie.parse().map_err(|_| AppError::internal("bad cookie"))?);
    Ok((StatusCode::NO_CONTENT, headers))
}
```

If `session::create_session(&state, user_id)` does not yet exist as a public helper, extract it from `auth/login.rs` (the body that builds the session row + the cookie header). Both login and reset must produce identical cookies — extracting is the right move.

If `AppError::gone(impl Into<String>)` doesn't exist yet, add a `Gone` variant to `AppError` and a `gone(...)` constructor mirroring `bad_request`.

- [ ] **Step 4: Create `backend/src/auth/password_change.rs`**

```rust
//! In-settings password change. Requires session + (current password OR
//! none, if the user is OAuth-only and has no password yet). Always
//! deletes every existing session and issues a fresh one (rotation).

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::IntoResponse,
};
use serde::Deserialize;

use crate::AppError;
use crate::auth::middleware::CurrentUser;
use crate::auth::password::{hash_password, validate_strength, verify_password};
use crate::auth::session;
use crate::http::AppState;

#[derive(Deserialize)]
pub struct Body {
    pub current_password: Option<String>,
    pub new_password: String,
}

pub async fn change(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<Body>,
) -> Result<impl IntoResponse, AppError> {
    validate_strength(&body.new_password).map_err(AppError::bad_request)?;

    // Verify current password only if the user actually has one.
    let row = sqlx::query!(
        "select password_hash from users where id = $1",
        user.id
    ).fetch_one(&state.pool).await?;

    if row.password_hash.is_some() {
        let current = body.current_password.ok_or_else(||
            AppError::unauthorized("wrong_current_password"))?;
        let stored = row.password_hash.clone().expect("checked is_some");
        let ok = tokio::task::spawn_blocking(move || verify_password(&current, &stored))
            .await.map_err(|e| AppError::internal(format!("argon2 join: {e}")))??;
        if !ok {
            return Err(AppError::unauthorized("wrong_current_password"));
        }
    }
    // OAuth-only path: no current_password expected.

    let new_pwd = body.new_password.clone();
    let pwd_hash = tokio::task::spawn_blocking(move || hash_password(&new_pwd))
        .await.map_err(|e| AppError::internal(format!("argon2 join: {e}")))??;

    let mut tx = state.pool.begin().await?;
    sqlx::query!(
        "update users set password_hash = $1, password_changed_at = now() where id = $2",
        pwd_hash, user.id
    ).execute(&mut *tx).await?;
    // Pure rotation: kill EVERY session including the current one, then
    // issue a fresh one for this browser. The browser keeps working
    // through the new cookie; other devices are signed out.
    sqlx::query!("delete from sessions where user_id = $1", user.id)
        .execute(&mut *tx).await?;
    tx.commit().await?;

    let cookie = session::create_session(&state, user.id).await?;
    let mut headers = HeaderMap::new();
    headers.insert(header::SET_COOKIE, cookie.parse()
        .map_err(|_| AppError::internal("bad cookie"))?);
    Ok((StatusCode::NO_CONTENT, headers))
}
```

- [ ] **Step 5: Wire the routes**

Open `backend/src/auth/mod.rs` and add:
```rust
pub mod password_change;
```

Open `backend/src/http/mod.rs` and add to the chain:
```rust
.route(
    "/api/auth/password-reset/confirm",
    post(crate::auth::password_reset::confirm),
)
.route(
    "/api/me/password-change",
    post(crate::auth::password_change::change),
)
```

- [ ] **Step 6: Refresh sqlx + check**

```bash
cd $ROOT/backend
DATABASE_URL=postgres://astrophoto:astrophoto@localhost:5434/astrophoto \
  cargo sqlx prepare -- --lib --tests --bins
cargo check
```

- [ ] **Step 7: Append integration tests**

Refactor `boot()` in `backend/tests/security_account.rs` to also return the `PgPool`:

```rust
async fn boot() -> (axum::Router, sqlx::PgPool, Arc<Mutex<Vec<SentMail>>>) {
    // ...same body...
    (app, pool, outbox)
}
```

Update existing call sites accordingly. Then append:

```rust
async fn signin(app: &axum::Router, email: &str, password: &str) -> String {
    // Returns the session cookie value (the `session=...` substring).
    let resp = app.clone()
        .oneshot(req_with_ip("POST", "/api/auth/login",
                             json!({"email": email, "password": password})))
        .await.unwrap();
    assert!(resp.status().is_success());
    resp.headers().get(header::SET_COOKIE).unwrap().to_str().unwrap().to_string()
}

async fn latest_token(pool: &sqlx::PgPool, email: &str) -> String {
    // The token raw value never lands in DB. We fetch the most recent token row
    // by user_id and *re-derive* nothing — instead, the test extracts the link
    // from the outbox and pulls the token from the URL.
    let mail_re = regex::Regex::new(r"/reset/([A-Za-z0-9_-]{43})").unwrap();
    // (Tests will inspect outbox directly; this function is unused. Remove if
    //  not referenced after Step 8.)
    let _ = (pool, email, mail_re);
    String::new()
}

#[tokio::test]
async fn password_reset_full_happy_path() {
    let (app, _pool, outbox) = boot().await;
    signup(&app, "marie@example.com", "longenoughpw1").await;

    // Request reset.
    app.clone().oneshot(req_with_ip("POST", "/api/auth/password-reset/request",
        json!({"email": "marie@example.com"}))).await.unwrap();
    let body = outbox.lock().unwrap()[0].body.clone();
    let token = body
        .lines()
        .find_map(|l| l.strip_prefix("http://localhost:5173/reset/")
                       .or_else(|| l.find("/reset/").map(|i| &l[i + 7..])))
        .expect("token in mail body");

    // Confirm.
    let resp = app.clone()
        .oneshot(req_with_ip("POST", "/api/auth/password-reset/confirm",
            json!({"token": token.trim(), "new_password": "evenlongerpw12"})))
        .await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    let cookie = resp.headers().get(header::SET_COOKIE).unwrap();
    assert!(cookie.to_str().unwrap().contains("session"));

    // Old password no longer works; new does.
    let resp_old = app.clone()
        .oneshot(req_with_ip("POST", "/api/auth/login",
            json!({"email": "marie@example.com", "password": "longenoughpw1"})))
        .await.unwrap();
    assert_eq!(resp_old.status(), StatusCode::UNAUTHORIZED);

    let resp_new = app.clone()
        .oneshot(req_with_ip("POST", "/api/auth/login",
            json!({"email": "marie@example.com", "password": "evenlongerpw12"})))
        .await.unwrap();
    assert!(resp_new.status().is_success());
}

#[tokio::test]
async fn password_reset_token_single_use() {
    let (app, _pool, outbox) = boot().await;
    signup(&app, "marie@example.com", "longenoughpw1").await;
    app.clone().oneshot(req_with_ip("POST", "/api/auth/password-reset/request",
        json!({"email": "marie@example.com"}))).await.unwrap();
    let token = outbox.lock().unwrap()[0].body
        .split("/reset/").nth(1).unwrap().split_whitespace().next().unwrap().to_string();

    // First confirm: 204.
    let r1 = app.clone()
        .oneshot(req_with_ip("POST", "/api/auth/password-reset/confirm",
            json!({"token": token, "new_password": "evenlongerpw12"})))
        .await.unwrap();
    assert_eq!(r1.status(), StatusCode::NO_CONTENT);
    // Second confirm: 410 Gone.
    let r2 = app
        .oneshot(req_with_ip("POST", "/api/auth/password-reset/confirm",
            json!({"token": token, "new_password": "anotherlongerpw9"})))
        .await.unwrap();
    assert_eq!(r2.status(), StatusCode::GONE);
}

#[tokio::test]
async fn password_change_invalidates_all_sessions_then_creates_fresh() {
    let (app, pool, _outbox) = boot().await;
    signup(&app, "marie@example.com", "longenoughpw1").await;
    let cookie = signin(&app, "marie@example.com", "longenoughpw1").await;

    let req = Request::builder()
        .method("POST")
        .uri("/api/me/password-change")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, cookie.split(';').next().unwrap())
        .body(Body::from(json!({
            "current_password": "longenoughpw1",
            "new_password": "evenlongerpw12"
        }).to_string())).unwrap();
    let mut req = req;
    req.extensions_mut().insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
    assert!(resp.headers().contains_key(header::SET_COOKIE),
        "must rotate the cookie");

    // Exactly one row in sessions for this user.
    let count: i64 = sqlx::query_scalar!(
        "select count(*) as \"c!\" from sessions s join users u on u.id = s.user_id
          where u.email = $1", "marie@example.com")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn password_reset_oauth_user_gets_set_password_template() {
    let (app, pool, outbox) = boot().await;
    // OAuth-only user: signup never happened; we insert directly with NULL password_hash.
    sqlx::query!(
        "insert into users (email, display_name, password_hash) values ($1, $2, null)",
        "oauth@x.test", "OAuthie"
    ).execute(&pool).await.unwrap();
    outbox.lock().unwrap().clear();

    app.oneshot(req_with_ip("POST", "/api/auth/password-reset/request",
        json!({"email": "oauth@x.test"}))).await.unwrap();

    let sent = outbox.lock().unwrap().clone();
    assert_eq!(sent.len(), 1);
    assert!(sent[0].subject.contains("Set a password"),
        "OAuth-only user should get the set-password subject, got: {}", sent[0].subject);
}

#[tokio::test]
async fn password_change_wrong_current_returns_401() {
    let (app, _pool, _outbox) = boot().await;
    signup(&app, "marie@example.com", "longenoughpw1").await;
    let cookie = signin(&app, "marie@example.com", "longenoughpw1").await;

    let mut req = Request::builder()
        .method("POST").uri("/api/me/password-change")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, cookie.split(';').next().unwrap())
        .body(Body::from(json!({
            "current_password": "WRONG", "new_password": "evenlongerpw12"
        }).to_string())).unwrap();
    req.extensions_mut().insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
```

(Add `regex = "1"` to `[dev-dependencies]` if not already present.)

- [ ] **Step 8: Run tests**

```bash
cd $ROOT/backend
cargo test --test security_account password -- --nocapture
```

Expected: all `password_*` tests PASS.

- [ ] **Step 9: Commit**

```bash
cd $ROOT
git add backend/src/auth/password_reset.rs \
        backend/src/auth/password_change.rs \
        backend/src/auth/password.rs \
        backend/src/auth/mod.rs \
        backend/src/auth/session.rs \
        backend/src/auth/login.rs \
        backend/src/error.rs \
        backend/src/http/mod.rs \
        backend/assets/common-passwords.txt \
        backend/.sqlx/ \
        backend/tests/security_account.rs \
        backend/Cargo.toml
git commit -m "feat(backend/auth): add password-reset confirm + in-settings password change (full session rotation)"
```

---

## Task 7: Email change flow (request + confirm)

**Files:**
- Create: `backend/src/auth/email_change.rs`
- Modify: `backend/src/auth/mod.rs` (`pub mod email_change;`)
- Modify: `backend/src/http/mod.rs` (mount 2 routes)
- Modify: `backend/tests/security_account.rs` (append tests)

- [ ] **Step 1: Create `backend/src/auth/email_change.rs`**

```rust
//! Email change: request issues a token to the *new* address; confirm
//! swaps it and notifies the *old* address.

use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
};
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::AppError;
use crate::auth::middleware::CurrentUser;
use crate::auth::password::verify_password;
use crate::http::AppState;
use crate::mail::templates;

const TTL_HOURS: i64 = 1;

#[derive(Deserialize)]
pub struct RequestBody {
    pub new_email: String,
    pub current_password: String,
}

#[derive(Serialize)]
pub struct ConfirmResponse {
    pub status: String,  // "success" | "expired" | "taken"
}

pub async fn request(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<RequestBody>,
) -> Result<impl IntoResponse, AppError> {
    let row = sqlx::query!(
        "select email as \"email!: String\", password_hash from users where id = $1",
        user.id
    ).fetch_one(&state.pool).await?;

    let pwd_hash = row.password_hash.ok_or_else(||
        AppError::bad_request("no_password_set"))?;
    let current = body.current_password.clone();
    let ok = tokio::task::spawn_blocking(move || verify_password(&current, &pwd_hash))
        .await.map_err(|e| AppError::internal(format!("argon2 join: {e}")))??;
    if !ok {
        return Err(AppError::unauthorized("wrong_password"));
    }

    let new_email = body.new_email.trim().to_lowercase();
    if new_email == row.email.to_lowercase() {
        return Err(AppError::bad_request("same_email"));
    }
    if !new_email.contains('@') {
        return Err(AppError::bad_request("invalid_email"));
    }

    // Invalidate any prior pending token for this user.
    sqlx::query!(
        "update email_change_tokens set used_at = now()
          where user_id = $1 and used_at is null",
        user.id
    ).execute(&state.pool).await?;

    let mut raw = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut raw);
    let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw);
    let hash: Vec<u8> = Sha256::digest(token.as_bytes()).to_vec();

    sqlx::query!(
        "insert into email_change_tokens (token_hash, user_id, new_email, expires_at)
          values ($1, $2, $3, now() + make_interval(hours => $4))",
        hash, user.id, new_email, TTL_HOURS as f64
    ).execute(&state.pool).await?;

    let link = format!(
        "{}/email-change/{}",
        state.config.public_base_url.trim_end_matches('/'),
        token
    );
    let (subject, body) = templates::email_change_request(&row.email, &link);
    state.mailer.send_plain(&new_email, &subject, &body).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct ConfirmBody {
    pub token: String,
}

pub async fn confirm(
    State(state): State<AppState>,
    Json(body): Json<ConfirmBody>,
) -> Result<Json<ConfirmResponse>, AppError> {
    let hash: Vec<u8> = Sha256::digest(body.token.as_bytes()).to_vec();
    let row = sqlx::query!(
        r#"select user_id, new_email as "new_email!: String", expires_at, used_at
             from email_change_tokens where token_hash = $1"#,
        hash
    ).fetch_optional(&state.pool).await?;

    let row = match row {
        Some(r) if r.used_at.is_none() && r.expires_at > chrono::Utc::now() => r,
        _ => return Ok(Json(ConfirmResponse { status: "expired".into() })),
    };

    let mut tx = state.pool.begin().await?;
    let old = sqlx::query!(
        "select email as \"email!: String\" from users where id = $1",
        row.user_id
    ).fetch_one(&mut *tx).await?;

    let updated = sqlx::query!(
        "update users set email = $1 where id = $2",
        row.new_email, row.user_id
    ).execute(&mut *tx).await;

    match updated {
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            tx.rollback().await.ok();
            return Ok(Json(ConfirmResponse { status: "taken".into() }));
        }
        Err(e) => return Err(e.into()),
        Ok(_) => {}
    }
    sqlx::query!("update email_change_tokens set used_at = now() where token_hash = $1", hash)
        .execute(&mut *tx).await?;
    tx.commit().await?;

    let masked = templates::mask_email(&row.new_email);
    let when = chrono::Utc::now().format("%Y-%m-%d %H:%M UTC").to_string();
    let (subject, body) = templates::email_change_notification(&masked, &when);
    state.mailer.send_plain(&old.email, &subject, &body).await?;

    Ok(Json(ConfirmResponse { status: "success".into() }))
}
```

- [ ] **Step 2: Wire**

`backend/src/auth/mod.rs`: `pub mod email_change;`
`backend/src/http/mod.rs`:
```rust
.route("/api/me/email-change/request", post(crate::auth::email_change::request))
.route("/api/auth/email-change/confirm", post(crate::auth::email_change::confirm))
```

- [ ] **Step 3: sqlx prepare + check**

```bash
cd $ROOT/backend
DATABASE_URL=postgres://astrophoto:astrophoto@localhost:5434/astrophoto \
  cargo sqlx prepare -- --lib --tests --bins
cargo check
```

- [ ] **Step 4: Append integration tests**

Append to `backend/tests/security_account.rs`:

```rust
#[tokio::test]
async fn email_change_full_happy_path() {
    let (app, pool, outbox) = boot().await;
    signup(&app, "marie@old.test", "longenoughpw1").await;
    let cookie = signin(&app, "marie@old.test", "longenoughpw1").await;
    outbox.lock().unwrap().clear();

    let mut req = Request::builder()
        .method("POST").uri("/api/me/email-change/request")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, cookie.split(';').next().unwrap())
        .body(Body::from(json!({"new_email": "marie@new.test",
                                "current_password": "longenoughpw1"}).to_string())).unwrap();
    req.extensions_mut().insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    let sent = outbox.lock().unwrap().clone();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].to, "marie@new.test");
    let token = sent[0].body.split("/email-change/").nth(1).unwrap()
        .split_whitespace().next().unwrap().to_string();

    let resp = app.clone().oneshot(req_with_ip("POST", "/api/auth/email-change/confirm",
        json!({"token": token}))).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(
        &resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["status"], "success");

    // Old address must receive the notification.
    let sent = outbox.lock().unwrap().clone();
    assert!(sent.iter().any(|m| m.to == "marie@old.test"
        && m.subject.contains("changed")));

    // Email row was actually updated.
    let row = sqlx::query!("select email as \"email!: String\" from users where email = $1",
                            "marie@new.test").fetch_one(&pool).await.unwrap();
    assert_eq!(row.email, "marie@new.test");
}

#[tokio::test]
async fn email_change_target_already_taken_returns_taken_status() {
    let (app, _pool, outbox) = boot().await;
    signup(&app, "marie@old.test", "longenoughpw1").await;
    signup(&app, "leah@taken.test", "longenoughpw2").await;
    let cookie = signin(&app, "marie@old.test", "longenoughpw1").await;
    outbox.lock().unwrap().clear();

    let mut req = Request::builder()
        .method("POST").uri("/api/me/email-change/request")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, cookie.split(';').next().unwrap())
        .body(Body::from(json!({"new_email": "leah@taken.test",
                                "current_password": "longenoughpw1"}).to_string())).unwrap();
    req.extensions_mut().insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
    app.clone().oneshot(req).await.unwrap();
    let token = outbox.lock().unwrap()[0].body.split("/email-change/").nth(1)
        .unwrap().split_whitespace().next().unwrap().to_string();

    let resp = app.oneshot(req_with_ip("POST", "/api/auth/email-change/confirm",
        json!({"token": token}))).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(
        &resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(body["status"], "taken");
}

#[tokio::test]
async fn email_change_pending_token_invalidated_on_new_request() {
    let (app, pool, outbox) = boot().await;
    signup(&app, "marie@old.test", "longenoughpw1").await;
    let cookie = signin(&app, "marie@old.test", "longenoughpw1").await;

    for new in ["a@x.test", "b@x.test"] {
        let mut req = Request::builder()
            .method("POST").uri("/api/me/email-change/request")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::COOKIE, cookie.split(';').next().unwrap())
            .body(Body::from(json!({"new_email": new,
                                    "current_password": "longenoughpw1"}).to_string())).unwrap();
        req.extensions_mut().insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
        app.clone().oneshot(req).await.unwrap();
    }

    let active: i64 = sqlx::query_scalar!(
        "select count(*) as \"c!\" from email_change_tokens where used_at is null"
    ).fetch_one(&pool).await.unwrap();
    assert_eq!(active, 1);
    let _ = outbox;
}
```

- [ ] **Step 5: Run tests**

```bash
cd $ROOT/backend
cargo test --test security_account email_change -- --nocapture
```

Expected: 3 PASS.

- [ ] **Step 6: Commit**

```bash
cd $ROOT
git add backend/src/auth/email_change.rs \
        backend/src/auth/mod.rs \
        backend/src/http/mod.rs \
        backend/.sqlx/ \
        backend/tests/security_account.rs
git commit -m "feat(backend/auth): add email-change request/confirm with old-address notification"
```

---

## Task 8: Profile + Preferences endpoints

**Files:**
- Create: `backend/src/users/mod.rs` (if not present yet — check first)
- Create: `backend/src/users/profile.rs`
- Create: `backend/src/users/preferences.rs`
- Modify: `backend/src/lib.rs` (`pub mod users;` if not yet)
- Modify: `backend/src/http/mod.rs` (mount 4 routes)
- Modify: `backend/src/api_types.rs` (add `Profile`, `Preferences`)
- Modify: `backend/tests/security_account.rs`

- [ ] **Step 1: Verify whether `users/` already exists**

```bash
ls $ROOT/backend/src/users/ 2>/dev/null || echo "not yet"
```

If it doesn't exist, create `backend/src/users/mod.rs` with `pub mod profile;` and `pub mod preferences;` and add `pub mod users;` to `backend/src/lib.rs`. If it does exist (from an earlier task), just append the two `pub mod` lines.

- [ ] **Step 2: Add types to `backend/src/api_types.rs`**

```rust
#[derive(Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct Profile {
    pub display_name: String,
    pub bio: Option<String>,
}

#[derive(Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct Preferences {
    pub theme: String,    // "dark" | "light"
    pub density: String,  // "work" | "data"
}
```

(If `users.bio` does not yet exist as a column, add `add column bio text` to a small migration `0004_user_bio.sql` AND apply it. If you'd rather defer bio, drop it from the struct and from the handler; the design's Profile section then shows display_name only.)

For this plan we assume **no bio column** exists — the design lists "PROFILE" without explicit bio. Update the struct to:

```rust
#[derive(Serialize, Deserialize, ts_rs::TS)]
#[ts(export)]
pub struct Profile {
    pub display_name: String,
}
```

- [ ] **Step 3: Implement `users/profile.rs`**

```rust
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Deserialize;

use crate::AppError;
use crate::api_types::Profile;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

pub async fn get(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<Profile>, AppError> {
    let row = sqlx::query!(
        "select display_name from users where id = $1", user.id
    ).fetch_one(&state.pool).await?;
    Ok(Json(Profile { display_name: row.display_name }))
}

#[derive(Deserialize)]
pub struct PutBody {
    pub display_name: Option<String>,
}

pub async fn put(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<PutBody>,
) -> Result<impl IntoResponse, AppError> {
    if let Some(name) = body.display_name {
        let trimmed = name.trim();
        if trimmed.is_empty() || trimmed.chars().count() > 60 {
            return Err(AppError::bad_request("invalid_display_name"));
        }
        sqlx::query!("update users set display_name = $1 where id = $2",
                     trimmed, user.id)
            .execute(&state.pool).await?;
    }
    Ok(StatusCode::NO_CONTENT)
}
```

- [ ] **Step 4: Implement `users/preferences.rs`**

```rust
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Deserialize;

use crate::AppError;
use crate::api_types::Preferences;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

pub async fn get(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<Preferences>, AppError> {
    let row = sqlx::query!(
        "select theme, density from users where id = $1", user.id
    ).fetch_one(&state.pool).await?;
    Ok(Json(Preferences { theme: row.theme, density: row.density }))
}

#[derive(Deserialize)]
pub struct PutBody {
    pub theme: Option<String>,
    pub density: Option<String>,
}

pub async fn put(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<PutBody>,
) -> Result<impl IntoResponse, AppError> {
    if let Some(t) = &body.theme {
        if t != "dark" && t != "light" {
            return Err(AppError::bad_request("invalid_theme"));
        }
    }
    if let Some(d) = &body.density {
        if d != "work" && d != "data" {
            return Err(AppError::bad_request("invalid_density"));
        }
    }
    sqlx::query!(
        "update users
            set theme = coalesce($1, theme),
                density = coalesce($2, density)
          where id = $3",
        body.theme, body.density, user.id
    ).execute(&state.pool).await?;
    Ok(StatusCode::NO_CONTENT)
}
```

- [ ] **Step 5: Mount routes**

`backend/src/http/mod.rs`:
```rust
.route("/api/me/profile", get(crate::users::profile::get).put(crate::users::profile::put))
.route("/api/me/preferences", get(crate::users::preferences::get).put(crate::users::preferences::put))
```

- [ ] **Step 6: sqlx prepare + types codegen + check**

```bash
cd $ROOT/backend
DATABASE_URL=postgres://astrophoto:astrophoto@localhost:5434/astrophoto \
  cargo sqlx prepare -- --lib --tests --bins
cargo check
cd $ROOT
just types
```

- [ ] **Step 7: Append a smoke integration test**

```rust
#[tokio::test]
async fn profile_get_put_round_trip() {
    let (app, _pool, _outbox) = boot().await;
    signup(&app, "marie@x.test", "longenoughpw1").await;
    let cookie = signin(&app, "marie@x.test", "longenoughpw1").await;
    let cookie_h = cookie.split(';').next().unwrap().to_string();

    // PUT
    let mut req = Request::builder()
        .method("PUT").uri("/api/me/profile")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, &cookie_h)
        .body(Body::from(json!({"display_name": "Marie Dubois"}).to_string())).unwrap();
    req.extensions_mut().insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
    assert_eq!(app.clone().oneshot(req).await.unwrap().status(), StatusCode::NO_CONTENT);

    // GET
    let mut req = Request::builder()
        .method("GET").uri("/api/me/profile")
        .header(header::COOKIE, &cookie_h)
        .body(Body::empty()).unwrap();
    req.extensions_mut().insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
    let resp = app.oneshot(req).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(
        &resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(v["display_name"], "Marie Dubois");
}

#[tokio::test]
async fn preferences_default_dark_work_then_updated() {
    let (app, _pool, _outbox) = boot().await;
    signup(&app, "marie@x.test", "longenoughpw1").await;
    let cookie = signin(&app, "marie@x.test", "longenoughpw1").await;
    let cookie_h = cookie.split(';').next().unwrap().to_string();

    let mut req = Request::builder().method("GET").uri("/api/me/preferences")
        .header(header::COOKIE, &cookie_h).body(Body::empty()).unwrap();
    req.extensions_mut().insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
    let resp = app.clone().oneshot(req).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(
        &resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(v["theme"], "dark");
    assert_eq!(v["density"], "work");

    let mut req = Request::builder().method("PUT").uri("/api/me/preferences")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, &cookie_h)
        .body(Body::from(json!({"theme": "light"}).to_string())).unwrap();
    req.extensions_mut().insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
    assert_eq!(app.oneshot(req).await.unwrap().status(), StatusCode::NO_CONTENT);
}
```

- [ ] **Step 8: Run + commit**

```bash
cd $ROOT/backend && cargo test --test security_account profile -- --nocapture
cargo test --test security_account preferences -- --nocapture
cd $ROOT
git add backend/src/users/ \
        backend/src/lib.rs \
        backend/src/api_types.rs \
        backend/src/http/mod.rs \
        backend/.sqlx/ \
        backend/tests/security_account.rs \
        frontend/src/lib/api/types.ts
git commit -m "feat(backend/users): add profile + preferences (theme/density) endpoints"
```

---

## Task 9: Sessions — list, last_used tracking, revoke, sign-out-others

**Files:**
- Create: `backend/src/users/sessions.rs`
- Modify: `backend/src/users/mod.rs` (`pub mod sessions;`)
- Modify: `backend/src/auth/middleware.rs` (5-min throttled `last_used_at` update)
- Modify: `backend/src/http/mod.rs` (mount 3 routes)
- Modify: `backend/src/api_types.rs` (`SessionRow`)
- Modify: `backend/tests/security_account.rs`

- [ ] **Step 1: Add the session label parser + endpoint**

Create `backend/src/users/sessions.rs`:

```rust
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Serialize;
use uuid::Uuid;

use crate::AppError;
use crate::auth::middleware::{CurrentSessionId, CurrentUser};
use crate::http::AppState;

#[derive(Serialize, ts_rs::TS)]
#[ts(export)]
pub struct SessionRow {
    pub id: String,
    pub browser: String,
    pub browser_version: String,
    pub os: String,
    pub os_version: String,
    pub category: String,
    pub ip: String,
    pub last_used_at: String,   // RFC3339
    pub created_at: String,
    pub is_current: bool,
}

fn parse_label(ua: &str) -> (String, String, String, String, String) {
    let parser = woothee::parser::Parser::new();
    match parser.parse(ua) {
        Some(r) => (
            r.name.to_string(),
            r.version.to_string(),
            r.os.to_string(),
            r.os_version.to_string(),
            r.category.to_string(),
        ),
        None => ("unknown".into(), String::new(), "unknown".into(),
                 String::new(), "unknown".into()),
    }
}

pub async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    CurrentSessionId(current_id): CurrentSessionId,
) -> Result<Json<Vec<SessionRow>>, AppError> {
    let rows = sqlx::query!(
        r#"select id, user_agent, ip,
                  last_used_at, created_at,
                  (id = $2) as "is_current!: bool"
             from sessions
            where user_id = $1 and expires_at > now()
            order by is_current desc, last_used_at desc"#,
        user.id, current_id
    ).fetch_all(&state.pool).await?;

    let out = rows.into_iter().map(|r| {
        let ua = r.user_agent.unwrap_or_default();
        let (browser, bv, os, osv, cat) = parse_label(&ua);
        SessionRow {
            id: hex::encode(&r.id),
            browser, browser_version: bv, os, os_version: osv, category: cat,
            ip: r.ip.map(|n| n.to_string()).unwrap_or_default(),
            last_used_at: r.last_used_at.to_rfc3339(),
            created_at: r.created_at.to_rfc3339(),
            is_current: r.is_current,
        }
    }).collect();
    Ok(Json(out))
}

pub async fn revoke(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    CurrentSessionId(current_id): CurrentSessionId,
    Path(id_hex): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let id = hex::decode(&id_hex).map_err(|_| AppError::bad_request("bad_id"))?;
    if id == current_id {
        return Err(AppError::bad_request("use_logout"));
    }
    let res = sqlx::query!(
        "delete from sessions where id = $1 and user_id = $2",
        id, user.id
    ).execute(&state.pool).await?;
    if res.rows_affected() == 0 {
        return Err(AppError::not_found("session"));
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn sign_out_others(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    CurrentSessionId(current_id): CurrentSessionId,
) -> Result<impl IntoResponse, AppError> {
    sqlx::query!(
        "delete from sessions where user_id = $1 and id != $2",
        user.id, current_id
    ).execute(&state.pool).await?;
    Ok(StatusCode::NO_CONTENT)
}
```

If `hex` is not in `Cargo.toml`, add `hex = "0.4"`.

If `AppError::not_found` doesn't exist, add it (mirroring `bad_request`).

- [ ] **Step 2: Add `CurrentSessionId` extractor to `auth/middleware.rs`**

Open `backend/src/auth/middleware.rs`. Locate the existing `CurrentUser` extractor. Add a sibling extractor that returns the raw session id (`Vec<u8>`) by reading the cookie and looking up the row:

```rust
pub struct CurrentSessionId(pub Vec<u8>);

#[axum::async_trait]
impl<S> axum::extract::FromRequestParts<S> for CurrentSessionId
where
    S: Send + Sync,
    AppState: axum::extract::FromRef<S>,
{
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut axum::http::request::Parts, state: &S)
        -> Result<Self, Self::Rejection>
    {
        // Reuse the same cookie parsing as CurrentUser; depending on the existing
        // shape of session::cookie_from_headers / session::resolve, expose a helper
        // that returns just the id bytes. The simplest path: extend `CurrentUser`'s
        // helper to also stash the session id in `parts.extensions`.
        parts.extensions.get::<CurrentSessionId>()
            .cloned()
            .ok_or_else(|| AppError::unauthorized("no_session"))
    }
}

impl Clone for CurrentSessionId {
    fn clone(&self) -> Self { CurrentSessionId(self.0.clone()) }
}
```

The pragmatic implementation: in the existing middleware that resolves `CurrentUser`, also call `parts.extensions.insert(CurrentSessionId(session_id_bytes.clone()))` once the row is found. Adjust to fit the actual middleware shape.

- [ ] **Step 3: Throttled `last_used_at` update in middleware**

In the same middleware, after the session row is verified valid, fire-and-await:

```rust
let _ = sqlx::query!(
    "update sessions set last_used_at = now()
      where id = $1 and last_used_at < now() - interval '5 minutes'",
    session_id_bytes
).execute(&state.pool).await;
```

The 5-minute `WHERE` makes 99% of consecutive requests no-ops. Failures are intentionally swallowed (logged at `tracing::warn!` if you want a breadcrumb).

- [ ] **Step 4: Mount routes**

```rust
.route("/api/me/sessions", get(crate::users::sessions::list))
.route("/api/me/sessions/:id", axum::routing::delete(crate::users::sessions::revoke))
.route("/api/me/sessions/sign-out-others", post(crate::users::sessions::sign_out_others))
```

- [ ] **Step 5: sqlx prepare + types + check**

```bash
cd $ROOT/backend
DATABASE_URL=postgres://astrophoto:astrophoto@localhost:5434/astrophoto \
  cargo sqlx prepare -- --lib --tests --bins
cd $ROOT && just types
cd $ROOT/backend && cargo check
```

- [ ] **Step 6: Tests**

Append:

```rust
#[tokio::test]
async fn sessions_list_marks_current_first() {
    let (app, pool, _outbox) = boot().await;
    signup(&app, "marie@x.test", "longenoughpw1").await;
    let cookie_a = signin(&app, "marie@x.test", "longenoughpw1").await;
    let _cookie_b = signin(&app, "marie@x.test", "longenoughpw1").await;

    let mut req = Request::builder().method("GET").uri("/api/me/sessions")
        .header(header::COOKIE, cookie_a.split(';').next().unwrap())
        .body(Body::empty()).unwrap();
    req.extensions_mut().insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
    let resp = app.oneshot(req).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(
        &resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    let arr = v.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["is_current"], true);
    assert_eq!(arr[1]["is_current"], false);
    let _ = pool;
}

#[tokio::test]
async fn revoke_current_session_returns_400() {
    let (app, pool, _outbox) = boot().await;
    signup(&app, "marie@x.test", "longenoughpw1").await;
    let cookie = signin(&app, "marie@x.test", "longenoughpw1").await;

    // Find current session id.
    let row = sqlx::query!("select id from sessions limit 1")
        .fetch_one(&pool).await.unwrap();
    let id_hex = hex::encode(&row.id);

    let mut req = Request::builder().method("DELETE")
        .uri(format!("/api/me/sessions/{id_hex}"))
        .header(header::COOKIE, cookie.split(';').next().unwrap())
        .body(Body::empty()).unwrap();
    req.extensions_mut().insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn sign_out_others_keeps_current_kills_rest() {
    let (app, pool, _outbox) = boot().await;
    signup(&app, "marie@x.test", "longenoughpw1").await;
    let cookie_a = signin(&app, "marie@x.test", "longenoughpw1").await;
    let _ = signin(&app, "marie@x.test", "longenoughpw1").await;
    let _ = signin(&app, "marie@x.test", "longenoughpw1").await;

    let mut req = Request::builder().method("POST")
        .uri("/api/me/sessions/sign-out-others")
        .header(header::COOKIE, cookie_a.split(';').next().unwrap())
        .body(Body::empty()).unwrap();
    req.extensions_mut().insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
    assert_eq!(app.oneshot(req).await.unwrap().status(), StatusCode::NO_CONTENT);

    let count: i64 = sqlx::query_scalar!(
        "select count(*) as \"c!\" from sessions s join users u on u.id = s.user_id
         where u.email = $1", "marie@x.test"
    ).fetch_one(&pool).await.unwrap();
    assert_eq!(count, 1);
}
```

- [ ] **Step 7: Run + commit**

```bash
cd $ROOT/backend && cargo test --test security_account session -- --nocapture
cd $ROOT
git add backend/src/users/sessions.rs backend/src/users/mod.rs \
        backend/src/auth/middleware.rs \
        backend/src/api_types.rs \
        backend/src/http/mod.rs \
        backend/Cargo.toml backend/Cargo.lock \
        backend/.sqlx/ \
        backend/tests/security_account.rs \
        backend/src/error.rs \
        frontend/src/lib/api/types.ts
git commit -m "feat(backend/users): sessions list/revoke/sign-out-others + last_used throttled tracking"
```

---

## Task 10: Account deletion (request, cancel, /api/auth/me extension)

**Files:**
- Create: `backend/src/users/deletion.rs`
- Modify: `backend/src/users/mod.rs` (`pub mod deletion;`)
- Modify: `backend/src/auth/me.rs` (extend `User` DTO with `pending_deletion_at`)
- Modify: `backend/src/api_types.rs` (extend `User`)
- Modify: `backend/src/http/mod.rs` (mount 2 routes)
- Modify: `backend/tests/security_account.rs`

- [ ] **Step 1: Implement `users/deletion.rs`**

```rust
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Deserialize;

use crate::AppError;
use crate::auth::middleware::CurrentUser;
use crate::auth::password::verify_password;
use crate::http::AppState;
use crate::mail::templates;

#[derive(Deserialize)]
pub struct RequestBody {
    pub current_password: Option<String>,
    pub confirmation_phrase: String,
}

const REQUIRED_PHRASE: &str = "DELETE MY ACCOUNT";

pub async fn request(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<RequestBody>,
) -> Result<impl IntoResponse, AppError> {
    if body.confirmation_phrase != REQUIRED_PHRASE {
        return Err(AppError::bad_request("phrase_mismatch"));
    }
    let row = sqlx::query!(
        "select email as \"email!: String\", display_name, password_hash from users where id = $1",
        user.id
    ).fetch_one(&state.pool).await?;

    if let Some(stored) = row.password_hash.clone() {
        let pwd = body.current_password.ok_or_else(||
            AppError::unauthorized("wrong_password"))?;
        let ok = tokio::task::spawn_blocking(move || verify_password(&pwd, &stored))
            .await.map_err(|e| AppError::internal(format!("argon2 join: {e}")))??;
        if !ok {
            return Err(AppError::unauthorized("wrong_password"));
        }
    }

    sqlx::query!(
        "update users set pending_deletion_at = now() + interval '7 days'
          where id = $1 and pending_deletion_at is null",
        user.id
    ).execute(&state.pool).await?;

    let when_human = (chrono::Utc::now() + chrono::Duration::days(7))
        .format("%A %e %B %Y at %H:%M UTC").to_string();
    let cancel_link = format!(
        "{}/settings/delete",
        state.config.public_base_url.trim_end_matches('/')
    );
    let (subject, body) = templates::account_deletion_scheduled(
        &row.display_name, &when_human, &cancel_link);
    state.mailer.send_plain(&row.email, &subject, &body).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn cancel(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<impl IntoResponse, AppError> {
    let row = sqlx::query!(
        "update users set pending_deletion_at = null
          where id = $1 and pending_deletion_at is not null
        returning email as \"email!: String\", display_name",
        user.id
    ).fetch_optional(&state.pool).await?;

    if let Some(r) = row {
        let (subject, body) = templates::account_deletion_cancelled(&r.display_name);
        state.mailer.send_plain(&r.email, &subject, &body).await?;
    }
    Ok(StatusCode::NO_CONTENT)
}
```

- [ ] **Step 2: Extend `/api/auth/me`**

Open `backend/src/api_types.rs`. Extend the `User` struct:
```rust
pub pending_deletion_at: Option<String>,  // RFC3339, present only when scheduled
```

Open `backend/src/auth/me.rs`. The query needs to also select `pending_deletion_at`:

```rust
let row = sqlx::query!(
    "select pending_deletion_at from users where id = $1", user.id
).fetch_one(&state.pool).await?;
// ...build dto with:
pending_deletion_at: row.pending_deletion_at.map(|t| t.to_rfc3339()),
```

(Either fetch in addition to existing fields, or extend the existing query — depending on how `me::handler` is shaped.)

- [ ] **Step 3: Mount routes**

```rust
.route("/api/me/delete-request", post(crate::users::deletion::request))
.route("/api/me/delete-cancel", post(crate::users::deletion::cancel))
```

- [ ] **Step 4: sqlx prepare + types + check**

```bash
cd $ROOT/backend
DATABASE_URL=postgres://astrophoto:astrophoto@localhost:5434/astrophoto \
  cargo sqlx prepare -- --lib --tests --bins
cd $ROOT && just types && cd $ROOT/backend && cargo check
```

- [ ] **Step 5: Tests**

```rust
#[tokio::test]
async fn delete_request_with_correct_password_and_phrase_succeeds() {
    let (app, pool, outbox) = boot().await;
    signup(&app, "marie@x.test", "longenoughpw1").await;
    let cookie = signin(&app, "marie@x.test", "longenoughpw1").await;

    let mut req = Request::builder().method("POST").uri("/api/me/delete-request")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, cookie.split(';').next().unwrap())
        .body(Body::from(json!({
            "current_password": "longenoughpw1",
            "confirmation_phrase": "DELETE MY ACCOUNT"
        }).to_string())).unwrap();
    req.extensions_mut().insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
    assert_eq!(app.oneshot(req).await.unwrap().status(), StatusCode::NO_CONTENT);

    let pending: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar!(
        "select pending_deletion_at from users where email = $1", "marie@x.test"
    ).fetch_one(&pool).await.unwrap();
    assert!(pending.is_some());
    assert!(outbox.lock().unwrap().iter().any(|m| m.subject.contains("scheduled")));
}

#[tokio::test]
async fn delete_request_wrong_phrase_returns_400() {
    let (app, _pool, _outbox) = boot().await;
    signup(&app, "marie@x.test", "longenoughpw1").await;
    let cookie = signin(&app, "marie@x.test", "longenoughpw1").await;

    let mut req = Request::builder().method("POST").uri("/api/me/delete-request")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, cookie.split(';').next().unwrap())
        .body(Body::from(json!({
            "current_password": "longenoughpw1",
            "confirmation_phrase": "delete my account"
        }).to_string())).unwrap();
    req.extensions_mut().insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
    assert_eq!(app.oneshot(req).await.unwrap().status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn delete_request_idempotent_does_not_extend_grace() {
    let (app, pool, _outbox) = boot().await;
    signup(&app, "marie@x.test", "longenoughpw1").await;
    let cookie = signin(&app, "marie@x.test", "longenoughpw1").await;
    let cookie_h = cookie.split(';').next().unwrap().to_string();

    for _ in 0..2 {
        let mut req = Request::builder().method("POST").uri("/api/me/delete-request")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::COOKIE, &cookie_h)
            .body(Body::from(json!({
                "current_password": "longenoughpw1",
                "confirmation_phrase": "DELETE MY ACCOUNT"
            }).to_string())).unwrap();
        req.extensions_mut().insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
        app.clone().oneshot(req).await.unwrap();
    }
    let pending: chrono::DateTime<chrono::Utc> = sqlx::query_scalar!(
        "select pending_deletion_at as \"p!: chrono::DateTime<chrono::Utc>\"
           from users where email = $1", "marie@x.test"
    ).fetch_one(&pool).await.unwrap();
    let dt = (pending - chrono::Utc::now()).num_hours();
    assert!(dt > 167 && dt < 169, "grace must remain ~7 days, got {dt}h");
}

#[tokio::test]
async fn delete_cancel_clears_pending_and_emails() {
    let (app, pool, outbox) = boot().await;
    signup(&app, "marie@x.test", "longenoughpw1").await;
    let cookie = signin(&app, "marie@x.test", "longenoughpw1").await;
    let cookie_h = cookie.split(';').next().unwrap().to_string();

    // Mark for deletion.
    let mut req = Request::builder().method("POST").uri("/api/me/delete-request")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, &cookie_h)
        .body(Body::from(json!({
            "current_password": "longenoughpw1",
            "confirmation_phrase": "DELETE MY ACCOUNT"
        }).to_string())).unwrap();
    req.extensions_mut().insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
    app.clone().oneshot(req).await.unwrap();
    outbox.lock().unwrap().clear();

    // Cancel.
    let mut req = Request::builder().method("POST").uri("/api/me/delete-cancel")
        .header(header::COOKIE, &cookie_h).body(Body::empty()).unwrap();
    req.extensions_mut().insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
    assert_eq!(app.oneshot(req).await.unwrap().status(), StatusCode::NO_CONTENT);

    let pending: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar!(
        "select pending_deletion_at from users where email = $1", "marie@x.test"
    ).fetch_one(&pool).await.unwrap();
    assert!(pending.is_none());
    assert!(outbox.lock().unwrap().iter().any(|m| m.subject.contains("cancelled")));
}
```

- [ ] **Step 6: Run + commit**

```bash
cd $ROOT/backend && cargo test --test security_account delete -- --nocapture
cd $ROOT
git add backend/src/users/deletion.rs backend/src/users/mod.rs \
        backend/src/auth/me.rs backend/src/api_types.rs \
        backend/src/http/mod.rs \
        backend/.sqlx/ backend/tests/security_account.rs \
        frontend/src/lib/api/types.ts
git commit -m "feat(backend/users): account deletion request/cancel + /api/auth/me exposes pending_deletion_at"
```

---

## Task 11: Purge worker (S3 cleanup, per-user error containment)

**Files:**
- Create: `backend/src/jobs/mod.rs`
- Create: `backend/src/jobs/purge_deletions.rs`
- Modify: `backend/src/lib.rs` (`pub mod jobs;`)
- Modify: `backend/src/storage/mod.rs` (extend `Storage` trait or add free function for batch delete)
- Modify: `backend/src/main.rs` (spawn the worker after the router is mounted)
- Modify: `backend/tests/security_account.rs` (call `purge_once` directly)

- [ ] **Step 1: Add a `delete_objects(&[String])` to the storage layer**

Open `backend/src/storage/mod.rs`. Extend the trait:

```rust
#[async_trait]
pub trait Storage: Send + Sync {
    // ...existing methods...
    async fn delete_objects(&self, keys: &[String]) -> Result<(), AppError>;
}
```

Implement on the S3 backend by issuing one or more `DeleteObjects` calls of up to 1,000 keys each. Implement on `MemoryStorage` by removing keys from the in-memory map. (Both implementations should treat unknown keys as no-ops.)

- [ ] **Step 2: Create `backend/src/jobs/mod.rs`**

```rust
pub mod purge_deletions;
```

- [ ] **Step 3: Create `backend/src/jobs/purge_deletions.rs`**

```rust
//! Hourly worker: hard-delete accounts whose grace period has elapsed.
//! Per-user errors are logged and skipped — one bad account never stalls
//! the whole hourly batch.

use std::sync::Arc;
use std::time::Duration;

use sqlx::PgPool;
use uuid::Uuid;

use crate::AppError;
use crate::storage::Storage;

pub fn spawn(pool: PgPool, storage: Arc<dyn Storage>) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(3600));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            ticker.tick().await;
            if let Err(e) = purge_once(&pool, storage.as_ref()).await {
                tracing::error!(error = ?e, "purge_deletions cycle failed");
            }
        }
    });
}

pub async fn purge_once(pool: &PgPool, storage: &dyn Storage) -> Result<u64, AppError> {
    let due: Vec<Uuid> = sqlx::query_scalar!(
        "select id from users
          where pending_deletion_at is not null
            and pending_deletion_at < now()"
    ).fetch_all(pool).await?;

    if due.is_empty() {
        return Ok(0);
    }

    let mut deleted = 0u64;
    for user_id in &due {
        match purge_one_user(pool, storage, *user_id).await {
            Ok(()) => deleted += 1,
            Err(e) => tracing::error!(
                user_id = %user_id, error = ?e,
                "purge_one_user failed; skipping"
            ),
        }
    }
    tracing::info!(deleted, total_due = due.len(), "purge cycle done");
    Ok(deleted)
}

async fn purge_one_user(
    pool: &PgPool,
    storage: &dyn Storage,
    user_id: Uuid,
) -> Result<(), AppError> {
    let rows = sqlx::query!(
        "select storage_key, thumbnail_key from photos where owner_id = $1",
        user_id
    ).fetch_all(pool).await?;

    let to_delete: Vec<String> = rows.into_iter()
        .flat_map(|r| std::iter::once(r.storage_key)
            .chain(r.thumbnail_key.into_iter()))
        .collect();

    if !to_delete.is_empty() {
        storage.delete_objects(&to_delete).await?;
    }
    sqlx::query!("delete from users where id = $1", user_id)
        .execute(pool).await?;
    Ok(())
}
```

If the photos table column for the thumbnail is named differently (`thumb_key` etc.), adjust the SELECT.

- [ ] **Step 4: Wire `pub mod jobs;` and spawn from `main.rs`**

Open `backend/src/lib.rs`: add `pub mod jobs;`.

Open `backend/src/main.rs`. After the router is built and `serve` is launched (or just before, depending on the existing structure), spawn the worker:

```rust
astrophoto::jobs::purge_deletions::spawn(pool.clone(), storage.clone());
```

- [ ] **Step 5: sqlx prepare + check**

```bash
cd $ROOT/backend
DATABASE_URL=postgres://astrophoto:astrophoto@localhost:5434/astrophoto \
  cargo sqlx prepare -- --lib --tests --bins
cargo check
```

- [ ] **Step 6: Tests** — directly invoke `purge_once`

```rust
#[tokio::test]
async fn purge_worker_deletes_users_past_grace() {
    let (_app, pool, _outbox) = boot().await;
    let storage = Arc::new(MemoryStorage::default());

    // Create user A whose grace already elapsed, user B who is mid-grace.
    let a = sqlx::query_scalar!(
        "insert into users (email, display_name, password_hash, pending_deletion_at)
         values ($1, 'A', null, now() - interval '1 hour') returning id",
        "purge-a@x.test"
    ).fetch_one(&pool).await.unwrap();
    let _b = sqlx::query_scalar!(
        "insert into users (email, display_name, password_hash, pending_deletion_at)
         values ($1, 'B', null, now() + interval '6 days') returning id",
        "purge-b@x.test"
    ).fetch_one(&pool).await.unwrap();

    let n = astrophoto::jobs::purge_deletions::purge_once(&pool, storage.as_ref())
        .await.unwrap();
    assert_eq!(n, 1);

    let still: Vec<String> = sqlx::query_scalar!(
        "select email as \"e!: String\" from users where email like 'purge-%' order by email"
    ).fetch_all(&pool).await.unwrap();
    assert_eq!(still, vec!["purge-b@x.test"]);
    let _ = a;
}

#[tokio::test]
async fn purge_pseudonymises_comments_keeps_body() {
    let (_app, pool, _outbox) = boot().await;
    let storage = Arc::new(MemoryStorage::default());

    // Two users: leah owns a photo; marie comments; marie deletes her account.
    let leah = sqlx::query_scalar!(
        "insert into users (email, display_name, password_hash) values ($1,'Leah',null) returning id",
        "leah@x.test").fetch_one(&pool).await.unwrap();
    let marie = sqlx::query_scalar!(
        "insert into users (email, display_name, password_hash, pending_deletion_at)
         values ($1,'Marie',null, now() - interval '1 hour') returning id",
        "marie@x.test").fetch_one(&pool).await.unwrap();
    let photo = sqlx::query_scalar!(
        "insert into photos (owner_id, storage_key, original_name, thumbnail_key)
         values ($1, 'k1', 'a.jpg', 'k1-thumb') returning id", leah)
        .fetch_one(&pool).await.unwrap();
    sqlx::query!("insert into comments (photo_id, author_id, body)
                  values ($1, $2, 'gorgeous Hα')", photo, marie)
        .execute(&pool).await.unwrap();

    astrophoto::jobs::purge_deletions::purge_once(&pool, storage.as_ref())
        .await.unwrap();

    let row = sqlx::query!(
        "select author_id, body from comments where photo_id = $1", photo
    ).fetch_one(&pool).await.unwrap();
    assert!(row.author_id.is_none(), "author_id must be NULL after purge");
    assert_eq!(row.body, "gorgeous Hα", "body must be preserved");
}
```

(Adjust the photos `insert` to match the actual NOT NULL columns. If `photos` requires more fields, add them with sensible defaults.)

- [ ] **Step 7: Run + commit**

```bash
cd $ROOT/backend && cargo test --test security_account purge -- --nocapture
cd $ROOT
git add backend/src/jobs/ backend/src/lib.rs \
        backend/src/storage/ \
        backend/src/main.rs \
        backend/.sqlx/ \
        backend/tests/security_account.rs
git commit -m "feat(backend/jobs): in-process purge worker (S3 cleanup + per-user error containment)"
```

---

## Task 12: RGPD export endpoint (`GET /api/me/export.json`)

**Files:**
- Create: `backend/src/users/export.rs`
- Modify: `backend/src/users/mod.rs` (`pub mod export;`)
- Modify: `backend/src/storage/mod.rs` (add `signed_url(&str, ttl_secs)` if not present)
- Modify: `backend/src/http/mod.rs` (mount route)
- Modify: `backend/tests/security_account.rs`

- [ ] **Step 1: Confirm or add `signed_url` on the Storage trait**

```bash
grep -n "signed_url\|presign" $ROOT/backend/src/storage/mod.rs $ROOT/backend/src/storage/*.rs
```

If a presign helper already exists for the upload flow, reuse it; if not, add to the trait:

```rust
async fn signed_url(&self, key: &str, ttl_secs: u64) -> Result<String, AppError>;
```

Implement on the S3 backend via the existing `aws-sdk-s3` `presigned` API (TTL up to 7 days). Implement on `MemoryStorage` by returning `format!("memory://{key}")` (tests don't follow the link).

- [ ] **Step 2: Implement `users/export.rs`**

```rust
use axum::{
    extract::State,
    http::{StatusCode, header},
    response::IntoResponse,
};
use serde::Serialize;

use crate::AppError;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

#[derive(Serialize)]
struct Export {
    exported_at: String,
    user: ExportUser,
    photos: Vec<ExportPhoto>,
    comments_authored: Vec<ExportComment>,
    appreciations_given: Vec<ExportAppreciation>,
    follows: ExportFollows,
}
#[derive(Serialize)]
struct ExportUser { id: String, email: String, display_name: String, created_at: String }
#[derive(Serialize)]
struct ExportPhoto {
    id: String, title: Option<String>, caption: Option<String>,
    captured_at: Option<String>, exif: serde_json::Value,
    original_url: String, thumbnail_url: Option<String>,
}
#[derive(Serialize)]
struct ExportComment { id: String, photo_id: String, body: String, created_at: String }
#[derive(Serialize)]
struct ExportAppreciation { photo_id: String, created_at: String }
#[derive(Serialize)]
struct ExportFollows { following: Vec<String>, followers: Vec<String> }

const TTL_SECS: u64 = 7 * 24 * 3600;

pub async fn handler(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<impl IntoResponse, AppError> {
    let u = sqlx::query!(
        "select email as \"email!: String\", display_name, created_at
           from users where id = $1", user.id
    ).fetch_one(&state.pool).await?;

    let photos_rows = sqlx::query!(
        "select id, title, caption, captured_at, exif,
                storage_key, thumbnail_key
           from photos where owner_id = $1", user.id
    ).fetch_all(&state.pool).await?;

    let mut photos = Vec::with_capacity(photos_rows.len());
    for p in photos_rows {
        let original_url = state.storage.signed_url(&p.storage_key, TTL_SECS).await?;
        let thumb_url = match p.thumbnail_key {
            Some(k) => Some(state.storage.signed_url(&k, TTL_SECS).await?),
            None => None,
        };
        photos.push(ExportPhoto {
            id: p.id.to_string(),
            title: p.title,
            caption: p.caption,
            captured_at: p.captured_at.map(|t| t.to_rfc3339()),
            exif: p.exif.unwrap_or(serde_json::Value::Null),
            original_url,
            thumbnail_url: thumb_url,
        });
    }

    let comments = sqlx::query!(
        "select id, photo_id, body, created_at from comments where author_id = $1",
        user.id
    ).fetch_all(&state.pool).await?
     .into_iter().map(|r| ExportComment {
         id: r.id.to_string(), photo_id: r.photo_id.to_string(),
         body: r.body, created_at: r.created_at.to_rfc3339(),
     }).collect();

    let appreciations = sqlx::query!(
        "select photo_id, created_at from appreciations where user_id = $1",
        user.id
    ).fetch_all(&state.pool).await?
     .into_iter().map(|r| ExportAppreciation {
         photo_id: r.photo_id.to_string(), created_at: r.created_at.to_rfc3339(),
     }).collect();

    let following: Vec<String> = sqlx::query_scalar!(
        "select followed_id::text as \"f!: String\" from follows where follower_id = $1",
        user.id
    ).fetch_all(&state.pool).await?;
    let followers: Vec<String> = sqlx::query_scalar!(
        "select follower_id::text as \"f!: String\" from follows where followed_id = $1",
        user.id
    ).fetch_all(&state.pool).await?;

    let payload = Export {
        exported_at: chrono::Utc::now().to_rfc3339(),
        user: ExportUser {
            id: user.id.to_string(), email: u.email, display_name: u.display_name,
            created_at: u.created_at.to_rfc3339(),
        },
        photos, comments_authored: comments, appreciations_given: appreciations,
        follows: ExportFollows { following, followers },
    };

    let json = serde_json::to_string_pretty(&payload)
        .map_err(|e| AppError::internal(format!("export serialise: {e}")))?;
    let filename = format!("astrophoto-export-{}-{}.json",
        user.id, chrono::Utc::now().format("%Y-%m-%d"));

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/json".to_string()),
            (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{filename}\"")),
        ],
        json,
    ))
}
```

- [ ] **Step 3: Wire**

`backend/src/users/mod.rs`: `pub mod export;`
`backend/src/http/mod.rs`:
```rust
.route("/api/me/export.json", get(crate::users::export::handler))
```

- [ ] **Step 4: sqlx prepare + check**

```bash
cd $ROOT/backend
DATABASE_URL=postgres://astrophoto:astrophoto@localhost:5434/astrophoto \
  cargo sqlx prepare -- --lib --tests --bins
cargo check
```

- [ ] **Step 5: Test**

```rust
#[tokio::test]
async fn export_json_returns_attachment_with_signed_urls() {
    let (app, pool, _outbox) = boot().await;
    signup(&app, "marie@x.test", "longenoughpw1").await;
    let cookie = signin(&app, "marie@x.test", "longenoughpw1").await;

    // Insert one photo for marie.
    let user_id: uuid::Uuid = sqlx::query_scalar!(
        "select id from users where email = $1", "marie@x.test"
    ).fetch_one(&pool).await.unwrap();
    sqlx::query!(
        "insert into photos (owner_id, storage_key, original_name, thumbnail_key)
         values ($1, 'k1', 'a.jpg', 'k1-thumb')", user_id)
        .execute(&pool).await.unwrap();

    let mut req = Request::builder().method("GET").uri("/api/me/export.json")
        .header(header::COOKIE, cookie.split(';').next().unwrap())
        .body(Body::empty()).unwrap();
    req.extensions_mut().insert(std::net::SocketAddr::from(([127, 0, 0, 1], 9999)));
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let cd = resp.headers().get(header::CONTENT_DISPOSITION).unwrap()
        .to_str().unwrap().to_string();
    assert!(cd.contains("attachment"));

    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["user"]["email"], "marie@x.test");
    assert_eq!(v["photos"].as_array().unwrap().len(), 1);
    assert!(v["photos"][0]["original_url"].as_str().unwrap()
        .starts_with("memory://k1"));
}
```

- [ ] **Step 6: Commit**

```bash
cd $ROOT/backend && cargo test --test security_account export -- --nocapture
cd $ROOT
git add backend/src/users/export.rs backend/src/users/mod.rs \
        backend/src/storage/ \
        backend/src/http/mod.rs \
        backend/.sqlx/ \
        backend/tests/security_account.rs
git commit -m "feat(backend/users): GET /api/me/export.json (RGPD JSON dump with signed S3 URLs)"
```

---

## Task 13: Frontend foundations — Modal, settings primitives, theme/density SSR

**Files:**
- Create: `frontend/src/lib/components/Modal.svelte`
- Create: `frontend/src/lib/components/settings/Section.svelte`
- Create: `frontend/src/lib/components/settings/Row.svelte`
- Create: `frontend/src/lib/components/settings/AutosaveField.svelte`
- Modify: `frontend/src/app.html` (add `%theme%` / `%density%` placeholders)
- Modify: `frontend/src/hooks.server.ts` (parse cookies, populate `event.locals.preferences`, rewrite chunk)
- Modify: `frontend/src/app.d.ts` (declare `Locals.preferences`)

- [ ] **Step 1: `Modal.svelte`**

```svelte
<script lang="ts">
  import { onMount } from 'svelte';

  let { open = $bindable(false), title, children, onclose }: {
    open: boolean;
    title: string;
    children: import('svelte').Snippet;
    onclose?: () => void;
  } = $props();

  let dialogEl: HTMLDivElement;
  let invokerBefore: HTMLElement | null = null;

  $effect(() => {
    if (open) {
      invokerBefore = document.activeElement as HTMLElement | null;
      // Focus the first focusable inside the dialog after the DOM updates.
      queueMicrotask(() => {
        const focusable = dialogEl?.querySelector<HTMLElement>(
          'a, button, input, textarea, select, [tabindex]:not([tabindex="-1"])'
        );
        focusable?.focus();
      });
    } else if (invokerBefore) {
      invokerBefore.focus();
    }
  });

  function onKey(e: KeyboardEvent) {
    if (!open) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      open = false;
      onclose?.();
    }
    if (e.key === 'Tab' && dialogEl) {
      // Trap focus.
      const focusables = dialogEl.querySelectorAll<HTMLElement>(
        'a, button, input, textarea, select, [tabindex]:not([tabindex="-1"])'
      );
      if (!focusables.length) return;
      const first = focusables[0];
      const last = focusables[focusables.length - 1];
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault(); last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault(); first.focus();
      }
    }
  }

  onMount(() => {
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });
</script>

{#if open}
  <div class="modal-overlay" onclick={() => { open = false; onclose?.(); }}></div>
  <div
    class="modal-dialog"
    role="dialog"
    aria-modal="true"
    aria-label={title}
    bind:this={dialogEl}
    onclick={(e) => e.stopPropagation()}
  >
    {@render children()}
  </div>
{/if}

<style>
  .modal-overlay {
    position: fixed; inset: 0;
    background: var(--bg-overlay);
    z-index: 50;
  }
  .modal-dialog {
    position: fixed; top: 50%; left: 50%; transform: translate(-50%, -50%);
    background: var(--bg-raised); border: 1px solid var(--border-default);
    padding: 32px; max-width: 640px; width: 90vw; z-index: 51;
    border-radius: var(--r-md, 4px);
  }
</style>
```

- [ ] **Step 2: `settings/Section.svelte`**

```svelte
<script lang="ts">
  let { title, description, tone = 'default', children }: {
    title: string;
    description?: string;
    tone?: 'default' | 'danger';
    children: import('svelte').Snippet;
  } = $props();
</script>

<section class="setting-section" data-tone={tone}>
  <h2>{title}</h2>
  {#if description}<p class="desc">{description}</p>{/if}
  <div class="body">{@render children()}</div>
</section>

<style>
  .setting-section {
    border-bottom: 1px solid var(--border-subtle);
    margin-bottom: 40px; padding-bottom: 40px;
  }
  h2 {
    font-family: var(--font-display);
    font-size: 26px; font-style: italic; font-weight: 600;
    margin: 0 0 8px; color: var(--fg-primary);
  }
  .setting-section[data-tone="danger"] h2 { color: var(--danger); }
  .desc {
    font-size: 13px; color: var(--fg-muted);
    max-width: 560px; margin: 0 0 24px;
  }
</style>
```

- [ ] **Step 3: `settings/Row.svelte`**

```svelte
<script lang="ts">
  let { label, hint, children }: {
    label: string;
    hint?: string;
    children: import('svelte').Snippet;
  } = $props();
</script>

<div class="row">
  <div class="label-cell">
    <span class="label">{label}</span>
    {#if hint}<span class="hint">{hint}</span>{/if}
  </div>
  <div class="field">{@render children()}</div>
</div>

<style>
  .row { display: grid; grid-template-columns: 160px 1fr; gap: 24px; align-items: start; padding: 16px 0; }
  .label-cell { display: flex; flex-direction: column; gap: 4px; }
  .label {
    font-family: var(--font-mono); font-size: 11px;
    text-transform: uppercase; letter-spacing: 0.16em;
    color: var(--fg-muted);
  }
  .hint { font-family: var(--font-mono); font-size: 11px; color: var(--fg-faint); }
</style>
```

- [ ] **Step 4: `settings/AutosaveField.svelte`**

```svelte
<script lang="ts">
  let { value = $bindable(''), name, action, type = 'text' }: {
    value: string; name: string; action: string; type?: string;
  } = $props();

  let savedAt: number | null = $state(null);
  let error = $state(false);
  let timer: number | undefined;

  async function save() {
    const fd = new FormData();
    fd.set(name, value);
    try {
      const r = await fetch(action, { method: 'POST', body: fd });
      if (!r.ok) throw new Error(String(r.status));
      error = false;
      savedAt = Date.now();
    } catch {
      error = true;
    }
  }

  function onInput(e: Event) {
    value = (e.target as HTMLInputElement).value;
    if (timer) clearTimeout(timer);
    timer = window.setTimeout(save, 600);
  }

  function showSaved() {
    return savedAt && (Date.now() - savedAt) < 2000;
  }
</script>

<div class="autosave">
  <input {type} {name} {value} oninput={onInput} class="input" />
  {#if showSaved()}<span class="saved">● Saved</span>{/if}
  {#if error}<span class="err">● Save failed — retry</span>{/if}
</div>

<style>
  .autosave { display: flex; align-items: center; gap: 8px; }
  .saved { font-family: var(--font-mono); font-size: 11px; color: var(--accent); }
  .err   { font-family: var(--font-mono); font-size: 11px; color: var(--danger); }
</style>
```

- [ ] **Step 5: Theme/density SSR**

Open `frontend/src/app.html`. Replace the `<html ...>` opening tag with:
```html
<html lang="en" data-theme="%theme%" data-density="%density%">
```

Open `frontend/src/app.d.ts`. Inside `interface Locals`, add:
```ts
preferences: { theme: 'dark' | 'light'; density: 'work' | 'data' };
```

Open `frontend/src/hooks.server.ts`. At the top of `handle`, after the existing cookie parsing:

```typescript
function parseCookie(header: string, name: string): string | null {
  const re = new RegExp('(?:^|;\\s*)' + name + '=([^;]+)');
  const m = header.match(re);
  return m ? decodeURIComponent(m[1]) : null;
}

const themeCookie = parseCookie(cookie, 'theme');
const densityCookie = parseCookie(cookie, 'density');
const theme = (themeCookie === 'light' ? 'light' : 'dark') as 'dark' | 'light';
const density = (densityCookie === 'data' ? 'data' : 'work') as 'work' | 'data';
event.locals.preferences = { theme, density };

return resolve(event, {
  transformPageChunk: ({ html }) =>
    html.replace('%theme%', theme).replace('%density%', density)
});
```

(Refactor: move the existing `return resolve(event)` to use the same options call; both branches need the rewrite.)

- [ ] **Step 6: Smoke check the dev server**

```bash
cd $ROOT && just dev
# In another terminal, hit the home and inspect the HTML.
curl -s http://localhost:5173/ | head -2 | grep -o 'data-theme="[a-z]*"'
```

Expected: `data-theme="dark"`. Set `theme=light` cookie via curl and re-fetch:

```bash
curl -s --cookie "theme=light" http://localhost:5173/ | head -2 | grep -o 'data-theme="[a-z]*"'
```

Expected: `data-theme="light"`.

- [ ] **Step 7: Commit**

```bash
cd $ROOT
git add frontend/src/lib/components/ \
        frontend/src/app.html frontend/src/app.d.ts frontend/src/hooks.server.ts
git commit -m "feat(frontend): Modal + settings primitives + theme/density SSR (no flash)"
```

---

## Task 14: Public flows — `/reset/*` (3 pages) and `/email-change/[token]`

**Files:**
- Create: `frontend/src/routes/reset/+page.svelte`
- Create: `frontend/src/routes/reset/+page.server.ts`
- Create: `frontend/src/routes/reset/sent/+page.svelte`
- Create: `frontend/src/routes/reset/[token]/+page.svelte`
- Create: `frontend/src/routes/reset/[token]/+page.server.ts`
- Create: `frontend/src/routes/email-change/[token]/+page.server.ts`
- Modify: `frontend/src/lib/api/client.ts` (add 4 client methods)

- [ ] **Step 1: API client methods**

Open `frontend/src/lib/api/client.ts`. Append (mirroring the existing `api.signup` / `api.login` shape):

```typescript
export const api = {
  // ...existing methods...

  passwordResetRequest: (email: string, opts?: { fetch?: typeof fetch }) =>
    request('POST', '/api/auth/password-reset/request', { email }, opts),

  passwordResetConfirm: (token: string, new_password: string, opts?: { fetch?: typeof fetch }) =>
    request('POST', '/api/auth/password-reset/confirm', { token, new_password }, opts),

  emailChangeConfirm: (token: string, opts?: { fetch?: typeof fetch }) =>
    request<{ status: 'success' | 'expired' | 'taken' }>(
      'POST', '/api/auth/email-change/confirm', { token }, opts),
};
```

(If `request` is not the actual helper name, use whatever wraps `fetch` in this file. Keep the same patterns.)

- [ ] **Step 2: Step 01 — `/reset/+page.svelte` + `+page.server.ts`**

`+page.server.ts`:
```typescript
import { api } from '$lib/api/client';
import { fail, redirect } from '@sveltejs/kit';
import type { Actions } from './$types';

export const actions: Actions = {
  default: async ({ request, fetch }) => {
    const fd = await request.formData();
    const email = String(fd.get('email') ?? '').trim();
    if (!email) return fail(400, { error: 'missing_email' });
    await api.passwordResetRequest(email, { fetch });
    throw redirect(303, `/reset/sent?email=${encodeURIComponent(email)}`);
  }
};
```

`+page.svelte`:
```svelte
<script lang="ts">
  import { enhance } from '$app/forms';
  let { form } = $props();
</script>

<div class="reset-shell">
  <h1>We'll send you a link <em>to find your way back.</em></h1>
  <p>Single-use. Expires in one hour.</p>

  <form method="POST" use:enhance>
    <label class="lbl">EMAIL</label>
    <input type="email" name="email" required class="input" />
    {#if form?.error}<p class="err">Please enter a valid email.</p>{/if}
    <button type="submit" class="btn btn-primary">Send reset link</button>
    <a class="back" href="/signin">← Back to sign in</a>
  </form>
</div>

<style>
  .reset-shell { max-width: 720px; margin: 80px auto; padding: 48px 64px; }
  /* additional styling per design tokens */
</style>
```

- [ ] **Step 3: Step 02 — `/reset/sent/+page.svelte`**

```svelte
<script lang="ts">
  import { page } from '$app/stores';
  let email = $derived($page.url.searchParams.get('email') ?? '');
  let secondsLeft = $state(60);
  let resending = $state(false);
  let resentOk = $state(false);

  $effect(() => {
    const t = setInterval(() => {
      secondsLeft = Math.max(0, secondsLeft - 1);
    }, 1000);
    return () => clearInterval(t);
  });

  async function resend() {
    resending = true;
    await fetch('/api/auth/password-reset/request', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ email })
    });
    resending = false; resentOk = true;
    secondsLeft = 60;
  }
</script>

<div class="reset-shell">
  <h1>A link is on its way <em>to {email}.</em></h1>
  <p>Open the email and click the link to set a new password.</p>

  <pre class="email-preview">
EMAIL PREVIEW · PLAIN TEXT

From:    Astrophoto &lt;noreply@astrophoto.pics&gt;
To:      {email}
Subject: Reset your Astrophoto password

Open this link to choose a new password:

  https://astrophoto.pics/reset/&lt;your-token&gt;

The link is single-use and expires in one hour.
  </pre>

  <div class="actions">
    {#if secondsLeft > 0}
      <button class="btn btn-ghost" disabled>Resend in 0:{String(secondsLeft).padStart(2, '0')}</button>
    {:else}
      <button class="btn btn-ghost" onclick={resend} disabled={resending}>
        {resending ? 'Sending…' : (resentOk ? 'Sent again' : 'Resend link')}
      </button>
    {/if}
    <a class="btn btn-secondary" href="/reset">Use a different email</a>
  </div>
</div>
```

- [ ] **Step 4: Step 03 — `/reset/[token]/+page.svelte` + `+page.server.ts`**

`+page.server.ts`:
```typescript
import { fail, redirect } from '@sveltejs/kit';
import { api, ApiError } from '$lib/api/client';
import type { Actions, PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params, fetch }) => {
  // We do not have a "token-validate" endpoint — render the form
  // optimistically; expired-state is handled at submit.
  return { token: params.token };
};

export const actions: Actions = {
  default: async ({ request, params, fetch, cookies }) => {
    const fd = await request.formData();
    const new_password = String(fd.get('new_password') ?? '');
    if (new_password.length < 12) return fail(400, { error: 'too_short' });
    try {
      const resp = await fetch('/api/auth/password-reset/confirm', {
        method: 'POST', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ token: params.token, new_password })
      });
      if (resp.status === 410) {
        return fail(410, { error: 'expired_or_used' });
      }
      if (!resp.ok) return fail(500, { error: 'server' });
      // The backend's Set-Cookie header is forwarded automatically by SvelteKit's
      // server fetch when the response is consumed below. Force-cookie is not
      // needed; the new session cookie reaches the browser through the redirect.
      throw redirect(303, '/');
    } catch (e) {
      if (e instanceof Response) throw e;
      return fail(500, { error: 'server' });
    }
  }
};
```

`+page.svelte`:
```svelte
<script lang="ts">
  import { enhance } from '$app/forms';
  let { data, form } = $props();
  let pwd = $state('');

  function strength(p: string): number {
    if (p.length < 8) return 1;
    if (p.length < 12) return 2;
    if (p.length < 16) return 3;
    return 4;
  }
</script>

{#if form?.error === 'expired_or_used'}
  <div class="panel danger">
    This link has expired or has already been used.
    <a class="btn btn-primary" href="/reset">Request a new link</a>
  </div>
{:else}
  <div class="reset-shell">
    <h1>Choose a <em>new password</em>.</h1>

    <form method="POST" use:enhance>
      <label class="lbl">NEW PASSWORD</label>
      <input type="password" name="new_password" required minlength="12"
             bind:value={pwd} class="input" />
      <div class="strength">
        {#each [1, 2, 3, 4] as bucket}
          <span class="seg" class:on={strength(pwd) >= bucket}></span>
        {/each}
      </div>

      {#if pwd.length > 0 && pwd.length < 12}
        <p class="warn">Use at least 12 characters.</p>
      {/if}

      {#if form?.error === 'too_short'}
        <p class="err">Password must be at least 12 characters.</p>
      {/if}

      <button type="submit" class="btn btn-primary">Set new password & sign in</button>
    </form>
  </div>
{/if}
```

- [ ] **Step 5: `/email-change/[token]/+page.server.ts`**

```typescript
import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params, fetch }) => {
  const resp = await fetch('/api/auth/email-change/confirm', {
    method: 'POST', headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ token: params.token })
  });
  const body = await resp.json() as { status: 'success' | 'expired' | 'taken' };
  if (body.status === 'success') {
    throw redirect(303, '/settings/email?changed=1');
  }
  return { status: body.status };
};
```

(`+page.svelte` co-located renders an error panel matching the `data.status`.)

- [ ] **Step 6: Smoke test in the browser**

Bring up `just dev`. Visit `http://localhost:5173/reset`, enter a known email (one created via signup). Confirm the redirect to `/reset/sent?email=…`, open MailHog at `http://localhost:8025`, click the link inside the message, set a new password, and verify auto-login.

- [ ] **Step 7: Commit**

```bash
cd $ROOT
git add frontend/src/routes/reset/ \
        frontend/src/routes/email-change/ \
        frontend/src/lib/api/client.ts
git commit -m "feat(frontend): public password-reset (3 steps) + email-change confirm route"
```

---

## Task 15: `/settings/+layout` shell + Profile/Appearance/Sessions pages

**Files:**
- Create: `frontend/src/routes/settings/+layout.svelte`
- Create: `frontend/src/routes/settings/+layout.server.ts`
- Create: `frontend/src/routes/settings/+page.server.ts` (redirect to /profile)
- Create: `frontend/src/routes/settings/profile/+page.svelte` + `+page.server.ts`
- Create: `frontend/src/routes/settings/appearance/+page.svelte` + `+page.server.ts`
- Create: `frontend/src/routes/settings/sessions/+page.svelte` + `+page.server.ts`
- Modify: `frontend/src/lib/api/client.ts` (`getProfile`, `putProfile`, `getPreferences`, `putPreferences`, `listSessions`, `revokeSession`, `signOutOthers`)

- [ ] **Step 1: Settings layout shell**

`+layout.server.ts`:
```typescript
import { redirect } from '@sveltejs/kit';
import type { LayoutServerLoad } from './$types';

export const load: LayoutServerLoad = async ({ locals, url }) => {
  if (!locals.user) {
    throw redirect(303, `/signin?next=${encodeURIComponent(url.pathname)}`);
  }
  return { user: locals.user };
};
```

`+layout.svelte`:
```svelte
<script lang="ts">
  import { page } from '$app/stores';
  let { children } = $props();

  const items = [
    { slug: 'profile',       label: 'PROFILE',          enabled: true,  tone: '' },
    { slug: 'equipment',     label: 'EQUIPMENT',        enabled: false, tone: '' },
    { slug: 'notifications', label: 'NOTIFICATIONS',    enabled: false, tone: '' },
    { slug: 'email',         label: 'EMAIL & SECURITY', enabled: true,  tone: '' },
    { slug: 'appearance',    label: 'APPEARANCE',       enabled: true,  tone: '' },
    { slug: 'sessions',      label: 'SESSIONS',         enabled: true,  tone: '' },
    { slug: 'delete',        label: 'DELETE ACCOUNT',   enabled: true,  tone: 'danger' }
  ];

  let active = $derived($page.url.pathname.split('/').pop() ?? 'profile');
</script>

<div class="settings-shell">
  <header class="settings-head">
    <span class="eyebrow">PREFERENCES</span>
    <h1>Account <em>settings</em></h1>
  </header>

  <div class="settings-grid">
    <nav class="settings-nav" aria-label="Settings sections">
      {#each items as item}
        {#if item.enabled}
          <a class="nav-item" class:active={active === item.slug} class:danger={item.tone === 'danger'}
             href="/settings/{item.slug}">{item.label}</a>
        {:else}
          <span class="nav-item disabled">
            {item.label}
            <em class="chip chip-soon">SOON</em>
          </span>
        {/if}
      {/each}
      <p class="footer-note">
        ALL CHANGES AUTOSAVE<br>
        EXCEPT EMAIL · PASSWORD<br>
        · DELETION
      </p>
    </nav>
    <main class="settings-content">{@render children()}</main>
  </div>
</div>

<style>
  .settings-shell { max-width: 1280px; margin: 0 auto; padding: 64px; }
  .eyebrow { font-family: var(--font-mono); font-size: 11px; text-transform: uppercase; letter-spacing: 0.16em; color: var(--fg-muted); }
  h1 { font-family: var(--font-display); font-size: 48px; font-weight: 600; }
  .settings-grid { display: grid; grid-template-columns: 240px 720px; gap: 64px; margin-top: 32px; }
  .settings-nav { position: sticky; top: 0; align-self: start; }
  .nav-item { display: block; padding: 12px 0; font-family: var(--font-mono); font-size: 12px; letter-spacing: 0.12em; color: var(--fg-muted); text-decoration: none; }
  .nav-item.active { color: var(--accent); border-left: 1px solid var(--accent); padding-left: 12px; background: var(--bg-accent-tint); }
  .nav-item.danger { color: var(--danger); }
  .nav-item.disabled { color: var(--fg-faint); cursor: not-allowed; }
  .chip-soon { font-style: normal; margin-left: 8px; font-size: 10px; padding: 2px 6px; border: 1px dashed var(--border-default); }
  .footer-note { font-family: var(--font-mono); font-size: 10px; color: var(--fg-faint); margin-top: 32px; line-height: 1.6; }
</style>
```

`/settings/+page.server.ts`:
```typescript
import { redirect } from '@sveltejs/kit';
export const load = () => { throw redirect(303, '/settings/profile'); };
```

- [ ] **Step 2: Profile page**

`/settings/profile/+page.server.ts`:
```typescript
import type { PageServerLoad, Actions } from './$types';
import { api } from '$lib/api/client';

export const load: PageServerLoad = async ({ fetch }) => ({
  profile: await api.getProfile({ fetch })
});

export const actions: Actions = {
  default: async ({ request, fetch }) => {
    const fd = await request.formData();
    await api.putProfile({ display_name: String(fd.get('display_name') ?? '') }, { fetch });
    return { ok: true };
  }
};
```

`/settings/profile/+page.svelte`:
```svelte
<script lang="ts">
  import Section from '$lib/components/settings/Section.svelte';
  import Row from '$lib/components/settings/Row.svelte';
  import AutosaveField from '$lib/components/settings/AutosaveField.svelte';
  let { data } = $props();
  let name = $state(data.profile.display_name);
</script>

<Section title="Profile" description="How you appear under your photos.">
  <Row label="DISPLAY NAME">
    <AutosaveField bind:value={name} name="display_name" action="?/default" />
  </Row>
</Section>
```

- [ ] **Step 3: Appearance page**

`/settings/appearance/+page.server.ts`:
```typescript
import type { PageServerLoad, Actions } from './$types';
import { api } from '$lib/api/client';

export const load: PageServerLoad = async ({ locals }) => ({
  preferences: locals.preferences
});

export const actions: Actions = {
  setTheme: async ({ request, fetch, cookies }) => {
    const fd = await request.formData();
    const theme = String(fd.get('theme') ?? 'dark');
    cookies.set('theme', theme, { path: '/', maxAge: 60 * 60 * 24 * 365, sameSite: 'lax' });
    await api.putPreferences({ theme }, { fetch });
    return { ok: true };
  },
  setDensity: async ({ request, fetch, cookies }) => {
    const fd = await request.formData();
    const density = String(fd.get('density') ?? 'work');
    cookies.set('density', density, { path: '/', maxAge: 60 * 60 * 24 * 365, sameSite: 'lax' });
    await api.putPreferences({ density }, { fetch });
    return { ok: true };
  }
};
```

`/settings/appearance/+page.svelte`:
```svelte
<script lang="ts">
  import Section from '$lib/components/settings/Section.svelte';
  import Row from '$lib/components/settings/Row.svelte';
  let { data } = $props();
</script>

<Section title="Appearance" description="The interface adapts to dark-adapted eyes by default.">
  <Row label="THEME">
    <form method="POST" action="?/setTheme" class="chip-group">
      <button name="theme" value="dark"  class:active={data.preferences.theme === 'dark'}>DARK</button>
      <button name="theme" value="light" class:active={data.preferences.theme === 'light'}>LIGHT</button>
    </form>
  </Row>
  <Row label="DENSITY">
    <form method="POST" action="?/setDensity" class="chip-group">
      <button name="density" value="work" class:active={data.preferences.density === 'work'}>WORK</button>
      <button name="density" value="data" class:active={data.preferences.density === 'data'}>DATA</button>
    </form>
  </Row>
</Section>
```

- [ ] **Step 4: Sessions page**

`/settings/sessions/+page.server.ts`:
```typescript
import type { PageServerLoad, Actions } from './$types';
import { api } from '$lib/api/client';

export const load: PageServerLoad = async ({ fetch }) => ({
  sessions: await api.listSessions({ fetch })
});

export const actions: Actions = {
  revoke: async ({ request, fetch }) => {
    const fd = await request.formData();
    const id = String(fd.get('id') ?? '');
    await api.revokeSession(id, { fetch });
    return { ok: true };
  },
  signOutOthers: async ({ fetch }) => {
    await api.signOutOthers({ fetch });
    return { ok: true };
  }
};
```

`/settings/sessions/+page.svelte`:
```svelte
<script lang="ts">
  import Section from '$lib/components/settings/Section.svelte';
  let { data } = $props();
  let confirming = $state(false);

  function relative(iso: string): string {
    const dt = (Date.now() - new Date(iso).getTime()) / 1000;
    if (dt < 60) return 'just now';
    if (dt < 3600) return `${Math.floor(dt/60)} minutes ago`;
    if (dt < 86400) return `${Math.floor(dt/3600)} hours ago`;
    return `${Math.floor(dt/86400)} days ago`;
  }
</script>

<Section title="Active sessions" description="Devices currently signed in to this account.">
  <ul class="sessions" role="list">
    {#each data.sessions as s (s.id)}
      <li class="session-row" class:current={s.is_current}>
        <span class="dot" class:on={s.is_current}></span>
        <div class="info">
          <strong>{s.os} · {s.browser}{#if s.is_current} <em class="muted-accent">· this device</em>{/if}</strong>
          <span class="meta">{s.browser} {s.browser_version} · {s.os} {s.os_version}</span>
          <span class="meta">IP {s.ip} · {relative(s.last_used_at)}</span>
        </div>
        {#if !s.is_current}
          <form method="POST" action="?/revoke">
            <input type="hidden" name="id" value={s.id} />
            <button class="btn btn-danger btn-sm" aria-label="Revoke session: {s.os} {s.browser}">Revoke</button>
          </form>
        {/if}
      </li>
    {/each}
  </ul>

  {#if data.sessions.filter(s => !s.is_current).length > 0}
    {#if confirming}
      <form method="POST" action="?/signOutOthers">
        <p>End {data.sessions.filter(s => !s.is_current).length} other session(s)?</p>
        <button class="btn btn-secondary">Confirm sign-out</button>
        <button type="button" onclick={() => { confirming = false; }}>Cancel</button>
      </form>
    {:else}
      <button class="btn btn-secondary" onclick={() => { confirming = true; }}>
        Sign out of all other sessions
      </button>
    {/if}
  {/if}
</Section>
```

- [ ] **Step 5: API client additions**

```typescript
getProfile: (opts?: { fetch?: typeof fetch }) =>
  request<{ display_name: string }>('GET', '/api/me/profile', undefined, opts),
putProfile: (body: { display_name?: string }, opts?: { fetch?: typeof fetch }) =>
  request('PUT', '/api/me/profile', body, opts),
getPreferences: (opts?: { fetch?: typeof fetch }) =>
  request<{ theme: string; density: string }>('GET', '/api/me/preferences', undefined, opts),
putPreferences: (body: { theme?: string; density?: string }, opts?: { fetch?: typeof fetch }) =>
  request('PUT', '/api/me/preferences', body, opts),
listSessions: (opts?: { fetch?: typeof fetch }) =>
  request<Array<{ id: string; browser: string; browser_version: string; os: string; os_version: string; ip: string; last_used_at: string; created_at: string; is_current: boolean }>>(
    'GET', '/api/me/sessions', undefined, opts),
revokeSession: (id: string, opts?: { fetch?: typeof fetch }) =>
  request('DELETE', `/api/me/sessions/${encodeURIComponent(id)}`, undefined, opts),
signOutOthers: (opts?: { fetch?: typeof fetch }) =>
  request('POST', '/api/me/sessions/sign-out-others', undefined, opts),
```

- [ ] **Step 6: Browser smoke**

Run `just dev`. Sign in. Visit `/settings`. Confirm:
- Redirect to `/settings/profile` works.
- Editing the display name autosaves; "● Saved" appears.
- Theme toggle changes the page on next request (cookie set).
- Sessions page lists at least the current session, marks it accordingly.

- [ ] **Step 7: Commit**

```bash
cd $ROOT
git add frontend/src/routes/settings/ frontend/src/lib/api/client.ts
git commit -m "feat(frontend): /settings shell + Profile, Appearance, Sessions pages"
```

---

## Task 16: `/settings/email`, `/settings/password`, `/settings/delete`, grace banner

**Files:**
- Create: `frontend/src/routes/settings/email/+page.svelte` + `+page.server.ts`
- Create: `frontend/src/routes/settings/password/+page.svelte` + `+page.server.ts`
- Create: `frontend/src/routes/settings/delete/+page.svelte` + `+page.server.ts`
- Modify: `frontend/src/routes/+layout.svelte` (add grace banner)
- Modify: `frontend/src/routes/+layout.server.ts` (load photo count when grace active)
- Modify: `frontend/src/lib/api/client.ts` (`requestEmailChange`, `changePassword`, `requestDeletion`, `cancelDeletion`)
- Test: `frontend/tests/e2e/security_account.spec.ts`

- [ ] **Step 1: Email page (modal-style)**

`/settings/email/+page.server.ts`:
```typescript
import type { PageServerLoad, Actions } from './$types';
import { fail } from '@sveltejs/kit';
import { api } from '$lib/api/client';

export const load: PageServerLoad = async ({ locals }) => ({
  user: locals.user,
  changed: false
});

export const actions: Actions = {
  requestChange: async ({ request, fetch }) => {
    const fd = await request.formData();
    const new_email = String(fd.get('new_email') ?? '').trim();
    const current_password = String(fd.get('current_password') ?? '');
    if (!new_email) return fail(400, { error: 'missing_email' });
    try {
      await api.requestEmailChange(new_email, current_password, { fetch });
      return { ok: true };
    } catch (e: unknown) {
      return fail(401, { error: 'wrong_password' });
    }
  }
};
```

`/settings/email/+page.svelte`:
```svelte
<script lang="ts">
  import Section from '$lib/components/settings/Section.svelte';
  import Row from '$lib/components/settings/Row.svelte';
  import Modal from '$lib/components/Modal.svelte';
  let { data, form } = $props();
  let showModal = $state(false);
</script>

<Section title="Sign-in identity" description="The email used to sign in and recover your account.">
  <Row label="EMAIL">
    <span class="value">{data.user.email}</span>
    <button class="btn btn-secondary btn-sm" onclick={() => { showModal = true; }}>Change…</button>
  </Row>
</Section>

<Modal bind:open={showModal} title="Change email">
  <span class="eyebrow">● CHANGE EMAIL · VERIFICATION REQUIRED</span>
  <h2>Change <em>your sign-in email</em></h2>
  <form method="POST" action="?/requestChange">
    <label class="lbl">NEW EMAIL</label>
    <input type="email" name="new_email" required class="input" />
    <label class="lbl">CURRENT PASSWORD</label>
    <input type="password" name="current_password" required class="input" />
    {#if form?.error === 'wrong_password'}<p class="err">Wrong password.</p>{/if}
    {#if form?.ok}<p class="ok">Check your new inbox for a confirmation link.</p>{/if}
    <button type="submit" class="btn btn-primary">Send confirmation link</button>
  </form>
</Modal>
```

- [ ] **Step 2: Password page**

`/settings/password/+page.server.ts`:
```typescript
import type { Actions } from './$types';
import { fail } from '@sveltejs/kit';
import { api } from '$lib/api/client';

export const actions: Actions = {
  default: async ({ request, fetch }) => {
    const fd = await request.formData();
    const current_password = String(fd.get('current_password') ?? '') || undefined;
    const new_password = String(fd.get('new_password') ?? '');
    if (new_password.length < 12) return fail(400, { error: 'too_short' });
    try {
      await api.changePassword({ current_password, new_password }, { fetch });
      return { ok: true };
    } catch {
      return fail(401, { error: 'wrong_password' });
    }
  }
};
```

`/settings/password/+page.svelte`:
```svelte
<script lang="ts">
  import Section from '$lib/components/settings/Section.svelte';
  let { form } = $props();
</script>

<Section title="Change password">
  <form method="POST">
    <label class="lbl">CURRENT PASSWORD</label>
    <input type="password" name="current_password" class="input" />
    <a class="link-accent" href="/reset">I don't remember it →</a>

    <label class="lbl">NEW PASSWORD</label>
    <input type="password" name="new_password" required minlength="12" class="input" />

    {#if form?.error === 'wrong_password'}<p class="err">Wrong current password.</p>{/if}
    {#if form?.error === 'too_short'}<p class="err">Use at least 12 characters.</p>{/if}
    {#if form?.ok}<p class="ok">Password changed. Other devices have been signed out.</p>{/if}

    <button type="submit" class="btn btn-primary">Save new password</button>
  </form>
</Section>
```

- [ ] **Step 3: Delete page**

`/settings/delete/+page.server.ts`:
```typescript
import type { PageServerLoad, Actions } from './$types';
import { fail, redirect } from '@sveltejs/kit';
import { api } from '$lib/api/client';

export const load: PageServerLoad = async ({ locals }) => ({
  pending_deletion_at: locals.user?.pending_deletion_at ?? null
});

export const actions: Actions = {
  request: async ({ request, fetch }) => {
    const fd = await request.formData();
    const phrase = String(fd.get('confirmation_phrase') ?? '');
    const current_password = String(fd.get('current_password') ?? '') || undefined;
    if (phrase !== 'DELETE MY ACCOUNT') return fail(400, { error: 'phrase' });
    try {
      await api.requestDeletion({ confirmation_phrase: phrase, current_password }, { fetch });
      throw redirect(303, '/settings/delete');
    } catch (e) {
      if (e instanceof Response) throw e;
      return fail(401, { error: 'wrong_password' });
    }
  },
  cancel: async ({ fetch }) => {
    await api.cancelDeletion({ fetch });
    throw redirect(303, '/settings/delete');
  }
};
```

`/settings/delete/+page.svelte`:
```svelte
<script lang="ts">
  import Section from '$lib/components/settings/Section.svelte';
  let { data, form } = $props();
  let phrase = $state('');
</script>

{#if data.pending_deletion_at}
  <div class="panel danger">
    <span class="eyebrow">● DELETION SCHEDULED</span>
    <p>Your account will be permanently erased on {new Date(data.pending_deletion_at).toLocaleString()}.</p>
    <form method="POST" action="?/cancel">
      <button class="btn btn-primary">Cancel deletion · keep my account</button>
    </form>
    <a class="btn btn-secondary" href="/api/me/export.json" download>Download my archive (JSON)</a>
  </div>
{:else}
  <Section title="Delete account" tone="danger" description="Closing your account erases your photos, comments, and identity. There is a 7-day grace period.">
    <form method="POST" action="?/request">
      <label class="lbl">CURRENT PASSWORD</label>
      <input type="password" name="current_password" class="input" />
      <label class="lbl">TYPE “DELETE MY ACCOUNT” TO CONFIRM</label>
      <input type="text" name="confirmation_phrase" bind:value={phrase} class="input" />
      {#if form?.error === 'phrase'}<p class="err">The phrase doesn't match.</p>{/if}
      {#if form?.error === 'wrong_password'}<p class="err">Wrong password.</p>{/if}
      <button type="submit" class="btn btn-danger" disabled={phrase !== 'DELETE MY ACCOUNT'}>
        Begin 7-day deletion
      </button>
    </form>
  </Section>
{/if}
```

- [ ] **Step 4: Grace banner in root layout**

Open `frontend/src/routes/+layout.server.ts`. After loading the user, conditionally count photos:

```typescript
import type { LayoutServerLoad } from './$types';

export const load: LayoutServerLoad = async ({ locals, fetch }) => {
  let frame_count: number | null = null;
  if (locals.user?.pending_deletion_at) {
    const r = await fetch('/api/photos?owner=me&count_only=1');
    if (r.ok) {
      const v = await r.json();
      frame_count = typeof v.count === 'number' ? v.count : null;
    }
  }
  return { user: locals.user, frame_count };
};
```

(If `/api/photos` doesn't yet support `count_only`, prefer adding a tiny `GET /api/me/photos/count` to the backend instead. Either way, the frontend just needs the integer.)

Open `frontend/src/routes/+layout.svelte`. Above the existing main slot, add:

```svelte
<script lang="ts">
  import { page } from '$app/stores';
  let { data, children } = $props();
  let countdown = $state('');

  function refresh() {
    if (!data.user?.pending_deletion_at) return;
    const left = new Date(data.user.pending_deletion_at).getTime() - Date.now();
    if (left <= 0) { countdown = 'imminent'; return; }
    const days = Math.floor(left / 86_400_000);
    const hours = Math.floor((left % 86_400_000) / 3_600_000);
    countdown = `${days} days, ${hours} hours`;
  }

  $effect(() => {
    refresh();
    const t = setInterval(refresh, 60_000);
    return () => clearInterval(t);
  });
</script>

{#if data.user?.pending_deletion_at}
  <div class="grace-banner">
    <span class="eyebrow">● ACCOUNT MARKED FOR DELETION</span>
    Permanent removal in <strong>{countdown}</strong>
    {#if data.frame_count !== null} · {data.frame_count} frames will be erased{/if}
    <form method="POST" action="/settings/delete?/cancel" class="cancel-form">
      <button class="link-accent">Cancel deletion</button>
    </form>
  </div>
{/if}

{@render children()}

<style>
  .grace-banner {
    background: var(--bg-danger-tint); border-bottom: 1px solid var(--danger);
    color: var(--fg-primary); font-family: var(--font-mono); font-size: 12px;
    padding: 12px 64px; display: flex; gap: 24px; align-items: center;
  }
  .grace-banner .cancel-form { margin-left: auto; }
  .link-accent { color: var(--accent); background: none; border: 0; text-decoration: underline; cursor: pointer; }
</style>
```

- [ ] **Step 5: API client additions**

```typescript
requestEmailChange: (new_email: string, current_password: string, opts?: { fetch?: typeof fetch }) =>
  request('POST', '/api/me/email-change/request', { new_email, current_password }, opts),

changePassword: (body: { current_password?: string; new_password: string }, opts?: { fetch?: typeof fetch }) =>
  request('POST', '/api/me/password-change', body, opts),

requestDeletion: (body: { current_password?: string; confirmation_phrase: string }, opts?: { fetch?: typeof fetch }) =>
  request('POST', '/api/me/delete-request', body, opts),

cancelDeletion: (opts?: { fetch?: typeof fetch }) =>
  request('POST', '/api/me/delete-cancel', undefined, opts),
```

- [ ] **Step 6: e2e tests**

Create `frontend/tests/e2e/security_account.spec.ts`:

```typescript
import { test, expect } from '@playwright/test';

const MAILHOG = 'http://localhost:8025';

async function latestMailLink(page, recipient: string, prefix: string) {
  const r = await page.request.get(`${MAILHOG}/api/v2/messages`);
  const json = await r.json();
  const msgs = json.items.filter((m: any) =>
    m.Content.Headers.To?.[0]?.includes(recipient));
  const body = msgs[0].Content.Body as string;
  const match = body.match(new RegExp(`${prefix}/[A-Za-z0-9_-]+`));
  return match ? match[0] : null;
}

test('reset password from sign-in, click MailHog link, set new password, land authenticated', async ({ page, request }) => {
  const email = `e2e-${Date.now()}@reset.test`;
  // Signup
  await request.post('http://localhost:8080/api/auth/signup', {
    data: { email, password: 'longenoughpw1', display_name: 'E2E' }
  });
  // Request reset
  await page.goto('http://localhost:5173/reset');
  await page.getByLabel('EMAIL').fill(email);
  await page.getByRole('button', { name: 'Send reset link' }).click();
  await expect(page).toHaveURL(/\/reset\/sent/);
  // Pull link from MailHog
  const link = await latestMailLink(page, email, '/reset');
  expect(link).toBeTruthy();
  await page.goto(`http://localhost:5173${link!}`);
  await page.getByLabel('NEW PASSWORD').fill('a-strong-new-password-x9');
  await page.getByRole('button', { name: 'Set new password & sign in' }).click();
  await expect(page).toHaveURL(/\/$/);
});

test('toggle theme persists across reload', async ({ page }) => {
  await page.goto('http://localhost:5173/settings/appearance');
  // Sign in if redirected (test fixture should handle, omitted here for brevity).
  await page.getByRole('button', { name: 'LIGHT' }).click();
  await page.reload();
  const html = await page.locator('html').getAttribute('data-theme');
  expect(html).toBe('light');
});
```

(Add a `playwright.config.ts` test fixture for sign-in if not already present; the second test omits it for brevity.)

- [ ] **Step 7: Browser smoke**

Run `just dev`. Sign in. Visit `/settings/email`, click Change…, fill the modal, check MailHog (`http://localhost:8025`) for the confirmation link, click it, verify the email row updates and a flash appears. Visit `/settings/password`, change the password, verify "other devices signed out". Visit `/settings/delete`, type the phrase, click Begin 7-day deletion, verify the grace banner appears on every authenticated route. Click Cancel deletion, verify the banner disappears.

- [ ] **Step 8: Commit**

```bash
cd $ROOT
git add frontend/src/routes/settings/email/ \
        frontend/src/routes/settings/password/ \
        frontend/src/routes/settings/delete/ \
        frontend/src/routes/+layout.svelte \
        frontend/src/routes/+layout.server.ts \
        frontend/src/lib/api/client.ts \
        frontend/tests/
git commit -m "feat(frontend): /settings/{email,password,delete} + grace banner in root layout"
```

---

## Done

After Task 16:

1. Push the branch and open a PR linking the spec.
2. Verify `just check` passes (rust + sqlx-prepared + svelte-check + lints).
3. Manual check that the gate `feat/phase-8a-security` (the working branch) sees:
   - The `mailhog` container is up.
   - `MAIL_FROM` is set in `.env` for local; for prod, AWS SES SMTP credentials and a verified `MAIL_FROM` sender.
   - The first sign-in after deploy still works (no orphaned sessions from the new `last_used_at` column — the `default now()` clause covers existing rows).

**Phase 8b (Photos & polish — drafts surfaced, replace image, polish 8.5)** is brainstormed and planned separately.

