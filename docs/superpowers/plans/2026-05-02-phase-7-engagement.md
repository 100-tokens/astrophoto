# Phase 7 Engagement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the social engagement layer — appreciations (♥, count, no public list), comments (1-level threading, photo owner moderates), follows (asymmetric), and a following feed on the authenticated home with graceful fallback to the public gallery.

**Architecture:** New `engagement/` module on the backend with three sibling files (appreciations, comments, follows), each containing handlers + inline SQL queries. Schema lands in migration `0002_engagement.sql`. The frontend gets three new components (AppreciateButton, FollowButton, CommentsSection) wired through SvelteKit form actions; the gallery loader gains a following-feed branch.

**Tech Stack:** Existing — axum 0.7, sqlx 0.8 (compile-time SQL via `.sqlx/`), Svelte 5 runes, ts-rs (Rust → TS codegen), tower (no new deps).

**Spec reference:** `docs/superpowers/specs/2026-05-02-phase-7-engagement-design.md`

**Working directory:** `/Volumes/Pascal4Tb/Projects/astrophoto/` (referred to below as `$ROOT`).

**Branch:** `feat/phase-7-engagement` (already created, spec committed).

**Live infra prereqs:**
- Postgres on `localhost:5434` (`astrophoto`/`astrophoto`/`astrophoto`)
- MinIO on `localhost:9100`/`9101`
- For all `cargo` commands compiling SQL macros: `export DATABASE_URL=postgres://astrophoto:astrophoto@localhost:5434/astrophoto`
- After every task with new SQL macros: `cargo sqlx prepare -- --lib --tests --bins` and commit `.sqlx/`

---

## Task 1: Migration `0002_engagement.sql`

**Files:**
- Create: `backend/migrations/0002_engagement.sql`

- [ ] **Step 1: Create the migration**

```sql
-- Phase 7: appreciations, comments, follows.

create table appreciations (
    user_id    uuid not null references users(id) on delete cascade,
    photo_id   uuid not null references photos(id) on delete cascade,
    created_at timestamptz not null default now(),
    primary key (user_id, photo_id)
);
create index appreciations_photo_id_idx on appreciations (photo_id);

create table comments (
    id          uuid primary key default gen_random_uuid(),
    photo_id    uuid not null references photos(id) on delete cascade,
    author_id   uuid not null references users(id) on delete cascade,
    body        text not null check (length(body) between 1 and 2000),
    created_at  timestamptz not null default now()
);
create index comments_photo_created_idx on comments (photo_id, created_at);

create table follows (
    follower_id uuid not null references users(id) on delete cascade,
    followed_id uuid not null references users(id) on delete cascade,
    created_at  timestamptz not null default now(),
    primary key (follower_id, followed_id),
    check (follower_id <> followed_id)
);
create index follows_followed_idx on follows (followed_id);
```

- [ ] **Step 2: Apply against the live dev DB**

```bash
cd $ROOT/backend
DATABASE_URL=postgres://astrophoto:astrophoto@localhost:5434/astrophoto sqlx migrate run
```

Expected: `Applied 2/migrate engagement`.

- [ ] **Step 3: Verify schema**

```bash
docker compose exec -T postgres psql -U astrophoto -d astrophoto -c "\dt"
```

Expected: tables `appreciations`, `comments`, `follows` present in the listing alongside the existing tables.

- [ ] **Step 4: Commit**

```bash
cd $ROOT
git add backend/migrations/0002_engagement.sql
git commit -m "feat(backend): migration 0002 for engagement (appreciations, comments, follows)"
```

---

## Task 2: Appreciations module

**Files:**
- Create: `backend/src/engagement/mod.rs`
- Create: `backend/src/engagement/appreciations.rs`
- Modify: `backend/src/lib.rs` (add `pub mod engagement;`)
- Modify: `backend/src/http/mod.rs` (mount 4 routes)

- [ ] **Step 1: Create `backend/src/engagement/mod.rs`**

```rust
pub mod appreciations;
```

(`comments` and `follows` get added to this file in Tasks 3 and 4.)

- [ ] **Step 2: Create `backend/src/engagement/appreciations.rs`**

```rust
//! Appreciations: idempotent ♥ toggle on a photo. Auth required to
//! mutate, public to read counts; the per-user state has its own
//! auth-required endpoint.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Serialize;
use uuid::Uuid;

use crate::AppError;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

#[derive(Serialize)]
pub struct CountResponse {
    pub count: i64,
}

#[derive(Serialize)]
pub struct StateResponse {
    pub appreciated: bool,
}

pub async fn appreciate(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(photo_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    sqlx::query!(
        r#"
        insert into appreciations (user_id, photo_id)
        values ($1, $2)
        on conflict (user_id, photo_id) do nothing
        "#,
        user.id,
        photo_id
    )
    .execute(&state.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn unappreciate(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(photo_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    sqlx::query!(
        "delete from appreciations where user_id = $1 and photo_id = $2",
        user.id,
        photo_id
    )
    .execute(&state.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn count(
    State(state): State<AppState>,
    Path(photo_id): Path<Uuid>,
) -> Result<Json<CountResponse>, AppError> {
    let row = sqlx::query!(
        r#"select count(*) as "count!" from appreciations where photo_id = $1"#,
        photo_id
    )
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(CountResponse { count: row.count }))
}

pub async fn state_for_user(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(photo_id): Path<Uuid>,
) -> Result<Json<StateResponse>, AppError> {
    let row = sqlx::query!(
        "select 1 as one from appreciations where user_id = $1 and photo_id = $2 limit 1",
        user.id,
        photo_id
    )
    .fetch_optional(&state.pool)
    .await?;
    Ok(Json(StateResponse {
        appreciated: row.is_some(),
    }))
}
```

- [ ] **Step 3: Wire `pub mod engagement;` in `backend/src/lib.rs`**

Open `backend/src/lib.rs`. Below the existing `pub mod` lines (e.g. `pub mod photos;`, `pub mod users;`, ...), add:

```rust
pub mod engagement;
```

- [ ] **Step 4: Mount the routes**

Open `backend/src/http/mod.rs`. In the `pub fn router(...)` chain, add four route lines next to the photos routes:

```rust
.route(
    "/api/photos/:id/appreciate",
    axum::routing::post(crate::engagement::appreciations::appreciate)
        .delete(crate::engagement::appreciations::unappreciate),
)
.route(
    "/api/photos/:id/appreciations/count",
    axum::routing::get(crate::engagement::appreciations::count),
)
.route(
    "/api/photos/:id/appreciation-state",
    axum::routing::get(crate::engagement::appreciations::state_for_user),
)
```

- [ ] **Step 5: Refresh sqlx cache + cargo check**

```bash
cd $ROOT/backend
DATABASE_URL=postgres://astrophoto:astrophoto@localhost:5434/astrophoto cargo sqlx prepare -- --lib --tests --bins
cargo check
```

Expected: clean.

- [ ] **Step 6: Smoke test live**

Backend running (with the standard env vars from previous phases). New terminal:

```bash
EMAIL="appreciate-$(date +%s)@test.com"
SIGNUP=$(curl -s -c /tmp/c.txt -X POST http://localhost:8080/api/auth/signup \
  -H 'Content-Type: application/json' \
  -d "{\"email\":\"$EMAIL\",\"password\":\"longenoughpw\",\"display_name\":\"Test\"}")

# Use one of the existing photo ids from the seeded gallery
PHOTO_ID=$(curl -s 'http://localhost:8080/api/photos?limit=1' \
  | python3 -c 'import sys,json; print(json.load(sys.stdin)["photos"][0]["id"])')

# Initial count
curl -s "http://localhost:8080/api/photos/$PHOTO_ID/appreciations/count"

# Appreciate (idempotent — call twice)
curl -i -s -b /tmp/c.txt -X POST "http://localhost:8080/api/photos/$PHOTO_ID/appreciate" | head -2
curl -i -s -b /tmp/c.txt -X POST "http://localhost:8080/api/photos/$PHOTO_ID/appreciate" | head -2

# Count goes to 1, state for user true
curl -s "http://localhost:8080/api/photos/$PHOTO_ID/appreciations/count"
curl -s -b /tmp/c.txt "http://localhost:8080/api/photos/$PHOTO_ID/appreciation-state"

# Un-appreciate (idempotent)
curl -i -s -b /tmp/c.txt -X DELETE "http://localhost:8080/api/photos/$PHOTO_ID/appreciate" | head -2
curl -s "http://localhost:8080/api/photos/$PHOTO_ID/appreciations/count"
curl -s -b /tmp/c.txt "http://localhost:8080/api/photos/$PHOTO_ID/appreciation-state"
```

Expected:
- Both POSTs return 204.
- Count: `{"count":0}` initial → `{"count":1}` after first POST → still `{"count":1}` after second POST → `{"count":0}` after DELETE.
- State: `{"appreciated":true}` then `{"appreciated":false}`.

- [ ] **Step 7: Commit**

```bash
cd $ROOT
git add backend/src/engagement/mod.rs \
        backend/src/engagement/appreciations.rs \
        backend/src/lib.rs \
        backend/src/http/mod.rs \
        backend/.sqlx/
git commit -m "feat(backend/engagement): add appreciations (♥ toggle + count + state)"
```

---

## Task 3: Comments module

**Files:**
- Create: `backend/src/engagement/comments.rs`
- Modify: `backend/src/engagement/mod.rs` (add `pub mod comments;`)
- Modify: `backend/src/http/mod.rs` (mount 3 routes)

- [ ] **Step 1: Create `backend/src/engagement/comments.rs`**

```rust
//! Comments: 1-level (flat). Anyone can read, auth required to post.
//! Delete authorized for the comment author OR the photo owner.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::AppError;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

#[derive(Serialize)]
pub struct Comment {
    pub id: String,
    pub photo_id: String,
    pub author_id: String,
    pub author_display_name: String,
    pub body: String,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct ListResponse {
    pub comments: Vec<Comment>,
}

#[derive(Deserialize, Validate)]
pub struct CreateBody {
    #[validate(length(min = 1, max = 2000))]
    pub body: String,
}

struct CommentRow {
    id: Uuid,
    photo_id: Uuid,
    author_id: Uuid,
    author_display_name: String,
    body: String,
    created_at: DateTime<Utc>,
}

impl From<CommentRow> for Comment {
    fn from(r: CommentRow) -> Self {
        Comment {
            id: r.id.to_string(),
            photo_id: r.photo_id.to_string(),
            author_id: r.author_id.to_string(),
            author_display_name: r.author_display_name,
            body: r.body,
            created_at: r.created_at.to_rfc3339(),
        }
    }
}

pub async fn list(
    State(state): State<AppState>,
    Path(photo_id): Path<Uuid>,
) -> Result<Json<ListResponse>, AppError> {
    let rows = sqlx::query_as!(
        CommentRow,
        r#"
        select c.id, c.photo_id, c.author_id,
               u.display_name as author_display_name,
               c.body, c.created_at
        from comments c
        join users u on u.id = c.author_id
        where c.photo_id = $1
        order by c.created_at asc
        "#,
        photo_id
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(ListResponse {
        comments: rows.into_iter().map(Into::into).collect(),
    }))
}

pub async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(photo_id): Path<Uuid>,
    Json(body): Json<CreateBody>,
) -> Result<(StatusCode, Json<Comment>), AppError> {
    body.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // Verify the photo exists; surfaces 404 (photos table FK enforces it
    // anyway, but better error message than a constraint violation).
    let exists = sqlx::query!("select id from photos where id = $1", photo_id)
        .fetch_optional(&state.pool)
        .await?;
    if exists.is_none() {
        return Err(AppError::NotFound);
    }

    let row = sqlx::query_as!(
        CommentRow,
        r#"
        with inserted as (
            insert into comments (photo_id, author_id, body)
            values ($1, $2, $3)
            returning id, photo_id, author_id, body, created_at
        )
        select i.id, i.photo_id, i.author_id,
               u.display_name as author_display_name,
               i.body, i.created_at
        from inserted i
        join users u on u.id = i.author_id
        "#,
        photo_id,
        user.id,
        body.body,
    )
    .fetch_one(&state.pool)
    .await?;

    Ok((StatusCode::CREATED, Json(row.into())))
}

pub async fn delete(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(comment_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    // Look up: who wrote it AND who owns the photo it's on.
    let row = sqlx::query!(
        r#"
        select c.author_id, p.owner_id as photo_owner_id
        from comments c
        join photos p on p.id = c.photo_id
        where c.id = $1
        "#,
        comment_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    if row.author_id != user.id && row.photo_owner_id != user.id {
        return Err(AppError::Forbidden);
    }

    sqlx::query!("delete from comments where id = $1", comment_id)
        .execute(&state.pool)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
```

- [ ] **Step 2: Wire the module**

In `backend/src/engagement/mod.rs`, add:

```rust
pub mod comments;
```

- [ ] **Step 3: Mount the routes**

In `backend/src/http/mod.rs`, append next to the appreciations routes:

```rust
.route(
    "/api/photos/:id/comments",
    axum::routing::get(crate::engagement::comments::list)
        .post(crate::engagement::comments::create),
)
.route(
    "/api/comments/:id",
    axum::routing::delete(crate::engagement::comments::delete),
)
```

- [ ] **Step 4: Refresh sqlx cache + cargo check**

```bash
cd $ROOT/backend
DATABASE_URL=postgres://astrophoto:astrophoto@localhost:5434/astrophoto cargo sqlx prepare -- --lib --tests --bins
cargo check
```

- [ ] **Step 5: Smoke test live**

```bash
# Reuse cookie from Task 2 smoke (or sign up a fresh user). Let's sign up two:
EMAIL_A="a-$(date +%s)@test.com"
curl -s -c /tmp/cA.txt -X POST http://localhost:8080/api/auth/signup \
  -H 'Content-Type: application/json' \
  -d "{\"email\":\"$EMAIL_A\",\"password\":\"longenoughpw\",\"display_name\":\"User A\"}" >/dev/null

EMAIL_B="b-$(date +%s)@test.com"
curl -s -c /tmp/cB.txt -X POST http://localhost:8080/api/auth/signup \
  -H 'Content-Type: application/json' \
  -d "{\"email\":\"$EMAIL_B\",\"password\":\"longenoughpw\",\"display_name\":\"User B\"}" >/dev/null

PHOTO_ID=$(curl -s 'http://localhost:8080/api/photos?limit=1' \
  | python3 -c 'import sys,json; print(json.load(sys.stdin)["photos"][0]["id"])')

# A posts a comment
COMMENT_ID=$(curl -s -b /tmp/cA.txt -X POST "http://localhost:8080/api/photos/$PHOTO_ID/comments" \
  -H 'Content-Type: application/json' -d '{"body":"Test comment from A"}' \
  | python3 -c 'import sys,json; print(json.load(sys.stdin)["id"])')

# List
curl -s "http://localhost:8080/api/photos/$PHOTO_ID/comments" | python3 -m json.tool

# B tries to delete A's comment — 403
curl -i -s -b /tmp/cB.txt -X DELETE "http://localhost:8080/api/comments/$COMMENT_ID" | head -2

# A deletes own — 204
curl -i -s -b /tmp/cA.txt -X DELETE "http://localhost:8080/api/comments/$COMMENT_ID" | head -2

# Confirm gone
curl -s "http://localhost:8080/api/photos/$PHOTO_ID/comments" | python3 -m json.tool
```

Expected: B's DELETE returns 403, A's returns 204, list ends empty.

- [ ] **Step 6: Commit**

```bash
cd $ROOT
git add backend/src/engagement/comments.rs \
        backend/src/engagement/mod.rs \
        backend/src/http/mod.rs \
        backend/.sqlx/
git commit -m "feat(backend/engagement): add comments (list/create/delete with auth rules)"
```

---

## Task 4: Follows module

**Files:**
- Create: `backend/src/engagement/follows.rs`
- Modify: `backend/src/engagement/mod.rs` (add `pub mod follows;`)
- Modify: `backend/src/http/mod.rs` (mount 4 routes)

- [ ] **Step 1: Create `backend/src/engagement/follows.rs`**

```rust
//! Follows: asymmetric, idempotent toggle. Auth required to mutate.
//! Counts public.

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Serialize;
use uuid::Uuid;

use crate::AppError;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

#[derive(Serialize)]
pub struct CountResponse {
    pub count: i64,
}

pub async fn follow(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(target_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    if user.id == target_id {
        return Err(AppError::Validation("cannot follow yourself".into()));
    }
    sqlx::query!(
        r#"
        insert into follows (follower_id, followed_id)
        values ($1, $2)
        on conflict (follower_id, followed_id) do nothing
        "#,
        user.id,
        target_id,
    )
    .execute(&state.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn unfollow(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(target_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    sqlx::query!(
        "delete from follows where follower_id = $1 and followed_id = $2",
        user.id,
        target_id,
    )
    .execute(&state.pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn followers_count(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<CountResponse>, AppError> {
    let row = sqlx::query!(
        r#"select count(*) as "count!" from follows where followed_id = $1"#,
        user_id
    )
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(CountResponse { count: row.count }))
}

pub async fn following_count(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<CountResponse>, AppError> {
    let row = sqlx::query!(
        r#"select count(*) as "count!" from follows where follower_id = $1"#,
        user_id
    )
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(CountResponse { count: row.count }))
}
```

- [ ] **Step 2: Wire the module**

In `backend/src/engagement/mod.rs`, add:

```rust
pub mod follows;
```

- [ ] **Step 3: Mount the routes**

In `backend/src/http/mod.rs`, append:

```rust
.route(
    "/api/users/:id/follow",
    axum::routing::post(crate::engagement::follows::follow)
        .delete(crate::engagement::follows::unfollow),
)
.route(
    "/api/users/:id/followers/count",
    axum::routing::get(crate::engagement::follows::followers_count),
)
.route(
    "/api/users/:id/following/count",
    axum::routing::get(crate::engagement::follows::following_count),
)
```

- [ ] **Step 4: Refresh sqlx cache + cargo check**

```bash
cd $ROOT/backend
DATABASE_URL=postgres://astrophoto:astrophoto@localhost:5434/astrophoto cargo sqlx prepare -- --lib --tests --bins
cargo check
```

- [ ] **Step 5: Smoke test live**

```bash
# Reuse cA / cB from Task 3 smoke
USER_A_ID=$(curl -s -b /tmp/cA.txt http://localhost:8080/api/auth/me | python3 -c 'import sys,json; print(json.load(sys.stdin)["id"])')
USER_B_ID=$(curl -s -b /tmp/cB.txt http://localhost:8080/api/auth/me | python3 -c 'import sys,json; print(json.load(sys.stdin)["id"])')

# A follows B (idempotent)
curl -i -s -b /tmp/cA.txt -X POST "http://localhost:8080/api/users/$USER_B_ID/follow" | head -2
curl -i -s -b /tmp/cA.txt -X POST "http://localhost:8080/api/users/$USER_B_ID/follow" | head -2

# Counts
curl -s "http://localhost:8080/api/users/$USER_B_ID/followers/count"
curl -s "http://localhost:8080/api/users/$USER_A_ID/following/count"

# A tries to follow self — 422
curl -i -s -b /tmp/cA.txt -X POST "http://localhost:8080/api/users/$USER_A_ID/follow" | head -2

# A unfollows B (idempotent)
curl -i -s -b /tmp/cA.txt -X DELETE "http://localhost:8080/api/users/$USER_B_ID/follow" | head -2
curl -s "http://localhost:8080/api/users/$USER_B_ID/followers/count"
```

Expected:
- Both POSTs return 204; counts go to 1 and stay at 1 (idempotent).
- Self-follow returns 422.
- DELETE returns 204; followers count back to 0.

- [ ] **Step 6: Commit**

```bash
cd $ROOT
git add backend/src/engagement/follows.rs \
        backend/src/engagement/mod.rs \
        backend/src/http/mod.rs \
        backend/.sqlx/
git commit -m "feat(backend/engagement): add follows (asymmetric toggle + counts)"
```

---

## Task 5: Extend `/api/auth/me` with `following_ids`

**Files:**
- Modify: `backend/src/auth/me.rs`
- Modify: `backend/src/api_types.rs` (extend `User`)
- Modify: `backend/src/users/mod.rs` (`From<UserRow> for User` no longer enough — needs the IDs separately)

The `User` DTO needs `following_ids: Vec<String>`. The `From<UserRow>` impl can't supply this (no DB access). So `me.rs` constructs the DTO inline.

- [ ] **Step 1: Extend `api_types.rs`**

Open `backend/src/api_types.rs`. Modify the `User` struct:

```rust
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "User.ts")]
pub struct User {
    pub id: String,
    pub email: String,
    pub display_name: String,
    pub created_at: String,
    pub following_ids: Vec<String>,
}
```

- [ ] **Step 2: Update `From<UserRow> for User`** in `backend/src/users/mod.rs`

The existing impl returns a `User` with no `following_ids`. Two paths: either drop the `From` impl entirely (callers always need to fetch follows separately), or have `From` produce `following_ids: vec![]` and let callers override. Pick option 2 — it's a sensible default for places that don't care.

In `backend/src/users/mod.rs`, modify the `From` impl:

```rust
impl From<UserRow> for User {
    fn from(r: UserRow) -> Self {
        User {
            id: r.id.to_string(),
            email: r.email,
            display_name: r.display_name,
            created_at: r.created_at.to_rfc3339(),
            following_ids: vec![],
        }
    }
}
```

- [ ] **Step 3: Modify `backend/src/auth/me.rs` to fetch and populate `following_ids`**

Replace the body of `handler`:

```rust
use axum::{Json, extract::State, response::IntoResponse};

use crate::AppError;
use crate::api_types::User;
use crate::auth::middleware::CurrentUser;
use crate::http::AppState;

pub async fn handler(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> Result<impl IntoResponse, AppError> {
    let following_ids = sqlx::query!(
        "select followed_id from follows where follower_id = $1 limit 500",
        user.id
    )
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .map(|r| r.followed_id.to_string())
    .collect();

    let dto = User {
        id: user.id.to_string(),
        email: user.email,
        display_name: user.display_name,
        created_at: user.created_at.to_rfc3339(),
        following_ids,
    };

    Ok(Json(dto))
}
```

- [ ] **Step 4: Refresh cache + check**

```bash
cd $ROOT/backend
DATABASE_URL=postgres://astrophoto:astrophoto@localhost:5434/astrophoto cargo sqlx prepare -- --lib --tests --bins
cargo check
cargo test --lib
```

Expected: clean. The existing `users::queries::find_by_id`-based usages still work because `From<UserRow>` returns `following_ids: vec![]` (signin/signup endpoints aren't expected to populate this; only `/me` does).

- [ ] **Step 5: Smoke test**

```bash
# Reuse cA / cB
USER_B_ID=$(curl -s -b /tmp/cB.txt http://localhost:8080/api/auth/me | python3 -c 'import sys,json; print(json.load(sys.stdin)["id"])')

# A follows B again
curl -s -b /tmp/cA.txt -X POST "http://localhost:8080/api/users/$USER_B_ID/follow" >/dev/null

# /me returns following_ids containing B
curl -s -b /tmp/cA.txt http://localhost:8080/api/auth/me | python3 -m json.tool
```

Expected: the `/me` JSON includes `"following_ids": ["<B-uuid>"]`.

- [ ] **Step 6: Commit**

```bash
cd $ROOT
git add backend/src/auth/me.rs \
        backend/src/api_types.rs \
        backend/src/users/mod.rs \
        backend/.sqlx/
git commit -m "feat(backend/auth): /me returns following_ids (cap 500)"
```

---

## Task 6: Extend `/api/photos/:id` with engagement counts

**Files:**
- Modify: `backend/src/photos/get.rs`

- [ ] **Step 1: Add count fields to `PhotoDetail`**

In `backend/src/photos/get.rs`, find the `PhotoDetail` struct. Append:

```rust
    pub appreciation_count: i64,
    pub comment_count: i64,
```

(Right after `pub created_at: String,`.)

- [ ] **Step 2: Update `From<PhotoRow>` to keep counts at 0 by default**

The `From<PhotoRow> for PhotoDetail` impl returns counts of 0; the handler overrides them via direct construction. To keep `From` viable, just add the new fields with default 0:

```rust
impl From<PhotoRow> for PhotoDetail {
    fn from(p: PhotoRow) -> Self {
        Self {
            id: p.id.to_string(),
            owner_id: p.owner_id.to_string(),
            // ... existing fields ...
            created_at: p.created_at.to_rfc3339(),
            appreciation_count: 0,
            comment_count: 0,
        }
    }
}
```

- [ ] **Step 3: Update the handler to fetch and populate counts**

Replace the body of `handler`:

```rust
pub async fn handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<PhotoDetail>, AppError> {
    let row = queries::find_by_id(&state.pool, id)
        .await?
        .ok_or(AppError::NotFound)?;

    let appreciation_count = sqlx::query!(
        r#"select count(*) as "count!" from appreciations where photo_id = $1"#,
        id
    )
    .fetch_one(&state.pool)
    .await?
    .count;

    let comment_count = sqlx::query!(
        r#"select count(*) as "count!" from comments where photo_id = $1"#,
        id
    )
    .fetch_one(&state.pool)
    .await?
    .count;

    let mut dto: PhotoDetail = row.into();
    dto.appreciation_count = appreciation_count;
    dto.comment_count = comment_count;
    Ok(Json(dto))
}
```

The `dto` must be `mut` and constructed via `From` then patched (the alternative is rebuilding all 18 fields by hand — verbose). Clippy may flag this as "unnecessary mut" if optimised; if so, add `#[allow(clippy::needless_pass_by_value)]` or restructure.

- [ ] **Step 4: Cache + check + tests**

```bash
cd $ROOT/backend
DATABASE_URL=postgres://astrophoto:astrophoto@localhost:5434/astrophoto cargo sqlx prepare -- --lib --tests --bins
cargo check
cargo test --test photos
```

Expected: existing photos integration test still passes (it doesn't assert the new fields are 0; it just unmarshals JSON, which tolerates extra fields).

- [ ] **Step 5: Commit**

```bash
cd $ROOT
git add backend/src/photos/get.rs backend/.sqlx/
git commit -m "feat(backend/photos): include appreciation_count + comment_count in detail"
```

---

## Task 7: Engagement integration tests

**Files:**
- Create: `backend/tests/engagement.rs`

- [ ] **Step 1: Write the test file**

```rust
//! Integration tests for the engagement layer (Phase 7):
//! appreciations, comments, follows.
//!
//! Uses an ephemeral Postgres via testcontainers and an in-memory
//! Storage so the upload pipeline works without MinIO.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use astrophoto::storage::MemoryStorage;
use astrophoto::{Config, db, http};
use axum::{
    body::Body,
    http::{Request, header},
};
use bytes::Bytes;
use http_body_util::BodyExt as _;
use image::{DynamicImage, ImageFormat, RgbImage};
use std::io::Cursor;
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
        public_base_url: "http://localhost:8080".into(),
        s3_endpoint: None,
        s3_region: "us-east-1".into(),
        s3_bucket: "x".into(),
        s3_access_key: "a".into(),
        s3_secret_key: "s".into(),
        s3_path_style: true,
        oauth_google_client_id: String::new(),
        oauth_google_client_secret: String::new(),
        oauth_google_redirect_url: String::new(),
    }
}

fn make_test_jpeg() -> Vec<u8> {
    let img = DynamicImage::ImageRgb8(RgbImage::from_fn(200, 150, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, 64])
    }));
    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, ImageFormat::Jpeg).unwrap();
    buf.into_inner()
}

async fn signup(
    app: &axum::Router,
    email: &str,
    name: &str,
) -> (String /* user_id */, String /* cookie */) {
    let body = serde_json::json!({
        "email": email, "password": "longenoughpw", "display_name": name,
    });
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/signup")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let cookie = resp
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let id = v["id"].as_str().unwrap().to_string();
    (id, cookie)
}

async fn upload(app: &axum::Router, cookie: &str) -> String {
    let boundary = "----testboundary";
    let jpeg = make_test_jpeg();
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(b"Content-Disposition: form-data; name=\"file\"; filename=\"t.jpg\"\r\n");
    body.extend_from_slice(b"Content-Type: image/jpeg\r\n\r\n");
    body.extend_from_slice(&jpeg);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/photos")
                .header(header::COOKIE, cookie)
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 202);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    v["id"].as_str().unwrap().to_string()
}

async fn boot_app() -> (axum::Router, sqlx::PgPool, PgImage) {
    let pg = PgImage::default().start().await.unwrap();
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let storage = Arc::new(MemoryStorage::new());
    let app = http::router(pool.clone(), config_for(&url), storage);
    (app, pool, pg)
}

async fn json_get(app: &axum::Router, uri: &str, cookie: Option<&str>) -> serde_json::Value {
    let mut req = Request::builder().uri(uri);
    if let Some(c) = cookie {
        req = req.header(header::COOKIE, c);
    }
    let resp = app
        .clone()
        .oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    if !status.is_success() {
        panic!("GET {uri} failed: {status} {:?}", String::from_utf8_lossy(&bytes));
    }
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn appreciation_toggle() {
    let (app, _pool, _pg) = boot_app().await;

    let (_owner_id, owner_cookie) = signup(&app, "owner@example.com", "Owner").await;
    let photo_id = upload(&app, &owner_cookie).await;
    // Wait for the background pipeline to finish.
    for _ in 0..30 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let v = json_get(&app, &format!("/api/photos/{photo_id}"), None).await;
        if v["status"] == "ready" {
            break;
        }
    }

    let (_other_id, cookie) = signup(&app, "u@example.com", "U").await;

    // Initial count is 0
    let v = json_get(&app, &format!("/api/photos/{photo_id}/appreciations/count"), None).await;
    assert_eq!(v["count"].as_i64().unwrap(), 0);

    // POST appreciate twice (idempotent)
    for _ in 0..2 {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/photos/{photo_id}/appreciate"))
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 204);
    }

    let v = json_get(&app, &format!("/api/photos/{photo_id}/appreciations/count"), None).await;
    assert_eq!(v["count"].as_i64().unwrap(), 1);

    let v = json_get(
        &app,
        &format!("/api/photos/{photo_id}/appreciation-state"),
        Some(&cookie),
    )
    .await;
    assert_eq!(v["appreciated"].as_bool().unwrap(), true);

    // DELETE twice (idempotent)
    for _ in 0..2 {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/photos/{photo_id}/appreciate"))
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 204);
    }

    let v = json_get(&app, &format!("/api/photos/{photo_id}/appreciations/count"), None).await;
    assert_eq!(v["count"].as_i64().unwrap(), 0);
}

#[tokio::test]
async fn comment_create_list_delete_authorization() {
    let (app, _pool, _pg) = boot_app().await;

    let (owner_id, owner_cookie) = signup(&app, "owner@example.com", "Owner").await;
    let photo_id = upload(&app, &owner_cookie).await;
    for _ in 0..30 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let v = json_get(&app, &format!("/api/photos/{photo_id}"), None).await;
        if v["status"] == "ready" {
            break;
        }
    }
    let _ = owner_id;

    let (_b_id, b_cookie) = signup(&app, "b@example.com", "B").await;
    let (_c_id, c_cookie) = signup(&app, "c@example.com", "C").await;

    // B posts a comment
    let body = serde_json::json!({ "body": "Looks great!" });
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/photos/{photo_id}/comments"))
                .header(header::COOKIE, &b_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let comment_id = v["id"].as_str().unwrap().to_string();

    // List
    let v = json_get(&app, &format!("/api/photos/{photo_id}/comments"), None).await;
    assert_eq!(v["comments"].as_array().unwrap().len(), 1);

    // C tries to delete B's comment — 403
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/comments/{comment_id}"))
                .header(header::COOKIE, &c_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);

    // Owner (the photo owner) deletes B's comment — 204
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/comments/{comment_id}"))
                .header(header::COOKIE, &owner_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    let v = json_get(&app, &format!("/api/photos/{photo_id}/comments"), None).await;
    assert_eq!(v["comments"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn follow_toggle_and_counts() {
    let (app, _pool, _pg) = boot_app().await;

    let (a_id, a_cookie) = signup(&app, "a@example.com", "A").await;
    let (b_id, _b_cookie) = signup(&app, "b@example.com", "B").await;

    // Initial: A's following count = 0, B's followers = 0
    let v = json_get(&app, &format!("/api/users/{a_id}/following/count"), None).await;
    assert_eq!(v["count"].as_i64().unwrap(), 0);
    let v = json_get(&app, &format!("/api/users/{b_id}/followers/count"), None).await;
    assert_eq!(v["count"].as_i64().unwrap(), 0);

    // A follows B (twice — idempotent)
    for _ in 0..2 {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/users/{b_id}/follow"))
                    .header(header::COOKIE, &a_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), 204);
    }

    // Counts updated
    let v = json_get(&app, &format!("/api/users/{a_id}/following/count"), None).await;
    assert_eq!(v["count"].as_i64().unwrap(), 1);
    let v = json_get(&app, &format!("/api/users/{b_id}/followers/count"), None).await;
    assert_eq!(v["count"].as_i64().unwrap(), 1);

    // /me returns the followed_id
    let v = json_get(&app, "/api/auth/me", Some(&a_cookie)).await;
    let following = v["following_ids"].as_array().unwrap();
    assert_eq!(following.len(), 1);
    assert_eq!(following[0].as_str().unwrap(), b_id);

    // Self-follow is rejected (422)
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/users/{a_id}/follow"))
                .header(header::COOKIE, &a_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 422);

    // Unfollow
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/users/{b_id}/follow"))
                .header(header::COOKIE, &a_cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    let v = json_get(&app, &format!("/api/users/{b_id}/followers/count"), None).await;
    assert_eq!(v["count"].as_i64().unwrap(), 0);
}
```

- [ ] **Step 2: Run the tests**

```bash
cd $ROOT/backend && cargo test --test engagement -- --nocapture
```

Expected: 3 passing tests, ~30s total (testcontainers startup).

- [ ] **Step 3: Commit**

```bash
cd $ROOT
git add backend/tests/engagement.rs
git commit -m "test(backend/engagement): integration tests for appreciations, comments, follows"
```

---

## Task 8: Following feed branch on `/`

**Files:**
- Modify: `backend/src/photos/list.rs` (add `following=true` query param branch)
- Modify: `backend/src/photos/queries.rs` (add `list_following`)
- Modify: `frontend/src/routes/+page.server.ts` (call following first when authed)
- Modify: `frontend/src/lib/api/client.ts` (add `following` flag to `photos.list`)

- [ ] **Step 1: Add `list_following` query**

In `backend/src/photos/queries.rs`, append:

```rust
pub async fn list_following(
    pool: &PgPool,
    follower_id: Uuid,
    limit: i64,
) -> Result<Vec<PhotoRow>, AppError> {
    let rows = sqlx::query_as!(
        PhotoRow,
        r#"
        select p.id, p.owner_id, p.storage_key, p.original_name, p.bytes, p.mime,
               p.width, p.height, p.taken_at, p.camera, p.lens, p.iso,
               p.exposure_s, p.focal_mm, p.target, p.caption, p.status, p.created_at
        from photos p
        join follows f on f.followed_id = p.owner_id
        where f.follower_id = $1 and p.status = 'ready'
        order by p.created_at desc
        limit $2
        "#,
        follower_id,
        limit
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
```

- [ ] **Step 2: Wire `following=true` into the list handler**

In `backend/src/photos/list.rs`, find `ListQuery`. Add:

```rust
#[derive(Deserialize)]
pub struct ListQuery {
    pub owner_id: Option<Uuid>,
    pub limit: Option<i64>,
    pub following: Option<bool>,
}
```

Then in `handler`, before the existing branch:

```rust
pub async fn handler(
    State(state): State<AppState>,
    user: crate::auth::middleware::OptionalUser,
    Query(q): Query<ListQuery>,
) -> Result<Json<ListResponse>, AppError> {
    let limit = q.limit.unwrap_or(50).clamp(1, 200);

    let rows = if q.following.unwrap_or(false) {
        let follower = user.0.ok_or(AppError::Unauthorized)?;
        queries::list_following(&state.pool, follower.id, limit).await?
    } else if let Some(id) = q.owner_id {
        queries::list_by_owner(&state.pool, id, limit).await?
    } else {
        queries::list_recent_public(&state.pool, limit).await?
    };

    Ok(Json(ListResponse {
        photos: rows.into_iter().map(Into::into).collect(),
    }))
}
```

`OptionalUser` is the extractor that returns `OptionalUser(Option<UserRow>)`.

- [ ] **Step 3: Refresh sqlx + check + tests**

```bash
cd $ROOT/backend
DATABASE_URL=postgres://astrophoto:astrophoto@localhost:5434/astrophoto cargo sqlx prepare -- --lib --tests --bins
cargo check
cargo test
```

- [ ] **Step 4: Update frontend `client.ts` to support the flag**

Open `frontend/src/lib/api/client.ts`. Find the `photos` namespace. Update `list`:

```ts
photos: {
  list: (
    opts: { ownerId?: string; limit?: number; following?: boolean } = {},
    apiOpts?: ApiCall
  ) => {
    const qs = new URLSearchParams();
    if (opts.ownerId) qs.set('owner_id', opts.ownerId);
    if (opts.limit != null) qs.set('limit', String(opts.limit));
    if (opts.following) qs.set('following', 'true');
    const path = qs.toString() ? `/api/photos?${qs}` : '/api/photos';
    return request<PhotoListResponse>('GET', path, undefined, apiOpts);
  },
  // ... rest unchanged
}
```

- [ ] **Step 5: Update gallery loader for the following branch**

In `frontend/src/routes/+page.server.ts`, before the existing public-list call:

```ts
export const load: PageServerLoad = async ({ fetch, locals }) => {
  let realPhotos: PhotoSummary[] = [];

  // 1. Authenticated user with follows: try the personalised feed first.
  if (locals.user) {
    try {
      const r = await api.photos.list({ following: true }, { fetch });
      realPhotos = r.photos;
    } catch {
      // ignore — fall through to public list
    }
  }

  // 2. Anonymous, OR auth user follows nobody, OR follows-with-no-photos:
  //    show the public feed.
  if (realPhotos.length === 0) {
    try {
      const r = await api.photos.list({ limit: 24 }, { fetch });
      realPhotos = r.photos;
    } catch {
      // backend down — fall back to placeholder demo content
    }
  }

  // ... rest of the existing function unchanged
};
```

The `realPhotos.length === 0` branch already exists; you're moving the
public-list call into it conditionally.

- [ ] **Step 6: Smoke test**

Backend running. Frontend running. Open:
1. `http://localhost:5173/` while signed out → public gallery (existing behavior).
2. Sign in as a user who follows nobody → same public gallery.
3. From that account, follow another user via curl:
   ```bash
   USER_B_ID=$(curl -s 'http://localhost:8080/api/photos?limit=1' \
     | python3 -c 'import sys,json; print(json.load(sys.stdin)["photos"][0]["owner_id"])')
   curl -s -b /tmp/cA.txt -X POST "http://localhost:8080/api/users/$USER_B_ID/follow"
   ```
   Refresh `/` → now shows only the followed user's photos.
4. Unfollow → `/` returns to public gallery.

(This step depends on Tasks 9-10 to expose the Follow button in UI — for now use curl.)

- [ ] **Step 7: Commit**

```bash
cd $ROOT
git add backend/src/photos/list.rs \
        backend/src/photos/queries.rs \
        backend/.sqlx/ \
        frontend/src/lib/api/client.ts \
        frontend/src/routes/+page.server.ts
git commit -m "feat(frontend): following feed on / with public fallback"
```

---

## Task 9: AppreciateButton + FollowButton frontend components

**Files:**
- Create: `frontend/src/lib/components/AppreciateButton.svelte`
- Create: `frontend/src/lib/components/FollowButton.svelte`
- Modify: `frontend/src/lib/api/client.ts` (add appreciate/follow methods)

- [ ] **Step 1: Extend `client.ts` with appreciate + follow methods**

```ts
// Append to the api object:
appreciations: {
  count: (photoId: string, opts?: ApiCall) =>
    request<{ count: number }>('GET', `/api/photos/${photoId}/appreciations/count`, undefined, opts),
  state: (photoId: string, opts?: ApiCall) =>
    request<{ appreciated: boolean }>('GET', `/api/photos/${photoId}/appreciation-state`, undefined, opts),
  appreciate: (photoId: string, opts?: ApiCall) =>
    request<void>('POST', `/api/photos/${photoId}/appreciate`, undefined, opts),
  unappreciate: (photoId: string, opts?: ApiCall) =>
    request<void>('DELETE', `/api/photos/${photoId}/appreciate`, undefined, opts)
},
follows: {
  follow: (userId: string, opts?: ApiCall) =>
    request<void>('POST', `/api/users/${userId}/follow`, undefined, opts),
  unfollow: (userId: string, opts?: ApiCall) =>
    request<void>('DELETE', `/api/users/${userId}/follow`, undefined, opts),
  followersCount: (userId: string, opts?: ApiCall) =>
    request<{ count: number }>('GET', `/api/users/${userId}/followers/count`, undefined, opts),
  followingCount: (userId: string, opts?: ApiCall) =>
    request<{ count: number }>('GET', `/api/users/${userId}/following/count`, undefined, opts)
}
```

- [ ] **Step 2: Create `AppreciateButton.svelte`**

```svelte
<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { api } from '$lib/api/client';

  interface Props {
    photoId: string;
    initialCount: number;
    initialAppreciated?: boolean;
  }

  let { photoId, initialCount, initialAppreciated = false }: Props = $props();

  let count = $state(initialCount);
  let appreciated = $state(initialAppreciated);
  let pending = $state(false);

  async function toggle() {
    if (!page.data.user) {
      await goto(`/signin?return=${encodeURIComponent(page.url.pathname)}`);
      return;
    }
    if (pending) return;
    pending = true;

    const wasOn = appreciated;
    appreciated = !appreciated;
    count += appreciated ? 1 : -1;

    try {
      if (wasOn) await api.appreciations.unappreciate(photoId);
      else await api.appreciations.appreciate(photoId);
    } catch {
      // rollback
      appreciated = wasOn;
      count += wasOn ? 1 : -1;
    } finally {
      pending = false;
    }
  }
</script>

<button
  type="button"
  class="appreciate {appreciated ? 'on' : ''}"
  aria-pressed={appreciated}
  disabled={pending}
  onclick={toggle}
>
  {#if count === 0 && !appreciated}
    ♡ Appreciate
  {:else}
    ♡ {count}
  {/if}
</button>

<style>
  .appreciate {
    background: transparent;
    color: var(--fg-secondary);
    border: 1px solid var(--border-strong);
    padding: 0 12px;
    height: 28px;
    border-radius: 2px;
    font-family: var(--font-ui);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: all 150ms ease;
  }
  .appreciate.on {
    color: var(--accent);
    border-color: var(--accent);
    background: rgba(232, 164, 58, 0.06);
  }
  .appreciate:disabled {
    opacity: 0.6;
    cursor: progress;
  }
  .appreciate:hover:not(:disabled) {
    border-color: var(--accent);
  }
</style>
```

- [ ] **Step 3: Create `FollowButton.svelte`**

```svelte
<script lang="ts">
  import { goto, invalidateAll } from '$app/navigation';
  import { page } from '$app/state';
  import { api } from '$lib/api/client';

  interface Props {
    userId: string;
    initialFollowing: boolean;
  }

  let { userId, initialFollowing }: Props = $props();

  let following = $state(initialFollowing);
  let pending = $state(false);
  let hovering = $state(false);

  async function toggle() {
    if (!page.data.user) {
      await goto(`/signin?return=${encodeURIComponent(page.url.pathname)}`);
      return;
    }
    if (pending) return;
    pending = true;

    const wasOn = following;
    following = !following;

    try {
      if (wasOn) await api.follows.unfollow(userId);
      else await api.follows.follow(userId);
      // Refresh layout data so /me's following_ids updates and any
      // dependent view (like the gallery feed) recomputes.
      await invalidateAll();
    } catch {
      following = wasOn;
    } finally {
      pending = false;
      hovering = false;
    }
  }

  let label = $derived(
    following ? (hovering ? 'Unfollow' : '✓ Following') : 'Follow'
  );
</script>

<button
  type="button"
  class="follow {following ? 'on' : ''} {following && hovering ? 'hover-off' : ''}"
  disabled={pending}
  onclick={toggle}
  onmouseenter={() => (hovering = true)}
  onmouseleave={() => (hovering = false)}
>
  {label}
</button>

<style>
  .follow {
    background: var(--accent);
    color: var(--accent-ink);
    border: 1px solid var(--accent);
    padding: 0 16px;
    height: 36px;
    border-radius: 2px;
    font-family: var(--font-ui);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition: all 150ms ease;
  }
  .follow.on {
    background: transparent;
    color: var(--accent);
  }
  .follow.on.hover-off {
    color: var(--danger);
    border-color: var(--danger);
  }
  .follow:disabled {
    opacity: 0.6;
    cursor: progress;
  }
</style>
```

- [ ] **Step 4: Wire AppreciateButton into the photo detail page**

Open `frontend/src/routes/photo/[slug]/+page.svelte`. Find the action row that currently renders something like `<button>♡ {appreciations} appreciations</button>`. Replace with:

```svelte
<script lang="ts">
  // ... existing imports ...
  import AppreciateButton from '$lib/components/AppreciateButton.svelte';
  // ...
</script>

<!-- In the action row -->
<AppreciateButton
  photoId={data.photo.slug}
  initialCount={data.photo.appreciations}
  initialAppreciated={data.photo.isAppreciated ?? false}
/>
```

In `frontend/src/routes/photo/[slug]/+page.server.ts`, in the UUID branch, fetch `appreciation-state` if logged in:

```ts
let isAppreciated = false;
if (locals.user) {
  try {
    const cookie = request_cookie_string({ /* ... */ });
    // Reuse the existing API call pattern
    const stateRes = await fetch(`${API}/api/photos/${params.slug}/appreciation-state`, {
      headers: { Cookie: cookie }
    });
    if (stateRes.ok) {
      const state = (await stateRes.json()) as { appreciated: boolean };
      isAppreciated = state.appreciated;
    }
  } catch {
    // ignore
  }
}
```

The exact cookie-forwarding code follows the same pattern as Phase 5
already established in this file. Inline `request.headers.get('cookie')`
is fine.

In the same return shape, add:
- `appreciations: photo.appreciation_count`
- `comments: photo.comment_count`
- `isAppreciated: isAppreciated`

(The placeholder NGC 7000 branch doesn't need these — its hardcoded
values stay.)

- [ ] **Step 5: Wire FollowButton into the profile page**

In `frontend/src/routes/u/[username]/+page.server.ts`, in the UUID branch, fetch initial following state from `locals.user.following_ids`:

```ts
const isFollowing = locals.user?.following_ids?.includes(params.username) ?? false;
const isSelf = locals.user?.id === params.username;

return {
  profile: { /* existing fields */ },
  photos,
  isReal: true as const,
  isFollowing,
  isSelf
};
```

(Note: `locals.user` currently has `{ id, displayName }` per Phase 4 hooks. To populate `following_ids` we'll have hooks.server.ts grab them from `/me`. See Step 6.)

In `frontend/src/routes/u/[username]/+page.svelte`, replace the placeholder `<button class="btn btn-primary">Follow</button>` with:

```svelte
<script lang="ts">
  // ...
  import FollowButton from '$lib/components/FollowButton.svelte';
</script>

{#if !data.isSelf}
  <FollowButton userId={data.profile.username} initialFollowing={data.isFollowing} />
{/if}
```

- [ ] **Step 6: Update `hooks.server.ts` to expose `following_ids` in locals**

Open `frontend/src/hooks.server.ts`. In the `if (cookie.includes(...))` block, after the `api.me(...)` call:

```ts
event.locals.user = {
  id: user.id,
  displayName: user.display_name,
  following_ids: user.following_ids ?? []
};
```

Open `frontend/src/app.d.ts` and update the Locals interface:

```ts
namespace App {
  interface Locals {
    user: {
      id: string;
      displayName: string;
      following_ids: string[];
    } | null;
  }
}
```

(`PageData.user` may need the same change — adapt as `pnpm check` complains.)

- [ ] **Step 7: Verify**

```bash
cd $ROOT
pnpm -C frontend check
pnpm -C frontend lint
pnpm -C frontend build
just check
```

All clean.

- [ ] **Step 8: Commit**

```bash
git add frontend/src/lib/components/AppreciateButton.svelte \
        frontend/src/lib/components/FollowButton.svelte \
        frontend/src/lib/api/client.ts \
        frontend/src/routes/photo/[slug]/+page.server.ts \
        frontend/src/routes/photo/[slug]/+page.svelte \
        frontend/src/routes/u/[username]/+page.server.ts \
        frontend/src/routes/u/[username]/+page.svelte \
        frontend/src/hooks.server.ts \
        frontend/src/app.d.ts
git commit -m "feat(frontend): AppreciateButton + FollowButton wired with optimistic updates"
```

---

## Task 10: CommentsSection component

**Files:**
- Create: `frontend/src/lib/components/CommentsSection.svelte`
- Modify: `frontend/src/routes/photo/[slug]/+page.svelte` (add the section)
- Modify: `frontend/src/routes/photo/[slug]/+page.server.ts` (load comments + add form actions)
- Modify: `frontend/src/lib/api/client.ts` (add comments methods)

- [ ] **Step 1: Extend `client.ts` with comments methods**

```ts
comments: {
  list: (photoId: string, opts?: ApiCall) =>
    request<{ comments: Comment[] }>('GET', `/api/photos/${photoId}/comments`, undefined, opts),
  create: (photoId: string, body: string, opts?: ApiCall) =>
    request<Comment>('POST', `/api/photos/${photoId}/comments`, { body }, opts),
  delete: (commentId: string, opts?: ApiCall) =>
    request<void>('DELETE', `/api/comments/${commentId}`, undefined, opts)
}
```

Add the type at the top of the file:

```ts
export interface Comment {
  id: string;
  photo_id: string;
  author_id: string;
  author_display_name: string;
  body: string;
  created_at: string;
}
```

- [ ] **Step 2: Create `CommentsSection.svelte`**

```svelte
<script lang="ts">
  import { invalidateAll } from '$app/navigation';
  import type { Comment } from '$lib/api/client';

  interface Props {
    photoId: string;
    photoOwnerId: string;
    comments: Comment[];
    currentUser: { id: string; displayName: string } | null;
  }

  let { photoId, photoOwnerId, comments, currentUser }: Props = $props();

  let body = $state('');
  let posting = $state(false);
  let error = $state<string | null>(null);

  async function postComment() {
    if (posting || body.trim().length === 0) return;
    posting = true;
    error = null;
    try {
      const form = new FormData();
      form.append('body', body);
      const res = await fetch(`?/comment`, {
        method: 'POST',
        body: form
      });
      if (!res.ok) {
        error = 'Failed to post comment.';
        return;
      }
      body = '';
      await invalidateAll();
    } finally {
      posting = false;
    }
  }

  async function deleteComment(commentId: string) {
    if (!confirm('Delete this comment?')) return;
    const form = new FormData();
    form.append('id', commentId);
    const res = await fetch(`?/deleteComment`, {
      method: 'POST',
      body: form
    });
    if (res.ok) {
      await invalidateAll();
    }
  }

  function timeAgo(iso: string): string {
    const d = new Date(iso);
    const seconds = (Date.now() - d.getTime()) / 1000;
    if (seconds < 60) return 'just now';
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
    if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
    if (seconds < 7 * 86400) return `${Math.floor(seconds / 86400)}d ago`;
    return d.toISOString().slice(0, 10);
  }

  function canDelete(c: Comment): boolean {
    if (!currentUser) return false;
    return c.author_id === currentUser.id || photoOwnerId === currentUser.id;
  }
</script>

<section class="comments">
  <div class="t-eyebrow comments-header">
    COMMENTS · {comments.length}
  </div>

  {#if comments.length === 0}
    <p class="empty">No comments yet.</p>
  {/if}

  {#each comments as c (c.id)}
    <div class="comment">
      <div class="meta">
        <span class="author">
          {c.author_display_name}{#if currentUser && c.author_id === currentUser.id}
            <span class="you"> · You</span>
          {/if}
        </span>
        <span class="time">{timeAgo(c.created_at)}</span>
      </div>
      <p class="body">{c.body}</p>
      {#if canDelete(c)}
        <button type="button" class="delete" onclick={() => deleteComment(c.id)}>
          Delete
        </button>
      {/if}
    </div>
  {/each}

  {#if currentUser}
    <form
      method="POST"
      class="composer"
      onsubmit={(e) => {
        e.preventDefault();
        postComment();
      }}
    >
      <textarea
        bind:value={body}
        placeholder="Add a comment..."
        rows="3"
        maxlength="2000"
      ></textarea>
      {#if error}
        <p class="error">{error}</p>
      {/if}
      <button type="submit" class="post" disabled={posting || body.trim().length === 0}>
        {posting ? 'Posting...' : 'Post'}
      </button>
    </form>
  {:else}
    <p class="signin-prompt">
      <a href="/signin">Sign in</a> to comment.
    </p>
  {/if}
</section>

<style>
  .comments {
    padding: 24px 32px 32px;
    border-top: 1px solid var(--border-default);
  }
  .comments-header {
    color: var(--fg-primary);
    letter-spacing: 0.16em;
    margin-bottom: 16px;
  }
  .empty {
    color: var(--fg-muted);
    font-size: 13px;
    padding: 8px 0;
  }
  .comment {
    padding: 12px 0;
    border-bottom: 1px dashed var(--border-subtle);
  }
  .comment:last-child {
    border-bottom: 0;
  }
  .meta {
    display: flex;
    justify-content: space-between;
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--fg-muted);
    margin-bottom: 6px;
  }
  .author {
    color: var(--fg-primary);
  }
  .you {
    color: var(--accent);
  }
  .body {
    margin: 0;
    font-size: 14px;
    line-height: 1.55;
    color: var(--fg-secondary);
    white-space: pre-wrap;
    word-wrap: break-word;
  }
  .delete {
    margin-top: 6px;
    background: transparent;
    border: 0;
    padding: 0;
    color: var(--danger);
    font-family: var(--font-mono);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    cursor: pointer;
  }
  .delete:hover {
    text-decoration: underline;
  }
  .composer {
    margin-top: 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  textarea {
    width: 100%;
    background: var(--bg-base);
    color: var(--fg-primary);
    border: 1px solid var(--border-default);
    border-radius: 2px;
    padding: 8px 12px;
    font-family: var(--font-ui);
    font-size: 14px;
    line-height: 1.55;
    resize: vertical;
  }
  textarea:focus {
    outline: 0;
    border-color: var(--accent);
    box-shadow: 0 0 0 3px rgba(232, 164, 58, 0.12);
  }
  .post {
    align-self: flex-end;
    background: var(--accent);
    color: var(--accent-ink);
    border: 0;
    padding: 0 16px;
    height: 32px;
    border-radius: 2px;
    font-family: var(--font-ui);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .post:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
  .error {
    color: var(--danger);
    font-size: 12px;
    margin: 0;
  }
  .signin-prompt {
    margin-top: 16px;
    color: var(--fg-muted);
    font-size: 13px;
  }
  .signin-prompt a {
    color: var(--accent);
  }
</style>
```

- [ ] **Step 3: Add form actions in `+page.server.ts`**

In `frontend/src/routes/photo/[slug]/+page.server.ts`, add at the bottom (after the `load` function):

```ts
import { fail } from '@sveltejs/kit';
import type { Actions } from './$types';

export const actions: Actions = {
  comment: async ({ request, params, fetch, cookies }) => {
    const data = await request.formData();
    const body = String(data.get('body') ?? '').trim();
    if (body.length === 0 || body.length > 2000) {
      return fail(422, { message: 'Comment must be 1-2000 chars.' });
    }
    const cookie = cookies.getAll().map((c) => `${c.name}=${c.value}`).join('; ');
    const res = await fetch(`${API}/api/photos/${params.slug}/comments`, {
      method: 'POST',
      credentials: 'include',
      headers: { 'Content-Type': 'application/json', Cookie: cookie },
      body: JSON.stringify({ body })
    });
    if (!res.ok) {
      return fail(res.status, { message: `Failed: ${await res.text()}` });
    }
    return { ok: true };
  },

  deleteComment: async ({ request, fetch, cookies }) => {
    const data = await request.formData();
    const id = String(data.get('id') ?? '');
    if (!id) return fail(400, { message: 'Missing comment id.' });
    const cookie = cookies.getAll().map((c) => `${c.name}=${c.value}`).join('; ');
    const res = await fetch(`${API}/api/comments/${id}`, {
      method: 'DELETE',
      credentials: 'include',
      headers: { Cookie: cookie }
    });
    if (!res.ok) {
      return fail(res.status, { message: `Failed: ${await res.text()}` });
    }
    return { ok: true };
  }
};
```

In `load`, also fetch the comments list:

```ts
let comments: Comment[] = [];
try {
  const res = await fetch(`${API}/api/photos/${params.slug}/comments`);
  if (res.ok) {
    const body = (await res.json()) as { comments: Comment[] };
    comments = body.comments;
  }
} catch {
  // ignore
}

// In the return shape (UUID branch):
return {
  // ...
  comments,
  ownerId: photo.owner_id
};
```

- [ ] **Step 4: Render `<CommentsSection>` in `+page.svelte`**

```svelte
<script lang="ts">
  // ...
  import CommentsSection from '$lib/components/CommentsSection.svelte';
</script>

<!-- After the EXIF section, still inside the aside: -->
{#if data.comments != null}
  <CommentsSection
    photoId={data.photo.slug}
    photoOwnerId={data.ownerId}
    comments={data.comments}
    currentUser={data.user}
  />
{/if}
```

`data.user` is the layout-supplied auth user (per `app.d.ts`). The
canonical NGC 7000 placeholder branch doesn't supply `comments` so the
section is hidden there.

- [ ] **Step 5: Verify**

```bash
cd $ROOT
pnpm -C frontend check
pnpm -C frontend lint
pnpm -C frontend build
just check
```

All clean.

- [ ] **Step 6: Commit**

```bash
git add frontend/src/lib/components/CommentsSection.svelte \
        frontend/src/lib/api/client.ts \
        frontend/src/routes/photo/[slug]/+page.server.ts \
        frontend/src/routes/photo/[slug]/+page.svelte
git commit -m "feat(frontend): comments section on photo detail with form actions"
```

---

## Task 11: Profile follower count + final verification

**Files:**
- Modify: `frontend/src/routes/u/[username]/+page.server.ts` (fetch followers count)

- [ ] **Step 1: Fetch follower count in the UUID branch**

In the UUID branch of the load, after getting `photoCount`:

```ts
let followerCount = 0;
try {
  const res = await fetch(`${API}/api/users/${params.username}/followers/count`);
  if (res.ok) {
    const body = (await res.json()) as { count: number };
    followerCount = body.count;
  }
} catch {
  // ignore
}
```

Then in the user object:

```ts
const user: User = {
  // ...
  followers: followerCount,
  // ... rest unchanged
};
```

- [ ] **Step 2: Verify**

```bash
cd $ROOT && just check
```

- [ ] **Step 3: Commit**

```bash
git add frontend/src/routes/u/[username]/+page.server.ts
git commit -m "feat(frontend): profile shows real follower count"
```

---

## Task 12: Browser smoke test + merge

**Files:**
- None (verification + merge)

- [ ] **Step 1: Restart backend + frontend**

```bash
pkill -f "target/debug/astrophoto" 2>/dev/null
pkill -f "vite dev" 2>/dev/null
sleep 1

cd $ROOT/backend
APP_BIND=127.0.0.1:8080 \
APP_LOG=info \
APP_DATABASE_URL=postgres://astrophoto:astrophoto@localhost:5434/astrophoto \
APP_SESSION_DOMAIN=localhost APP_SESSION_SECURE=false \
APP_PUBLIC_BASE_URL=http://localhost:8080 \
APP_S3_ENDPOINT=http://localhost:9100 \
APP_S3_REGION=us-east-1 APP_S3_BUCKET=astrophoto \
APP_S3_ACCESS_KEY=minioadmin APP_S3_SECRET_KEY=minioadmin \
APP_S3_PATH_STYLE=true \
cargo run --bin astrophoto > /tmp/astrophoto-backend.log 2>&1 &

cd $ROOT/frontend
VITE_API_BASE_URL=http://localhost:8080 pnpm dev > /tmp/astrophoto-fe.log 2>&1 &
```

- [ ] **Step 2: Browser smoke checklist**

Open `http://localhost:5173/` and verify:

1. **As anonymous user**: gallery shows public photos.
2. **Sign up a new user "Alice"**: redirects to `/`, header shows avatar `A`.
3. **Click on a photo** → photo detail loads. The action row shows:
   - `♡ Appreciate` button (count 0).
   - Comments section with "No comments yet." and a textarea.
4. **Click `♡ Appreciate`**: button turns amber and reads `♡ 1`. Reload the page → still `♡ 1`.
5. **Type "Hello world" in the textarea, click Post**: comment appears, header shows "COMMENTS · 1".
6. **Click `Delete` on your own comment**: it disappears.
7. **Click on the photo's owner avatar (e.g. `D` for Demo Astrographer)** → land at `/u/<demo-uuid>`. Click `Follow` → label changes to `✓ Following`. Stat row shows `1 followers`.
8. **Refresh `/`**: gallery now shows ONLY photos uploaded by Demo. (The Following feed branch is active.)
9. **Go back to the profile, click `Following`**: hover should show `Unfollow`. Click → label back to `Follow`, follower count back to 0.
10. **Refresh `/`**: gallery returns to public feed.

If any step fails, stop and fix before merging.

- [ ] **Step 3: Final `just check` and `cargo test`**

```bash
cd $ROOT && just check
cd backend && cargo test
```

Expected: all green. Test count: 22 (15 unit + 4 integration: healthz, auth, photos, engagement).

Wait — engagement tests have 3 tests in 1 file. Total integration: 4 files × varying counts = healthz(1) + auth(1+1 ignored) + photos(1) + engagement(3) = 7 integration tests across 4 files. Unit count is similar to before. Total `cargo test` summary should show ~22 passed + 1 ignored.

- [ ] **Step 4: Merge to main + tag**

```bash
cd $ROOT
git checkout main
git merge --no-ff feat/phase-7-engagement -m "Merge feat/phase-7-engagement: appreciations + comments + follows + feed"
git tag -a v0.6.0-engagement -m "Astrophoto v0.6.0 — engagement layer"
git branch -d feat/phase-7-engagement
```

---

## Self-Review

**Spec coverage:**
- Migration 0002 → Task 1 ✓
- Appreciation endpoints (POST/DELETE/count/state) → Task 2 ✓
- Comment endpoints (list/create/delete with auth) → Task 3 ✓
- Follow endpoints (POST/DELETE/counts) → Task 4 ✓
- `/me` → following_ids extension → Task 5 ✓
- `/api/photos/:id` → counts extension → Task 6 ✓
- Integration tests (3 tests) → Task 7 ✓
- Following feed branch on `/` → Task 8 ✓
- AppreciateButton + FollowButton frontend → Task 9 ✓
- CommentsSection frontend → Task 10 ✓
- Profile follower count → Task 11 ✓
- Browser smoke + merge → Task 12 ✓

**Placeholder scan:** the only "fill in details" feel is in Task 9 Step 4 where I say "the exact cookie-forwarding code follows the same pattern as Phase 5 already established in this file" — that's referencing existing code in the file; the engineer is told to follow the established pattern. Acceptable. No literal "TBD" or unfilled blocks.

**Type consistency:**
- `User` DTO has `following_ids: Vec<String>` (Task 5). Frontend `Locals.user` has `following_ids: string[]` (Task 9, Step 6). Names match.
- `Comment` DTO has `id, photo_id, author_id, author_display_name, body, created_at` (Task 3). Frontend interface mirrors (Task 10, Step 1). Match.
- `PhotoDetail` extension `appreciation_count`, `comment_count` (Task 6). Frontend reads as `data.photo.appreciations`, `data.photo.comments` — wait, that's a rename, not the raw field. The load function maps `photo.appreciation_count → appreciations` and `photo.comment_count → comments`. Document this in Task 9 Step 4 explicitly.

Let me fix that ambiguity inline in Task 9 Step 4. Actually the renaming hint is already there (`appreciations: photo.appreciation_count`). Good.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-05-02-phase-7-engagement.md`. Two execution options:

**1. Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration.

**2. Inline Execution** — I execute tasks in this session using executing-plans, batch execution with checkpoints for review.

**Which approach?**
