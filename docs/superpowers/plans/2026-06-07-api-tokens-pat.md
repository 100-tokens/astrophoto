# API Tokens (PAT) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Personal access tokens so the PixInsight plugin (and any native client) can call the publish-flow API with `Authorization: Bearer astrophoto_pat_…`.

**Architecture:** New `api_tokens` table storing SHA-256 hashes of opaque secrets. The session `resolve()` in `auth/middleware.rs` grows a Bearer branch that resolves a token to its user and marks the request `TokenAuth`; `AdminUser` and the token-management endpoints reject token-authenticated callers. Management endpoints under `/api/me/tokens` mirror the existing `/api/me/sessions` module; a `/settings/tokens` page mirrors `/settings/sessions`.

**Tech Stack:** axum + sqlx (compile-checked macros), `sha2`/`rand`/`base64` (already in Cargo.toml), ts-rs generated types, SvelteKit form actions.

**Branch:** `feat/api-tokens` off `main`. One commit per task. Spec: `astrophoto-pixinsight/docs/superpowers/specs/2026-06-07-astrophoto-publisher-design.md` (separate repo).

**Verification reality (from project memory):** local testcontainers are unreliable (Docker Desktop crashes under load). Primary gates: `cargo clippy --all-targets -- -D warnings`, `SQLX_OFFLINE=true cargo check`, `pnpm check`, and `just check` at the end. Unit-test the pure helpers (no DB). Run a single DB integration test only if Docker is up (`docker info` succeeds); otherwise note it and rely on staging verification.

---

### Task 1: Migration — `api_tokens`

**Files:**
- Create: `backend/migrations/<timestamp>_add_api_tokens.sql` (via `just db-migrate add_api_tokens`)

- [ ] **Step 1: Generate the migration file**

Run from repo root: `just db-migrate add_api_tokens`
Expected: prints the created file path under `backend/migrations/`.

- [ ] **Step 2: Fill in the SQL**

```sql
-- Personal access tokens for native clients (PixInsight plugin).
-- Only the SHA-256 of the secret is stored; `prefix` is the first
-- characters of the secret for display ("astrophoto_pat_AbCdE…").
create table api_tokens (
    id           uuid primary key default gen_random_uuid(),
    user_id      uuid not null references users(id) on delete cascade,
    name         text not null,
    token_hash   bytea not null unique,
    prefix       text not null,
    scope        text not null default 'publish',
    created_at   timestamptz not null default now(),
    last_used_at timestamptz,
    revoked_at   timestamptz
);

create index api_tokens_user_id_idx on api_tokens (user_id);
```

- [ ] **Step 3: Apply locally**

Run: `just db-reset` (or `cargo sqlx migrate run` from `backend/` if a reset is too destructive for current local state — check `docker compose ps` first; postgres must be running).
Expected: migration applies cleanly.

- [ ] **Step 4: Commit**

```bash
git add backend/migrations/
git commit -m "feat(auth): api_tokens table for personal access tokens"
```

### Task 2: Token secret helpers (pure, unit-tested)

**Files:**
- Create: `backend/src/auth/tokens.rs` (helpers + handlers; this task adds helpers + tests only)
- Modify: `backend/src/auth/mod.rs` (add `pub mod tokens;`)

- [ ] **Step 1: Write the failing tests** (bottom of `tokens.rs`)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_has_prefix_and_length() {
        let s = generate_secret();
        assert!(s.starts_with(TOKEN_PREFIX));
        // 15-char prefix + 43 base64url chars for 32 bytes
        assert_eq!(s.len(), TOKEN_PREFIX.len() + 43);
    }

    #[test]
    fn hash_is_stable_and_32_bytes() {
        let s = generate_secret();
        assert_eq!(hash_secret(&s), hash_secret(&s));
        assert_eq!(hash_secret(&s).len(), 32);
    }

    #[test]
    fn two_secrets_differ() {
        assert_ne!(generate_secret(), generate_secret());
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run from `backend/`: `SQLX_OFFLINE=true cargo test auth::tokens -- --nocapture`
Expected: compile FAIL (`generate_secret` not defined).

- [ ] **Step 3: Implement the helpers** (top of `tokens.rs`)

```rust
//! Personal access tokens ("PAT") for native clients (PixInsight
//! plugin). Secrets look like `astrophoto_pat_<43 url-safe chars>`;
//! only their SHA-256 is persisted.

use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};

pub const TOKEN_PREFIX: &str = "astrophoto_pat_";

/// Length of the displayable prefix stored alongside the hash.
const DISPLAY_PREFIX_LEN: usize = 20; // "astrophoto_pat_" + 5 chars

pub fn generate_secret() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    format!(
        "{TOKEN_PREFIX}{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    )
}

pub fn hash_secret(secret: &str) -> Vec<u8> {
    Sha256::digest(secret.as_bytes()).to_vec()
}

pub fn display_prefix(secret: &str) -> &str {
    &secret[..DISPLAY_PREFIX_LEN]
}
```

Add `pub mod tokens;` to `backend/src/auth/mod.rs`.

- [ ] **Step 4: Run tests to verify pass**

Run: `SQLX_OFFLINE=true cargo test auth::tokens -- --nocapture`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add backend/src/auth/
git commit -m "feat(auth): PAT secret generation + hashing helpers"
```

### Task 3: Bearer branch in the auth middleware

**Files:**
- Modify: `backend/src/auth/middleware.rs` (the `resolve()` fn at lines 41-78; `AdminUser` impl at 105-118)

- [ ] **Step 1: Add the `TokenAuth` marker + Bearer resolution**

In `middleware.rs`, add near the top:

```rust
/// Marker stashed in request extensions when authentication came from a
/// Bearer PAT instead of a session cookie. `AdminUser` and the token-
/// management endpoints reject such requests.
#[derive(Clone, Copy)]
pub struct TokenAuth;
```

At the START of `resolve()` (before the cookie lookup), insert:

```rust
use axum::http::header::AUTHORIZATION;
use crate::auth::tokens::{TOKEN_PREFIX, hash_secret};

if let Some(value) = parts
    .headers
    .get(AUTHORIZATION)
    .and_then(|v| v.to_str().ok())
    .and_then(|s| s.strip_prefix("Bearer "))
{
    // Only our PAT format short-circuits here; anything else falls
    // through to the cookie path so future schemes stay possible.
    if value.starts_with(TOKEN_PREFIX) {
        let hash = hash_secret(value);
        let Some(row) = sqlx::query!(
            r#"select id, user_id from api_tokens
                where token_hash = $1 and revoked_at is null"#,
            hash
        )
        .fetch_optional(&state.pool)
        .await?
        else {
            return Ok(None);
        };

        parts.extensions.insert(TokenAuth);

        // Throttled last_used_at, same contract as sessions below.
        if let Err(e) = sqlx::query!(
            "update api_tokens set last_used_at = now() \
             where id = $1 and (last_used_at is null \
                or last_used_at < now() - interval '5 minutes')",
            row.id
        )
        .execute(&state.pool)
        .await
        {
            tracing::warn!(error = %e, "api_token last_used_at update failed");
        }

        return queries::find_by_id(&state.pool, row.user_id).await;
    }
}
```

- [ ] **Step 2: Lock admin out of token auth**

In `AdminUser::from_request_parts` (line ~112), after the successful
resolve, add before the `is_admin` match:

```rust
if parts.extensions.get::<TokenAuth>().is_some() {
    return Err(AppError::Forbidden);
}
```

- [ ] **Step 3: Compile check**

Run from `backend/`: `cargo sqlx prepare` (postgres must be running with the migration applied) then `SQLX_OFFLINE=true cargo clippy --all-targets -- -D warnings`
Expected: clean. Commit the `.sqlx/` diff with this task.

- [ ] **Step 4: Commit**

```bash
git add backend/src/auth/middleware.rs backend/.sqlx
git commit -m "feat(auth): resolve Authorization Bearer PATs to users"
```

### Task 4: Token management endpoints

**Files:**
- Modify: `backend/src/auth/tokens.rs` (handlers)
- Modify: `backend/src/api_types.rs` (DTOs — follow the `SessionRow` ts-rs pattern found there)
- Modify: `backend/src/http/mod.rs` (mount routes next to the `/api/me/sessions` block at ~line 293)

- [ ] **Step 1: DTOs in `api_types.rs`** (mirror SessionRow's derives exactly — open the file and copy its attribute set)

```rust
pub struct ApiTokenRow {
    pub id: String,
    pub name: String,
    pub prefix: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub revoked_at: Option<String>,
}

pub struct ApiTokenCreated {
    pub id: String,
    pub name: String,
    pub prefix: String,
    /// Full secret — returned exactly once, at creation.
    pub secret: String,
}
```

- [ ] **Step 2: Handlers in `tokens.rs`**

```rust
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::AppError;
use crate::api_types::{ApiTokenCreated, ApiTokenRow};
use crate::auth::middleware::{CurrentUser, TokenAuth};
use crate::http::AppState;

#[derive(serde::Deserialize)]
pub struct CreateBody {
    pub name: String,
}

/// Guard shared by the management handlers: a PAT must not mint or
/// revoke PATs — that stays a logged-in-browser operation.
fn reject_token_auth(parts_has_token: bool) -> Result<(), AppError> {
    if parts_has_token {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

pub async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    token_auth: Option<axum::Extension<TokenAuth>>,
    Json(body): Json<CreateBody>,
) -> Result<Json<ApiTokenCreated>, AppError> {
    reject_token_auth(token_auth.is_some())?;
    let name = body.name.trim();
    if name.is_empty() || name.len() > 80 {
        return Err(AppError::bad_request("name"));
    }
    let secret = super::tokens::generate_secret();
    let hash = super::tokens::hash_secret(&secret);
    let prefix = super::tokens::display_prefix(&secret).to_string();
    let row = sqlx::query!(
        r#"insert into api_tokens (user_id, name, token_hash, prefix)
           values ($1, $2, $3, $4)
           returning id"#,
        user.id,
        name,
        hash,
        prefix
    )
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(ApiTokenCreated {
        id: row.id.to_string(),
        name: name.to_string(),
        prefix,
        secret,
    }))
}

pub async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<Json<Vec<ApiTokenRow>>, AppError> {
    let rows = sqlx::query!(
        r#"select id, name, prefix, created_at, last_used_at, revoked_at
             from api_tokens where user_id = $1
            order by created_at desc"#,
        user.id
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(
        rows.into_iter()
            .map(|r| ApiTokenRow {
                id: r.id.to_string(),
                name: r.name,
                prefix: r.prefix,
                created_at: r.created_at.to_rfc3339(),
                last_used_at: r.last_used_at.map(|t| t.to_rfc3339()),
                revoked_at: r.revoked_at.map(|t| t.to_rfc3339()),
            })
            .collect(),
    ))
}

pub async fn revoke(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    token_auth: Option<axum::Extension<TokenAuth>>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    reject_token_auth(token_auth.is_some())?;
    let res = sqlx::query!(
        "update api_tokens set revoked_at = now() \
         where id = $1 and user_id = $2 and revoked_at is null",
        id,
        user.id
    )
    .execute(&state.pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::not_found("token"));
    }
    Ok(StatusCode::NO_CONTENT)
}
```

NOTE for the implementer: `Option<axum::Extension<TokenAuth>>` works as
an extractor only if `TokenAuth` is `Clone` (it is). If the project's
axum version trips on it, fall back to reading
`parts.extensions.get::<TokenAuth>()` via a tiny extractor struct —
check how `CurrentSessionId` does it in `middleware.rs:120-134`.

- [ ] **Step 3: Mount routes** in `http/mod.rs` next to the sessions block:

```rust
.route(
    "/api/me/tokens",
    axum::routing::get(crate::auth::tokens::list)
        .post(crate::auth::tokens::create),
)
.route(
    "/api/me/tokens/:id",
    axum::routing::delete(crate::auth::tokens::revoke),
)
```

- [ ] **Step 4: Prepare + gates**

Run from `backend/`: `cargo sqlx prepare && cargo clippy --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add backend/src backend/.sqlx
git commit -m "feat(auth): /api/me/tokens create/list/revoke endpoints"
```

### Task 5: Generated TS types + API client helpers

**Files:**
- Modify: `frontend/src/lib/api/client.ts` (next to `listSessions`, ~line 186)
- Generated: `frontend/src/lib/api/ApiTokenRow.ts`, `ApiTokenCreated.ts`

- [ ] **Step 1: Regenerate types**

Run from repo root: `just types`
Expected: new `ApiTokenRow.ts` / `ApiTokenCreated.ts` appear; commit-able diff.

- [ ] **Step 2: Client helpers** (mirror the sessions trio)

```ts
createApiToken: (name: string, opts?: ApiCall) =>
  request<import('./ApiTokenCreated').ApiTokenCreated>('POST', '/api/me/tokens', { name }, opts),

listApiTokens: (opts?: ApiCall) =>
  request<import('./ApiTokenRow').ApiTokenRow[]>('GET', '/api/me/tokens', undefined, opts),

revokeApiToken: (id: string, opts?: ApiCall) =>
  request<void>('DELETE', `/api/me/tokens/${encodeURIComponent(id)}`, undefined, opts),
```

- [ ] **Step 3: Gate**

Run from `frontend/`: `pnpm check`
Expected: 0 errors.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/lib/api
git commit -m "feat(api): generated types + client helpers for PATs"
```

### Task 6: `/settings/tokens` page

**Files:**
- Create: `frontend/src/routes/settings/tokens/+page.server.ts`
- Create: `frontend/src/routes/settings/tokens/+page.svelte`
- Modify: the settings nav — find where `/settings/sessions` is linked (`grep -rn "settings/sessions" frontend/src/routes/settings/+layout.svelte frontend/src/lib` ) and add a "PixInsight & API" entry pointing at `/settings/tokens`.

- [ ] **Step 1: Server load + actions** (mirror `settings/sessions/+page.server.ts`)

```ts
import type { PageServerLoad, Actions } from './$types';
import { api } from '$lib/api/client';

export const load: PageServerLoad = async ({ fetch }) => ({
  tokens: await api.listApiTokens({ fetch })
});

export const actions: Actions = {
  create: async ({ request, fetch }) => {
    const fd = await request.formData();
    const name = String(fd.get('name') ?? '').trim();
    if (!name) return { ok: false, error: 'name_required' };
    const created = await api.createApiToken(name, { fetch });
    // The secret crosses to the page exactly once, in the action result.
    return { ok: true, created };
  },
  revoke: async ({ request, fetch }) => {
    const fd = await request.formData();
    const id = String(fd.get('id') ?? '');
    await api.revokeApiToken(id, { fetch });
    return { ok: true };
  }
};
```

- [ ] **Step 2: Page component** — Svelte 5 runes ONLY (`$props()`, `$derived`; no `export let`). Mirror the sessions page's table/markup classes (open `settings/sessions/+page.svelte` first and reuse its structure). Functional skeleton:

```svelte
<script lang="ts">
  import { enhance } from '$app/forms';
  let { data, form } = $props();
  let tokens = $derived(data.tokens);
</script>

<h2>PixInsight & API tokens</h2>

{#if form?.created}
  <div class="token-reveal">
    <p>Copy this token now — it will not be shown again.</p>
    <code>{form.created.secret}</code>
    <button type="button" onclick={() => navigator.clipboard.writeText(form.created.secret)}>Copy</button>
  </div>
{/if}

<form method="POST" action="?/create" use:enhance>
  <input name="name" placeholder="Token name (e.g. PixInsight on iMac)" required maxlength="80" />
  <button type="submit">Create token</button>
</form>

<ul>
  {#each tokens as t (t.id)}
    <li>
      <strong>{t.name}</strong> <code>{t.prefix}…</code>
      — created {t.created_at}
      {#if t.revoked_at}(revoked){:else}
        <form method="POST" action="?/revoke" use:enhance>
          <input type="hidden" name="id" value={t.id} />
          <button type="submit">Revoke</button>
        </form>
      {/if}
    </li>
  {/each}
</ul>
```

- [ ] **Step 3: Gate + visual sanity**

Run from `frontend/`: `pnpm check`
Expected: 0 errors. If `just dev` is running, load `http://localhost:5173/settings/tokens`, create a token, see the secret once, revoke it.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/routes/settings
git commit -m "feat(settings): API token management page"
```

### Task 7: End-of-branch gates + integrate

- [ ] **Step 1: Full gates**

Run from repo root: `just check`
Expected: zero output failures. Fix anything that surfaces (fmt with `just fmt` ONLY on files this branch touched — never crate-wide, per project memory).

- [ ] **Step 2: Optional DB integration test** — only if `docker info` succeeds: pick the existing auth/session integration test file as a template and add one test that creates a user + token row and asserts a Bearer request hits a `CurrentUser` route (e.g. `GET /api/me/tokens`) while `__Host-session` is absent. If Docker is flaky, skip and note it in the PR body.

- [ ] **Step 3: PR + merge**

```bash
git push -u origin feat/api-tokens
gh pr create --title "feat(auth): personal access tokens for native clients" --body "..."
```
Merge once gates are green (repo CI is gitleaks-only). Then redeploy staging manually (koyeb; per project memory the deploy is manual) and smoke-test: create a token via the staging UI, then `curl -H "Authorization: Bearer <secret>" https://astrophoto-staging-xavyo-008151d0.koyeb.app/api/me/tokens` → must 403 (token-auth rejected on management) and `curl …/api/equipment/setups` → 200 `[]` or setups list.
