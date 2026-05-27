# CLAUDE.md

Astrophoto: Rust + SvelteKit app for amateur astrophotographers to upload,
tag, and share images. Backend: axum + sqlx + Postgres. Frontend:
SvelteKit SSR + Svelte 5 runes. Image storage: **AWS S3** in prod/staging
(bucket `astrophoto-images-<env>`) fronted by CloudFront + **Lambda@Edge
(origin-request)** using sharp for on-the-fly transforms. Two dev paths
supported, picked via `.env`: (a) MinIO + the backend's `/cdn/img/<id>`
route for local resize (no AWS account needed); (b) real AWS S3
(`astrophoto-images-dev`) + same local `/cdn/img/<id>` route, for
prod-parity testing of CORS, IAM, and SigV4. CloudFront is not deployed
in dev; use `APP_CDN_LOCAL_FALLBACK=true` to opt into the backend's
`/cdn/img/` route on non-localhost hosts (e.g., staging before CloudFront
is provisioned).

## Staging

Deployed on Koyeb. Resources as of 2026-05:

- **Frontend:** `https://astrophoto-staging-web-xavyo-eadbe1f6.koyeb.app`
- **Backend:** `https://astrophoto-staging-xavyo-008151d0.koyeb.app`
- **CDN:** `https://ddo5booq71gbx.cloudfront.net` (CloudFront distribution
  `E2B1QQ4K2EISGE`, S3 bucket `astrophoto-images-staging`)
- Runbook: `docs/operations/aws-s3-cloudfront.md`

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

- **`photo_filters` is the source of truth — `photos.filters` is a
  denormalized cache.** Every writer of the junction (metadata PUT
  with `filter_item_ids`, apply-setup) must call
  `crate::photos::filters_cache::rebuild(&mut tx, photo_id)` in the
  same transaction. Do NOT update `photos.filters` directly except
  via that helper. The legacy `filters: Option<String>` body on
  metadata PUT survives for back-compat (it writes the string cache
  but does not auto-populate the junction); when both are present in
  the same request, the structured `filter_item_ids` branch wins
  because `rebuild` runs last. See `backend/src/photos/metadata.rs`
  and `apply_setup.rs`.
- The `photos.status` field can be `processing` for a few seconds after
  upload while `spawn_blocking` decodes thumbnails. UI must handle this.
- EXIF datetimes are naive (no timezone). We store as `timestamptz` and
  assume UTC unless the camera embeds GPS + offset.
- MinIO (dev) does not enforce all S3 quirks. Test path-style vs
  virtual-hosted addressing with `aws-sdk-s3` config matching prod.
  When in doubt, switch the worktree's `.env` to the AWS-S3 dev path
  (`APP_S3_BUCKET=astrophoto-images-dev`, `APP_S3_REGION=ap-southeast-1`,
  `APP_S3_ENDPOINT` removed entirely, `APP_S3_PATH_STYLE=false`,
  credentials from the `astrophoto-dev-uploader` IAM user) and re-run.
- Display master pattern: every photo has both an
  `originals/<id>.<ext>` (archival) and `display/<id>.jpg` (4096 px /
  q=85 — what the CDN transforms). Never plumb originals through the
  CDN.
- Presigned PUT signs the EXACT body byte count. Calling
  `Storage::presigned_put` with anything other than the file's real
  size will produce URLs that S3 rejects with `SignatureDoesNotMatch`.
  Tier limits are enforced in the upload-init handler before signing,
  not via the signed `content-length`. (Discovered the hard way during
  Batch C smoke-testing — fixed in commit 4a382cc.)
- Session cookie is `__Host-session` in prod (HTTPS) and plain
  `session` in dev (HTTP). The `__Host-` prefix is browser-enforced
  to require `Secure`, so dev over plain HTTP must drop the prefix
  or the cookie is silently rejected. The read path accepts both via
  `session::COOKIE_NAMES`. Same pattern for the OAuth state cookie.
- Session cookie SameSite policy splits on the `Secure` flag:
  `SameSite=None` when `secure=true` (prod / staging), `SameSite=Lax`
  when `secure=false` (dev). Frontend and backend live on different
  sibling subdomains in the Koyeb staging deploy
  (`*-web-*.koyeb.app` vs `*-008151d0.koyeb.app`) with no shared
  parent — `Lax` blocks the browser→backend cross-origin fetch even
  with `credentials: 'include'`. Cross-site exposure stays bounded
  because CORS allows exactly one origin (`APP_CORS_ORIGIN`). Don't
  flip back to `Lax` without re-reading `auth/session.rs::cookie_header`.
- The OAuth state cookie still uses `SameSite=Lax` — that flow is a
  top-level cross-site redirect (Google → backend callback), and `Lax`
  is exactly what's needed there. Do not change without reading
  `auth/oauth_google.rs`.
- Argon2 password verification is CPU-bound: always inside
  `spawn_blocking`. Same for image decode.
- CDN transform uses **Lambda@Edge (origin-request)**, not a Lambda
  Function URL. Lambda Function URL Block Public Access (FUBPA) silently
  blocks Function URLs in this AWS account since 2024-Q4. See
  `docs/operations/aws-s3-cloudfront.md` for the full architecture.
- `APP_CDN_LOCAL_FALLBACK=true` opts into the backend's `/cdn/img/`
  route even on non-localhost hosts. Use this in staging when CloudFront
  is not yet provisioned, or in prod for emergency CDN bypass. Leave
  unset in normal operation.
- **The SvelteKit `/api/*` reverse proxy is body-size-capped by the Node
  adapter, NOT by the backend.** `frontend/src/routes/api/[...rest]/+server.ts`
  forwards every browser→API call so the session cookie stays same-origin.
  `@sveltejs/adapter-node` enforces `BODY_SIZE_LIMIT` (default **512 KB**)
  on the incoming stream — so any large-body POST routed through the proxy
  (plate-solve re-solve, photo replace) is killed *at the frontend* before
  reaching axum, surfacing as a useless SvelteKit `500 {"message":"Internal
  Error"}` (note: capital-I, no `error` field — that's how you tell it from
  a backend `AppError`, which is `{"error":...,"message":"internal error"}`).
  Most uploads dodge this by going **direct to S3** via a presigned PUT, so
  the proxy cap stayed invisible until the side-channel endpoints shipped.
  Whenever a backend `/api/*` route's `DefaultBodyLimit` is raised, raise
  `BODY_SIZE_LIMIT` on **both** frontend Koyeb services
  (`astrophoto-prod-web`, `astrophoto-staging-web`) to a value just above
  the backend cap (currently `MAX_XISF_BYTES` = 128 MiB → set
  `BODY_SIZE_LIMIT=140000000`). The proxy streams the body
  (`init.body = request.body; init.duplex = 'half'`) so it never buffers the
  whole file; do not revert that to `arrayBuffer()`.

---

## References

- Design docs: `docs/superpowers/specs/`
- Svelte 5 LLM context: https://svelte.dev/llms-small.txt
- sqlx macro docs: https://docs.rs/sqlx/latest/sqlx/macro.query.html
- agents.md spec (this file is also linked as `AGENTS.md`):
  https://agents.md
