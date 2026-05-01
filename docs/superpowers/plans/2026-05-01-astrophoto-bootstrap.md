# Astrophoto Bootstrap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bootstrap a Rust + SvelteKit monorepo at `/Volumes/Pascal4Tb/Projects/astrophoto/` with a Karpathy/Cherny-inspired `CLAUDE.md`, working `just check` quality gates, applied DB migrations, a backend `/healthz` endpoint returning 200, and a SvelteKit landing page rendering "Astrophoto".

**Architecture:** Monorepo with `backend/` (axum + sqlx + Postgres) and `frontend/` (SvelteKit SSR with Svelte 5 runes). S3-compatible storage abstracted behind a `Storage` trait. Local dev orchestrated via `compose.yml` (Postgres + MinIO) and a `justfile`.

**Tech Stack:** Rust 2024 edition (axum 0.7, sqlx 0.8, tokio, thiserror, tracing, ts-rs, kamadak-exif, image, aws-sdk-s3, argon2, oauth2), SvelteKit (Svelte 5 runes, adapter-node, TypeScript strict), PostgreSQL 16, MinIO (dev) / Cloudflare R2 (prod), Docker Compose, just, pnpm.

**Spec reference:** `docs/superpowers/specs/2026-05-01-astrophoto-bootstrap-design.md`

**Working directory for all commands:** `/Volumes/Pascal4Tb/Projects/astrophoto/` (referred to below as `$ROOT`).

---

## Task 1: Initialize git repo and root config files

**Files:**
- Create: `.gitignore`
- Create: `.editorconfig`
- Create: `rust-toolchain.toml`

- [ ] **Step 1: Initialize git**

```bash
cd /Volumes/Pascal4Tb/Projects/astrophoto
git init -b main
```

- [ ] **Step 2: Create `.gitignore`**

```gitignore
# Rust
/backend/target/
**/*.rs.bk
# sqlx offline data — DO commit (CI uses it)
!/backend/.sqlx/

# Node / SvelteKit
/frontend/node_modules/
/frontend/.svelte-kit/
/frontend/build/
/frontend/.vite/

# Env
.env
.env.local
.env.*.local
!.env.example

# OS / editor
.DS_Store
*.swp
.idea/
.vscode/
!.vscode/extensions.json

# Local data volumes from compose
/data/
```

- [ ] **Step 3: Create `.editorconfig`**

```editorconfig
root = true

[*]
charset = utf-8
end_of_line = lf
indent_style = space
indent_size = 4
insert_final_newline = true
trim_trailing_whitespace = true

[*.{js,ts,svelte,html,css,json,yml,yaml,md}]
indent_size = 2

[*.toml]
indent_size = 2

[*.md]
trim_trailing_whitespace = false
```

- [ ] **Step 4: Create `rust-toolchain.toml`**

```toml
[toolchain]
channel = "1.85.0"
components = ["rustfmt", "clippy"]
profile = "minimal"
```

- [ ] **Step 5: Commit**

```bash
git add .gitignore .editorconfig rust-toolchain.toml
git commit -m "chore: init repo with .gitignore, .editorconfig, rust-toolchain"
```

---

## Task 2: Write CLAUDE.md (the main deliverable)

**Files:**
- Create: `CLAUDE.md`

- [ ] **Step 1: Write `CLAUDE.md`**

```markdown
# CLAUDE.md

Astrophoto: Rust + SvelteKit app for amateur astrophotographers to upload,
tag, and share images. Backend: axum + sqlx + Postgres. Frontend:
SvelteKit SSR + Svelte 5 runes. Storage: S3-compatible (Cloudflare R2 in
prod, MinIO in dev).

For broader context: @README.md and @docs/superpowers/specs/ for design
docs.

---

## Philosophy

Read this first. The rest is operational.

- The code is the documentation. `backend/src/main.rs`, the migration
  files, and the SvelteKit routes tell the truth. This file only covers
  what cannot be inferred from reading code.
- Simple beats clever. A 100-line handler that does one thing reads
  better than a 30-line handler that calls four traits. Reject
  abstractions that pay back in fewer than two callers.
- Fail fast and loud. Panic at boot for missing config. Never swallow an
  error to "make tests pass". The `unwrap_used` lint is denied for a
  reason.
- Be slow, defensive, careful, paranoid. You are an over-eager junior.
  Read before you write. When unsure, propose two approaches and ask. Do
  not invent fields, columns, or function names you have not verified.
- YOU MUST NOT change code or comments you do not understand. If a
  block looks weird, ask why before editing. Comments encode invariants.
- IMPORTANT: keep diffs minimal. One concern per commit. Do not
  refactor drive-by while fixing a bug.

---

## Repo map

- `backend/`     Cargo crate, axum API. Entry: `backend/src/main.rs`.
- `frontend/`    SvelteKit app. Entry: `frontend/src/routes/+layout.svelte`.
- `backend/migrations/`  SQL migrations. Append-only, numbered, never
                 edit shipped files.
- `compose.yml`  Local dev stack (postgres, minio).
- `justfile`     All common commands. Run `just` to list.
- `docs/superpowers/specs/`  Design docs. Read before large changes.
- `docs/superpowers/plans/`  Implementation plans.

---

## Bash commands

Use these. Do not invent variants.

- `just dev`              Start postgres+minio+backend+frontend, hot reload.
- `just check`            Run all quality gates (see Workflow below).
                          Must pass before claiming a task done.
- `just test`             All tests. Backend uses testcontainers; Docker
                          must be running.
- `just db-reset`         Drop + recreate dev db, run migrations.
- `just db-migrate name`  Create a new timestamped migration file.
- `just types`            Regenerate `frontend/src/lib/api/types.ts` from
                          Rust types via ts-rs.
- `just fmt`              rustfmt + prettier.

Backend-only (run from `backend/`):
- `cargo sqlx prepare`    Required after changing any SQL macro. Commit
                          the resulting `.sqlx/` directory.
- `cargo clippy --all-targets -- -D warnings`

Frontend-only (run from `frontend/`):
- `pnpm check`            svelte-check; fails on type errors.
- `pnpm test:e2e`         Playwright. Requires backend running.

---

## Code style

### Rust

- Edition 2024. MSRV pinned in `rust-toolchain.toml`. Don't bump without
  agreement.
- Errors: one `AppError` enum in `backend/src/error.rs` (`thiserror`)
  that implements `IntoResponse`. Handlers return
  `Result<T, AppError>`.
- IMPORTANT: no `.unwrap()` or `.expect()` in non-test code outside
  `main.rs` boot path. Use `?` and `AppError`. `clippy::unwrap_used` is
  denied.
- `anyhow` is forbidden in library code. Allowed only in tests and the
  binary's `main`.
- Async: never call blocking I/O or CPU-heavy work inside an async fn.
  Use `tokio::task::spawn_blocking` for image decoding, password
  hashing, large file I/O.
- Mutex: prefer `std::sync::Mutex` over `tokio::sync::Mutex` unless the
  guard must be held across `.await`.
- `tokio::spawn` requires `'static`. Clone or move owned data; never
  spawn a future borrowing a local.
- Database: write SQL directly. Use `sqlx::query!` / `query_as!` macros
  so the schema is checked at compile time. No ORM. No Repository
  pattern.
- Module layout: feature-folders (`auth/`, `photos/`, `storage/`). Each
  has `mod.rs` (public surface), `queries.rs` (SQL), and optionally
  `service.rs` (orchestration). Avoid traits with one impl.
- Logging: `tracing` only. Never `println!` or `eprintln!` outside
  tests.

### SvelteKit / Svelte 5

- IMPORTANT: this project uses **Svelte 5 with runes only**. Do NOT
  write Svelte 4 syntax (`export let`, `$:` reactive labels, lifecycle
  imports from `'svelte'`). If unsure, fetch the small llms file:
  https://svelte.dev/llms-small.txt
- Reactivity: prefer `$derived` over `$effect`. `$effect` is for side
  effects only (DOM, subscriptions, timers). Computed values are
  `$derived`.
- Props: `let { foo, bar } = $props();` is fine because `$props()` is
  itself reactive. Do NOT destructure into local consts elsewhere;
  reactivity is lost.
- Never export a primitive `$state` across modules. Export an object,
  a class instance, or a `{ get, set }` pair.
- Data loading: server `load` functions in `+page.server.ts` for
  SSR-able routes. Only use universal `+page.ts` when data must be
  reusable on client-side navigation without refetching.
- Mutations: form actions, not client-side fetch, for primary flows
  (login, signup, upload). CSRF is automatic with form actions.
- TypeScript strict. `any` is forbidden; use `unknown` and narrow.
- API types come from `frontend/src/lib/api/types.ts` (generated). Don't
  hand-edit; run `just types`.

---

## Workflow

- Before claiming a task done: `just check` must pass with zero output.
  Pasting "should work" without running it is a regression.
- Run a single test, not the whole suite, while iterating
  (`cargo test <name> -- --nocapture`).
- After changing any `sqlx::query!` invocation, run `cargo sqlx prepare`
  and commit `.sqlx/`. CI uses offline mode.
- After changing API types in Rust, run `just types`. Commit the diff.
- New migration: `just db-migrate add_X`. Edit the generated SQL.
  Never edit a migration that has been merged to main.
- Tests with a real DB use `testcontainers`. Docker must be running.
  The harness creates a fresh database per test.

---

## Repository etiquette

- Branches: `feat/<short-slug>`, `fix/<short-slug>`, `chore/<slug>`. No
  issue numbers in branch names.
- Commits: imperative subject, ~60 chars. Body explains WHY when not
  obvious. One concern per commit.
- PRs: link the spec under `docs/superpowers/specs/` if relevant.
- Never merge with failing `just check`. Never bypass git hooks
  (`--no-verify`).

---

## Gotchas

- The `photos.status` field can be `processing` for a few seconds after
  upload while `spawn_blocking` decodes thumbnails. UI must handle this.
- EXIF datetimes are naive (no timezone). We store as `timestamptz` and
  assume UTC unless the camera embeds GPS + offset.
- MinIO (dev) does not enforce all S3 quirks. Test path-style vs
  virtual-hosted addressing with `aws-sdk-s3` config matching prod.
- Session cookies are `__Host-session`, `Secure`, `HttpOnly`,
  `SameSite=Lax`. The Lax (not Strict) is intentional for the OAuth
  redirect flow; do not change without reading `auth/oauth.rs`.
- Argon2 password verification is CPU-bound: always inside
  `spawn_blocking`. Same for image decode.

---

## References

- Design docs: `docs/superpowers/specs/`
- Svelte 5 LLM context: https://svelte.dev/llms-small.txt
- sqlx macro docs: https://docs.rs/sqlx/latest/sqlx/macro.query.html
- agents.md spec (this file is also linked as `AGENTS.md`):
  https://agents.md
```

- [ ] **Step 2: Verify line count**

Run: `wc -l CLAUDE.md`
Expected: between 150 and 200 lines (target ~180).

- [ ] **Step 3: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: add CLAUDE.md (Karpathy/Cherny-inspired)"
```

---

## Task 3: Add AGENTS.md symlink and README.md

**Files:**
- Create: `AGENTS.md` (symlink → `CLAUDE.md`)
- Create: `README.md`

- [ ] **Step 1: Create symlink**

```bash
ln -s CLAUDE.md AGENTS.md
```

- [ ] **Step 2: Verify symlink**

Run: `ls -l AGENTS.md`
Expected: `lrwxr-xr-x ... AGENTS.md -> CLAUDE.md`

- [ ] **Step 3: Write `README.md`**

```markdown
# Astrophoto

Web app for amateur astrophotographers to upload, tag, and share images.

- **Backend**: Rust (axum + sqlx + PostgreSQL)
- **Frontend**: SvelteKit (Svelte 5 runes, SSR)
- **Storage**: S3-compatible (Cloudflare R2 in prod, MinIO in dev)

## Quick start

```bash
# Prereqs: Docker, just, rustup, pnpm
cp .env.example .env
just dev
```

Open <http://localhost:5173>.

## Common commands

```bash
just              # list all commands
just check        # quality gates (fmt, clippy, types, lints)
just test         # run all tests (Docker required)
just db-reset     # drop + recreate dev db
```

## Repository

- `backend/` — Rust API.
- `frontend/` — SvelteKit app.
- `docs/superpowers/specs/` — design documents.
- `docs/superpowers/plans/` — implementation plans.
- `CLAUDE.md` — instructions for AI coding assistants.

## License

MIT.
```

- [ ] **Step 4: Commit**

```bash
git add AGENTS.md README.md
git commit -m "docs: add README and AGENTS.md symlink"
```

---

## Task 4: Write `.env.example`

**Files:**
- Create: `.env.example`

- [ ] **Step 1: Write `.env.example`**

```bash
# Astrophoto local dev — copy to .env and edit if needed.

# --- Backend ---
APP_BIND=0.0.0.0:8080
APP_LOG=info
APP_DATABASE_URL=postgres://astrophoto:astrophoto@localhost:5432/astrophoto
APP_SESSION_DOMAIN=localhost
APP_SESSION_SECURE=false           # true in prod (HTTPS only)
APP_PUBLIC_BASE_URL=http://localhost:8080

# --- Storage (MinIO local; swap for R2 in prod) ---
APP_S3_ENDPOINT=http://localhost:9000
APP_S3_REGION=us-east-1
APP_S3_BUCKET=astrophoto
APP_S3_ACCESS_KEY=minioadmin
APP_S3_SECRET_KEY=minioadmin
APP_S3_PATH_STYLE=true             # MinIO requires path-style

# --- OAuth (leave blank to disable Google login) ---
APP_OAUTH_GOOGLE_CLIENT_ID=
APP_OAUTH_GOOGLE_CLIENT_SECRET=
APP_OAUTH_GOOGLE_REDIRECT_URL=http://localhost:8080/api/auth/oauth/google/callback

# --- Frontend ---
VITE_API_BASE_URL=http://localhost:8080
PUBLIC_APP_NAME=Astrophoto
```

- [ ] **Step 2: Commit**

```bash
git add .env.example
git commit -m "chore: add .env.example"
```

---

## Task 5: Write the `justfile`

**Files:**
- Create: `justfile`

- [ ] **Step 1: Write `justfile`**

```just
set dotenv-load := true

# Default — list all commands.
default:
    @just --list

# Start full dev stack (postgres, minio, backend, frontend).
dev:
    docker compose up -d postgres minio
    sleep 2
    just _migrate
    (cd backend && cargo run) & \
    (cd frontend && pnpm dev) & \
    wait

# Run all quality gates. MUST pass before claiming a task done.
check:
    cd backend && cargo fmt --check
    cd backend && cargo clippy --all-targets -- -D warnings
    cd frontend && pnpm check
    cd frontend && pnpm lint

# Run all tests. Docker must be running (testcontainers).
test:
    cd backend && cargo test --all-targets
    cd frontend && pnpm test

# Format Rust + frontend.
fmt:
    cd backend && cargo fmt
    cd frontend && pnpm format

# Drop and recreate the dev database, apply migrations.
db-reset:
    docker compose exec -T postgres psql -U astrophoto -d postgres \
        -c "drop database if exists astrophoto;" \
        -c "create database astrophoto;"
    just _migrate

# Create a new timestamped migration file: just db-migrate add_users
db-migrate name:
    cd backend && sqlx migrate add {{name}}

# Apply pending migrations (internal).
_migrate:
    cd backend && sqlx migrate run

# Regenerate TypeScript types from Rust source.
types:
    cd backend && cargo run --bin gen-types > ../frontend/src/lib/api/types.ts
    cd frontend && pnpm prettier --write src/lib/api/types.ts
```

- [ ] **Step 2: Verify `just` lists tasks**

Run: `just`
Expected: prints commands `dev`, `check`, `test`, `fmt`, `db-reset`, `db-migrate`, `types`. (Will work after `just` is installed; if not, skip and verify after install.)

- [ ] **Step 3: Commit**

```bash
git add justfile
git commit -m "chore: add justfile with dev/check/test/db commands"
```

---

## Task 6: Write minimal `compose.yml` (postgres + minio)

**Files:**
- Create: `compose.yml`

- [ ] **Step 1: Write `compose.yml`**

```yaml
name: astrophoto

services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: astrophoto
      POSTGRES_PASSWORD: astrophoto
      POSTGRES_DB: astrophoto
    ports:
      - "5432:5432"
    volumes:
      - ./data/postgres:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U astrophoto"]
      interval: 5s
      timeout: 5s
      retries: 5

  minio:
    image: minio/minio:latest
    command: server /data --console-address ":9001"
    environment:
      MINIO_ROOT_USER: minioadmin
      MINIO_ROOT_PASSWORD: minioadmin
    ports:
      - "9000:9000"   # S3 API
      - "9001:9001"   # Console
    volumes:
      - ./data/minio:/data
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9000/minio/health/live"]
      interval: 5s
      timeout: 5s
      retries: 5
```

- [ ] **Step 2: Verify it parses**

Run: `docker compose config -q`
Expected: no output, exit 0.

- [ ] **Step 3: Commit**

```bash
git add compose.yml
git commit -m "chore: add compose.yml for postgres+minio dev stack"
```

---

## Task 7: Backend `Cargo.toml` with pinned dependencies

**Files:**
- Create: `backend/Cargo.toml`
- Create: `backend/src/lib.rs` (placeholder so cargo accepts the crate)
- Create: `backend/src/main.rs` (placeholder)

- [ ] **Step 1: Create `backend/Cargo.toml`**

```toml
[package]
name = "astrophoto"
version = "0.1.0"
edition = "2024"
publish = false

[lib]
name = "astrophoto"
path = "src/lib.rs"

[[bin]]
name = "astrophoto"
path = "src/main.rs"

[[bin]]
name = "gen-types"
path = "src/bin/gen-types.rs"

[dependencies]
# HTTP / runtime
axum = { version = "0.7", features = ["macros", "multipart"] }
tokio = { version = "1", features = ["full"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["trace", "cors", "request-id"] }
hyper = "1"

# DB
sqlx = { version = "0.8", default-features = false, features = [
    "runtime-tokio-rustls", "postgres", "macros", "migrate",
    "uuid", "chrono", "ipnetwork", "json"
] }

# Errors / config / logging
thiserror = "1"
anyhow = "1"  # main.rs only
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
figment = { version = "0.10", features = ["env", "toml"] }

# Auth / crypto
argon2 = "0.5"
oauth2 = { version = "4", default-features = false, features = ["rustls-tls"] }
rand = "0.8"
base64 = "0.22"

# Storage / images
aws-sdk-s3 = "1"
aws-config = "1"
kamadak-exif = "0.5"
image = { version = "0.25", default-features = false, features = ["jpeg", "png", "tiff"] }

# Misc
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
validator = { version = "0.18", features = ["derive"] }
ts-rs = { version = "10", features = ["chrono-impl", "uuid-impl"] }

[dev-dependencies]
testcontainers = "0.23"
testcontainers-modules = { version = "0.11", features = ["postgres"] }
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
tokio = { version = "1", features = ["full", "test-util"] }

[lints.rust]
unsafe_code = "forbid"

[lints.clippy]
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"

[profile.dev]
opt-level = 0
debug = 1
```

- [ ] **Step 2: Create placeholder `backend/src/lib.rs`**

```rust
//! Astrophoto crate root. Modules added in later tasks.
```

- [ ] **Step 3: Create placeholder `backend/src/main.rs`**

```rust
fn main() {
    println!("astrophoto bootstrap placeholder");
}
```

- [ ] **Step 4: Verify it builds**

Run: `cd backend && cargo check`
Expected: compiles cleanly. Many warnings about unused deps are OK at this stage.

- [ ] **Step 5: Commit**

```bash
git add backend/Cargo.toml backend/Cargo.lock backend/src/lib.rs backend/src/main.rs
git commit -m "feat(backend): scaffold Cargo crate with pinned dependencies"
```

---

## Task 8: Backend `error.rs` — `AppError` with unit test

**Files:**
- Create: `backend/src/error.rs`
- Modify: `backend/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Append to `backend/src/error.rs`:

```rust
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found")]
    NotFound,

    #[error("unauthorized")]
    Unauthorized,

    #[error("forbidden")]
    Forbidden,

    #[error("validation: {0}")]
    Validation(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error(transparent)]
    Database(#[from] sqlx::Error),

    #[error("internal: {0}")]
    Internal(String),
}

impl AppError {
    fn status(&self) -> StatusCode {
        match self {
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::Forbidden => StatusCode::FORBIDDEN,
            AppError::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Database(_) | AppError::Internal(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    fn code(&self) -> &'static str {
        match self {
            AppError::NotFound => "not-found",
            AppError::Unauthorized => "unauthorized",
            AppError::Forbidden => "forbidden",
            AppError::Validation(_) => "validation",
            AppError::Conflict(_) => "conflict",
            AppError::Database(_) | AppError::Internal(_) => "internal",
        }
    }
}

#[derive(Serialize)]
struct Body<'a> {
    error: &'a str,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        if status.is_server_error() {
            tracing::error!(error = %self, "server error");
        }
        let body = Body {
            error: self.code(),
            message: self.to_string(),
        };
        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[tokio::test]
    async fn not_found_maps_to_404() {
        let resp = AppError::NotFound.into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let bytes = to_bytes(resp.into_body(), 1024).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["error"], "not-found");
    }

    #[tokio::test]
    async fn validation_maps_to_422() {
        let resp = AppError::Validation("bad email".into()).into_response();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let bytes = to_bytes(resp.into_body(), 1024).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["error"], "validation");
        assert!(v["message"].as_str().unwrap().contains("bad email"));
    }
}
```

- [ ] **Step 2: Expose the module in `lib.rs`**

Replace `backend/src/lib.rs` with:

```rust
pub mod error;

pub use error::AppError;
```

- [ ] **Step 3: Run the tests**

Run: `cd backend && cargo test --lib error`
Expected: 2 passed.

- [ ] **Step 4: Commit**

```bash
git add backend/src/error.rs backend/src/lib.rs
git commit -m "feat(backend): add AppError with IntoResponse + tests"
```

---

## Task 9: Backend `config.rs` — env-driven `Config` with unit test

**Files:**
- Create: `backend/src/config.rs`
- Modify: `backend/src/lib.rs`

- [ ] **Step 1: Write `config.rs`**

```rust
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub bind: String,
    pub log: String,
    pub database_url: String,
    pub session_domain: String,
    pub session_secure: bool,
    pub public_base_url: String,

    pub s3_endpoint: Option<String>,
    pub s3_region: String,
    pub s3_bucket: String,
    pub s3_access_key: String,
    pub s3_secret_key: String,
    pub s3_path_style: bool,

    #[serde(default)]
    pub oauth_google_client_id: String,
    #[serde(default)]
    pub oauth_google_client_secret: String,
    #[serde(default)]
    pub oauth_google_redirect_url: String,
}

impl Config {
    /// Load from env vars prefixed with `APP_`. Panics on missing required vars.
    pub fn from_env() -> Self {
        figment::Figment::new()
            .merge(figment::providers::Env::prefixed("APP_"))
            .extract()
            .expect("invalid configuration: check APP_* environment variables")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_from_env() {
        // SAFETY: tests run single-threaded for env mutation; this is a smoke
        // test of figment wiring, acceptable in a #[cfg(test)] block.
        unsafe {
            std::env::set_var("APP_BIND", "0.0.0.0:1234");
            std::env::set_var("APP_LOG", "debug");
            std::env::set_var("APP_DATABASE_URL", "postgres://x");
            std::env::set_var("APP_SESSION_DOMAIN", "localhost");
            std::env::set_var("APP_SESSION_SECURE", "false");
            std::env::set_var("APP_PUBLIC_BASE_URL", "http://localhost:8080");
            std::env::set_var("APP_S3_REGION", "us-east-1");
            std::env::set_var("APP_S3_BUCKET", "b");
            std::env::set_var("APP_S3_ACCESS_KEY", "a");
            std::env::set_var("APP_S3_SECRET_KEY", "s");
            std::env::set_var("APP_S3_PATH_STYLE", "true");
        }
        let cfg = Config::from_env();
        assert_eq!(cfg.bind, "0.0.0.0:1234");
        assert_eq!(cfg.log, "debug");
        assert!(!cfg.session_secure);
        assert!(cfg.s3_path_style);
    }
}
```

> Note: `clippy::expect_used` is denied repo-wide, but boot-time `expect()`
> in `Config::from_env` is intentional (fail-fast on missing config).
> Add `#[allow(clippy::expect_used)]` on the function if clippy complains.

- [ ] **Step 2: Update `backend/src/config.rs` to silence the lint**

Add `#[allow(clippy::expect_used)]` immediately above `pub fn from_env()`:

```rust
    #[allow(clippy::expect_used)]
    pub fn from_env() -> Self {
```

- [ ] **Step 3: Expose the module in `lib.rs`**

Replace `backend/src/lib.rs`:

```rust
pub mod config;
pub mod error;

pub use config::Config;
pub use error::AppError;
```

- [ ] **Step 4: Run the test**

Run: `cd backend && cargo test --lib config`
Expected: 1 passed.

- [ ] **Step 5: Commit**

```bash
git add backend/src/config.rs backend/src/lib.rs
git commit -m "feat(backend): add Config loaded from APP_* env vars"
```

---

## Task 10: Backend `db.rs` — `PgPool` setup

**Files:**
- Create: `backend/src/db.rs`
- Modify: `backend/src/lib.rs`

- [ ] **Step 1: Write `backend/src/db.rs`**

```rust
use sqlx::postgres::{PgPool, PgPoolOptions};

use crate::AppError;

pub async fn connect(database_url: &str) -> Result<PgPool, AppError> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(database_url)
        .await?;
    Ok(pool)
}
```

- [ ] **Step 2: Expose the module in `lib.rs`**

Replace `backend/src/lib.rs`:

```rust
pub mod config;
pub mod db;
pub mod error;

pub use config::Config;
pub use error::AppError;
```

- [ ] **Step 3: Verify it compiles**

Run: `cd backend && cargo check`
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add backend/src/db.rs backend/src/lib.rs
git commit -m "feat(backend): add Postgres pool helper"
```

---

## Task 11: Initial migration `0001_init.sql`

**Files:**
- Create: `backend/migrations/0001_init.sql`

- [ ] **Step 1: Write the migration**

```sql
-- 0001 init: users, oauth identities, sessions, photos, thumbnails.

create extension if not exists pgcrypto;
create extension if not exists pg_trgm;
create extension if not exists citext;

create table users (
    id            uuid primary key default gen_random_uuid(),
    email         citext unique not null,
    password_hash text,
    display_name  text not null,
    created_at    timestamptz not null default now()
);

create table oauth_identities (
    user_id    uuid not null references users(id) on delete cascade,
    provider   text not null,
    subject    text not null,
    created_at timestamptz not null default now(),
    primary key (provider, subject)
);

create table sessions (
    id         bytea primary key,
    user_id    uuid not null references users(id) on delete cascade,
    expires_at timestamptz not null,
    created_at timestamptz not null default now(),
    user_agent text,
    ip         inet
);
create index sessions_user_id_idx   on sessions (user_id);
create index sessions_expires_at_idx on sessions (expires_at);

create table photos (
    id            uuid primary key default gen_random_uuid(),
    owner_id      uuid not null references users(id) on delete cascade,
    storage_key   text not null,
    original_name text not null,
    bytes         bigint not null,
    mime          text not null,
    width         int,
    height        int,
    -- EXIF (denormalized for query)
    taken_at      timestamptz,
    camera        text,
    lens          text,
    iso           int,
    exposure_s    double precision,
    focal_mm      double precision,
    -- Astro
    ra_deg        double precision,
    dec_deg       double precision,
    target        text,
    -- Raw + metadata
    exif_json     jsonb,
    caption       text,
    status        text not null default 'ready',
    created_at    timestamptz not null default now()
);
create index photos_owner_created_idx on photos (owner_id, created_at desc);
create index photos_caption_trgm_idx
    on photos using gin (caption gin_trgm_ops);
create index photos_target_idx on photos (target);

create table thumbnails (
    photo_id    uuid not null references photos(id) on delete cascade,
    size        int not null,
    storage_key text not null,
    bytes       bigint not null,
    primary key (photo_id, size)
);
```

- [ ] **Step 2: Apply against a clean Postgres**

```bash
docker compose up -d postgres
sleep 2
cd backend
sqlx database create --database-url postgres://astrophoto:astrophoto@localhost:5432/astrophoto || true
sqlx migrate run --database-url postgres://astrophoto:astrophoto@localhost:5432/astrophoto
```

Expected: `Applied 1/migrate init (...)`

- [ ] **Step 3: Verify schema**

```bash
docker compose exec -T postgres psql -U astrophoto -d astrophoto -c "\dt"
```

Expected: `users`, `oauth_identities`, `sessions`, `photos`, `thumbnails`, `_sqlx_migrations`.

- [ ] **Step 4: Run sqlx prepare to create offline metadata**

```bash
cd backend && cargo sqlx prepare -- --lib
```

Expected: creates `backend/.sqlx/` directory (may be empty until macros exist; commit anyway).

- [ ] **Step 5: Commit**

```bash
git add backend/migrations/0001_init.sql backend/.sqlx
git commit -m "feat(backend): add 0001 init migration (users, photos, sessions)"
```

---

## Task 12: Backend `http/` module with `/healthz` handler

**Files:**
- Create: `backend/src/http/mod.rs`
- Create: `backend/src/http/health.rs`
- Modify: `backend/src/lib.rs`

- [ ] **Step 1: Write `backend/src/http/health.rs`**

```rust
use axum::{Json, extract::State, http::StatusCode};
use serde::Serialize;
use sqlx::PgPool;

use crate::AppError;

#[derive(Serialize)]
pub struct HealthBody {
    pub status: &'static str,
    pub db: &'static str,
}

pub async fn healthz(State(pool): State<PgPool>) -> Result<(StatusCode, Json<HealthBody>), AppError> {
    sqlx::query_scalar::<_, i32>("select 1")
        .fetch_one(&pool)
        .await?;
    Ok((StatusCode::OK, Json(HealthBody { status: "ok", db: "ok" })))
}
```

- [ ] **Step 2: Write `backend/src/http/mod.rs`**

```rust
pub mod health;

use axum::{Router, routing::get};
use sqlx::PgPool;

pub fn router(pool: PgPool) -> Router {
    Router::new()
        .route("/healthz", get(health::healthz))
        .with_state(pool)
}
```

- [ ] **Step 3: Update `lib.rs`**

```rust
pub mod config;
pub mod db;
pub mod error;
pub mod http;

pub use config::Config;
pub use error::AppError;
```

- [ ] **Step 4: Verify it compiles**

Run: `cd backend && cargo check`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add backend/src/http backend/src/lib.rs
git commit -m "feat(backend): add /healthz handler"
```

---

## Task 13: Backend `main.rs` wires everything

**Files:**
- Modify: `backend/src/main.rs`

- [ ] **Step 1: Replace `backend/src/main.rs`**

```rust
use anyhow::Result;
use astrophoto::{Config, db, http};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = Config::from_env();
    init_tracing(&cfg.log);

    let pool = db::connect(&cfg.database_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let app = http::router(pool).layer(TraceLayer::new_for_http());

    let listener = TcpListener::bind(&cfg.bind).await?;
    tracing::info!(bind = %cfg.bind, "astrophoto listening");
    axum::serve(listener, app).await?;
    Ok(())
}

fn init_tracing(log: &str) {
    let filter = EnvFilter::try_new(log).unwrap_or_else(|_| EnvFilter::new("info"));
    let layer = if std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        fmt::layer().compact().boxed()
    } else {
        fmt::layer().json().boxed()
    };
    tracing_subscriber::registry().with(filter).with(layer).init();
}
```

> Note: `unwrap_or_else` and `expect` in this file are explicitly allowed
> as boot-path code per CLAUDE.md. If clippy complains, add
> `#[allow(clippy::unwrap_used)]` at the top of the file.

- [ ] **Step 2: Add allow at top of main.rs if clippy fails**

Prepend:

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]
```

- [ ] **Step 3: Build the binary**

Run: `cd backend && cargo build --bin astrophoto`
Expected: builds.

- [ ] **Step 4: Smoke test against running Postgres**

```bash
docker compose up -d postgres
sleep 2
cd backend
APP_BIND=127.0.0.1:8080 \
APP_LOG=info \
APP_DATABASE_URL=postgres://astrophoto:astrophoto@localhost:5432/astrophoto \
APP_SESSION_DOMAIN=localhost \
APP_SESSION_SECURE=false \
APP_PUBLIC_BASE_URL=http://localhost:8080 \
APP_S3_REGION=us-east-1 \
APP_S3_BUCKET=astrophoto \
APP_S3_ACCESS_KEY=minioadmin \
APP_S3_SECRET_KEY=minioadmin \
APP_S3_PATH_STYLE=true \
cargo run --bin astrophoto &
SERVER_PID=$!
sleep 3
curl -s http://127.0.0.1:8080/healthz
kill $SERVER_PID
```

Expected output: `{"status":"ok","db":"ok"}`

- [ ] **Step 5: Commit**

```bash
git add backend/src/main.rs
git commit -m "feat(backend): wire main.rs (config, migrate, healthz, tracing)"
```

---

## Task 14: Backend integration test for `/healthz`

**Files:**
- Create: `backend/tests/healthz.rs`

- [ ] **Step 1: Write the integration test**

```rust
use astrophoto::{db, http};
use axum::{body::Body, http::Request};
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use tower::ServiceExt;

#[tokio::test]
async fn healthz_returns_ok_with_real_postgres() {
    let pg = Postgres::default()
        .start()
        .await
        .expect("postgres container failed to start");
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");

    let pool = db::connect(&url).await.expect("connect");
    sqlx::migrate!("./migrations").run(&pool).await.expect("migrate");

    let app = http::router(pool);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let bytes = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["status"], "ok");
    assert_eq!(v["db"], "ok");
}
```

- [ ] **Step 2: Run it**

```bash
docker info > /dev/null   # confirm Docker is running
cd backend
cargo test --test healthz
```

Expected: `1 passed`. Container is started and torn down automatically.

- [ ] **Step 3: Commit**

```bash
git add backend/tests/healthz.rs
git commit -m "test(backend): add /healthz integration test with testcontainers"
```

---

## Task 15: `gen-types` binary (ts-rs codegen, placeholder)

**Files:**
- Create: `backend/src/bin/gen-types.rs`
- Create: `backend/src/api_types.rs`
- Modify: `backend/src/lib.rs`

- [ ] **Step 1: Create `backend/src/api_types.rs`**

```rust
//! Types exported to the frontend via ts-rs.
//! Mirror DTOs only; never expose internal structs.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../frontend/src/lib/api/types.ts")]
pub struct Health {
    pub status: String,
    pub db: String,
}
```

- [ ] **Step 2: Create `backend/src/bin/gen-types.rs`**

```rust
//! Emits `frontend/src/lib/api/types.ts`. Invoked via `just types`.
//! Each `#[ts(export)]` type writes itself when `cargo test --features ts-rs`
//! runs; we trigger the same machinery here for one-shot generation.

fn main() {
    // ts-rs writes files via a `#[test]` it injects with `export`. Running
    // `cargo test --no-run` is the canonical trigger; emit a marker so the
    // user knows the path:
    println!(
        "// AUTO-GENERATED. Run `just types` to regenerate.\n\
         // Source: backend/src/api_types.rs"
    );
}
```

- [ ] **Step 3: Update `backend/src/lib.rs`**

```rust
pub mod api_types;
pub mod config;
pub mod db;
pub mod error;
pub mod http;

pub use config::Config;
pub use error::AppError;
```

- [ ] **Step 4: Update `justfile` `types` recipe**

Replace the `types:` recipe in the justfile with:

```just
# Regenerate TypeScript types from Rust (uses ts-rs export-on-test).
types:
    mkdir -p frontend/src/lib/api
    cd backend && cargo test --lib export_bindings -- --nocapture || true
    cd backend && cargo run --bin gen-types > /dev/null
    @echo "// generated by ts-rs; see backend/src/api_types.rs" > frontend/src/lib/api/_header.ts
    cd frontend && pnpm prettier --write 'src/lib/api/**/*.ts' || true
```

- [ ] **Step 5: Verify the binary builds**

Run: `cd backend && cargo build --bin gen-types`
Expected: builds.

- [ ] **Step 6: Commit**

```bash
git add backend/src/api_types.rs backend/src/bin/gen-types.rs backend/src/lib.rs justfile
git commit -m "feat(backend): add ts-rs codegen scaffolding (Health type)"
```

---

## Task 16: Frontend skeleton — package.json, configs, app.html

**Files:**
- Create: `frontend/package.json`
- Create: `frontend/svelte.config.js`
- Create: `frontend/vite.config.ts`
- Create: `frontend/tsconfig.json`
- Create: `frontend/.npmrc`
- Create: `frontend/src/app.html`
- Create: `frontend/src/app.css`

- [ ] **Step 1: Create `frontend/package.json`**

```json
{
  "name": "astrophoto-frontend",
  "version": "0.1.0",
  "private": true,
  "scripts": {
    "dev": "vite dev --port 5173",
    "build": "vite build",
    "preview": "vite preview",
    "check": "svelte-kit sync && svelte-check --tsconfig ./tsconfig.json",
    "lint": "prettier --check . && eslint .",
    "format": "prettier --write .",
    "test": "vitest run",
    "test:e2e": "playwright test"
  },
  "devDependencies": {
    "@sveltejs/adapter-node": "^5.2.0",
    "@sveltejs/kit": "^2.7.0",
    "@sveltejs/vite-plugin-svelte": "^4.0.0",
    "@playwright/test": "^1.48.0",
    "@types/node": "^22.0.0",
    "eslint": "^9.0.0",
    "eslint-config-prettier": "^9.1.0",
    "eslint-plugin-svelte": "^2.45.0",
    "prettier": "^3.3.0",
    "prettier-plugin-svelte": "^3.2.0",
    "svelte": "^5.0.0",
    "svelte-check": "^4.0.0",
    "typescript": "^5.6.0",
    "typescript-eslint": "^8.10.0",
    "vite": "^5.4.0",
    "vitest": "^2.1.0"
  },
  "type": "module"
}
```

- [ ] **Step 2: Create `frontend/.npmrc`**

```
engine-strict=true
node-linker=hoisted
```

- [ ] **Step 3: Create `frontend/svelte.config.js`**

```js
import adapter from '@sveltejs/adapter-node';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter(),
    alias: {
      $lib: 'src/lib'
    }
  }
};

export default config;
```

- [ ] **Step 4: Create `frontend/vite.config.ts`**

```ts
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  server: {
    port: 5173
  }
});
```

- [ ] **Step 5: Create `frontend/tsconfig.json`**

```json
{
  "extends": "./.svelte-kit/tsconfig.json",
  "compilerOptions": {
    "allowJs": true,
    "checkJs": true,
    "esModuleInterop": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "skipLibCheck": true,
    "sourceMap": true,
    "strict": true,
    "moduleResolution": "bundler",
    "noUncheckedIndexedAccess": true,
    "exactOptionalPropertyTypes": true
  }
}
```

- [ ] **Step 6: Create `frontend/src/app.html`**

```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <link rel="icon" href="%sveltekit.assets%/favicon.ico" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Astrophoto</title>
    %sveltekit.head%
  </head>
  <body data-sveltekit-preload-data="hover">
    <div id="app">%sveltekit.body%</div>
  </body>
</html>
```

- [ ] **Step 7: Create `frontend/src/app.css`**

```css
:root {
  font-family: system-ui, -apple-system, "Segoe UI", Helvetica, Arial, sans-serif;
  color-scheme: light dark;
  --bg: #0c0e14;
  --fg: #e7ecf2;
  --accent: #6cf;
}

body {
  background: var(--bg);
  color: var(--fg);
  margin: 0;
  min-height: 100dvh;
}

a {
  color: var(--accent);
}
```

- [ ] **Step 8: Install dependencies**

```bash
cd frontend
pnpm install
```

Expected: installs without errors.

- [ ] **Step 9: Commit**

```bash
cd /Volumes/Pascal4Tb/Projects/astrophoto
git add frontend/package.json frontend/pnpm-lock.yaml frontend/.npmrc \
        frontend/svelte.config.js frontend/vite.config.ts frontend/tsconfig.json \
        frontend/src/app.html frontend/src/app.css
git commit -m "feat(frontend): scaffold SvelteKit (Svelte 5, adapter-node, TS strict)"
```

---

## Task 17: Frontend `lib/api/` and `hooks.server.ts` stub

**Files:**
- Create: `frontend/src/lib/api/types.ts`
- Create: `frontend/src/lib/api/client.ts`
- Create: `frontend/src/hooks.server.ts`
- Create: `frontend/src/app.d.ts`

- [ ] **Step 1: Create `frontend/src/lib/api/types.ts`**

```ts
// AUTO-GENERATED-BY: just types  (overwrite by ts-rs).
// This file is committed so the frontend type-checks before the codegen
// has run. Real generation occurs in Task 15 + future API additions.

export interface Health {
  status: string;
  db: string;
}
```

- [ ] **Step 2: Create `frontend/src/lib/api/client.ts`**

```ts
import type { Health } from './types';

const BASE = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? '';

async function get<T>(path: string, fetchFn: typeof fetch = fetch): Promise<T> {
  const res = await fetchFn(`${BASE}${path}`, {
    credentials: 'include',
    headers: { Accept: 'application/json' }
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`GET ${path} ${res.status}: ${text}`);
  }
  return (await res.json()) as T;
}

export const api = {
  health: (f: typeof fetch = fetch) => get<Health>('/healthz', f)
};
```

- [ ] **Step 3: Create `frontend/src/hooks.server.ts`**

```ts
import type { Handle } from '@sveltejs/kit';

// Reads the session cookie and resolves event.locals.user.
// MVP: stub. Real session resolution arrives with the auth feature.
export const handle: Handle = async ({ event, resolve }) => {
  event.locals.user = null;
  return resolve(event);
};
```

- [ ] **Step 4: Create `frontend/src/app.d.ts`**

```ts
declare global {
  namespace App {
    interface Locals {
      user: { id: string; displayName: string } | null;
    }
    interface PageData {
      user: { id: string; displayName: string } | null;
    }
  }
}

export {};
```

- [ ] **Step 5: Verify svelte-check passes**

```bash
cd frontend && pnpm check
```

Expected: 0 errors.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/lib/api frontend/src/hooks.server.ts frontend/src/app.d.ts
git commit -m "feat(frontend): add api client, types, hooks.server stub"
```

---

## Task 18: Frontend `+layout.svelte` and `+page.svelte`

**Files:**
- Create: `frontend/src/routes/+layout.svelte`
- Create: `frontend/src/routes/+layout.server.ts`
- Create: `frontend/src/routes/+page.svelte`
- Create: `frontend/src/routes/+page.server.ts`

- [ ] **Step 1: Create `+layout.server.ts`**

```ts
import type { LayoutServerLoad } from './$types';

export const load: LayoutServerLoad = async ({ locals }) => {
  return { user: locals.user };
};
```

- [ ] **Step 2: Create `+layout.svelte`**

```svelte
<script lang="ts">
  import '../app.css';

  let { children, data } = $props();
</script>

<header>
  <a href="/">Astrophoto</a>
  {#if data.user}
    <span>Hi, {data.user.displayName}</span>
  {/if}
</header>

<main>
  {@render children()}
</main>

<style>
  header {
    padding: 1rem 2rem;
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }
  main {
    padding: 0 2rem 4rem;
    max-width: 64rem;
    margin: 0 auto;
  }
</style>
```

- [ ] **Step 3: Create `+page.server.ts`**

```ts
import { api } from '$lib/api/client';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ fetch }) => {
  try {
    const health = await api.health(fetch);
    return { health };
  } catch {
    return { health: null };
  }
};
```

- [ ] **Step 4: Create `+page.svelte`**

```svelte
<script lang="ts">
  let { data } = $props();
  let healthLine = $derived(
    data.health ? `backend: ${data.health.status} (db ${data.health.db})` : 'backend: unreachable'
  );
</script>

<h1>Astrophoto</h1>
<p>Upload, tag, and share your astrophotographs.</p>

<p class="health">{healthLine}</p>

<style>
  h1 {
    margin-top: 4rem;
    font-size: clamp(2rem, 6vw, 3.5rem);
  }
  .health {
    font-family: ui-monospace, monospace;
    color: #888;
    margin-top: 2rem;
  }
</style>
```

- [ ] **Step 5: Run svelte-check**

```bash
cd frontend && pnpm check
```

Expected: 0 errors.

- [ ] **Step 6: Run a build to confirm SSR compiles**

```bash
cd frontend && pnpm build
```

Expected: builds without errors. (The `health` fetch will fail at build time if no backend; the load handles it.)

- [ ] **Step 7: Commit**

```bash
git add frontend/src/routes
git commit -m "feat(frontend): add layout + landing page with backend health probe"
```

---

## Task 19: Add `backend` and `frontend` services to `compose.yml`

**Files:**
- Modify: `compose.yml`
- Create: `backend/Dockerfile`
- Create: `frontend/Dockerfile`

- [ ] **Step 1: Create `backend/Dockerfile`**

```dockerfile
# syntax=docker/dockerfile:1.7
FROM rust:1.85-slim AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY .sqlx ./.sqlx
COPY src ./src
COPY migrations ./migrations
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin astrophoto

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/astrophoto /usr/local/bin/astrophoto
COPY --from=builder /app/migrations ./migrations
EXPOSE 8080
CMD ["astrophoto"]
```

- [ ] **Step 2: Create `frontend/Dockerfile`**

```dockerfile
# syntax=docker/dockerfile:1.7
FROM node:22-alpine AS builder
WORKDIR /app
RUN corepack enable
COPY package.json pnpm-lock.yaml .npmrc ./
RUN pnpm install --frozen-lockfile
COPY . .
RUN pnpm build

FROM node:22-alpine
WORKDIR /app
COPY --from=builder /app/build ./build
COPY --from=builder /app/package.json ./package.json
COPY --from=builder /app/node_modules ./node_modules
EXPOSE 3000
ENV PORT=3000
CMD ["node", "build"]
```

- [ ] **Step 3: Replace `compose.yml`**

```yaml
name: astrophoto

services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: astrophoto
      POSTGRES_PASSWORD: astrophoto
      POSTGRES_DB: astrophoto
    ports:
      - "5432:5432"
    volumes:
      - ./data/postgres:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U astrophoto"]
      interval: 5s
      timeout: 5s
      retries: 5

  minio:
    image: minio/minio:latest
    command: server /data --console-address ":9001"
    environment:
      MINIO_ROOT_USER: minioadmin
      MINIO_ROOT_PASSWORD: minioadmin
    ports:
      - "9000:9000"
      - "9001:9001"
    volumes:
      - ./data/minio:/data
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9000/minio/health/live"]
      interval: 5s
      timeout: 5s
      retries: 5

  backend:
    build: ./backend
    profiles: ["full"]    # not started by `just dev`; only `docker compose --profile full up`
    environment:
      APP_BIND: 0.0.0.0:8080
      APP_LOG: info
      APP_DATABASE_URL: postgres://astrophoto:astrophoto@postgres:5432/astrophoto
      APP_SESSION_DOMAIN: localhost
      APP_SESSION_SECURE: "false"
      APP_PUBLIC_BASE_URL: http://localhost:8080
      APP_S3_ENDPOINT: http://minio:9000
      APP_S3_REGION: us-east-1
      APP_S3_BUCKET: astrophoto
      APP_S3_ACCESS_KEY: minioadmin
      APP_S3_SECRET_KEY: minioadmin
      APP_S3_PATH_STYLE: "true"
    depends_on:
      postgres:
        condition: service_healthy
      minio:
        condition: service_healthy
    ports:
      - "8080:8080"

  frontend:
    build: ./frontend
    profiles: ["full"]
    environment:
      ORIGIN: http://localhost:5173
      VITE_API_BASE_URL: http://backend:8080
    depends_on:
      - backend
    ports:
      - "5173:3000"
```

> Note: `backend` and `frontend` use the `full` profile so `just dev`
> stays fast (only postgres + minio + cargo run + pnpm dev). For a full
> dockerized run: `docker compose --profile full up`.

- [ ] **Step 4: Validate compose**

Run: `docker compose config -q`
Expected: exit 0.

- [ ] **Step 5: Commit**

```bash
git add backend/Dockerfile frontend/Dockerfile compose.yml
git commit -m "chore: dockerize backend+frontend (full profile in compose)"
```

---

## Task 20: Wire `just check` and final smoke test

**Files:**
- Modify: `justfile` (only if needed)

- [ ] **Step 1: Run the full quality gate**

```bash
cd /Volumes/Pascal4Tb/Projects/astrophoto
just check
```

Expected: zero errors. If clippy reports issues, fix them in-place, do
NOT silence them with allows beyond the `expect_used` boot-path
exception already in place.

- [ ] **Step 2: Run the full test suite**

```bash
just test
```

Expected: all backend tests pass (including `healthz` integration test).
Frontend `vitest` has no tests yet — the run should report "0 tests"
without failing.

- [ ] **Step 3: Manual dev smoke test**

Open four terminals (or one, if you trust `&`):

```bash
# Terminal 1
docker compose up -d postgres minio
sleep 3
cd backend && sqlx migrate run --database-url postgres://astrophoto:astrophoto@localhost:5432/astrophoto

# Terminal 2
cd backend
APP_BIND=127.0.0.1:8080 APP_LOG=info \
  APP_DATABASE_URL=postgres://astrophoto:astrophoto@localhost:5432/astrophoto \
  APP_SESSION_DOMAIN=localhost APP_SESSION_SECURE=false \
  APP_PUBLIC_BASE_URL=http://localhost:8080 \
  APP_S3_REGION=us-east-1 APP_S3_BUCKET=astrophoto \
  APP_S3_ACCESS_KEY=minioadmin APP_S3_SECRET_KEY=minioadmin \
  APP_S3_PATH_STYLE=true \
  cargo run

# Terminal 3
cd frontend
VITE_API_BASE_URL=http://localhost:8080 pnpm dev

# Terminal 4
curl -s http://localhost:8080/healthz
# Expected: {"status":"ok","db":"ok"}
curl -s http://localhost:5173 | head -20
# Expected: HTML containing "Astrophoto" and a hydration script
```

- [ ] **Step 4: Tag the bootstrap commit**

```bash
git tag -a v0.0.1-bootstrap -m "Astrophoto bootstrap: CLAUDE.md + Rust+SvelteKit skeleton"
```

- [ ] **Step 5: Final commit (if any straggler files from check fixes)**

```bash
git status
# If there are changes:
git add -A
git commit -m "chore: bootstrap polish from just check fixes"
```

---

## Self-review (completed)

**Spec coverage:**
- §"Repository layout" → Tasks 1, 7, 16 ✓
- §"Backend architecture" → Tasks 7-15 ✓
- §"Frontend architecture" → Tasks 16-18 ✓
- §"Domain model" → Task 11 (initial migration) ✓
- §"Image pipeline" → out of scope for bootstrap (post-MVP next plan)
- §"Testing" → Tasks 8, 9, 14 ✓ (full feature tests are next-plan)
- §"Deployment" → Tasks 6, 19 ✓
- §"CLAUDE.md content" → Task 2 ✓
- §"Bootstrap deliverables" → checklist matches Tasks 1-19 ✓

**Placeholder scan:** none. Every code block is concrete; every command
has expected output; no "TBD" anywhere.

**Type consistency:** `Health { status, db }` is identical in
`backend/src/api_types.rs` (Task 15), `backend/src/http/health.rs`
(Task 12), and `frontend/src/lib/api/types.ts` (Task 17). Field names
match. `AppError` variants used in handlers (Task 12) all exist in
`error.rs` (Task 8).

---

## Execution Handoff

Plan complete and saved to
`docs/superpowers/plans/2026-05-01-astrophoto-bootstrap.md`. Two
execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per
task, review between tasks, fast iteration.

**2. Inline Execution** — I execute tasks in this session using the
executing-plans skill, with batch execution and checkpoints for review.

**Which approach?**
