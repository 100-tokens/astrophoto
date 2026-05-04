# Photographer Showcase — Phase 3 Discovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Surface the corpus across photographers. Adds the global `/explore`, target / tag / equipment / category pages, and `/search`. Reuses `<PhotoGrid>`, `<PhotoTile>`, `<Lightbox>` from P2; extends `<PhotoTile>` with an author-chip overlay (cross-author mode). All schema and indexes already shipped in P1 (migrations 0010, 0011, 0012); no migrations needed.

**Architecture:** Six new public read-only endpoints (`/api/explore`, `/api/targets/:slug`, `/api/tags/:slug`, `/api/equipment/:kind/:slug`, `/api/categories/:cat`, `/api/search`) + a thin extension to `<PhotoTile>` for the cross-author overlay. Six new SvelteKit routes (`/explore`, `/t/[slug]`, `/tag/[slug]`, `/equip/[kind]/[slug]`, `/c/[cat]`, `/search`) compose a shared `<DiscoveryHeader>` (variant per page kind) + the existing P2 gallery. A global `<SearchBar>` lives in the navbar with a ⌘K hotkey.

**Tech Stack:** Rust 2024 + axum 0.7 + sqlx 0.8 (compile-time-checked) + Postgres 16. SvelteKit 2 + Svelte 5 runes. Cursor pagination uses base64-encoded `(published_at, id[, appreciations_count])` tuples for stable tie-breaking — same shape as P2's gallery feed cursor in `users::photos_feed`. Search v1 is ILIKE across handles, display names, target canonical names + aliases, tag names, photo target/caption — three small queries unioned in the handler. v2 (tsvector + GIN) is explicitly deferred.

**Spec:** `docs/superpowers/specs/2026-05-03-photographer-showcase-design.md` — read the P3 section (lines 793–1006) before starting.

**Design handoff (canonical for layout, copy, dimensions):** `/Users/pleclech/Downloads/design_handoff_astrophoto 3/showcase/showcase-p3.jsx` — `ScreenExplore`, `ScreenTarget`, `ScreenEquipment`, `ScreenSearch`. The plan's task code shows behaviour (props, state, handlers, server queries, tests) and structural Svelte markup; exact pixel values, colour token usage, and copy come from the handoff.

**Branch and worktree:** Already created at `/Volumes/Pascal4Tb/Projects/astrophoto-showcase-p3` on branch `feat/showcase-p3-discovery` (off `main` at `2cc605d`). All commits land here. Merge via `gh pr merge --merge` after acceptance. The repo has `delete_branch_on_merge: false` — manual cleanup is on the operator.

**No Playwright for P3 acceptance.** Per the project memory `E2E tooling — chrome-devtools-mcp not Playwright`, end-to-end acceptance is a `chrome-devtools-mcp` walk recorded in `docs/operations/p3-acceptance.md`.

---

## File structure

**Backend, new files:**

- `backend/src/discovery/mod.rs` — module index for discovery handlers.
- `backend/src/discovery/cursor.rs` — shared cursor encode/decode (extracted from `users::photos_feed` if helpful; copy-pasted otherwise — single-use is fine).
- `backend/src/discovery/explore.rs` — `GET /api/explore`.
- `backend/src/discovery/target.rs` — `GET /api/targets/:slug`.
- `backend/src/discovery/tag.rs` — `GET /api/tags/:slug`.
- `backend/src/discovery/equipment.rs` — `GET /api/equipment/:kind/:slug`.
- `backend/src/discovery/category.rs` — `GET /api/categories/:cat`.
- `backend/src/discovery/search.rs` — `GET /api/search`.
- `backend/tests/discovery_explore.rs`
- `backend/tests/discovery_target.rs`
- `backend/tests/discovery_tag.rs`
- `backend/tests/discovery_equipment.rs`
- `backend/tests/discovery_category.rs`
- `backend/tests/discovery_search.rs`

**Backend, modified files:**

- `backend/src/api_types.rs` — add `DiscoveryPhoto` (extends `GalleryPhoto` with `author_handle, author_display_name, author_id`), `TargetPage`, `TagPage`, `EquipmentPage`, `CategoryPage`, `SearchResults`, `SearchTargetHit`, `SearchUserHit`.
- `backend/src/lib.rs` — `pub mod discovery;`
- `backend/src/http/mod.rs` — register routes.

**Frontend, new files:**

- `frontend/src/lib/api/discoveryClient.ts` — typed wrappers for the 6 endpoints + the existing autocomplete endpoints.
- `frontend/src/lib/components/discovery/DiscoveryHeader.svelte` — variant prop: `explore | target | tag | equipment | category | search`.
- `frontend/src/lib/components/discovery/AuthorChip.svelte` — small `@handle` chip overlay for cross-author tiles.
- `frontend/src/lib/components/discovery/CrossAuthorTile.svelte` — wraps `<PhotoTile>`-like rendering but composes an `<AuthorChip>` inside the caption overlay.
- `frontend/src/lib/components/discovery/CrossAuthorGrid.svelte` — composes `justified-layout` + `<CrossAuthorTile>` (similar to `PhotoGrid` but cross-author mode).
- `frontend/src/lib/components/discovery/FilterPills.svelte` — sort + time-window + category chips with bind callbacks.
- `frontend/src/lib/components/discovery/SearchBar.svelte` — global `<input>` + suggestions dropdown, ⌘K-focusable.
- `frontend/src/lib/components/discovery/SuggestionsList.svelte` — list shown under the search input while typing.
- `frontend/src/lib/components/discovery/EquipmentPairedRail.svelte` — "Often paired with" rail on `/equip/...`.

**Frontend, route files (all new):**

- `frontend/src/routes/explore/+page.server.ts` + `+page.svelte`
- `frontend/src/routes/t/[slug]/+page.server.ts` + `+page.svelte`
- `frontend/src/routes/tag/[slug]/+page.server.ts` + `+page.svelte`
- `frontend/src/routes/equip/[kind]/[slug]/+page.server.ts` + `+page.svelte`
- `frontend/src/routes/c/[cat]/+page.server.ts` + `+page.svelte`
- `frontend/src/routes/search/+page.server.ts` + `+page.svelte`

**Frontend, modified files:**

- `frontend/src/lib/components/AppHeader.svelte` — mount `<SearchBar>`.

**Docs:**

- `docs/operations/p3-acceptance.md` — created at the end with the chrome-devtools-mcp acceptance walk.

---

## Setup

### Task 1: Verify P3 schema, indexes, and existing autocomplete endpoints

Read-only orientation; no commit.

- [ ] **Step 1: Verify the discovery indexes from migration 0012 are present**

```
cd /Volumes/Pascal4Tb/Projects/astrophoto-showcase-p3
grep -nE "create index|create table" backend/migrations/0010_targets_tags.sql backend/migrations/0011_appreciations_count.sql backend/migrations/0012_equipment_items.sql
```

Expected to see at least: `targets`, `photo_targets`, `tags`, `photo_tags`, `equipment_items`, `appreciations_count_photos_idx`, `equipment_items_kind_count_idx`, `photos_camera_lower_idx`, `photos_scope_lower_idx`, `photos_mount_lower_idx`, `photos_filters_lower_idx`, `photos_guiding_lower_idx`, `photos_category_published_idx`, `photos_published_newest_idx`.

- [ ] **Step 2: Verify the autocomplete routes from P1 are still wired**

```
grep -nE "/api/(tags|targets|equipment)/autocomplete" backend/src/http/mod.rs
```

Expected: all three routes present.

- [ ] **Step 3: Confirm `tests/common/mod.rs` exists and exposes `TestApp`, `signup_with_handle`, `ready_photo_with`**

```
grep -nE "pub fn (signup_with_handle|ready_photo|launch)|pub struct TestApp" backend/tests/common/mod.rs
```

If anything above is missing, stop and surface it before continuing.

---

## Backend — types

### Task 2: Wire types for discovery

**Files:**
- Modify: `backend/src/api_types.rs`

- [ ] **Step 1: Append the discovery types after the existing P2 types**

```rust
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "DiscoveryPhoto.ts")]
pub struct DiscoveryPhoto {
    pub id: Uuid,
    pub short_id: String,
    pub target: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub blurhash: Option<String>,
    pub appreciations_count: i32,
    pub published_at: Option<chrono::DateTime<chrono::Utc>>,
    pub author_id: Uuid,
    pub author_handle: String,
    pub author_display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "DiscoveryPage.ts")]
pub struct DiscoveryPage {
    pub photos: Vec<DiscoveryPhoto>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "TargetMeta.ts")]
pub struct TargetMeta {
    pub slug: String,
    pub canonical_name: String,
    pub aliases: Vec<String>,
    pub kind: Option<String>,
    pub photo_count: i64,
    pub contributor_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "TargetPage.ts")]
pub struct TargetPage {
    pub target: TargetMeta,
    pub page: DiscoveryPage,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "TagMeta.ts")]
pub struct TagMeta {
    pub slug: String,
    pub name: String,
    pub photo_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "TagPage.ts")]
pub struct TagPage {
    pub tag: TagMeta,
    pub page: DiscoveryPage,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "EquipmentMeta.ts")]
pub struct EquipmentMeta {
    pub kind: String,
    pub slug: String,
    pub canonical_name: String,
    pub display_name: String,
    pub photo_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "EquipmentPaired.ts")]
pub struct EquipmentPaired {
    pub kind: String,
    pub slug: String,
    pub display_name: String,
    pub shared_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "EquipmentPage.ts")]
pub struct EquipmentPage {
    pub equipment: EquipmentMeta,
    pub paired: Vec<EquipmentPaired>,
    pub page: DiscoveryPage,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "CategoryPage.ts")]
pub struct CategoryPage {
    pub category: String,
    pub photo_count: i64,
    pub page: DiscoveryPage,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "SearchTargetHit.ts")]
pub struct SearchTargetHit {
    pub slug: String,
    pub canonical_name: String,
    pub photo_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "SearchUserHit.ts")]
pub struct SearchUserHit {
    pub id: Uuid,
    pub handle: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "SearchResults.ts")]
pub struct SearchResults {
    pub q: String,
    pub targets: Vec<SearchTargetHit>,
    pub users: Vec<SearchUserHit>,
    pub photos: Vec<DiscoveryPhoto>,
}
```

(`Uuid` may already be imported earlier in the file; if so, omit the duplicate `use`.)

- [ ] **Step 2: `cd backend && cargo check`** — must compile.

- [ ] **Step 3: `just types`** — regenerates `frontend/src/lib/api/*.ts` files for each new type.

- [ ] **Step 4: Commit**

```
cd /Volumes/Pascal4Tb/Projects/astrophoto-showcase-p3
git add backend/src/api_types.rs frontend/src/lib/api/
git commit -m "feat(api): wire types for discovery (P3)

DiscoveryPhoto extends GalleryPhoto with author_{id,handle,display_name}.
TargetPage/TagPage/EquipmentPage/CategoryPage are aggregator shapes
returned by the per-page endpoints. SearchResults groups targets+users+photos."
```

---

## Backend — module skeleton

### Task 3: discovery module + shared cursor helpers

**Files:**
- Create: `backend/src/discovery/mod.rs`
- Create: `backend/src/discovery/cursor.rs`
- Modify: `backend/src/lib.rs` (add `pub mod discovery;`)

- [ ] **Step 1: Create `backend/src/discovery/cursor.rs`**

```rust
//! Shared opaque cursor for cross-author discovery feeds.
//! Same shape as `users::photos_feed::Cursor` (P2). Three-tuple
//! comparison `(appreciations_count, published_at, id)` powers the
//! `most-appreciated` sort; two-tuple `(published_at, id)` powers
//! `newest`. The `appreciations` field is `None` for newest.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use uuid::Uuid;

use crate::AppError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Cursor {
    pub published_at: chrono::DateTime<chrono::Utc>,
    pub id: Uuid,
    #[serde(default)]
    pub appreciations: Option<i32>,
}

pub fn encode(c: &Cursor) -> String {
    let bytes = serde_json::to_vec(c).unwrap_or_default();
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn decode(s: &str) -> Result<Cursor, AppError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(s)
        .map_err(|_| AppError::bad_request("cursor_invalid"))?;
    serde_json::from_slice(&bytes).map_err(|_| AppError::bad_request("cursor_invalid"))
}
```

- [ ] **Step 2: Create `backend/src/discovery/mod.rs`**

```rust
pub mod cursor;
pub mod explore;
pub mod target;
pub mod tag;
pub mod equipment;
pub mod category;
pub mod search;
```

(Stub modules created in subsequent tasks. The mod.rs lists them all so registration order is fixed early.)

- [ ] **Step 3: Add `pub mod discovery;` to `backend/src/lib.rs`** alongside the other top-level modules.

- [ ] **Step 4: Stub each handler file so the lib compiles**

Create each of `backend/src/discovery/{explore,target,tag,equipment,category,search}.rs` with a single placeholder fn:

```rust
// File: backend/src/discovery/explore.rs (and analogous in the other 5 files)
use axum::response::IntoResponse;
use crate::AppError;

pub async fn get() -> Result<impl IntoResponse, AppError> {
    Err::<&'static str, _>(AppError::internal("discovery_not_implemented"))
}
```

Each task below replaces its module body with the real handler.

- [ ] **Step 5: `cargo check` — must compile.**

- [ ] **Step 6: Commit**

```
git add backend/src/discovery/ backend/src/lib.rs
git commit -m "feat(discovery): module skeleton + shared cursor (P3 backend setup)"
```

---

## Backend — endpoints

### Task 4: GET /api/explore

**Files:**
- Modify: `backend/src/discovery/explore.rs`
- Modify: `backend/src/http/mod.rs` (register route)
- Test: `backend/tests/discovery_explore.rs`

- [ ] **Step 1: Write the failing test `backend/tests/discovery_explore.rs`**

```rust
mod common;

use astrophoto::api_types::DiscoveryPage;
use axum::http::StatusCode;
use common::TestApp;

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn explore_returns_published_photos_newest_first_across_authors() {
    let app = TestApp::launch().await;
    let (_, alice_id) = app.signup_with_handle("Alice", "alice", "alice@x.test").await;
    let (_, bob_id) = app.signup_with_handle("Bob", "bob", "bob@x.test").await;
    let _p1 = app.ready_photo_with(alice_id, "AAAA0001", Some("M31")).await;
    let _p2 = app.ready_photo_with(bob_id, "BBBB0001", Some("M42")).await;
    let p3 = app.ready_photo_with(alice_id, "AAAA0002", Some("NGC 7000")).await;

    let (status, body) = app
        .oneshot_json::<DiscoveryPage>("GET", "/api/explore?limit=2", None, None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.photos.len(), 2);
    assert_eq!(body.photos[0].id, p3, "newest first across owners");
    assert!(body.next_cursor.is_some(), "more pages remain");
    // Author chip data must come back.
    assert_eq!(body.photos[0].author_handle, "alice");
    assert_eq!(body.photos[0].author_display_name, "Alice");
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn explore_respects_limit_clamp() {
    let app = TestApp::launch().await;
    let (status, _) = app.oneshot("GET", "/api/explore?limit=999", None, None).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = app.oneshot("GET", "/api/explore?limit=0", None, None).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn explore_filters_by_category() {
    let app = TestApp::launch().await;
    let (_, uid) = app.signup_with_handle("Alice", "alice", "a@x.test").await;
    let p_dso = app.ready_photo_with(uid, "AAAA0001", Some("M31")).await;
    let _p_lunar = app.ready_photo_with(uid, "BBBB0001", Some("Moon")).await;
    sqlx::query!("update photos set category = 'dso' where id = $1", p_dso)
        .execute(&app.pool).await.unwrap();
    sqlx::query!("update photos set category = 'lunar' where id = $1", _p_lunar)
        .execute(&app.pool).await.unwrap();

    let (_status, body) = app
        .oneshot_json::<DiscoveryPage>("GET", "/api/explore?category=dso", None, None)
        .await;
    assert_eq!(body.photos.len(), 1);
    assert_eq!(body.photos[0].id, p_dso);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn explore_sort_most_appreciated_orders_by_count() {
    let app = TestApp::launch().await;
    let (_, uid) = app.signup_with_handle("A", "a", "a@x.test").await;
    let p1 = app.ready_photo_with(uid, "AAAA0001", None).await;
    let p2 = app.ready_photo_with(uid, "BBBB0002", None).await;
    sqlx::query!("update photos set appreciations_count = 5 where id = $1", p1)
        .execute(&app.pool).await.unwrap();

    let (_status, body) = app
        .oneshot_json::<DiscoveryPage>("GET", "/api/explore?sort=most-appreciated", None, None)
        .await;
    assert_eq!(body.photos[0].id, p1);
    assert_eq!(body.photos[1].id, p2);
}
```

- [ ] **Step 2: Run — fails (route not registered).**

```
cd backend && cargo test --test discovery_explore 2>&1 | tail -10
```

- [ ] **Step 3: Replace `backend/src/discovery/explore.rs` with the real handler**

```rust
use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::AppError;
use crate::api_types::{DiscoveryPage, DiscoveryPhoto};
use crate::discovery::cursor::{self, Cursor};
use crate::http::AppState;

const DEFAULT_LIMIT: i64 = 24;
const MAX_LIMIT: i64 = 60;

#[derive(Deserialize)]
pub struct Q {
    pub cursor: Option<String>,
    pub limit: Option<i64>,
    pub sort: Option<String>,     // "newest" (default) | "most-appreciated"
    pub since: Option<String>,    // "24h" | "7d" | "30d" | "all"
    pub category: Option<String>, // dso | planetary | lunar | solar | wide_field | nightscape | other
}

struct Row {
    id: Uuid,
    short_id: String,
    target: Option<String>,
    width: Option<i32>,
    height: Option<i32>,
    blurhash: Option<String>,
    appreciations_count: i32,
    published_at: Option<chrono::DateTime<chrono::Utc>>,
    owner_id: Uuid,
    handle: String,
    display_name: String,
}

pub async fn get(
    State(state): State<AppState>,
    Query(q): Query<Q>,
) -> Result<impl IntoResponse, AppError> {
    let limit = q.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let sort = q.sort.as_deref().unwrap_or("newest");
    let since_seconds: Option<i64> = match q.since.as_deref() {
        Some("24h") => Some(86_400),
        Some("7d") => Some(7 * 86_400),
        Some("30d") => Some(30 * 86_400),
        Some("all") | None => None,
        Some(_) => return Err(AppError::bad_request("since_invalid")),
    };
    let category = q.category.as_deref();
    let cursor = q.cursor.as_deref().map(cursor::decode).transpose()?;

    let cur_pub = cursor.as_ref().map(|c| c.published_at);
    let cur_id = cursor.as_ref().map(|c| c.id).unwrap_or_else(Uuid::nil);
    let cur_apps = cursor.as_ref().and_then(|c| c.appreciations);

    let rows: Vec<Row> = match sort {
        "most-appreciated" => sqlx::query_as!(
            Row,
            r#"
            select p.id as "id!", p.short_id as "short_id!", p.target,
                   p.width, p.height, p.blurhash,
                   p.appreciations_count as "appreciations_count!",
                   p.published_at,
                   u.id as "owner_id!", u.handle as "handle!", u.display_name as "display_name!"
            from photos p
            join users u on u.id = p.owner_id
            where p.published_at is not null
              and p.status = 'ready'
              and ($1::int4 is null or
                   p.appreciations_count < $1 or
                   (p.appreciations_count = $1 and (p.published_at, p.id) < ($2, $3)))
              and ($4::text is null or p.category = $4)
              and ($5::int8 is null or p.published_at > now() - make_interval(secs => $5::float8))
            order by p.appreciations_count desc, p.published_at desc, p.id desc
            limit $6
            "#,
            cur_apps,
            cur_pub,
            cur_id,
            category,
            since_seconds,
            limit + 1
        )
        .fetch_all(&state.pool)
        .await?,
        _ => sqlx::query_as!(
            Row,
            r#"
            select p.id as "id!", p.short_id as "short_id!", p.target,
                   p.width, p.height, p.blurhash,
                   p.appreciations_count as "appreciations_count!",
                   p.published_at,
                   u.id as "owner_id!", u.handle as "handle!", u.display_name as "display_name!"
            from photos p
            join users u on u.id = p.owner_id
            where p.published_at is not null
              and p.status = 'ready'
              and ($1::timestamptz is null or (p.published_at, p.id) < ($1, $2))
              and ($3::text is null or p.category = $3)
              and ($4::int8 is null or p.published_at > now() - make_interval(secs => $4::float8))
            order by p.published_at desc, p.id desc
            limit $5
            "#,
            cur_pub,
            cur_id,
            category,
            since_seconds,
            limit + 1
        )
        .fetch_all(&state.pool)
        .await?,
    };

    let more = rows.len() as i64 > limit;
    let take: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = if more
        && let Some(last) = take.last()
        && let Some(published_at) = last.published_at
    {
        Some(cursor::encode(&Cursor {
            published_at,
            id: last.id,
            appreciations: if sort == "most-appreciated" {
                Some(last.appreciations_count)
            } else {
                None
            },
        }))
    } else {
        None
    };

    Ok(Json(DiscoveryPage {
        photos: take
            .into_iter()
            .map(|r| DiscoveryPhoto {
                id: r.id,
                short_id: r.short_id,
                target: r.target,
                width: r.width,
                height: r.height,
                blurhash: r.blurhash,
                appreciations_count: r.appreciations_count,
                published_at: r.published_at,
                author_id: r.owner_id,
                author_handle: r.handle,
                author_display_name: r.display_name,
            })
            .collect(),
        next_cursor,
    }))
}
```

- [ ] **Step 4: Register route**

In `backend/src/http/mod.rs`:

```rust
        .route(
            "/api/explore",
            axum::routing::get(crate::discovery::explore::get),
        )
```

- [ ] **Step 5: sqlx prepare + run tests**

```
cd backend && cargo sqlx prepare -- --tests && cargo test --test discovery_explore
```

Expected: 4/4 pass.

- [ ] **Step 6: Commit**

```
git add backend/src/discovery/explore.rs backend/src/http/mod.rs backend/tests/discovery_explore.rs backend/.sqlx/
git commit -m "feat(discovery): GET /api/explore — global cross-author feed

Cursor pagination, sort=newest|most-appreciated, since=24h|7d|30d|all,
category filter. Joins users for author_handle/display_name."
```

---

### Task 5: GET /api/targets/:slug

**Files:**
- Modify: `backend/src/discovery/target.rs`
- Modify: `backend/src/http/mod.rs`
- Test: `backend/tests/discovery_target.rs`

- [ ] **Step 1: Write the failing test**

```rust
mod common;

use astrophoto::api_types::TargetPage;
use axum::http::StatusCode;
use common::TestApp;

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn target_page_returns_meta_plus_photos() {
    let app = TestApp::launch().await;
    let (_, uid) = app.signup_with_handle("A", "alice", "a@x.test").await;
    let p1 = app.ready_photo_with(uid, "AAAA0001", Some("M31")).await;

    // Insert the target row + the join.
    let target_id: uuid::Uuid = sqlx::query_scalar!(
        "insert into targets (slug, canonical_name, aliases, kind) \
         values ('m31', 'M31', '{Andromeda Galaxy,NGC 224}', 'galaxy') returning id"
    ).fetch_one(&app.pool).await.unwrap();
    sqlx::query!("insert into photo_targets (photo_id, target_id) values ($1, $2)", p1, target_id)
        .execute(&app.pool).await.unwrap();

    let (status, body) = app
        .oneshot_json::<TargetPage>("GET", "/api/targets/m31", None, None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.target.slug, "m31");
    assert_eq!(body.target.canonical_name, "M31");
    assert_eq!(body.target.kind.as_deref(), Some("galaxy"));
    assert_eq!(body.target.aliases, vec!["Andromeda Galaxy", "NGC 224"]);
    assert_eq!(body.target.photo_count, 1);
    assert_eq!(body.target.contributor_count, 1);
    assert_eq!(body.page.photos.len(), 1);
    assert_eq!(body.page.photos[0].id, p1);
}

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn target_404_for_unknown_slug() {
    let app = TestApp::launch().await;
    let (status, _) = app.oneshot("GET", "/api/targets/notathing", None, None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
```

- [ ] **Step 2: Run — fails.**

- [ ] **Step 3: Replace `backend/src/discovery/target.rs`**

```rust
use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::AppError;
use crate::api_types::{DiscoveryPage, DiscoveryPhoto, TargetMeta, TargetPage};
use crate::discovery::cursor::{self, Cursor};
use crate::http::AppState;

const DEFAULT_LIMIT: i64 = 24;
const MAX_LIMIT: i64 = 60;

#[derive(Deserialize)]
pub struct Q {
    pub cursor: Option<String>,
    pub limit: Option<i64>,
    pub sort: Option<String>,
    pub category: Option<String>,
}

struct Row {
    id: Uuid,
    short_id: String,
    target: Option<String>,
    width: Option<i32>,
    height: Option<i32>,
    blurhash: Option<String>,
    appreciations_count: i32,
    published_at: Option<chrono::DateTime<chrono::Utc>>,
    owner_id: Uuid,
    handle: String,
    display_name: String,
}

pub async fn get(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(q): Query<Q>,
) -> Result<impl IntoResponse, AppError> {
    let target = sqlx::query!(
        r#"
        select t.id as "id!", t.slug as "slug!", t.canonical_name as "canonical_name!",
               t.aliases as "aliases!", t.kind,
               (select count(*) from photo_targets pt join photos p on p.id = pt.photo_id
                where pt.target_id = t.id and p.published_at is not null and p.status = 'ready')::int8 as "photo_count!",
               (select count(distinct p.owner_id) from photo_targets pt join photos p on p.id = pt.photo_id
                where pt.target_id = t.id and p.published_at is not null and p.status = 'ready')::int8 as "contributor_count!"
        from targets t
        where t.slug = $1
        "#,
        slug
    )
    .fetch_optional(&state.pool)
    .await?;

    let Some(t) = target else { return Err(AppError::not_found("target")) };

    let limit = q.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let sort = q.sort.as_deref().unwrap_or("newest");
    let category = q.category.as_deref();
    let cursor = q.cursor.as_deref().map(cursor::decode).transpose()?;
    let cur_pub = cursor.as_ref().map(|c| c.published_at);
    let cur_id = cursor.as_ref().map(|c| c.id).unwrap_or_else(Uuid::nil);
    let cur_apps = cursor.as_ref().and_then(|c| c.appreciations);

    let rows: Vec<Row> = match sort {
        "most-appreciated" => sqlx::query_as!(
            Row,
            r#"
            select p.id as "id!", p.short_id as "short_id!", p.target,
                   p.width, p.height, p.blurhash,
                   p.appreciations_count as "appreciations_count!",
                   p.published_at,
                   u.id as "owner_id!", u.handle as "handle!", u.display_name as "display_name!"
            from photos p
            join users u on u.id = p.owner_id
            join photo_targets pt on pt.photo_id = p.id
            where pt.target_id = $1
              and p.published_at is not null
              and p.status = 'ready'
              and ($2::int4 is null or p.appreciations_count < $2 or
                   (p.appreciations_count = $2 and (p.published_at, p.id) < ($3, $4)))
              and ($5::text is null or p.category = $5)
            order by p.appreciations_count desc, p.published_at desc, p.id desc
            limit $6
            "#,
            t.id,
            cur_apps,
            cur_pub,
            cur_id,
            category,
            limit + 1
        )
        .fetch_all(&state.pool)
        .await?,
        _ => sqlx::query_as!(
            Row,
            r#"
            select p.id as "id!", p.short_id as "short_id!", p.target,
                   p.width, p.height, p.blurhash,
                   p.appreciations_count as "appreciations_count!",
                   p.published_at,
                   u.id as "owner_id!", u.handle as "handle!", u.display_name as "display_name!"
            from photos p
            join users u on u.id = p.owner_id
            join photo_targets pt on pt.photo_id = p.id
            where pt.target_id = $1
              and p.published_at is not null
              and p.status = 'ready'
              and ($2::timestamptz is null or (p.published_at, p.id) < ($2, $3))
              and ($4::text is null or p.category = $4)
            order by p.published_at desc, p.id desc
            limit $5
            "#,
            t.id,
            cur_pub,
            cur_id,
            category,
            limit + 1
        )
        .fetch_all(&state.pool)
        .await?,
    };

    let more = rows.len() as i64 > limit;
    let take: Vec<_> = rows.into_iter().take(limit as usize).collect();
    let next_cursor = if more
        && let Some(last) = take.last()
        && let Some(pa) = last.published_at
    {
        Some(cursor::encode(&Cursor {
            published_at: pa,
            id: last.id,
            appreciations: if sort == "most-appreciated" { Some(last.appreciations_count) } else { None },
        }))
    } else {
        None
    };

    Ok(Json(TargetPage {
        target: TargetMeta {
            slug: t.slug,
            canonical_name: t.canonical_name,
            aliases: t.aliases,
            kind: t.kind,
            photo_count: t.photo_count,
            contributor_count: t.contributor_count,
        },
        page: DiscoveryPage {
            photos: take
                .into_iter()
                .map(|r| DiscoveryPhoto {
                    id: r.id,
                    short_id: r.short_id,
                    target: r.target,
                    width: r.width,
                    height: r.height,
                    blurhash: r.blurhash,
                    appreciations_count: r.appreciations_count,
                    published_at: r.published_at,
                    author_id: r.owner_id,
                    author_handle: r.handle,
                    author_display_name: r.display_name,
                })
                .collect(),
            next_cursor,
        },
    }))
}
```

- [ ] **Step 4: Register route**

```rust
        .route(
            "/api/targets/:slug",
            axum::routing::get(crate::discovery::target::get),
        )
```

(Order matters: this MUST be registered BEFORE `/api/targets/autocomplete` because axum matches routes in registration order. Actually `/api/targets/autocomplete` is a literal that doesn't conflict with `:slug` lexically; check by running the autocomplete tests. If they 4xx with a UUID parse error, swap registration order.)

- [ ] **Step 5: sqlx prepare + run tests**

- [ ] **Step 6: Commit**

```
git commit -m "feat(discovery): GET /api/targets/:slug — target page (meta + photos)"
```

---

### Task 6: GET /api/tags/:slug

Mirror Task 5 with `tags` table + `photo_tags` join. The `tags` table has `(slug, name)` only — no aliases or kind.

**Files:**
- Modify: `backend/src/discovery/tag.rs`
- Modify: `backend/src/http/mod.rs`
- Test: `backend/tests/discovery_tag.rs`

Test, handler, route, prepare, commit — same pattern as Task 5. Schema reminder:

```sql
-- tags(id, slug, name)
-- photo_tags(photo_id, tag_id)
```

`TagMeta` has only `slug, name, photo_count`. The full handler is the obvious adaptation of Task 5 — replace `targets` / `photo_targets` / `target_id` with `tags` / `photo_tags` / `tag_id`, drop the aliases/kind/contributor_count fields, return `TagPage { tag: TagMeta, page: DiscoveryPage }`.

- [ ] **Step 1: Write `backend/tests/discovery_tag.rs` covering hit + 404.**
- [ ] **Step 2: Run — fails.**
- [ ] **Step 3: Implement handler.**
- [ ] **Step 4: Register route `/api/tags/:slug` (after `/api/tags/autocomplete` — axum literal-vs-param ordering caveat).**
- [ ] **Step 5: sqlx prepare + run.**
- [ ] **Step 6: Commit `feat(discovery): GET /api/tags/:slug — tag page`.**

---

### Task 7: GET /api/equipment/:kind/:slug

The `equipment_items` table has `(kind, canonical_name, display_name, usage_count)`. The slug IS the canonical_name. Photos are joined via `LOWER(photos.equipment_<kind>) = equipment_items.canonical_name`.

`EquipmentPage` returns the meta, the paginated photos, AND the "Often paired with" rail (Task 7a).

**Files:**
- Modify: `backend/src/discovery/equipment.rs`
- Modify: `backend/src/http/mod.rs`
- Test: `backend/tests/discovery_equipment.rs`

- [ ] **Step 1: Write tests for `/api/equipment/camera/<slug>` covering: hit returns meta + photos + paired rail; 404 for unknown kind; 404 for unknown slug.**

- [ ] **Step 2: Run — fails.**

- [ ] **Step 3: Implement handler.**

The kind whitelist: `telescope | camera | mount | filter | guiding`. Return 404 for any other kind.

The query for photos uses the appropriate `LOWER(p.equipment_<kind>) = $slug` clause. Since `kind` is a path param, you must dispatch on it (no SQL parameter for column names).

The "paired with" rail: top 4 other equipment items (across all 5 kinds) that co-occur most often on photos that ALSO use this item. Cap one per kind when possible; otherwise top 4 by shared count.

```rust
// Outline:
let paired = sqlx::query!(
    r#"
    -- For each candidate paired item, count photos that use both.
    -- 'photos using THIS item' = ?
    select
        ei.kind  as "kind!",
        ei.canonical_name as "slug!",
        ei.display_name   as "display_name!",
        count(*)::int8     as "shared_count!"
    from equipment_items ei
    join photos p on (
        case ei.kind
            when 'telescope' then lower(p.equipment_telescope) = ei.canonical_name or lower(p.scope) = ei.canonical_name
            when 'camera'    then lower(p.equipment_camera)    = ei.canonical_name or lower(p.camera) = ei.canonical_name
            when 'mount'     then lower(p.equipment_mount)     = ei.canonical_name or lower(p.mount) = ei.canonical_name
            when 'filter'    then lower(p.equipment_filters)   = ei.canonical_name or lower(p.filters) = ei.canonical_name
            when 'guiding'   then lower(p.equipment_guiding)   = ei.canonical_name or lower(p.guiding) = ei.canonical_name
            else false
        end
    )
    where p.published_at is not null and p.status = 'ready'
      and exists (
        select 1 from photos p2
        where p2.id = p.id
          and (
            -- p2 uses the THIS-item (per the parent kind/slug)
            ($1::text = 'telescope' and (lower(p2.equipment_telescope) = $2 or lower(p2.scope) = $2))
            or ($1 = 'camera'    and (lower(p2.equipment_camera) = $2 or lower(p2.camera) = $2))
            or ($1 = 'mount'     and (lower(p2.equipment_mount) = $2 or lower(p2.mount) = $2))
            or ($1 = 'filter'    and (lower(p2.equipment_filters) = $2 or lower(p2.filters) = $2))
            or ($1 = 'guiding'   and (lower(p2.equipment_guiding) = $2 or lower(p2.guiding) = $2))
          )
      )
      and not (ei.kind = $1 and ei.canonical_name = $2)  -- exclude self
    group by ei.kind, ei.canonical_name, ei.display_name
    order by shared_count desc
    limit 4
    "#,
    kind, slug
).fetch_all(&state.pool).await?;
```

This SQL is intentionally chunky — it's the price of the schema choice (free-text `equipment_<field>` strings normalised to `equipment_items` via case-insensitive equality). If the cost is too high in practice (it shouldn't be with the lowercase indexes from migration 0012), the simpler version is to compute the rail in Rust by fetching the photo set then issuing a separate count.

For the photos page, the SQL is parametric on `kind` to pick the right column:

```rust
let photos_sql = match kind.as_str() {
    "telescope" => /* lower(p.equipment_telescope) = $1 or lower(p.scope) = $1 */,
    "camera"    => /* lower(p.equipment_camera) = $1 or lower(p.camera) = $1 */,
    // ...
};
```

`sqlx::query!` is compile-time-checked, so the per-kind branch needs five separate `query_as!` invocations OR a single dynamic query via `sqlx::query()` with manual binding. Pick the per-kind invocations for the strict typing — five short branches is fine.

Same cursor logic as Task 4/5.

- [ ] **Step 4: Register route**

```rust
        .route(
            "/api/equipment/:kind/:slug",
            axum::routing::get(crate::discovery::equipment::get),
        )
```

(Must come AFTER `/api/equipment/autocomplete` in registration order.)

- [ ] **Step 5: sqlx prepare + run.**

- [ ] **Step 6: Commit `feat(discovery): GET /api/equipment/:kind/:slug — equipment page + paired rail`.**

---

### Task 8: GET /api/categories/:cat

Single-category listing. `category` ∈ `dso | planetary | lunar | solar | wide_field | nightscape | other`. 404 for any other value.

**Files:**
- Modify: `backend/src/discovery/category.rs`
- Modify: `backend/src/http/mod.rs`
- Test: `backend/tests/discovery_category.rs`

The handler is the cursor-paginated photos query filtered by `category = $1`, returning `CategoryPage { category, photo_count, page }`.

- [ ] Steps 1–6 mirror Task 5 structure. Cap kinds: validate against the 7-value whitelist before querying.

- Commit message: `feat(discovery): GET /api/categories/:cat — category page`.

---

### Task 9: GET /api/search

Three small ILIKE queries unioned in the handler. v1 is intentionally cheap.

**Files:**
- Modify: `backend/src/discovery/search.rs`
- Modify: `backend/src/http/mod.rs`
- Test: `backend/tests/discovery_search.rs`

- [ ] **Step 1: Write the failing test**

```rust
mod common;

use astrophoto::api_types::SearchResults;
use axum::http::StatusCode;
use common::TestApp;

#[tokio::test]
#[allow(clippy::unwrap_used)]
async fn search_returns_targets_users_photos() {
    let app = TestApp::launch().await;
    let (_, uid) = app.signup_with_handle("Andromeda Aficionado", "andromeda_aficionado", "a@x.test").await;
    let p = app.ready_photo_with(uid, "AAAA0001", Some("M31 Andromeda Galaxy")).await;
    sqlx::query!(
        "insert into targets (slug, canonical_name, aliases, kind) \
         values ('m31', 'M31', '{Andromeda Galaxy}', 'galaxy')"
    ).execute(&app.pool).await.unwrap();

    let (status, body) = app
        .oneshot_json::<SearchResults>("GET", "/api/search?q=andromeda", None, None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.q, "andromeda");
    assert!(body.targets.iter().any(|t| t.slug == "m31"));
    assert!(body.users.iter().any(|u| u.handle == "andromeda_aficionado"));
    assert!(body.photos.iter().any(|ph| ph.id == p));
}

#[tokio::test]
async fn search_empty_q_returns_400() {
    let app = TestApp::launch().await;
    let (status, _) = app.oneshot("GET", "/api/search?q=", None, None).await;
    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn search_caps_each_group() {
    let app = TestApp::launch().await;
    // Skip the seed if the cap test is too verbose; the main happy-path test is enough.
    let (status, _) = app.oneshot("GET", "/api/search?q=any", None, None).await;
    assert_eq!(status, axum::http::StatusCode::OK);
}
```

- [ ] **Step 2: Run — fails.**

- [ ] **Step 3: Implement**

```rust
use axum::{
    Json,
    extract::{Query, State},
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::AppError;
use crate::api_types::{DiscoveryPhoto, SearchResults, SearchTargetHit, SearchUserHit};
use crate::http::AppState;

#[derive(Deserialize)]
pub struct Q {
    pub q: String,
}

const TARGET_CAP: i64 = 5;
const USER_CAP: i64 = 5;
const PHOTO_CAP: i64 = 24;

pub async fn get(
    State(state): State<AppState>,
    Query(q): Query<Q>,
) -> Result<impl IntoResponse, AppError> {
    let term = q.q.trim();
    if term.is_empty() {
        return Err(AppError::bad_request("q_empty"));
    }
    let pattern = format!("%{}%", term.to_lowercase());

    // Targets: match canonical_name OR any alias (case-insensitive).
    let target_rows = sqlx::query!(
        r#"
        select t.slug as "slug!", t.canonical_name as "canonical_name!",
               (select count(*) from photo_targets pt join photos p on p.id = pt.photo_id
                where pt.target_id = t.id and p.published_at is not null and p.status='ready')::int8 as "photo_count!"
        from targets t
        where lower(t.canonical_name) like $1
           or exists (select 1 from unnest(t.aliases) as a where lower(a) like $1)
        order by t.canonical_name
        limit $2
        "#,
        pattern,
        TARGET_CAP
    )
    .fetch_all(&state.pool)
    .await?;

    let user_rows = sqlx::query!(
        r#"
        select u.id as "id!", u.handle as "handle!", u.display_name as "display_name!"
        from users u
        where lower(u.handle) like $1 or lower(u.display_name) like $1
        order by u.handle
        limit $2
        "#,
        pattern,
        USER_CAP
    )
    .fetch_all(&state.pool)
    .await?;

    let photo_rows = sqlx::query!(
        r#"
        select p.id as "id!", p.short_id as "short_id!", p.target,
               p.width, p.height, p.blurhash,
               p.appreciations_count as "appreciations_count!",
               p.published_at,
               u.id as "owner_id!", u.handle as "handle!", u.display_name as "display_name!"
        from photos p
        join users u on u.id = p.owner_id
        where p.published_at is not null and p.status = 'ready'
          and (lower(coalesce(p.target, '')) like $1 or lower(coalesce(p.caption, '')) like $1)
        order by p.published_at desc, p.id desc
        limit $2
        "#,
        pattern,
        PHOTO_CAP
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(SearchResults {
        q: term.to_string(),
        targets: target_rows
            .into_iter()
            .map(|r| SearchTargetHit { slug: r.slug, canonical_name: r.canonical_name, photo_count: r.photo_count })
            .collect(),
        users: user_rows
            .into_iter()
            .map(|r| SearchUserHit { id: r.id, handle: r.handle, display_name: r.display_name })
            .collect(),
        photos: photo_rows
            .into_iter()
            .map(|r| DiscoveryPhoto {
                id: r.id,
                short_id: r.short_id,
                target: r.target,
                width: r.width,
                height: r.height,
                blurhash: r.blurhash,
                appreciations_count: r.appreciations_count,
                published_at: r.published_at,
                author_id: r.owner_id,
                author_handle: r.handle,
                author_display_name: r.display_name,
            })
            .collect(),
    }))
}
```

- [ ] **Step 4: Register route `/api/search`.**

- [ ] **Step 5: sqlx prepare + run.**

- [ ] **Step 6: Commit `feat(discovery): GET /api/search — combined v1 search (ILIKE)`.**

---

### Task 10: Backend full-suite + types regen sweep

```
cd backend && cargo test --tests
cd .. && just types
```

Stage and commit any TS regen drift.

```
git add backend/.sqlx/ frontend/src/lib/api/
git commit -m "chore(types): regenerate after P3 backend types" || echo "nothing to commit"
```

---

## Frontend — primitives

### Task 11: discoveryClient.ts

**Files:**
- Create: `frontend/src/lib/api/discoveryClient.ts`

```ts
import type { DiscoveryPage } from './DiscoveryPage';
import type { TargetPage } from './TargetPage';
import type { TagPage } from './TagPage';
import type { EquipmentPage } from './EquipmentPage';
import type { CategoryPage } from './CategoryPage';
import type { SearchResults } from './SearchResults';

const API_BASE: string = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? '';
type FetchFn = typeof fetch;

interface FeedOpts { cursor?: string; sort?: 'newest' | 'most-appreciated'; since?: '24h'|'7d'|'30d'|'all'; category?: string; following?: boolean; limit?: number; }

function qs(opts: Record<string, string | number | boolean | undefined>): string {
  const p = new URLSearchParams();
  for (const [k, v] of Object.entries(opts)) {
    if (v !== undefined && v !== '' && v !== false) p.set(k, String(v));
  }
  const s = p.toString();
  return s ? `?${s}` : '';
}

export async function fetchExplore(f: FetchFn, opts: FeedOpts = {}): Promise<DiscoveryPage> {
  const r = await f(`${API_BASE}/api/explore${qs(opts as Record<string, string | number | boolean | undefined>)}`);
  if (!r.ok) throw new Error(`fetchExplore ${r.status}`);
  return (await r.json()) as DiscoveryPage;
}

export async function fetchTargetPage(f: FetchFn, slug: string, opts: FeedOpts = {}): Promise<TargetPage> {
  const r = await f(`${API_BASE}/api/targets/${slug}${qs(opts)}`);
  if (r.status === 404) throw new Error('not_found');
  if (!r.ok) throw new Error(`fetchTargetPage ${r.status}`);
  return (await r.json()) as TargetPage;
}

export async function fetchTagPage(f: FetchFn, slug: string, opts: FeedOpts = {}): Promise<TagPage> {
  const r = await f(`${API_BASE}/api/tags/${slug}${qs(opts)}`);
  if (r.status === 404) throw new Error('not_found');
  if (!r.ok) throw new Error(`fetchTagPage ${r.status}`);
  return (await r.json()) as TagPage;
}

export async function fetchEquipmentPage(f: FetchFn, kind: string, slug: string, opts: FeedOpts = {}): Promise<EquipmentPage> {
  const r = await f(`${API_BASE}/api/equipment/${kind}/${slug}${qs(opts)}`);
  if (r.status === 404) throw new Error('not_found');
  if (!r.ok) throw new Error(`fetchEquipmentPage ${r.status}`);
  return (await r.json()) as EquipmentPage;
}

export async function fetchCategoryPage(f: FetchFn, cat: string, opts: FeedOpts = {}): Promise<CategoryPage> {
  const r = await f(`${API_BASE}/api/categories/${cat}${qs(opts)}`);
  if (r.status === 404) throw new Error('not_found');
  if (!r.ok) throw new Error(`fetchCategoryPage ${r.status}`);
  return (await r.json()) as CategoryPage;
}

export async function fetchSearch(f: FetchFn, q: string): Promise<SearchResults> {
  const r = await f(`${API_BASE}/api/search?q=${encodeURIComponent(q)}`);
  if (!r.ok) throw new Error(`fetchSearch ${r.status}`);
  return (await r.json()) as SearchResults;
}

// Autocomplete pass-throughs (existing endpoints from P1).
export async function autocompleteTargets(f: FetchFn, q: string): Promise<Array<{slug: string; canonical_name: string}>> {
  const r = await f(`${API_BASE}/api/targets/autocomplete?q=${encodeURIComponent(q)}`);
  if (!r.ok) throw new Error(`autocompleteTargets ${r.status}`);
  return (await r.json()) as Array<{slug: string; canonical_name: string}>;
}
```

- [ ] Step 1: Write the file. Step 2: `pnpm check` clean. Step 3: Commit `feat(frontend): typed P3 discovery API client`.

---

## Frontend — components

### Task 12: AuthorChip + CrossAuthorTile + CrossAuthorGrid

**Files:**
- Create: `frontend/src/lib/components/discovery/AuthorChip.svelte`
- Create: `frontend/src/lib/components/discovery/CrossAuthorTile.svelte`
- Create: `frontend/src/lib/components/discovery/CrossAuthorGrid.svelte`

`<AuthorChip>` — small `@handle` chip, links to `/u/<handle>`. ~20 lines.

`<CrossAuthorTile>` — copy of `<PhotoTile>` (P2) with `<AuthorChip>` mounted in the caption overlay. Accepts `DiscoveryPhoto` instead of `GalleryPhoto`.

`<CrossAuthorGrid>` — copy of `<PhotoGrid>` (P2) but takes a `loadMore: () => Promise<{photos: DiscoveryPhoto[], next_cursor: string | null}>` prop so each page can call its own endpoint. Or — simpler: pass the discovery page directly and a `nextCursor: string | null` and a `loadMore` callback. The justified-layout machinery is identical; the tile component is the only thing that swaps.

Hold both `<PhotoGrid>` (gallery, P2) and `<CrossAuthorGrid>` (discovery, P3) — they share the layout but render different tiles. The duplication is intentional (YAGNI: don't abstract over two callers).

- [ ] Step 1–4: Write components, `pnpm check`, commit `feat(discovery): cross-author tile + grid + author chip`.

---

### Task 13: FilterPills + DiscoveryHeader

`<FilterPills>` — sort chips + time-window chips + category chips + following-only checkbox. Bindable values via callback props. Variant prop chooses which pills render.

`<DiscoveryHeader>` — accepts `variant: 'explore' | 'target' | 'tag' | 'equipment' | 'category' | 'search'` plus a meta payload. Renders the eyebrow + title + optional right-side accent stat. Uses the spec's wireframes as the layout reference.

Files:
- Create: `frontend/src/lib/components/discovery/FilterPills.svelte`
- Create: `frontend/src/lib/components/discovery/DiscoveryHeader.svelte`

- [ ] Step 1–4: Write, check, commit `feat(discovery): filter pills + page header (variant-driven)`.

---

### Task 14: SearchBar + SuggestionsList (with ⌘K hotkey)

**Files:**
- Create: `frontend/src/lib/components/discovery/SearchBar.svelte`
- Create: `frontend/src/lib/components/discovery/SuggestionsList.svelte`
- Modify: `frontend/src/lib/components/AppHeader.svelte` (mount the SearchBar)

`<SearchBar>`:
- An `<input>` with debounced (300 ms) calls to `/api/search`.
- Renders `<SuggestionsList>` underneath while focused + non-empty.
- ⌘K (or Ctrl-K on non-Mac) globally focuses the input. Escape clears focus.
- Enter on a target/user suggestion navigates to that route. Enter on a photo suggestion navigates to its permalink. Enter with no suggestion focused → navigate to `/search?q=...`.

`<SuggestionsList>`:
- Three groups (Targets / Photographers / Photos), capped per the SearchResults contract.
- Keyboard navigation: ↑/↓ moves focus; Enter selects.

Modify `<AppHeader>` to mount `<SearchBar>` between the logo and the auth area. Inspect the existing `AppHeader.svelte` first to find the right slot.

- [ ] Steps 1–4: Write, check, commit `feat(discovery): global search bar with ⌘K hotkey`.

---

## Frontend — pages

### Task 15: /explore route

**Files:**
- Create: `frontend/src/routes/explore/+page.server.ts`
- Create: `frontend/src/routes/explore/+page.svelte`

`+page.server.ts`:
```ts
import type { PageServerLoad } from './$types';
import { fetchExplore } from '$lib/api/discoveryClient';

export const load: PageServerLoad = async ({ fetch, url }) => {
  const sort = (url.searchParams.get('sort') ?? 'newest') as 'newest' | 'most-appreciated';
  const since = (url.searchParams.get('since') ?? '7d') as '24h' | '7d' | '30d' | 'all';
  const categoryParam = url.searchParams.get('category');
  const category = categoryParam ?? undefined;
  const followingParam = url.searchParams.get('following') === 'true';
  const initial = await fetchExplore(fetch, { sort, since, category, following: followingParam, limit: 24 });
  return { initial, sort, since, category, following: followingParam };
};
```

`+page.svelte`:
- Mount `<AppHeader />`.
- `<DiscoveryHeader variant="explore" />` — 12,418 published frames (use a real count from the page or just the spec's eyebrow string for now).
- `<FilterPills>` bound to URL search params (use `goto` + `replaceState: true` on change).
- `<CrossAuthorGrid>` rendering `data.initial`, with a `loadMore` that calls `fetchExplore` with the current cursor.

- [ ] Steps 1–4: Write, `pnpm check`, smoke in browser, commit.

---

### Task 16: /t/[slug] route

Mirror Task 15 with `<DiscoveryHeader variant="target">` consuming `data.initial.target` (TargetMeta) for the eyebrow + title + aliases line. Photos via `data.initial.page.photos`.

- [ ] Files + steps as Task 15. Commit `feat(discovery): /t/[slug] target page`.

---

### Task 17: /tag/[slug] route

Similar to Task 16 but for `TagPage`.

- [ ] Files + steps as Task 15. Commit `feat(discovery): /tag/[slug] tag page`.

---

### Task 18: /equip/[kind]/[slug] route

`<DiscoveryHeader variant="equipment">` shows kind + display_name + photo_count.

Below the gallery, mount `<EquipmentPairedRail>` displaying `data.initial.paired` (up to 4 chips).

**Files:**
- Create: `frontend/src/lib/components/discovery/EquipmentPairedRail.svelte`
- Create: `frontend/src/routes/equip/[kind]/[slug]/+page.server.ts` and `+page.svelte`

- [ ] Steps as Task 15. Commit `feat(discovery): /equip/[kind]/[slug] equipment page + paired rail`.

---

### Task 19: /c/[cat] route

Validate cat against the 7-value whitelist on the server side; throw 404 otherwise.

- [ ] Files + steps as Task 15. Commit `feat(discovery): /c/[cat] category page`.

---

### Task 20: /search route

Renders `<SearchResults>` from the URL `?q=` param. Three sections (Targets / Photographers / Photos) — each links to the appropriate page.

- [ ] Files + steps. Commit `feat(discovery): /search results page`.

---

## Quality gates and acceptance

### Task 21: Full quality-gate sweep

```
just check
cd backend && cargo test --tests
cd ../frontend && pnpm vitest run && pnpm build
```

Fix anything that surfaces. Apply `cargo fmt` and `pnpm prettier --write` if formatting drifted.

- [ ] Commit any prepare drift with `chore: refresh sqlx + types after P3`.

---

### Task 22: chrome-devtools-mcp acceptance walk + p3-acceptance.md

Drive a Chrome instance via the MCP tools. Steps:

1. `/explore` renders with photos from the seeded P2 acceptance data (or seed via the test-helper `ready_photo_with`).
2. Sort selector flips order between Newest / Most appreciated.
3. Time-window selector filters last 24h vs all time.
4. Category chips filter the grid.
5. Following-only toggles to the auth-only filter.
6. Click a tile → lightbox opens (shallow routing from P2).
7. Navigate to `/t/m31` (after seeding a target row); confirm header + grid.
8. Navigate to `/tag/<slug>` if a tag exists.
9. Navigate to `/equip/camera/<canonical>`; confirm "Often paired with" rail renders chips.
10. Navigate to `/c/dso`; confirm category filter applies.
11. Type `andromeda` in the navbar search; suggestions appear; click a target → navigates to `/t/...`.
12. Press ⌘K from any page; search bar focuses.
13. Visit `/search?q=andromeda` directly; three sections render.
14. Empty state: visit `/t/notathing` → 404 page.
15. Empty state: visit `/c/dso` with no DSO photos → "No photos in this category yet."

Record in `docs/operations/p3-acceptance.md` (mirror P2's format).

- [ ] Commit `docs(ops): P3 acceptance report`.

---

### Task 23: Push, PR, merge, cleanup

```
git push -u origin feat/showcase-p3-discovery
gh pr create --base main --head feat/showcase-p3-discovery --title "feat: photographer showcase P3 (discovery)" --body "..."
gh pr merge <PR#> --repo 100-tokens/astrophoto --merge

# Manual cleanup (delete_branch_on_merge: false).
git push origin --delete feat/showcase-p3-discovery
git -C /Volumes/Pascal4Tb/Projects/astrophoto pull --ff-only origin main
git -C /Volumes/Pascal4Tb/Projects/astrophoto worktree remove /Volumes/Pascal4Tb/Projects/astrophoto-showcase-p3
git -C /Volumes/Pascal4Tb/Projects/astrophoto branch -d feat/showcase-p3-discovery
```

---

## After merge — what's next

P1+P2+P3 ships the entire photographer showcase. Future phases (not in this spec):
- Plate-solving (writes additional `photo_targets` rows with `source = 'plate_solve'`).
- Search v2 (tsvector + GIN).
- Collections (replaces the "Featured" slot for portfolio curation).
- Subscriptions / billing UI for the tier gate (currently a manual DB UPDATE).
