# Email verification on signup — implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an email-confirmation step to password-based signup so new accounts must click a link from a real email before they can sign in.

**Architecture:** Mirror the existing password-reset flow. New nullable `users.email_verified_at` column (backfilled `= created_at` for all existing rows so the launch doesn't lock anyone out), new `email_verification_tokens` table identical in shape to `password_reset_tokens`. Signup creates the user, issues a token, sends an SES email; verify endpoint marks the token used + sets `email_verified_at` + creates the session. Sign-in blocks unverified users with HTTP 403. Google OAuth creations are auto-verified.

**Tech Stack:** Rust (axum, sqlx, lettre), PostgreSQL, SvelteKit (Svelte 5 runes). Mailer is the existing `crate::mail::Mailer::send_plain` over SMTP — in prod that's SES at `email-smtp.us-east-1.amazonaws.com:587`.

**Spec:** `docs/superpowers/specs/2026-05-13-email-verification-on-signup-design.md` (commit `93bde90`).

**Patterns to copy verbatim:**
- Token issuance + throttling: `backend/src/auth/password_reset.rs::request`
- Token confirmation + auto-login: `backend/src/auth/password_reset.rs::confirm`
- Mail template style: `backend/src/mail/templates.rs::password_reset`
- Frontend "check email" page: `frontend/src/routes/reset/sent/+page.svelte`
- Frontend `[token]` confirmation page: `frontend/src/routes/reset/[token]/+page.server.ts`

---

## File Structure

**Backend:**
- Create: `backend/migrations/0016_email_verification.sql`
- Create: `backend/src/auth/email_verify.rs` (new module with `request`, `verify`, `resend` handlers + tests)
- Modify: `backend/src/auth/mod.rs` (add `pub mod email_verify;`)
- Modify: `backend/src/auth/signup.rs` (drop auto-login; issue token; send mail; return 202)
- Modify: `backend/src/auth/login.rs` (block on `email_verified_at IS NULL`)
- Modify: `backend/src/auth/oauth_google.rs:226-235` (set `email_verified_at = now()` on brand-new account insert)
- Modify: `backend/src/users/queries.rs` (add `email_verified_at` to `UserRow` + queries that return it)
- Modify: `backend/src/mail/templates.rs` (add `email_verification(display_name, link)`)
- Modify: `backend/src/http/mod.rs` (wire two new routes)

**Frontend:**
- Modify: `frontend/src/routes/signup/+page.server.ts` (redirect to `/signup/check-email?email=…` on 202; do not set cookie)
- Create: `frontend/src/routes/signup/check-email/+page.svelte`
- Create: `frontend/src/routes/signup/check-email/+page.server.ts` (resend action)
- Create: `frontend/src/routes/verify/[token]/+page.server.ts`
- Modify: `frontend/src/routes/signin/+page.server.ts` (redirect to `/signup/check-email` on 403 `email_unverified`)
- Modify: `frontend/src/lib/api/client.ts` (add `verifyEmail`, `resendVerification` helpers — optional, mirror `passwordResetRequest`)

**Tests:** Add tests inline in each modified Rust module (the repo convention is `#[cfg(test)] mod tests` at the bottom, with testcontainers for DB-touching tests).

---

## Task 1: DB migration — schema only

**Files:** Create `backend/migrations/0016_email_verification.sql`.

- [ ] **Step 1: Write the migration file**

```sql
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
```

- [ ] **Step 2: Reset the local dev DB and run all migrations**

Run from repo root:

```bash
just db-reset
```

Expected: command exits 0; the last line of output mentions migration `0016_email_verification`.

- [ ] **Step 3: Verify the schema**

Run:

```bash
psql "$DATABASE_URL" -c "\d users" | grep email_verified_at
psql "$DATABASE_URL" -c "\d email_verification_tokens"
```

Expected: `email_verified_at | timestamp with time zone` in the users table; `email_verification_tokens` shows the five columns + primary key on `token_hash`.

- [ ] **Step 4: Regenerate sqlx offline metadata**

Run from `backend/`:

```bash
cargo sqlx prepare
```

Expected: command exits 0. May emit "no queries found" — that's fine for now (no Rust code uses the new table yet).

- [ ] **Step 5: Commit**

```bash
git add backend/migrations/0016_email_verification.sql backend/.sqlx
git commit -m "feat(db): migration 0016 email_verification (schema + backfill)"
```

---

## Task 2: Extend `UserRow` with `email_verified_at`

**Files:** Modify `backend/src/users/queries.rs`.

- [ ] **Step 1: Write the failing test**

Add to the bottom of `backend/src/users/queries.rs` (inside any existing `#[cfg(test)] mod tests`, or create one if absent):

```rust
#[cfg(test)]
mod tests_email_verified {
    use super::*;
    use crate::test_support::pg;

    #[tokio::test]
    async fn new_password_account_has_no_email_verified_at() {
        let pool = pg::new_pool().await;
        let user = create_with_password(&pool, "u@example.com", "u-abc", "U", "hash")
            .await
            .unwrap();
        assert!(user.email_verified_at.is_none());
    }

    #[tokio::test]
    async fn find_by_email_returns_verified_timestamp() {
        let pool = pg::new_pool().await;
        let user = create_with_password(&pool, "v@example.com", "v-abc", "V", "hash")
            .await
            .unwrap();
        sqlx::query!(
            "update users set email_verified_at = now() where id = $1",
            user.id
        )
        .execute(&pool)
        .await
        .unwrap();
        let fetched = find_by_email(&pool, "v@example.com").await.unwrap().unwrap();
        assert!(fetched.email_verified_at.is_some());
    }
}
```

- [ ] **Step 2: Run the tests — they should fail to compile**

Run from `backend/`:

```bash
cargo test --no-run users::queries::tests_email_verified
```

Expected: compile error — `email_verified_at` is not a field of `UserRow`.

- [ ] **Step 3: Add the field to `UserRow`**

Edit `backend/src/users/queries.rs:7-14`:

```rust
pub struct UserRow {
    pub id: Uuid,
    pub email: String,
    pub handle: String,
    pub display_name: String,
    pub password_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub email_verified_at: Option<DateTime<Utc>>,
}
```

- [ ] **Step 4: Update both queries to select the new column**

Edit `backend/src/users/queries.rs:23-35` (the `create_with_password` SQL block):

```rust
sqlx::query_as!(
    UserRow,
    r#"
    insert into users (email, handle, display_name, password_hash, password_changed_at)
    values ($1, $2, $3, $4, now())
    returning id, email::text as "email!", handle::text as "handle!", display_name, password_hash, created_at, email_verified_at
    "#,
    email,
    handle,
    display_name,
    password_hash,
)
```

Edit `backend/src/users/queries.rs:48-58` (the `find_by_email` SQL block):

```rust
let row = sqlx::query_as!(
    UserRow,
    r#"
    select id, email::text as "email!", handle::text as "handle!", display_name, password_hash, created_at, email_verified_at
    from users where email = $1
    "#,
    email
)
```

- [ ] **Step 5: Regenerate sqlx offline metadata**

Run from `backend/`:

```bash
cargo sqlx prepare
```

Expected: exit 0, `.sqlx/` updated.

- [ ] **Step 6: Run the tests — they should pass**

Run from `backend/`:

```bash
cargo test users::queries::tests_email_verified
```

Expected: both tests pass.

- [ ] **Step 7: Fix any compile errors elsewhere in the crate**

Run:

```bash
cargo check --all-targets
```

If any callers of `UserRow` are pattern-matching with `..` they'll be fine. If any are exhaustively destructuring, add `email_verified_at` to their match. Likely none — but check before committing.

Expected: zero errors.

- [ ] **Step 8: Commit**

```bash
git add backend/src/users/queries.rs backend/.sqlx
git commit -m "feat(users): expose email_verified_at on UserRow"
```

---

## Task 3: Mail template for verification email

**Files:** Modify `backend/src/mail/templates.rs`.

- [ ] **Step 1: Write the failing test**

Add to the bottom of `backend/src/mail/templates.rs` (in a `#[cfg(test)] mod tests` block; create if absent):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn email_verification_subject_and_body_shape() {
        let (subject, body) = email_verification("Galaxy Lover", "https://example.com/verify/abc123");
        assert_eq!(subject, "Confirm your Astrophoto account");
        assert!(body.contains("Galaxy Lover"));
        assert!(body.contains("https://example.com/verify/abc123"));
        assert!(body.contains("24 hours"));
        assert!(body.contains("Clear skies"));
    }
}
```

- [ ] **Step 2: Run the test — should fail to compile**

Run from `backend/`:

```bash
cargo test mail::templates::tests::email_verification_subject_and_body_shape
```

Expected: compile error — `email_verification` is not defined.

- [ ] **Step 3: Add the template function**

Append to `backend/src/mail/templates.rs`:

```rust
pub fn email_verification(display_name: &str, link: &str) -> (String, String) {
    let subject = "Confirm your Astrophoto account".to_string();
    let body = format!(
        "Hello {display_name},\n\n\
         Welcome! Open this link to confirm your email and finish setting up \
         your Astrophoto account:\n\n\
         {link}\n\n\
         This link is single-use and expires in 24 hours. If you didn't sign \
         up, you can ignore this message.\n\n\
         Clear skies,\n\
         The Astrophoto archive\n"
    );
    (subject, body)
}
```

- [ ] **Step 4: Run the test — should pass**

```bash
cargo test mail::templates::tests::email_verification_subject_and_body_shape
```

Expected: 1 passed.

- [ ] **Step 5: Commit**

```bash
git add backend/src/mail/templates.rs
git commit -m "feat(mail): plain-text template for email-verification"
```

---

## Task 4: `email_verify` module — `request` (token issuance helper)

This module starts as just an internal token-issuance helper. The HTTP handlers come in Task 5+6.

**Files:** Create `backend/src/auth/email_verify.rs`. Modify `backend/src/auth/mod.rs`.

- [ ] **Step 1: Wire the module**

Edit `backend/src/auth/mod.rs`. Add alongside the other `pub mod` lines:

```rust
pub mod email_verify;
```

- [ ] **Step 2: Write the failing test for the token-issuance helper**

Create `backend/src/auth/email_verify.rs` with:

```rust
//! Email verification: token issuance + verify endpoint + resend endpoint.
//! Mirrors the structure of `auth/password_reset.rs` exactly.

use std::net::IpAddr;

use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use sqlx::types::ipnetwork::IpNetwork;
use uuid::Uuid;

use crate::AppError;

pub(crate) const TTL_HOURS: i32 = 24;
pub(crate) const PER_EMAIL_COOLDOWN_SECS: f64 = 60.0;
pub(crate) const PER_HOUR_CAP: i64 = 5;

/// Generate a fresh token, insert its sha256 hash, return the raw token
/// (URL-safe base64, no padding) for embedding into the email link.
pub(crate) async fn issue_token(
    pool: &PgPool,
    user_id: Uuid,
    ip: Option<IpAddr>,
) -> Result<String, AppError> {
    let mut raw = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut raw);
    let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw);
    let hash: Vec<u8> = Sha256::digest(token.as_bytes()).to_vec();

    sqlx::query!(
        "insert into email_verification_tokens (token_hash, user_id, expires_at, request_ip)
          values ($1, $2, now() + make_interval(hours => $3), $4)",
        hash,
        user_id,
        TTL_HOURS,
        ip.map(IpNetwork::from)
    )
    .execute(pool)
    .await?;

    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::pg;
    use crate::users::queries::create_with_password;

    #[tokio::test]
    async fn issue_token_writes_a_row_we_can_find_by_hash() {
        let pool = pg::new_pool().await;
        let user = create_with_password(&pool, "tok@example.com", "tok-abc", "T", "hash")
            .await
            .unwrap();
        let token = issue_token(&pool, user.id, None).await.unwrap();
        assert_eq!(token.len(), 43); // 32 bytes → 43 chars URL-safe-base64 (no padding)

        let hash: Vec<u8> = Sha256::digest(token.as_bytes()).to_vec();
        let row = sqlx::query!(
            "select user_id, used_at, expires_at from email_verification_tokens where token_hash = $1",
            hash
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.user_id, user.id);
        assert!(row.used_at.is_none());
        assert!(row.expires_at > chrono::Utc::now());
    }
}
```

- [ ] **Step 3: Compile + run the test**

Run from `backend/`:

```bash
cargo sqlx prepare && cargo test auth::email_verify::tests::issue_token_writes_a_row_we_can_find_by_hash
```

Expected: 1 passed.

- [ ] **Step 4: Commit**

```bash
git add backend/src/auth/mod.rs backend/src/auth/email_verify.rs backend/.sqlx
git commit -m "feat(auth): email_verify module with token-issuance helper"
```

---

## Task 5: `email_verify::verify` — the click-through endpoint

**Files:** Modify `backend/src/auth/email_verify.rs`. Modify `backend/src/http/mod.rs`.

- [ ] **Step 1: Write the failing tests**

Append to `backend/src/auth/email_verify.rs` (inside the existing `mod tests`):

```rust
#[tokio::test]
async fn verify_marks_user_verified_and_consumes_token() {
    use crate::http::AppState;

    let pool = pg::new_pool().await;
    let user = create_with_password(&pool, "v1@example.com", "v1-abc", "V1", "hash")
        .await
        .unwrap();
    let token = issue_token(&pool, user.id, None).await.unwrap();

    let state = AppState::for_test(pool.clone()).await;
    let body = serde_json::json!({ "token": token });
    let resp = verify(
        axum::extract::State(state),
        axum::Json(serde_json::from_value(body).unwrap()),
    )
    .await
    .unwrap()
    .into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    assert!(resp.headers().get("set-cookie").is_some());

    let row = sqlx::query!(
        "select email_verified_at from users where id = $1",
        user.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(row.email_verified_at.is_some());
}

#[tokio::test]
async fn verify_with_used_token_returns_gone() {
    use crate::http::AppState;

    let pool = pg::new_pool().await;
    let user = create_with_password(&pool, "v2@example.com", "v2-abc", "V2", "hash")
        .await
        .unwrap();
    let token = issue_token(&pool, user.id, None).await.unwrap();
    let hash: Vec<u8> = Sha256::digest(token.as_bytes()).to_vec();
    sqlx::query!(
        "update email_verification_tokens set used_at = now() where token_hash = $1",
        hash
    )
    .execute(&pool)
    .await
    .unwrap();

    let state = AppState::for_test(pool.clone()).await;
    let body = serde_json::json!({ "token": token });
    let err = verify(
        axum::extract::State(state),
        axum::Json(serde_json::from_value(body).unwrap()),
    )
    .await
    .err()
    .unwrap();
    let resp = err.into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::GONE);
}

#[tokio::test]
async fn verify_with_expired_token_returns_gone() {
    use crate::http::AppState;

    let pool = pg::new_pool().await;
    let user = create_with_password(&pool, "v3@example.com", "v3-abc", "V3", "hash")
        .await
        .unwrap();
    let token = issue_token(&pool, user.id, None).await.unwrap();
    let hash: Vec<u8> = Sha256::digest(token.as_bytes()).to_vec();
    sqlx::query!(
        "update email_verification_tokens set expires_at = now() - interval '1 hour' where token_hash = $1",
        hash
    )
    .execute(&pool)
    .await
    .unwrap();

    let state = AppState::for_test(pool.clone()).await;
    let body = serde_json::json!({ "token": token });
    let err = verify(
        axum::extract::State(state),
        axum::Json(serde_json::from_value(body).unwrap()),
    )
    .await
    .err()
    .unwrap();
    let resp = err.into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::GONE);
}

#[tokio::test]
async fn verify_with_unknown_token_returns_gone() {
    use crate::http::AppState;

    let pool = pg::new_pool().await;
    let state = AppState::for_test(pool.clone()).await;
    let body = serde_json::json!({ "token": "definitely-not-a-real-token-xxx" });
    let err = verify(
        axum::extract::State(state),
        axum::Json(serde_json::from_value(body).unwrap()),
    )
    .await
    .err()
    .unwrap();
    let resp = err.into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::GONE);
}
```

(`AppState::for_test` is a test helper used elsewhere; if it doesn't exist, swap for whatever the codebase already uses — check `backend/src/auth/password_reset.rs` test module for the precise pattern.)

- [ ] **Step 2: Run the tests — they should fail to compile**

Run from `backend/`:

```bash
cargo test --no-run auth::email_verify::tests
```

Expected: compile error — `verify` is not defined.

- [ ] **Step 3: Implement the handler**

Append to `backend/src/auth/email_verify.rs` (above the `#[cfg(test)]` block):

```rust
use axum::{Json, extract::State, http::{HeaderMap, StatusCode, header}, response::IntoResponse};
use serde::Deserialize;

use crate::auth::session;
use crate::http::AppState;

#[derive(Deserialize)]
pub struct VerifyBody {
    pub token: String,
}

pub async fn verify(
    State(state): State<AppState>,
    Json(body): Json<VerifyBody>,
) -> Result<impl IntoResponse, AppError> {
    let hash: Vec<u8> = Sha256::digest(body.token.as_bytes()).to_vec();
    let row = sqlx::query!(
        r#"select user_id, expires_at, used_at
             from email_verification_tokens
            where token_hash = $1"#,
        hash
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::gone("expired_or_used"))?;

    if row.used_at.is_some() || row.expires_at < chrono::Utc::now() {
        return Err(AppError::gone("expired_or_used"));
    }

    let mut tx = state.pool.begin().await?;
    sqlx::query!(
        "update email_verification_tokens set used_at = now() where token_hash = $1",
        hash
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "update users set email_verified_at = now() where id = $1",
        row.user_id
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    // Auto-login on success: identical pattern to password_reset::confirm.
    let cookie = session::create_session(&state, row.user_id).await?;
    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        cookie
            .parse()
            .map_err(|_| AppError::internal("bad cookie"))?,
    );
    Ok((StatusCode::OK, headers))
}
```

- [ ] **Step 4: Wire the route**

Edit `backend/src/http/mod.rs`. After the `password-reset/confirm` route (around line 68), add:

```rust
        .route(
            "/api/auth/verify-email",
            post(crate::auth::email_verify::verify),
        )
```

- [ ] **Step 5: Run the tests — should pass**

Run from `backend/`:

```bash
cargo sqlx prepare && cargo test auth::email_verify::tests
```

Expected: 4 of 4 passed.

- [ ] **Step 6: Commit**

```bash
git add backend/src/auth/email_verify.rs backend/src/http/mod.rs backend/.sqlx
git commit -m "feat(auth): POST /api/auth/verify-email"
```

---

## Task 6: `email_verify::resend` — anti-enumeration resend with throttling

**Files:** Modify `backend/src/auth/email_verify.rs`. Modify `backend/src/http/mod.rs`.

- [ ] **Step 1: Write the failing tests**

Append to `backend/src/auth/email_verify.rs` test module:

```rust
#[tokio::test]
async fn resend_for_unverified_user_issues_new_token() {
    use crate::http::AppState;
    use std::net::SocketAddr;

    let pool = pg::new_pool().await;
    let user = create_with_password(&pool, "r1@example.com", "r1-abc", "R1", "hash")
        .await
        .unwrap();
    let before = sqlx::query_scalar!(
        "select count(*) from email_verification_tokens where user_id = $1",
        user.id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(0);

    let state = AppState::for_test(pool.clone()).await;
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let resp = resend(
        axum::extract::State(state),
        axum::extract::ConnectInfo(addr),
        axum::Json(ResendBody { email: "r1@example.com".into() }),
    )
    .await
    .unwrap()
    .into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::NO_CONTENT);

    let after = sqlx::query_scalar!(
        "select count(*) from email_verification_tokens where user_id = $1",
        user.id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(0);
    assert_eq!(after, before + 1);
}

#[tokio::test]
async fn resend_for_already_verified_user_is_silent_204() {
    use crate::http::AppState;
    use std::net::SocketAddr;

    let pool = pg::new_pool().await;
    let user = create_with_password(&pool, "r2@example.com", "r2-abc", "R2", "hash")
        .await
        .unwrap();
    sqlx::query!(
        "update users set email_verified_at = now() where id = $1",
        user.id
    )
    .execute(&pool)
    .await
    .unwrap();

    let state = AppState::for_test(pool.clone()).await;
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let resp = resend(
        axum::extract::State(state),
        axum::extract::ConnectInfo(addr),
        axum::Json(ResendBody { email: "r2@example.com".into() }),
    )
    .await
    .unwrap()
    .into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::NO_CONTENT);

    let count = sqlx::query_scalar!(
        "select count(*) from email_verification_tokens where user_id = $1",
        user.id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(0);
    assert_eq!(count, 0); // No new token, but response is still 204.
}

#[tokio::test]
async fn resend_for_unknown_email_is_silent_204() {
    use crate::http::AppState;
    use std::net::SocketAddr;

    let pool = pg::new_pool().await;
    let state = AppState::for_test(pool.clone()).await;
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let resp = resend(
        axum::extract::State(state),
        axum::extract::ConnectInfo(addr),
        axum::Json(ResendBody { email: "nobody@example.com".into() }),
    )
    .await
    .unwrap()
    .into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn resend_throttles_when_cooldown_active() {
    use crate::http::AppState;
    use std::net::SocketAddr;

    let pool = pg::new_pool().await;
    let user = create_with_password(&pool, "r3@example.com", "r3-abc", "R3", "hash")
        .await
        .unwrap();
    // First issuance counts as "we just sent one".
    issue_token(&pool, user.id, None).await.unwrap();

    let state = AppState::for_test(pool.clone()).await;
    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let resp = resend(
        axum::extract::State(state),
        axum::extract::ConnectInfo(addr),
        axum::Json(ResendBody { email: "r3@example.com".into() }),
    )
    .await
    .unwrap()
    .into_response();
    assert_eq!(resp.status(), axum::http::StatusCode::NO_CONTENT);

    // Still only the original token — cooldown blocked a fresh one.
    let count = sqlx::query_scalar!(
        "select count(*) from email_verification_tokens where user_id = $1",
        user.id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .unwrap_or(0);
    assert_eq!(count, 1);
}
```

- [ ] **Step 2: Run tests — should fail to compile**

```bash
cargo test --no-run auth::email_verify::tests
```

Expected: compile error — `resend`, `ResendBody` are not defined.

- [ ] **Step 3: Implement the handler**

Append to `backend/src/auth/email_verify.rs` (above the `#[cfg(test)]` block):

```rust
use axum::extract::ConnectInfo;
use std::net::SocketAddr;

use crate::mail::templates;

#[derive(Deserialize)]
pub struct ResendBody {
    pub email: String,
}

pub async fn resend(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(body): Json<ResendBody>,
) -> Result<impl IntoResponse, AppError> {
    let user = sqlx::query!(
        r#"select id, email as "email!: String", display_name, email_verified_at
             from users where email = $1"#,
        body.email
    )
    .fetch_optional(&state.pool)
    .await?;

    if let Some(u) = user {
        if u.email_verified_at.is_none() {
            // Throttle: per-user cooldown + per-(user-or-ip) hour cap.
            // Same shape as auth/password_reset.rs::request.
            let cooldown_hit = sqlx::query_scalar!(
                "select exists(
                    select 1 from email_verification_tokens
                    where user_id = $1
                      and created_at > now() - make_interval(secs => $2)
                )",
                u.id,
                PER_EMAIL_COOLDOWN_SECS
            )
            .fetch_one(&state.pool)
            .await?
            .unwrap_or(false);

            let hour_cap_hit = sqlx::query_scalar!(
                "select count(*) >= $2 from email_verification_tokens
                  where (user_id = $1 or request_ip = $3)
                    and created_at > now() - interval '1 hour'",
                u.id,
                PER_HOUR_CAP,
                IpNetwork::from(addr.ip())
            )
            .fetch_one(&state.pool)
            .await?
            .unwrap_or(false);

            if !cooldown_hit && !hour_cap_hit {
                let token = issue_token(&state.pool, u.id, Some(addr.ip())).await?;
                let link = format!(
                    "{}/verify/{}",
                    state.config.public_base_url.trim_end_matches('/'),
                    token
                );
                let (subject, mail_body) = templates::email_verification(&u.display_name, &link);
                if let Err(e) = state.mailer.send_plain(&u.email, &subject, &mail_body).await {
                    tracing::warn!(error = %e, user_id = %u.id, "email-verification mail send failed");
                }
            }
        }
    }

    Ok(StatusCode::NO_CONTENT)
}
```

- [ ] **Step 4: Wire the route**

Edit `backend/src/http/mod.rs`. Below the `verify-email` route from Task 5, add:

```rust
        .route(
            "/api/auth/resend-verification",
            post(crate::auth::email_verify::resend),
        )
```

- [ ] **Step 5: Run the tests — should pass**

```bash
cargo sqlx prepare && cargo test auth::email_verify::tests
```

Expected: 8 of 8 passed (4 from Task 5 + 4 from this task).

- [ ] **Step 6: Commit**

```bash
git add backend/src/auth/email_verify.rs backend/src/http/mod.rs backend/.sqlx
git commit -m "feat(auth): POST /api/auth/resend-verification with throttling"
```

---

## Task 7: Update signup handler — issue token, send mail, return 202

**Files:** Modify `backend/src/auth/signup.rs`.

- [ ] **Step 1: Write the failing test**

Append a test module to `backend/src/auth/signup.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::test_support::pg;
    use crate::http::AppState;
    use crate::auth::email_verify;
    use sha2::{Digest, Sha256};

    #[tokio::test]
    async fn signup_creates_unverified_user_and_issues_token() {
        let pool = pg::new_pool().await;
        let state = AppState::for_test(pool.clone()).await;

        let body = super::SignupBody {
            email: "new@example.com".into(),
            password: "long-enough-password-123".into(),
            display_name: "New User".into(),
            handle: "new-user".into(),
        };

        let resp = super::handler(
            axum::extract::State(state),
            axum::http::HeaderMap::new(),
            axum::Json(body),
        )
        .await
        .unwrap()
        .into_response();

        // 202 Accepted, NOT 201 Created, and no Set-Cookie.
        assert_eq!(resp.status(), axum::http::StatusCode::ACCEPTED);
        assert!(resp.headers().get("set-cookie").is_none());

        // User exists but is unverified.
        let row = sqlx::query!(
            "select email_verified_at from users where email = 'new@example.com'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(row.email_verified_at.is_none());

        // Exactly one verification token exists for this user.
        let count = sqlx::query_scalar!(
            "select count(*) from email_verification_tokens evt
              join users u on u.id = evt.user_id
              where u.email = 'new@example.com'"
        )
        .fetch_one(&pool)
        .await
        .unwrap()
        .unwrap_or(0);
        assert_eq!(count, 1);
    }
}
```

- [ ] **Step 2: Run — should fail (current handler returns 201 with Set-Cookie)**

```bash
cargo test auth::signup::tests::signup_creates_unverified_user_and_issues_token
```

Expected: assertion failure on status code or cookie or token count.

- [ ] **Step 3: Rewrite the handler**

Replace the entire body of `backend/src/auth/signup.rs` with:

```rust
use std::net::IpAddr;

use axum::{Json, extract::State, http::HeaderMap, response::IntoResponse};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::AppError;
use crate::auth::email_verify;
use crate::mail::templates;
use crate::users::queries;

#[derive(Deserialize, Validate)]
pub struct SignupBody {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 10, max = 200))]
    pub password: String,
    #[validate(length(min = 1, max = 100))]
    pub display_name: String,
    pub handle: String,
}

#[derive(Serialize)]
pub struct SignupResponse {
    pub status: &'static str,
    pub email: String,
}

pub async fn handler(
    State(state): State<crate::http::AppState>,
    headers: HeaderMap,
    Json(body): Json<SignupBody>,
) -> Result<impl IntoResponse, AppError> {
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;
    crate::auth::handle::validate(&body.handle).map_err(|e| AppError::Validation(e.to_string()))?;

    let hash = crate::auth::password::hash(body.password).await?;
    let user = queries::create_with_password(
        &state.pool,
        &body.email,
        &body.handle,
        &body.display_name,
        &hash,
    )
    .await?;

    let ip: Option<IpAddr> = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse().ok());

    let token = email_verify::issue_token(&state.pool, user.id, ip).await?;
    let link = format!(
        "{}/verify/{}",
        state.config.public_base_url.trim_end_matches('/'),
        token
    );
    let (subject, mail_body) = templates::email_verification(&user.display_name, &link);
    if let Err(e) = state.mailer.send_plain(&user.email, &subject, &mail_body).await {
        // Don't fail the request — operator can resend or reissue manually.
        tracing::warn!(error = %e, user_id = %user.id, "signup verification mail send failed");
    }

    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(SignupResponse {
            status: "verification_required",
            email: user.email,
        }),
    ))
}
```

- [ ] **Step 4: Run the test — should pass**

```bash
cargo test auth::signup::tests::signup_creates_unverified_user_and_issues_token
```

Expected: 1 passed.

- [ ] **Step 5: Commit**

```bash
git add backend/src/auth/signup.rs
git commit -m "feat(auth): signup returns 202 + emails verification link"
```

---

## Task 8: Block sign-in for unverified accounts

**Files:** Modify `backend/src/auth/login.rs`.

- [ ] **Step 1: Write the failing test**

Append a test module to `backend/src/auth/login.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::test_support::pg;
    use crate::http::AppState;
    use crate::auth::password;

    #[tokio::test]
    async fn login_blocked_for_unverified_user() {
        let pool = pg::new_pool().await;
        let hash = password::hash("long-enough-password-123".into()).await.unwrap();
        crate::users::queries::create_with_password(
            &pool, "unv@example.com", "unv-abc", "Unv", &hash
        ).await.unwrap();

        let state = AppState::for_test(pool.clone()).await;
        let err = super::handler(
            axum::extract::State(state),
            axum::http::HeaderMap::new(),
            axum::Json(super::LoginBody {
                email: "unv@example.com".into(),
                password: "long-enough-password-123".into(),
            }),
        )
        .await
        .err()
        .unwrap();
        let resp = err.into_response();
        assert_eq!(resp.status(), axum::http::StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn login_works_for_verified_user() {
        let pool = pg::new_pool().await;
        let hash = password::hash("long-enough-password-123".into()).await.unwrap();
        let user = crate::users::queries::create_with_password(
            &pool, "ver@example.com", "ver-abc", "Ver", &hash
        ).await.unwrap();
        sqlx::query!(
            "update users set email_verified_at = now() where id = $1",
            user.id
        )
        .execute(&pool)
        .await
        .unwrap();

        let state = AppState::for_test(pool.clone()).await;
        let resp = super::handler(
            axum::extract::State(state),
            axum::http::HeaderMap::new(),
            axum::Json(super::LoginBody {
                email: "ver@example.com".into(),
                password: "long-enough-password-123".into(),
            }),
        )
        .await
        .unwrap()
        .into_response();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
        assert!(resp.headers().get("set-cookie").is_some());
    }
}
```

- [ ] **Step 2: Run — first test should fail**

```bash
cargo test auth::login::tests
```

Expected: `login_blocked_for_unverified_user` fails (current handler returns 200 + cookie).

- [ ] **Step 3: Add the guard**

Edit `backend/src/auth/login.rs`. After the password verification block (around line 30), before constructing the session, insert:

```rust
    if user.email_verified_at.is_none() {
        // Block sign-in until the user clicks the verification link.
        // Don't reveal the original signup attempt's existence beyond
        // password validity — but we DO need a distinct code so the
        // frontend can redirect to /signup/check-email instead of
        // showing "invalid credentials".
        return Err(AppError::Forbidden);
    }
```

The handler must read `email_verified_at` from `user`. `UserRow` already exposes it after Task 2.

The frontend distinguishes "wrong password" (401) from "unverified" (403) by status alone. No body change is required, but we'll consume the 403 specifically on the frontend.

- [ ] **Step 4: Run the tests — should pass**

```bash
cargo test auth::login::tests
```

Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add backend/src/auth/login.rs
git commit -m "feat(auth): block sign-in for users with email_verified_at IS NULL"
```

---

## Task 9: Auto-verify Google OAuth signups

**Files:** Modify `backend/src/auth/oauth_google.rs`.

- [ ] **Step 1: Update the insert**

Edit `backend/src/auth/oauth_google.rs:224-235`. Change the SQL from:

```rust
    let row = sqlx::query!(
        r#"
        insert into users (email, display_name, handle)
        values ($1, $2, $3)
        returning id
        "#,
        info.email,
        display,
        placeholder_handle
    )
```

to:

```rust
    let row = sqlx::query!(
        r#"
        insert into users (email, display_name, handle, email_verified_at)
        values ($1, $2, $3, now())
        returning id
        "#,
        info.email,
        display,
        placeholder_handle
    )
```

Rationale: Google's OAuth flow already proves the user controls the email; no extra verification step needed.

- [ ] **Step 2: Regenerate sqlx offline metadata**

```bash
cargo sqlx prepare
```

- [ ] **Step 3: Sanity-check the whole backend builds + tests pass**

```bash
cargo check --all-targets
cargo test
```

Expected: zero errors. All tests pass.

- [ ] **Step 4: Commit**

```bash
git add backend/src/auth/oauth_google.rs backend/.sqlx
git commit -m "feat(auth): auto-verify google-oauth signups"
```

---

## Task 10: Frontend — signup page redirects to /signup/check-email

**Files:** Modify `frontend/src/routes/signup/+page.server.ts`.

- [ ] **Step 1: Edit the success-branch redirect**

In `frontend/src/routes/signup/+page.server.ts`, lines ~83-115. Replace the whole block from `// Forward the session cookie from the backend to the browser.` through the final `throw redirect(303, '/');` with:

```typescript
    // Backend now returns 202 Accepted with { status: 'verification_required', email }.
    // No cookie is set — the user must click the email link to finish signup.
    throw redirect(303, `/signup/check-email?email=${encodeURIComponent(email)}`);
```

The `res.ok` check earlier in the file already handles non-2xx error branches; a 202 is still `res.ok`, so the code path drops straight into the redirect.

- [ ] **Step 2: Type-check the frontend**

```bash
cd frontend && pnpm check
```

Expected: zero errors.

- [ ] **Step 3: Commit**

```bash
git add frontend/src/routes/signup/+page.server.ts
git commit -m "feat(signup-ui): redirect to /signup/check-email instead of auto-login"
```

---

## Task 11: Frontend — `/signup/check-email` page + resend action

**Files:**
- Create `frontend/src/routes/signup/check-email/+page.svelte`
- Create `frontend/src/routes/signup/check-email/+page.server.ts`

- [ ] **Step 1: Write the page**

Create `frontend/src/routes/signup/check-email/+page.svelte`:

```svelte
<script lang="ts">
  import { page } from '$app/state';
  import { enhance } from '$app/forms';
  import AppHeader from '$lib/components/AppHeader.svelte';
  import Button from '$lib/components/Button.svelte';

  let email = $derived(page.url.searchParams.get('email') ?? '');
  let expired = $derived(page.url.searchParams.get('expired') === '1');
  let secondsLeft = $state(60);
  let resending = $state(false);
  let resentOk = $state(false);

  $effect(() => {
    const t = setInterval(() => {
      secondsLeft = Math.max(0, secondsLeft - 1);
    }, 1000);
    return () => clearInterval(t);
  });
</script>

<svelte:head>
  <title>Check your email — Astrophoto</title>
</svelte:head>

<AppHeader />

<div class="check-email-screen">
  <div class="check-email-col">
    <div class="t-eyebrow" style="margin-bottom: 16px;">SIGN UP</div>
    <h1 class="t-h1">Check your email</h1>
    {#if expired}
      <p class="t-body" style="color: var(--color-warning, #c47);">
        That verification link has expired or was already used. Click resend to get a new one.
      </p>
    {/if}
    <p class="t-body">
      We sent a confirmation link to <strong>{email}</strong>. Open it to
      finish setting up your account. It can take a minute to arrive — don't
      forget to check spam.
    </p>

    <form
      method="POST"
      action="?/resend"
      use:enhance={() =>
        async ({ result }) => {
          resending = false;
          if (result.type === 'success') resentOk = true;
          secondsLeft = 60;
        }}
      onsubmit={() => {
        resending = true;
      }}
    >
      <input type="hidden" name="email" value={email} />
      <Button type="submit" disabled={resending || secondsLeft > 0}>
        {#if resending}
          Sending…
        {:else if secondsLeft > 0}
          Resend in {secondsLeft}s
        {:else}
          Resend confirmation
        {/if}
      </Button>
      {#if resentOk}
        <p class="t-body" style="margin-top: 8px;">
          If your account exists and isn't yet verified, we sent another link.
        </p>
      {/if}
    </form>

    <p class="t-body" style="margin-top: 24px;">
      <a href="/signin">Back to sign in</a>
    </p>
  </div>
</div>

<style>
  .check-email-screen {
    display: flex;
    justify-content: center;
    padding: 64px 24px;
  }
  .check-email-col {
    max-width: 480px;
    width: 100%;
  }
</style>
```

- [ ] **Step 2: Write the resend action**

Create `frontend/src/routes/signup/check-email/+page.server.ts`:

```typescript
import { fail } from '@sveltejs/kit';
import type { Actions } from './$types';

const API =
  process.env.BACKEND_URL ?? import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:8080';

export const actions: Actions = {
  resend: async ({ request, fetch }) => {
    const fd = await request.formData();
    const email = String(fd.get('email') ?? '').trim();
    if (!email) return fail(400, { error: 'missing_email' as const });

    try {
      // Backend always returns 204 No Content (anti-enumeration).
      await fetch(`${API}/api/auth/resend-verification`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email })
      });
    } catch {
      // Best-effort — frontend never reveals whether the address exists.
    }
    return { ok: true as const };
  }
};
```

- [ ] **Step 3: Type-check**

```bash
cd frontend && pnpm check
```

Expected: zero errors.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/routes/signup/check-email/
git commit -m "feat(signup-ui): /signup/check-email page with resend"
```

---

## Task 12: Frontend — `/verify/[token]` route

**Files:** Create `frontend/src/routes/verify/[token]/+page.server.ts`.

- [ ] **Step 1: Write the load function**

Create `frontend/src/routes/verify/[token]/+page.server.ts`:

```typescript
import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

const API =
  process.env.BACKEND_URL ?? import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:8080';

export const load: PageServerLoad = async ({ params, fetch, cookies }) => {
  let res: Response;
  try {
    res = await fetch(`${API}/api/auth/verify-email`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ token: params.token })
    });
  } catch {
    throw redirect(303, '/signup/check-email?expired=1');
  }

  if (res.status === 410) {
    throw redirect(303, '/signup/check-email?expired=1');
  }
  if (!res.ok) {
    throw redirect(303, '/signup/check-email?expired=1');
  }

  // Forward the session cookie that the backend set on auto-login.
  const setCookie = res.headers.get('set-cookie');
  if (setCookie) {
    const parts = setCookie.split(';').map((s) => s.trim());
    const pair = parts[0] ?? '';
    const attrs = parts.slice(1);
    const eq = pair.indexOf('=');
    const name = pair.slice(0, eq);
    const value = pair.slice(eq + 1);
    const opts: {
      path: string;
      httpOnly?: boolean;
      secure?: boolean;
      sameSite?: 'lax' | 'strict' | 'none';
      maxAge?: number;
    } = { path: '/' };
    for (const a of attrs) {
      const eqIdx = a.indexOf('=');
      const k = eqIdx === -1 ? a : a.slice(0, eqIdx);
      const v = eqIdx === -1 ? undefined : a.slice(eqIdx + 1);
      const kl = k.toLowerCase();
      if (kl === 'path' && v) opts.path = v;
      else if (kl === 'samesite' && v)
        opts.sameSite = v.toLowerCase() as 'lax' | 'strict' | 'none';
      else if (kl === 'httponly') opts.httpOnly = true;
      else if (kl === 'secure') opts.secure = true;
      else if (kl === 'max-age' && v) opts.maxAge = parseInt(v, 10);
    }
    cookies.set(name, value, opts);
  }

  throw redirect(303, '/');
};
```

The route has no `+page.svelte` because every code path throws a redirect.

- [ ] **Step 2: Type-check**

```bash
cd frontend && pnpm check
```

Expected: zero errors.

- [ ] **Step 3: Commit**

```bash
git add frontend/src/routes/verify/
git commit -m "feat(signup-ui): /verify/[token] route confirms email and signs in"
```

---

## Task 13: Frontend — sign-in 403 redirects to check-email

**Files:** Modify `frontend/src/routes/signin/+page.server.ts:44-50`.

- [ ] **Step 1: Add the 403 branch ahead of the existing 401 check**

Replace the block at lines 44-50:

```typescript
    if (!res.ok) {
      if (res.status === 401) {
        return fail(401, { email, message: 'Invalid email or password.' });
      }
      const txt = await res.text();
      return fail(500, { email, message: `Sign-in failed: ${txt}` });
    }
```

with:

```typescript
    if (!res.ok) {
      if (res.status === 401) {
        return fail(401, { email, message: 'Invalid email or password.' });
      }
      if (res.status === 403) {
        // Backend rejects sign-in for users with email_verified_at IS NULL.
        // Push the user to the check-email page to resend or wait for the link.
        throw redirect(303, `/signup/check-email?email=${encodeURIComponent(email)}`);
      }
      const txt = await res.text();
      return fail(500, { email, message: `Sign-in failed: ${txt}` });
    }
```

- [ ] **Step 2: Type-check**

```bash
cd frontend && pnpm check
```

Expected: zero errors.

- [ ] **Step 3: Commit**

```bash
git add frontend/src/routes/signin/+page.server.ts
git commit -m "feat(signup-ui): signin redirects to /signup/check-email on 403"
```

---

## Task 14: Local end-to-end smoke

Make sure the whole flow works in dev before deploying to prod.

- [ ] **Step 1: Start the dev stack with a fresh DB**

```bash
just db-reset && just dev
```

Wait for postgres + minio + backend + frontend + MailHog to come up. Backend should log `listening on 0.0.0.0:8080`.

- [ ] **Step 2: Sign up via the UI**

Open `http://localhost:5173/signup`, create an account with email `smoke@example.com`. After submitting, you should land on `/signup/check-email?email=smoke%40example.com`.

- [ ] **Step 3: Confirm the email was sent**

Open MailHog at `http://localhost:8025`. Expected: one inbox entry, subject `Confirm your Astrophoto account`, body contains a link starting with `http://localhost:5173/verify/`.

- [ ] **Step 4: Confirm sign-in is blocked**

In a separate browser tab, go to `http://localhost:5173/signin`, try the same email + password. Expected: redirected to `/signup/check-email?email=…`.

- [ ] **Step 5: Click the verification link**

From MailHog, click the link. Expected: lands on `/` signed in (header shows the display name).

- [ ] **Step 6: Sign out, sign back in**

Sign out. Sign back in with email + password. Expected: lands on `/` signed in (no more block).

- [ ] **Step 7: Verify Google OAuth flow still works**

In a fresh incognito window, sign up via "Sign in with Google". Expected: signed straight in without an email step (Google flow proves ownership).

- [ ] **Step 8: All backend tests pass**

```bash
just check && just test
```

Expected: both exit 0.

- [ ] **Step 9: Commit any incidental fixes**

If the smoke test surfaced bugs and you had to patch anything, commit those patches now with descriptive messages.

---

## Task 15: Deploy to prod and smoke test there

- [ ] **Step 1: Tag a release**

```bash
git tag -a "v$(date +%Y.%m.%d)" -m "Email verification on signup"
git push origin "v$(date +%Y.%m.%d)"
```

- [ ] **Step 2: Trigger Koyeb redeploy**

The current prod services don't have automatic tag-based deploy wired (see prod-deploy plan), so trigger manually:

```bash
koyeb service redeploy astrophoto-prod/backend
koyeb service redeploy astrophoto-prod-web/frontend
```

Wait for both new deployments to reach `HEALTHY`.

- [ ] **Step 3: Confirm the migration ran**

In the prod backend logs, look for the `0016_email_verification` migration line:

```bash
koyeb service logs 840f7123 | grep -i "0016\|migrat"
```

Expected: one line announcing the migration was applied.

- [ ] **Step 4: Prod smoke**

Use a real email you control. In a fresh incognito window:

1. Go to `https://www.astrophoto.pics/signup`, sign up.
2. Land on the check-email page.
3. Receive the email from `noreply@astrophoto.pics` via SES — usually under 30 s.
4. Click the link, land on `/` signed in.
5. Sign out, sign back in with email/password — works.

- [ ] **Step 5: Confirm SES delivery**

```bash
aws ses get-send-statistics --region us-east-1 --query 'SendDataPoints[-1]'
```

Expected: a recent `SendDataPoints` entry with `DeliveryAttempts: 1` (or more), `Bounces: 0`, `Complaints: 0`.

- [ ] **Step 6: Done**

Mark plan complete in the project. No code commits at this step — this is a verification gate.

---

## Self-review notes

- **Spec coverage:** every section of `2026-05-13-email-verification-on-signup-design.md` maps to a task — schema (Task 1), `UserRow` change (Task 2), template (Task 3), token helper (Task 4), `/verify-email` (Task 5), `/resend-verification` (Task 6), signup rewrite (Task 7), sign-in guard (Task 8), Google auto-verify (Task 9), four frontend changes (Tasks 10-13), local smoke (Task 14), prod smoke (Task 15).
- **One judgement-call carryover** from the spec — whether to expose `email_verified_at` in the `User` DTO — was **not** included as a task. The struct change in Task 2 already adds the field to `UserRow`; downstream DTO exposure can be a one-line follow-up if a "verified" badge UI is later wanted. Keeping out of the launch keeps the diff minimal.
- **Pattern fidelity:** every backend handler copies the password-reset structural template. The frontend `[token]` page copies the password-reset cookie-forwarding logic verbatim. Reviewers can diff side-by-side.
