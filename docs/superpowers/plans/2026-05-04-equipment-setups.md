# Equipment Setups Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship reusable per-user equipment setups (telescope + focal_modifier + camera + mount + filters + free-text guiding), applied to a photo at upload-verify with fill-empty default auto-apply and confirm-on-conflict manual selection.

**Architecture:** New tables `equipment_setups` and `setup_items` reference the existing canonical `equipment_items` dictionary. Photos gain a nullable `setup_id` FK and a denormalized `focal_modifier` text column. Backend handlers live next to existing equipment/photos code. Frontend adds `/settings/equipment` CRUD pages and a shared `SetupPicker` component that the existing upload-verify form (which doubles as the published-photo edit form) mounts.

**Tech Stack:** Rust + axum + sqlx (PostgreSQL) on the backend. SvelteKit + Svelte 5 runes on the frontend. ts-rs for the type bridge. testcontainers for backend integration tests. chrome-devtools-mcp for interactive smoke verification (no Playwright per project preference).

**Spec:** `docs/superpowers/specs/2026-05-04-equipment-setups-design.md` is canonical for behavior, schema, and decisions. When in doubt, re-read the spec.

---

## Pre-flight

### Task 0: Feature branch

**Files:** none

- [ ] **Step 1: Create the branch**

```bash
git checkout -b feat/equipment-setups
```

- [ ] **Step 2: Confirm clean tree**

Run: `git status`
Expected: `On branch feat/equipment-setups` and `nothing to commit, working tree clean`.

---

## Phase 1 — Migration

### Task 1: Migration 0014_equipment_setups

**Files:**
- Create: `backend/migrations/0014_equipment_setups.sql`

- [ ] **Step 1: Generate the migration file**

Run: `cd backend && cargo sqlx migrate add equipment_setups`
Expected: prints something like `Created migration: '<timestamp>_equipment_setups.sql'`. **The repo convention uses numeric prefixes (0014, …), not timestamps.** Rename the generated file to `0014_equipment_setups.sql` before editing.

- [ ] **Step 2: Replace the file's contents**

```sql
-- 0014 equipment_setups: per-user reusable gear bundles, applied at
-- upload-verify to fill the photo's equipment columns. See
-- docs/superpowers/specs/2026-05-04-equipment-setups-design.md.

-- 1. Extend equipment_items.kind to include focal_modifier. The
--    existing 'guiding' value is intentionally retained: legacy rows
--    persist inert. Do not "clean up" the constraint.
alter table equipment_items
    drop constraint equipment_items_kind_check;
alter table equipment_items
    add  constraint equipment_items_kind_check
         check (kind in ('telescope','camera','mount','filter',
                         'focal_modifier','guiding'));

-- 2. Setup container, owned by a user. name is unique per owner.
create table equipment_setups (
    id          uuid primary key default gen_random_uuid(),
    owner_id    uuid not null references users(id) on delete cascade,
    name        text not null,
    description text,
    location    text,
    is_remote   boolean not null default false,
    is_default  boolean not null default false,
    guiding     text,
    created_at  timestamptz not null default now(),
    updated_at  timestamptz not null default now(),
    unique (owner_id, name)
);
create index equipment_setups_owner_idx
    on equipment_setups (owner_id, updated_at desc);

-- 3. At most one default per user — DB-enforced via partial unique idx.
create unique index equipment_setups_owner_default_uidx
    on equipment_setups (owner_id) where is_default;

-- 4. Setup ↔ canonical item junction. Composite PK on
--    (setup_id, role, item_id) allows multi-filter and prevents the
--    same item being added twice in the same role.
create table setup_items (
    setup_id  uuid not null references equipment_setups(id) on delete cascade,
    role      text not null
        check (role in ('optical_tube','focal_modifier','main_camera',
                        'mount','filter')),
    item_id   uuid not null references equipment_items(id) on delete restrict,
    primary key (setup_id, role, item_id)
);
create index setup_items_item_idx on setup_items (item_id);

-- 5. Photo points back to the setup it originated from. on delete set
--    null because the photo's denormalized columns preserve historical
--    truth even after the setup is deleted.
alter table photos
    add column setup_id uuid references equipment_setups(id) on delete set null,
    add column focal_modifier text;

create index photos_setup_idx
    on photos (setup_id) where setup_id is not null;
create index photos_focal_modifier_lower_idx
    on photos (lower(focal_modifier))
    where published_at is not null and focal_modifier is not null;
```

- [ ] **Step 3: Apply against the dev database**

Run: `just db-reset`
Expected: dev DB recreated, all 14 migrations apply cleanly.

- [ ] **Step 4: Verify schema with psql**

Run: `psql $(grep DATABASE_URL .env | cut -d= -f2) -c "\d equipment_setups"` (or equivalent)
Expected: shows the table with the columns and indexes from above.

- [ ] **Step 5: Commit**

```bash
git add backend/migrations/0014_equipment_setups.sql
git commit -m "feat(db): equipment_setups + setup_items + photos.setup_id"
```

---

## Phase 2 — DTOs and ts-rs export

### Task 2: Setup DTOs

**Files:**
- Modify: `backend/src/api_types.rs`
- Modify: `backend/src/bin/gen-types.rs`

- [ ] **Step 1: Add the DTOs at the end of `backend/src/api_types.rs`**

```rust
/// One canonical equipment item (used inside a setup).
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "EquipmentItemRef.ts")]
pub struct EquipmentItemRef {
    pub id: String,
    pub kind: String,            // 'telescope'|'camera'|'mount'|'filter'|'focal_modifier'
    pub canonical_name: String,
    pub display_name: String,
}

/// One member of a setup (link between a setup and a canonical item).
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "SetupItem.ts")]
pub struct SetupItem {
    pub role: String,            // 'optical_tube'|'focal_modifier'|'main_camera'|'mount'|'filter'
    pub item: EquipmentItemRef,
}

/// Compact list-view summary — just the metadata + counts per role.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "SetupSummary.ts")]
pub struct SetupSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub is_remote: bool,
    pub is_default: bool,
    pub guiding: Option<String>,
    pub updated_at: String,      // RFC3339
    /// One entry per role with at least one item: { role -> count }.
    pub item_counts: Vec<RoleCount>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "RoleCount.ts")]
pub struct RoleCount {
    pub role: String,
    pub count: i64,
}

/// Detail-view setup with full item expansion.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "SetupDetail.ts")]
pub struct SetupDetail {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub is_remote: bool,
    pub is_default: bool,
    pub guiding: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub items: Vec<SetupItem>,
}

/// Body for POST/PATCH /api/equipment/setups[/:id]. Items replace-all
/// on PATCH (no merge). Unknown item_ids → 422.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "SetupInput.ts")]
pub struct SetupInput {
    pub name: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub is_remote: bool,
    pub is_default: bool,
    pub guiding: Option<String>,
    pub items: Vec<SetupInputItem>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "SetupInputItem.ts")]
pub struct SetupInputItem {
    pub role: String,
    pub item_id: String,
}

/// Body for POST /api/equipment/items resolve-or-create.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "EquipmentItemInput.ts")]
pub struct EquipmentItemInput {
    pub kind: String,
    pub display_name: String,
}

/// Body for POST /api/photos/:id/apply-setup.
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "ApplySetupInput.ts")]
pub struct ApplySetupInput {
    pub setup_id: String,
    /// "fill_empty" | "overwrite"
    pub mode: String,
}
```

- [ ] **Step 2: Wire the new types into `backend/src/bin/gen-types.rs`**

Find the `use astrophoto::api_types::{ ... };` block and add (alphabetically near existing entries):

```rust
ApplySetupInput, EquipmentItemInput, EquipmentItemRef, RoleCount,
SetupDetail, SetupInput, SetupInputItem, SetupItem, SetupSummary,
```

Then add corresponding `XYZ::export_all_to(out_dir)?;` lines under the existing block.

- [ ] **Step 3: Run cargo check**

Run: `cd backend && cargo check`
Expected: compiles. Any unused-import warnings are fine for now.

- [ ] **Step 4: Generate the TypeScript types**

Run: `just types`
Expected: writes 9 new `.ts` files under `frontend/src/lib/api/`.

- [ ] **Step 5: Commit**

```bash
git add backend/src/api_types.rs backend/src/bin/gen-types.rs frontend/src/lib/api
git commit -m "feat(types): equipment-setup DTOs and ts-rs export"
```

---

## Phase 3 — Backend item endpoint and autocomplete extension

### Task 3: Extend autocomplete to recognize `focal_modifier`

**Files:**
- Modify: `backend/src/equipment/autocomplete.rs`
- Modify: `backend/tests/equipment_autocomplete.rs`

- [ ] **Step 1: Add the failing test in `backend/tests/equipment_autocomplete.rs`**

Append a new `#[tokio::test]` that calls `GET /api/equipment/autocomplete?kind=focal_modifier&q=red` against a fixture row and asserts a 200 with the expected display_name. Mirror the existing tests' fixture setup.

```rust
#[tokio::test]
async fn focal_modifier_kind_is_supported() {
    let (app, pool) = harness().await;
    sqlx::query!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count)
         values ('focal_modifier','antares 0.7x reducer','Antares 0.7x Reducer',3)"
    )
    .execute(&pool)
    .await
    .unwrap();

    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/equipment/autocomplete?kind=focal_modifier&q=red")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let body: serde_json::Value =
        serde_json::from_slice(&axum::body::to_bytes(r.into_body(), 1 << 20).await.unwrap())
            .unwrap();
    let names: Vec<String> = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v["display_name"].as_str().unwrap().to_string())
        .collect();
    assert!(names.iter().any(|n| n == "Antares 0.7x Reducer"));
}
```

- [ ] **Step 2: Run the test, expect it to fail**

Run: `cd backend && cargo test --test equipment_autocomplete focal_modifier_kind_is_supported -- --nocapture`
Expected: 422 Validation error from the existing `VALID_KINDS` check.

- [ ] **Step 3: Extend `VALID_KINDS` in `backend/src/equipment/autocomplete.rs`**

Replace:
```rust
const VALID_KINDS: &[&str] = &["telescope", "camera", "mount", "filter", "guiding"];
```
with:
```rust
const VALID_KINDS: &[&str] =
    &["telescope", "camera", "mount", "filter", "focal_modifier", "guiding"];
```

Update the validation message correspondingly:
```rust
"kind must be telescope|camera|mount|filter|focal_modifier|guiding".into(),
```

- [ ] **Step 4: Re-run the test, expect pass**

Run: `cd backend && cargo test --test equipment_autocomplete -- --nocapture`
Expected: all autocomplete tests pass.

- [ ] **Step 5: Commit**

```bash
git add backend/src/equipment/autocomplete.rs backend/tests/equipment_autocomplete.rs
git commit -m "feat(equipment): autocomplete supports focal_modifier kind"
```

### Task 4: POST /api/equipment/items resolve-or-create

**Files:**
- Create: `backend/src/equipment/items_create.rs`
- Modify: `backend/src/equipment/mod.rs`
- Modify: `backend/src/http/mod.rs`
- Create: `backend/tests/equipment_items_create.rs`

- [ ] **Step 1: Add the failing test `backend/tests/equipment_items_create.rs`**

Use the same test harness shape as `equipment_autocomplete.rs`. Cover three cases: insert on miss (returns the row, usage_count=0), idempotent on hit (returns the existing row, usage_count unchanged), invalid kind → 422. Inline the harness helper if convenient or share via `tests/common/`.

```rust
#![allow(clippy::unwrap_used, clippy::expect_used)]

// ... (copy the harness pattern from equipment_autocomplete.rs) ...

#[tokio::test]
async fn insert_on_miss_returns_row_with_zero_count() {
    let (app, pool) = harness().await;
    let r = app.clone().oneshot(
        Request::builder()
            .method("POST")
            .uri("/api/equipment/items")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"kind":"telescope","display_name":"Sky-Watcher 200P"}"#))
            .unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 200);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(r.into_body(), 1<<20).await.unwrap()
    ).unwrap();
    assert_eq!(body["display_name"], "Sky-Watcher 200P");
    assert_eq!(body["canonical_name"], "sky-watcher 200p");
    let count: i32 = sqlx::query_scalar!(
        "select usage_count from equipment_items where canonical_name='sky-watcher 200p'"
    ).fetch_one(&pool).await.unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn idempotent_on_hit_does_not_increment() {
    let (app, pool) = harness().await;
    sqlx::query!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count)
         values ('telescope', 'celestron c8', 'Celestron C8', 7)"
    ).execute(&pool).await.unwrap();
    let r = app.clone().oneshot(
        Request::builder()
            .method("POST")
            .uri("/api/equipment/items")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"kind":"telescope","display_name":"Celestron C8"}"#))
            .unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 200);
    let count: i32 = sqlx::query_scalar!(
        "select usage_count from equipment_items where canonical_name='celestron c8'"
    ).fetch_one(&pool).await.unwrap();
    assert_eq!(count, 7);
}

#[tokio::test]
async fn invalid_kind_returns_422() {
    let (app, _pool) = harness().await;
    let r = app.clone().oneshot(
        Request::builder()
            .method("POST")
            .uri("/api/equipment/items")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"kind":"banana","display_name":"x"}"#))
            .unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 422);
}
```

- [ ] **Step 2: Run, expect compile/route failure**

Run: `cd backend && cargo test --test equipment_items_create -- --nocapture`
Expected: 404 (route not wired) on each case.

- [ ] **Step 3: Implement `backend/src/equipment/items_create.rs`**

```rust
//! POST /api/equipment/items
//!
//! Resolve-or-create one canonical equipment item. Returns the existing
//! row on hit (no usage_count bump — that counter remains photo-save
//! driven via `crate::equipment::upsert`) or inserts on miss with
//! usage_count = 0. Authenticated users only.

use axum::{Json, extract::State, response::IntoResponse};
use serde::Serialize;

use crate::api_types::EquipmentItemInput;
use crate::auth::middleware::CurrentUser;
use crate::error::AppError;
use crate::http::AppState;

const VALID_KINDS: &[&str] = &["telescope", "camera", "mount", "filter", "focal_modifier"];

#[derive(Serialize)]
pub struct Out {
    pub id: String,
    pub kind: String,
    pub canonical_name: String,
    pub display_name: String,
    pub usage_count: i32,
}

pub async fn handler(
    State(state): State<AppState>,
    _user: CurrentUser,
    Json(input): Json<EquipmentItemInput>,
) -> Result<impl IntoResponse, AppError> {
    if !VALID_KINDS.contains(&input.kind.as_str()) {
        return Err(AppError::Validation(
            "kind must be telescope|camera|mount|filter|focal_modifier".into(),
        ));
    }
    let display = input.display_name.trim();
    if display.is_empty() {
        return Err(AppError::Validation("display_name is required".into()));
    }
    let canonical = display.to_lowercase();

    // Try to read first; if absent, insert. The on-conflict-do-nothing
    // dance avoids racing with the photo-save upsert path which uses
    // its own usage_count semantics.
    let row = sqlx::query!(
        r#"
        with ins as (
            insert into equipment_items (kind, canonical_name, display_name, usage_count)
                 values ($1, $2, $3, 0)
            on conflict (kind, canonical_name) do nothing
            returning id, kind, canonical_name, display_name, usage_count
        )
        select id, kind, canonical_name, display_name, usage_count
          from ins
         union all
        select id, kind, canonical_name, display_name, usage_count
          from equipment_items
         where kind = $1 and canonical_name = $2
         limit 1
        "#,
        input.kind,
        canonical,
        display
    )
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(Out {
        id: row.id.to_string(),
        kind: row.kind,
        canonical_name: row.canonical_name,
        display_name: row.display_name,
        usage_count: row.usage_count,
    }))
}
```

- [ ] **Step 4: Wire the module in `backend/src/equipment/mod.rs`**

```rust
pub mod autocomplete;
pub mod items_create;
pub mod upsert;
```

- [ ] **Step 5: Wire the route in `backend/src/http/mod.rs`**

Inside the existing router builder, add (next to the existing `/api/equipment/autocomplete` route):

```rust
.route(
    "/api/equipment/items",
    axum::routing::post(crate::equipment::items_create::handler),
)
```

- [ ] **Step 6: Run cargo sqlx prepare**

Run: `cd backend && cargo sqlx prepare`
Expected: writes/updates `.sqlx/` JSON files for the new query.

- [ ] **Step 7: Run the test suite for this file**

Run: `cd backend && cargo test --test equipment_items_create -- --nocapture`
Expected: all three tests pass.

- [ ] **Step 8: Commit**

```bash
git add backend/src/equipment/items_create.rs backend/src/equipment/mod.rs \
        backend/src/http/mod.rs backend/tests/equipment_items_create.rs \
        backend/.sqlx
git commit -m "feat(equipment): POST /api/equipment/items resolve-or-create"
```

---

## Phase 4 — Backend setup CRUD

### Task 5: Setup module skeleton

**Files:**
- Create: `backend/src/equipment/setups/mod.rs`
- Modify: `backend/src/equipment/mod.rs`

- [ ] **Step 1: Create `backend/src/equipment/setups/mod.rs`**

```rust
//! Equipment setups: per-user reusable gear bundles. See
//! docs/superpowers/specs/2026-05-04-equipment-setups-design.md.

pub mod create;
pub mod delete;
pub mod get;
pub mod list;
pub mod update;

const VALID_ROLES: &[&str] = &[
    "optical_tube",
    "focal_modifier",
    "main_camera",
    "mount",
    "filter",
];

pub fn validate_role(role: &str) -> Result<(), crate::error::AppError> {
    if VALID_ROLES.contains(&role) {
        Ok(())
    } else {
        Err(crate::error::AppError::Validation(format!(
            "unknown role '{role}'"
        )))
    }
}
```

- [ ] **Step 2: Re-export in `backend/src/equipment/mod.rs`**

```rust
pub mod autocomplete;
pub mod items_create;
pub mod setups;
pub mod upsert;
```

(Empty handlers will be added in subsequent tasks; cargo check will fail until those land. Combined commit at the end of Task 6.)

### Task 6: GET /api/equipment/setups (list)

**Files:**
- Create: `backend/src/equipment/setups/list.rs`
- Modify: `backend/src/http/mod.rs`
- Create: `backend/tests/equipment_setups.rs` (will host all setup tests)

- [ ] **Step 1: Add failing test in `backend/tests/equipment_setups.rs`**

Set up the harness identically to `equipment_autocomplete.rs` plus a logged-in user (use `tests/common/auth.rs` if it exists, otherwise inline the signup-and-cookie helper from `auth.rs`). Test:

```rust
#[tokio::test]
async fn list_returns_owner_setups_only_with_role_counts() {
    let (app, pool) = harness().await;
    let cookie = signup_and_get_cookie(&app, "alice@example.com").await;
    let alice_id = lookup_user_id(&pool, "alice@example.com").await;
    let bob_id = create_other_user(&pool, "bob@example.com").await;

    // alice has 2 setups, one with two filters; bob has 1 (must not appear).
    let s1 = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name, is_default)
         values ($1, 'Backyard rig', true) returning id",
        alice_id
    ).fetch_one(&pool).await.unwrap();
    let _s2 = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name) values ($1, 'Travel rig') returning id",
        alice_id
    ).fetch_one(&pool).await.unwrap();
    sqlx::query!(
        "insert into equipment_setups (owner_id, name) values ($1, 'Bob rig')",
        bob_id
    ).execute(&pool).await.unwrap();

    // Two filter items in s1.
    let f1 = sqlx::query_scalar!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count)
         values ('filter','ha','Hα',0) returning id"
    ).fetch_one(&pool).await.unwrap();
    let f2 = sqlx::query_scalar!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count)
         values ('filter','oiii','OIII',0) returning id"
    ).fetch_one(&pool).await.unwrap();
    sqlx::query!(
        "insert into setup_items (setup_id, role, item_id) values ($1,'filter',$2),($1,'filter',$3)",
        s1, f1, f2
    ).execute(&pool).await.unwrap();

    let r = app.clone().oneshot(
        Request::builder()
            .uri("/api/equipment/setups")
            .header("cookie", &cookie)
            .body(Body::empty()).unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 200);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(r.into_body(), 1<<20).await.unwrap()
    ).unwrap();
    let arr = body.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    let backyard = arr.iter().find(|v| v["name"] == "Backyard rig").unwrap();
    assert_eq!(backyard["is_default"], true);
    let counts = backyard["item_counts"].as_array().unwrap();
    assert_eq!(counts.len(), 1);
    assert_eq!(counts[0]["role"], "filter");
    assert_eq!(counts[0]["count"], 2);
}
```

- [ ] **Step 2: Run, expect 404**

Run: `cd backend && cargo test --test equipment_setups list_returns_owner_setups_only_with_role_counts -- --nocapture`
Expected: 404 — route not wired.

- [ ] **Step 3: Implement `backend/src/equipment/setups/list.rs`**

```rust
//! GET /api/equipment/setups — caller's setups with per-role counts,
//! newest-updated first.

use axum::{Json, extract::State, response::IntoResponse};

use crate::api_types::{RoleCount, SetupSummary};
use crate::auth::middleware::CurrentUser;
use crate::error::AppError;
use crate::http::AppState;

pub async fn handler(
    State(state): State<AppState>,
    user: CurrentUser,
) -> Result<impl IntoResponse, AppError> {
    let rows = sqlx::query!(
        r#"
        select s.id, s.name, s.description, s.location,
               s.is_remote, s.is_default, s.guiding, s.updated_at,
               coalesce(
                 (select json_agg(json_build_object('role', si.role, 'count', si.cnt))
                    from (
                      select role, count(*) as cnt
                        from setup_items
                       where setup_id = s.id
                       group by role
                    ) as si),
                 '[]'::json
               ) as "item_counts!: serde_json::Value"
          from equipment_setups s
         where s.owner_id = $1
         order by s.updated_at desc
        "#,
        user.id
    )
    .fetch_all(&state.pool)
    .await?;

    let out: Vec<SetupSummary> = rows
        .into_iter()
        .map(|r| SetupSummary {
            id: r.id.to_string(),
            name: r.name,
            description: r.description,
            location: r.location,
            is_remote: r.is_remote,
            is_default: r.is_default,
            guiding: r.guiding,
            updated_at: r.updated_at.to_rfc3339(),
            item_counts: serde_json::from_value::<Vec<RoleCount>>(r.item_counts)
                .unwrap_or_default(),
        })
        .collect();
    Ok(Json(out))
}
```

- [ ] **Step 4: Wire the route in `backend/src/http/mod.rs`**

```rust
.route(
    "/api/equipment/setups",
    axum::routing::get(crate::equipment::setups::list::handler),
)
```

(Below this same line, add a `.post(crate::equipment::setups::create::handler)` chain when Task 7 lands. For now leave just the GET to keep the file compiling — Task 5's empty `pub mod create;` reference will fail; merge the wiring at the end of Task 7.)

- [ ] **Step 5: Provide stubs for the not-yet-implemented handlers**

To keep the build green between tasks, create empty stub files:

```rust
// backend/src/equipment/setups/create.rs
use axum::response::IntoResponse;
use crate::error::AppError;
pub async fn handler() -> Result<impl IntoResponse, AppError> {
    Err::<(), _>(AppError::Validation("not yet implemented".into()))
}
```

Mirror the same shape for `get.rs`, `update.rs`, `delete.rs`. These will be replaced in subsequent tasks.

- [ ] **Step 6: Run sqlx prepare + tests**

Run: `cd backend && cargo sqlx prepare && cargo test --test equipment_setups list_returns_owner_setups_only_with_role_counts -- --nocapture`
Expected: pass.

- [ ] **Step 7: Commit**

```bash
git add backend/src/equipment/setups backend/src/equipment/mod.rs \
        backend/src/http/mod.rs backend/tests/equipment_setups.rs \
        backend/.sqlx
git commit -m "feat(equipment): GET /api/equipment/setups list endpoint"
```

### Task 7: POST /api/equipment/setups (create)

**Files:**
- Modify: `backend/src/equipment/setups/create.rs` (replaces stub)
- Modify: `backend/src/http/mod.rs`
- Modify: `backend/tests/equipment_setups.rs`

- [ ] **Step 1: Add failing test cases**

Append:

```rust
#[tokio::test]
async fn create_persists_setup_with_items_and_clears_other_default() {
    let (app, pool) = harness().await;
    let cookie = signup_and_get_cookie(&app, "alice@example.com").await;
    let alice_id = lookup_user_id(&pool, "alice@example.com").await;

    sqlx::query!(
        "insert into equipment_setups (owner_id, name, is_default) values ($1,'Old default',true)",
        alice_id
    ).execute(&pool).await.unwrap();
    let scope_id = sqlx::query_scalar!(
        "insert into equipment_items (kind,canonical_name,display_name,usage_count)
         values ('telescope','sky-watcher 200p','Sky-Watcher 200P',0) returning id"
    ).fetch_one(&pool).await.unwrap();

    let body = serde_json::json!({
        "name": "Backyard rig",
        "description": null,
        "location": "Paris",
        "is_remote": false,
        "is_default": true,
        "guiding": null,
        "items": [{ "role": "optical_tube", "item_id": scope_id.to_string() }]
    });
    let r = app.clone().oneshot(
        Request::builder()
            .method("POST")
            .uri("/api/equipment/setups")
            .header("content-type", "application/json")
            .header("cookie", &cookie)
            .body(Body::from(body.to_string())).unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 200);

    let n_default: i64 = sqlx::query_scalar!(
        "select count(*) from equipment_setups where owner_id=$1 and is_default", alice_id
    ).fetch_one(&pool).await.unwrap().unwrap();
    assert_eq!(n_default, 1);

    let backyard_default: bool = sqlx::query_scalar!(
        "select is_default from equipment_setups where owner_id=$1 and name='Backyard rig'",
        alice_id
    ).fetch_one(&pool).await.unwrap();
    assert!(backyard_default);
}

#[tokio::test]
async fn create_unknown_item_id_returns_422() {
    let (app, _pool) = harness().await;
    let cookie = signup_and_get_cookie(&app, "alice@example.com").await;
    let body = serde_json::json!({
        "name": "x", "description": null, "location": null,
        "is_remote": false, "is_default": false, "guiding": null,
        "items": [{ "role": "optical_tube",
                    "item_id": "00000000-0000-0000-0000-000000000000" }]
    });
    let r = app.clone().oneshot(
        Request::builder()
            .method("POST")
            .uri("/api/equipment/setups")
            .header("content-type", "application/json")
            .header("cookie", &cookie)
            .body(Body::from(body.to_string())).unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 422);
}
```

- [ ] **Step 2: Run, expect failure**

Run: `cd backend && cargo test --test equipment_setups create_ -- --nocapture`
Expected: 500 (stub) on the first test.

- [ ] **Step 3: Implement `backend/src/equipment/setups/create.rs`**

```rust
//! POST /api/equipment/setups — create a setup with its items.
//! Default-exclusivity enforced in the same transaction (clear-others
//! before insert) so the partial unique idx never trips.

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use uuid::Uuid;

use crate::api_types::{SetupDetail, SetupInput};
use crate::auth::middleware::CurrentUser;
use crate::error::AppError;
use crate::http::AppState;

use super::validate_role;

pub async fn handler(
    State(state): State<AppState>,
    user: CurrentUser,
    Json(input): Json<SetupInput>,
) -> Result<impl IntoResponse, AppError> {
    // Validate items / roles up front so we don't open a transaction needlessly.
    if input.name.trim().is_empty() {
        return Err(AppError::Validation("name is required".into()));
    }
    let mut item_uuids = Vec::with_capacity(input.items.len());
    for it in &input.items {
        validate_role(&it.role)?;
        let uuid = Uuid::parse_str(&it.item_id)
            .map_err(|_| AppError::Validation("item_id is not a uuid".into()))?;
        item_uuids.push(uuid);
    }

    let mut tx = state.pool.begin().await?;

    if input.is_default {
        sqlx::query!(
            "update equipment_setups set is_default = false
             where owner_id = $1 and is_default",
            user.id
        )
        .execute(&mut *tx)
        .await?;
    }

    let row = sqlx::query!(
        r#"insert into equipment_setups
            (owner_id, name, description, location, is_remote, is_default, guiding)
            values ($1,$2,$3,$4,$5,$6,$7)
            returning id, created_at, updated_at"#,
        user.id, input.name.trim(),
        input.description.as_deref(), input.location.as_deref(),
        input.is_remote, input.is_default, input.guiding.as_deref()
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(unique_conflict)?;
    let setup_id = row.id;

    for (i, it) in input.items.iter().enumerate() {
        sqlx::query!(
            "insert into setup_items (setup_id, role, item_id) values ($1,$2,$3)",
            setup_id, it.role, item_uuids[i]
        )
        .execute(&mut *tx)
        .await
        .map_err(unknown_item_to_422)?;
    }

    tx.commit().await?;

    // Re-read for response shape (reuses Task 8's get_one query).
    let detail = super::get::load(&state.pool, user.id, setup_id).await?;
    Ok((StatusCode::CREATED, Json(detail)))
}

fn unique_conflict(e: sqlx::Error) -> AppError {
    if let Some(db) = e.as_database_error() {
        if db.code().as_deref() == Some("23505") {
            return AppError::Validation("a setup with this name already exists".into());
        }
    }
    e.into()
}

fn unknown_item_to_422(e: sqlx::Error) -> AppError {
    if let Some(db) = e.as_database_error() {
        if db.code().as_deref() == Some("23503") {
            return AppError::Validation("unknown item_id".into());
        }
    }
    e.into()
}
```

- [ ] **Step 4: Wire POST onto the existing route**

In `backend/src/http/mod.rs`, change the existing `/api/equipment/setups` GET binding into a method-router:

```rust
.route(
    "/api/equipment/setups",
    axum::routing::get(crate::equipment::setups::list::handler)
        .post(crate::equipment::setups::create::handler),
)
```

- [ ] **Step 5: sqlx prepare + run**

Run: `cd backend && cargo sqlx prepare && cargo test --test equipment_setups create_ -- --nocapture`
Expected: both tests pass.

- [ ] **Step 6: Commit**

```bash
git add backend/src/equipment/setups/create.rs backend/src/http/mod.rs \
        backend/tests/equipment_setups.rs backend/.sqlx
git commit -m "feat(equipment): POST /api/equipment/setups create endpoint"
```

### Task 8: GET /api/equipment/setups/:id (detail)

**Files:**
- Modify: `backend/src/equipment/setups/get.rs` (replaces stub)
- Modify: `backend/src/http/mod.rs`
- Modify: `backend/tests/equipment_setups.rs`

- [ ] **Step 1: Add failing tests**

```rust
#[tokio::test]
async fn get_one_returns_full_expansion() {
    let (app, pool) = harness().await;
    let cookie = signup_and_get_cookie(&app, "alice@example.com").await;
    let alice_id = lookup_user_id(&pool, "alice@example.com").await;
    let setup_id = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name) values ($1,'Backyard rig') returning id",
        alice_id
    ).fetch_one(&pool).await.unwrap();
    let scope_id = sqlx::query_scalar!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count)
         values ('telescope','sky-watcher 200p','Sky-Watcher 200P',0) returning id"
    ).fetch_one(&pool).await.unwrap();
    sqlx::query!(
        "insert into setup_items (setup_id, role, item_id) values ($1,'optical_tube',$2)",
        setup_id, scope_id
    ).execute(&pool).await.unwrap();

    let r = app.clone().oneshot(
        Request::builder()
            .uri(&format!("/api/equipment/setups/{setup_id}"))
            .header("cookie", &cookie)
            .body(Body::empty()).unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 200);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(r.into_body(), 1<<20).await.unwrap()
    ).unwrap();
    assert_eq!(body["name"], "Backyard rig");
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["role"], "optical_tube");
    assert_eq!(items[0]["item"]["display_name"], "Sky-Watcher 200P");
}

#[tokio::test]
async fn get_one_returns_404_for_other_user() {
    let (app, pool) = harness().await;
    let alice_cookie = signup_and_get_cookie(&app, "alice@example.com").await;
    let bob_id = create_other_user(&pool, "bob@example.com").await;
    let bob_setup = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name) values ($1,'Bob rig') returning id",
        bob_id
    ).fetch_one(&pool).await.unwrap();

    let r = app.clone().oneshot(
        Request::builder()
            .uri(&format!("/api/equipment/setups/{bob_setup}"))
            .header("cookie", &alice_cookie)
            .body(Body::empty()).unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 404);
}
```

- [ ] **Step 2: Run, expect 500/404**

Run: `cd backend && cargo test --test equipment_setups get_one -- --nocapture`

- [ ] **Step 3: Implement `backend/src/equipment/setups/get.rs`**

```rust
//! GET /api/equipment/setups/:id — full setup detail with item expansion.

use axum::{Json, extract::{Path, State}, response::IntoResponse};
use sqlx::PgPool;
use uuid::Uuid;

use crate::api_types::{EquipmentItemRef, SetupDetail, SetupItem};
use crate::auth::middleware::CurrentUser;
use crate::error::AppError;
use crate::http::AppState;

pub async fn handler(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let detail = load(&state.pool, user.id, id).await?;
    Ok(Json(detail))
}

/// Shared loader — also used by create/update to return the response body.
pub async fn load(pool: &PgPool, owner_id: Uuid, id: Uuid) -> Result<SetupDetail, AppError> {
    let s = sqlx::query!(
        r#"select id, name, description, location, is_remote, is_default,
                  guiding, created_at, updated_at
             from equipment_setups
            where id = $1 and owner_id = $2"#,
        id, owner_id
    )
    .fetch_optional(pool).await?
    .ok_or_else(|| AppError::NotFound("setup not found".into()))?;

    let items = sqlx::query!(
        r#"select si.role,
                  ei.id, ei.kind, ei.canonical_name, ei.display_name
             from setup_items si
             join equipment_items ei on ei.id = si.item_id
            where si.setup_id = $1
            order by si.role, ei.canonical_name"#,
        id
    )
    .fetch_all(pool).await?;

    Ok(SetupDetail {
        id: s.id.to_string(),
        name: s.name,
        description: s.description,
        location: s.location,
        is_remote: s.is_remote,
        is_default: s.is_default,
        guiding: s.guiding,
        created_at: s.created_at.to_rfc3339(),
        updated_at: s.updated_at.to_rfc3339(),
        items: items.into_iter().map(|r| SetupItem {
            role: r.role,
            item: EquipmentItemRef {
                id: r.id.to_string(),
                kind: r.kind,
                canonical_name: r.canonical_name,
                display_name: r.display_name,
            },
        }).collect(),
    })
}
```

(If `AppError::NotFound` doesn't exist, add it as a thin variant in `backend/src/error.rs` with a 404 IntoResponse mapping. Confirm by reading `error.rs` first.)

- [ ] **Step 4: Wire route**

```rust
.route(
    "/api/equipment/setups/:id",
    axum::routing::get(crate::equipment::setups::get::handler),
)
```

- [ ] **Step 5: sqlx prepare + run**

Run: `cd backend && cargo sqlx prepare && cargo test --test equipment_setups get_one -- --nocapture`

- [ ] **Step 6: Commit**

```bash
git add backend/src/equipment/setups/get.rs backend/src/http/mod.rs \
        backend/tests/equipment_setups.rs backend/.sqlx \
        backend/src/error.rs   # if you needed to add NotFound
git commit -m "feat(equipment): GET /api/equipment/setups/:id detail endpoint"
```

### Task 9: PATCH /api/equipment/setups/:id (update with replace-all items)

**Files:**
- Modify: `backend/src/equipment/setups/update.rs` (replaces stub)
- Modify: `backend/src/http/mod.rs`
- Modify: `backend/tests/equipment_setups.rs`

- [ ] **Step 1: Add failing tests**

```rust
#[tokio::test]
async fn update_replaces_items_and_meta() {
    let (app, pool) = harness().await;
    let cookie = signup_and_get_cookie(&app, "alice@example.com").await;
    let alice_id = lookup_user_id(&pool, "alice@example.com").await;
    let setup_id = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name, location)
         values ($1,'Backyard','Paris') returning id",
        alice_id
    ).fetch_one(&pool).await.unwrap();
    let i1 = sqlx::query_scalar!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count)
         values ('telescope','sky-watcher 200p','Sky-Watcher 200P',0) returning id"
    ).fetch_one(&pool).await.unwrap();
    let i2 = sqlx::query_scalar!(
        "insert into equipment_items (kind, canonical_name, display_name, usage_count)
         values ('camera','asi2600','ZWO ASI2600',0) returning id"
    ).fetch_one(&pool).await.unwrap();
    sqlx::query!(
        "insert into setup_items (setup_id, role, item_id) values ($1,'optical_tube',$2)",
        setup_id, i1
    ).execute(&pool).await.unwrap();

    let body = serde_json::json!({
        "name": "Backyard rig v2",
        "description": null,
        "location": "Paris",
        "is_remote": false,
        "is_default": false,
        "guiding": null,
        "items": [
            { "role": "optical_tube", "item_id": i1.to_string() },
            { "role": "main_camera",  "item_id": i2.to_string() }
        ]
    });
    let r = app.clone().oneshot(
        Request::builder()
            .method("PATCH")
            .uri(&format!("/api/equipment/setups/{setup_id}"))
            .header("content-type", "application/json")
            .header("cookie", &cookie)
            .body(Body::from(body.to_string())).unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 200);

    let n_items: i64 = sqlx::query_scalar!(
        "select count(*) from setup_items where setup_id=$1", setup_id
    ).fetch_one(&pool).await.unwrap().unwrap();
    assert_eq!(n_items, 2);
    let new_name: String = sqlx::query_scalar!(
        "select name from equipment_setups where id=$1", setup_id
    ).fetch_one(&pool).await.unwrap();
    assert_eq!(new_name, "Backyard rig v2");
}

#[tokio::test]
async fn update_promote_to_default_clears_previous() {
    let (app, pool) = harness().await;
    let cookie = signup_and_get_cookie(&app, "alice@example.com").await;
    let alice_id = lookup_user_id(&pool, "alice@example.com").await;
    sqlx::query!(
        "insert into equipment_setups (owner_id, name, is_default) values ($1,'Old',true)",
        alice_id
    ).execute(&pool).await.unwrap();
    let new_id = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name) values ($1,'New') returning id",
        alice_id
    ).fetch_one(&pool).await.unwrap();

    let body = serde_json::json!({
        "name": "New", "description": null, "location": null,
        "is_remote": false, "is_default": true, "guiding": null,
        "items": []
    });
    let r = app.clone().oneshot(
        Request::builder()
            .method("PATCH")
            .uri(&format!("/api/equipment/setups/{new_id}"))
            .header("content-type", "application/json")
            .header("cookie", &cookie)
            .body(Body::from(body.to_string())).unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 200);
    let n_default: i64 = sqlx::query_scalar!(
        "select count(*) from equipment_setups where owner_id=$1 and is_default", alice_id
    ).fetch_one(&pool).await.unwrap().unwrap();
    assert_eq!(n_default, 1);
}

#[tokio::test]
async fn update_returns_404_for_other_user() {
    let (app, pool) = harness().await;
    let alice_cookie = signup_and_get_cookie(&app, "alice@example.com").await;
    let bob_id = create_other_user(&pool, "bob@example.com").await;
    let bob_setup = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name) values ($1,'Bob') returning id",
        bob_id
    ).fetch_one(&pool).await.unwrap();
    let body = serde_json::json!({
        "name": "Hacked", "description": null, "location": null,
        "is_remote": false, "is_default": false, "guiding": null, "items": []
    });
    let r = app.clone().oneshot(
        Request::builder()
            .method("PATCH")
            .uri(&format!("/api/equipment/setups/{bob_setup}"))
            .header("content-type", "application/json")
            .header("cookie", &alice_cookie)
            .body(Body::from(body.to_string())).unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 404);
}
```

- [ ] **Step 2: Run, expect failures**

- [ ] **Step 3: Implement update**

```rust
//! PATCH /api/equipment/setups/:id — meta + items replace-all.

use axum::{Json, extract::{Path, State}, response::IntoResponse};
use uuid::Uuid;

use crate::api_types::{SetupDetail, SetupInput};
use crate::auth::middleware::CurrentUser;
use crate::error::AppError;
use crate::http::AppState;

use super::{validate_role};

pub async fn handler(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
    Json(input): Json<SetupInput>,
) -> Result<impl IntoResponse, AppError> {
    if input.name.trim().is_empty() {
        return Err(AppError::Validation("name is required".into()));
    }
    let mut item_uuids = Vec::with_capacity(input.items.len());
    for it in &input.items {
        validate_role(&it.role)?;
        let uuid = Uuid::parse_str(&it.item_id)
            .map_err(|_| AppError::Validation("item_id is not a uuid".into()))?;
        item_uuids.push(uuid);
    }

    let mut tx = state.pool.begin().await?;

    let exists: Option<(Uuid,)> = sqlx::query_as(
        "select id from equipment_setups where id = $1 and owner_id = $2 for update"
    )
    .bind(id).bind(user.id)
    .fetch_optional(&mut *tx).await?;
    if exists.is_none() {
        return Err(AppError::NotFound("setup not found".into()));
    }

    if input.is_default {
        sqlx::query!(
            "update equipment_setups set is_default=false
             where owner_id=$1 and is_default and id<>$2",
            user.id, id
        ).execute(&mut *tx).await?;
    }

    sqlx::query!(
        r#"update equipment_setups
              set name=$1, description=$2, location=$3,
                  is_remote=$4, is_default=$5, guiding=$6,
                  updated_at=now()
            where id=$7"#,
        input.name.trim(), input.description.as_deref(), input.location.as_deref(),
        input.is_remote, input.is_default, input.guiding.as_deref(), id
    ).execute(&mut *tx).await
        .map_err(super::create_unique_conflict_to_422)?; // see note below

    sqlx::query!("delete from setup_items where setup_id=$1", id)
        .execute(&mut *tx).await?;

    for (i, it) in input.items.iter().enumerate() {
        sqlx::query!(
            "insert into setup_items (setup_id, role, item_id) values ($1,$2,$3)",
            id, it.role, item_uuids[i]
        ).execute(&mut *tx).await
            .map_err(super::create_unknown_item_to_422)?;
    }

    tx.commit().await?;
    Ok(Json(super::get::load(&state.pool, user.id, id).await?))
}
```

Note: lift the two error-mapping helpers out of `create.rs` into `mod.rs` (rename to `unique_conflict_to_422` and `unknown_item_to_422`, make them `pub(super)`), so both create and update use them. Update `create.rs` accordingly. This avoids duplication.

- [ ] **Step 4: Wire PATCH**

In `http/mod.rs`, extend the per-id route:

```rust
.route(
    "/api/equipment/setups/:id",
    axum::routing::get(crate::equipment::setups::get::handler)
        .patch(crate::equipment::setups::update::handler),
)
```

- [ ] **Step 5: sqlx prepare + run**

- [ ] **Step 6: Commit**

```bash
git add backend/src/equipment/setups backend/src/http/mod.rs \
        backend/tests/equipment_setups.rs backend/.sqlx
git commit -m "feat(equipment): PATCH /api/equipment/setups/:id update endpoint"
```

### Task 10: DELETE /api/equipment/setups/:id

**Files:**
- Modify: `backend/src/equipment/setups/delete.rs` (replaces stub)
- Modify: `backend/src/http/mod.rs`
- Modify: `backend/tests/equipment_setups.rs`

- [ ] **Step 1: Add failing tests**

```rust
#[tokio::test]
async fn delete_clears_photos_setup_id_but_keeps_denorm_columns() {
    let (app, pool) = harness().await;
    let cookie = signup_and_get_cookie(&app, "alice@example.com").await;
    let alice_id = lookup_user_id(&pool, "alice@example.com").await;
    let setup_id = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name) values ($1,'X') returning id",
        alice_id
    ).fetch_one(&pool).await.unwrap();

    // Insert a stub photo referencing the setup. Adapt minimal fields to
    // your photos schema; helpers in tests/common may already do this.
    let photo_id = insert_stub_photo(
        &pool, alice_id,
        Some(setup_id), Some("Sky-Watcher 200P".into())  // (setup_id, scope)
    ).await;

    let r = app.clone().oneshot(
        Request::builder()
            .method("DELETE")
            .uri(&format!("/api/equipment/setups/{setup_id}"))
            .header("cookie", &cookie)
            .body(Body::empty()).unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 204);

    let row = sqlx::query!(
        "select setup_id, scope from photos where id=$1", photo_id
    ).fetch_one(&pool).await.unwrap();
    assert_eq!(row.setup_id, None);
    assert_eq!(row.scope.as_deref(), Some("Sky-Watcher 200P"));
}

#[tokio::test]
async fn delete_returns_404_for_other_user() {
    let (app, pool) = harness().await;
    let alice_cookie = signup_and_get_cookie(&app, "alice@example.com").await;
    let bob_id = create_other_user(&pool, "bob@example.com").await;
    let bob_setup = sqlx::query_scalar!(
        "insert into equipment_setups (owner_id, name) values ($1,'Bob') returning id",
        bob_id
    ).fetch_one(&pool).await.unwrap();
    let r = app.clone().oneshot(
        Request::builder()
            .method("DELETE")
            .uri(&format!("/api/equipment/setups/{bob_setup}"))
            .header("cookie", &alice_cookie)
            .body(Body::empty()).unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 404);
}
```

If `insert_stub_photo` isn't in `tests/common`, define it locally inside `equipment_setups.rs` — it just needs to insert a row with the minimal not-null columns of `photos` (id, owner_id, storage_key, original_name, bytes, original_uploaded_at, short_id, last_step) plus the optional setup_id and scope you pass in.

- [ ] **Step 2-5: Implement, wire, sqlx prepare, test**

```rust
//! DELETE /api/equipment/setups/:id

use axum::{extract::{Path, State}, http::StatusCode, response::IntoResponse};
use uuid::Uuid;

use crate::auth::middleware::CurrentUser;
use crate::error::AppError;
use crate::http::AppState;

pub async fn handler(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let res = sqlx::query!(
        "delete from equipment_setups where id=$1 and owner_id=$2",
        id, user.id
    ).execute(&state.pool).await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("setup not found".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}
```

Wire:
```rust
.route(
    "/api/equipment/setups/:id",
    axum::routing::get(crate::equipment::setups::get::handler)
        .patch(crate::equipment::setups::update::handler)
        .delete(crate::equipment::setups::delete::handler),
)
```

- [ ] **Step 6: Commit**

```bash
git add backend/src/equipment/setups/delete.rs backend/src/http/mod.rs \
        backend/tests/equipment_setups.rs backend/.sqlx
git commit -m "feat(equipment): DELETE /api/equipment/setups/:id endpoint"
```

---

## Phase 5 — Backend apply / detach + photo read-side updates

### Task 11: Photo read paths surface setup_id and focal_modifier

**Files:**
- Modify: `backend/src/photos/queries.rs` (every SELECT touching the photo row needs the two new columns)
- Modify: `backend/src/photos/get.rs`, `backend/src/photos/list.rs`, `backend/src/photos/metadata.rs` (response shape)
- Modify: `backend/src/api_types.rs` (PhotoDetail gains `setup_id: Option<String>` and `focal_modifier: Option<String>`)
- Modify: `backend/src/photos/metadata.rs` (PUT body accepts `focal_modifier: Option<Option<String>>` patch field)
- Modify: `backend/tests/photos.rs` (or wherever the photo-shape assertions live) — add the new fields

- [ ] **Step 1: Read `backend/src/photos/queries.rs` end-to-end**

Don't skim. Every photo read query is here. List the queries you'll need to touch.

- [ ] **Step 2: Add failing test for round-trip**

In `backend/tests/photos.rs` (or a new test file), assert that a PUT-PATCH with `focal_modifier: "Antares 0.7x Reducer"` is read back via GET.

- [ ] **Step 3: Add the two columns to every photo SELECT**

Tedious but mechanical. In each query that returns a photo row, add `setup_id`, `focal_modifier` to the column list and the corresponding struct.

- [ ] **Step 4: Add `focal_modifier` patch in `metadata.rs`**

In `MetadataUpdate`, add:
```rust
#[serde(default, with = "::serde_with::rust::double_option")]
pub focal_modifier: Option<Option<String>>,
```
Then thread it into the existing UPDATE statement following the same pattern as `scope`/`mount`. Also add it to the equipment_items upsert fan-out (as `kind = 'focal_modifier'`).

- [ ] **Step 5: Update PhotoDetail in `api_types.rs`**

Add `setup_id: Option<String>` and `focal_modifier: Option<String>` and re-run `just types`.

- [ ] **Step 6: sqlx prepare + cargo test**

Run: `cd backend && cargo sqlx prepare && cargo test`
Expected: full backend test suite green.

- [ ] **Step 7: Commit**

```bash
git add backend backend/.sqlx frontend/src/lib/api
git commit -m "feat(photos): surface setup_id and focal_modifier in photo APIs"
```

### Task 12: POST /api/photos/:id/apply-setup and detach-setup

**Files:**
- Create: `backend/src/photos/apply_setup.rs`
- Modify: `backend/src/photos/mod.rs`
- Modify: `backend/src/http/mod.rs`
- Create: `backend/tests/photos_apply_setup.rs`

- [ ] **Step 1: Add failing tests**

Cover all five rows in the spec's test plan:
- fill-empty preserves EXIF camera, fills empty columns, sets setup_id
- overwrite writes everything verbatim
- detach clears setup_id only
- cross-user setup → 404
- cross-user photo → 404

- [ ] **Step 2: Run tests, expect 404 (route not wired)**

- [ ] **Step 3: Implement `backend/src/photos/apply_setup.rs`**

```rust
//! POST /api/photos/:id/apply-setup       { setup_id, mode }
//! POST /api/photos/:id/detach-setup
//!
//! Two handlers, one file. mode = "fill_empty" | "overwrite".

use axum::{Json, extract::{Path, State}, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use uuid::Uuid;

use crate::api_types::ApplySetupInput;
use crate::auth::middleware::CurrentUser;
use crate::error::AppError;
use crate::http::AppState;

#[derive(Serialize)]
pub struct AppliedOut {
    pub setup_id: Option<String>,
    pub scope: Option<String>,
    pub focal_modifier: Option<String>,
    pub camera: Option<String>,
    pub mount: Option<String>,
    pub filters: Option<String>,
    pub guiding: Option<String>,
}

pub async fn apply(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(photo_id): Path<Uuid>,
    Json(input): Json<ApplySetupInput>,
) -> Result<impl IntoResponse, AppError> {
    let setup_uuid = Uuid::parse_str(&input.setup_id)
        .map_err(|_| AppError::Validation("setup_id is not a uuid".into()))?;
    let mode_overwrite = match input.mode.as_str() {
        "fill_empty" => false,
        "overwrite" => true,
        _ => return Err(AppError::Validation(
            "mode must be 'fill_empty' or 'overwrite'".into(),
        )),
    };

    let mut tx = state.pool.begin().await?;

    // Confirm both photo and setup belong to caller.
    let setup = sqlx::query!(
        "select guiding from equipment_setups where id=$1 and owner_id=$2",
        setup_uuid, user.id
    ).fetch_optional(&mut *tx).await?
        .ok_or_else(|| AppError::NotFound("setup not found".into()))?;

    let owns_photo: Option<(Uuid,)> = sqlx::query_as(
        "select id from photos where id=$1 and owner_id=$2 for update"
    ).bind(photo_id).bind(user.id)
        .fetch_optional(&mut *tx).await?;
    if owns_photo.is_none() {
        return Err(AppError::NotFound("photo not found".into()));
    }

    // Resolve canonical names from setup_items.
    let items = sqlx::query!(
        r#"select si.role, ei.display_name
             from setup_items si
             join equipment_items ei on ei.id = si.item_id
            where si.setup_id = $1
            order by si.role, ei.canonical_name"#,
        setup_uuid
    ).fetch_all(&mut *tx).await?;

    let mut scope: Option<String> = None;
    let mut focal_mod: Option<String> = None;
    let mut camera: Option<String> = None;
    let mut mount: Option<String> = None;
    let mut filters_buf: Vec<String> = vec![];
    for r in items {
        match r.role.as_str() {
            "optical_tube" => scope = Some(r.display_name),
            "focal_modifier" => focal_mod = Some(r.display_name),
            "main_camera" => camera = Some(r.display_name),
            "mount" => mount = Some(r.display_name),
            "filter" => filters_buf.push(r.display_name),
            _ => {}
        }
    }
    let filters = if filters_buf.is_empty() { None } else {
        Some(filters_buf.join(", "))
    };
    let guiding = setup.guiding;

    let updated = sqlx::query!(
        r#"
        update photos
           set scope          = case when $2::bool or scope is null
                                          or scope = '' then $3 else scope end,
               focal_modifier = case when $2::bool or focal_modifier is null
                                          or focal_modifier = '' then $4 else focal_modifier end,
               camera         = case when $2::bool or camera is null
                                          or camera = '' then $5 else camera end,
               mount          = case when $2::bool or mount is null
                                          or mount = '' then $6 else mount end,
               filters        = case when $2::bool or filters is null
                                          or filters = '' then $7 else filters end,
               guiding        = case when $2::bool or guiding is null
                                          or guiding = '' then $8 else guiding end,
               setup_id       = $9,
               updated_at     = now()
         where id = $1
       returning setup_id, scope, focal_modifier, camera, mount, filters, guiding
        "#,
        photo_id, mode_overwrite,
        scope, focal_mod, camera, mount, filters, guiding,
        setup_uuid
    ).fetch_one(&mut *tx).await?;

    tx.commit().await?;

    Ok(Json(AppliedOut {
        setup_id: updated.setup_id.map(|u| u.to_string()),
        scope: updated.scope,
        focal_modifier: updated.focal_modifier,
        camera: updated.camera,
        mount: updated.mount,
        filters: updated.filters,
        guiding: updated.guiding,
    }))
}

pub async fn detach(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(photo_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let res = sqlx::query!(
        "update photos set setup_id=null, updated_at=now()
         where id=$1 and owner_id=$2",
        photo_id, user.id
    ).execute(&state.pool).await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound("photo not found".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}
```

(Note: the existing `photos.updated_at` column may or may not exist — check `0001_init.sql` and the photos struct first. If absent, drop the `updated_at = now()` clause from both queries.)

- [ ] **Step 4: Wire module + routes**

In `backend/src/photos/mod.rs`:
```rust
pub mod apply_setup;
```

In `backend/src/http/mod.rs`:
```rust
.route(
    "/api/photos/:id/apply-setup",
    axum::routing::post(crate::photos::apply_setup::apply),
)
.route(
    "/api/photos/:id/detach-setup",
    axum::routing::post(crate::photos::apply_setup::detach),
)
```

- [ ] **Step 5: sqlx prepare + tests**

- [ ] **Step 6: Commit**

```bash
git add backend/src/photos/apply_setup.rs backend/src/photos/mod.rs \
        backend/src/http/mod.rs backend/tests/photos_apply_setup.rs \
        backend/.sqlx
git commit -m "feat(photos): apply-setup + detach-setup endpoints"
```

---

## Phase 6 — Frontend: settings/equipment CRUD

### Task 13: Extend EquipmentAutocomplete to support focal_modifier and resolve-or-create on commit

**Files:**
- Modify: `frontend/src/lib/components/EquipmentAutocomplete.svelte`

- [ ] **Step 1: Extend the `EquipmentKind` union**

Replace:
```ts
type EquipmentKind = 'telescope' | 'camera' | 'mount' | 'filter' | 'guiding';
```
with:
```ts
type EquipmentKind = 'telescope' | 'camera' | 'mount' | 'filter' | 'focal_modifier' | 'guiding';
```

- [ ] **Step 2: Add an optional `onCommit` callback**

The setup form needs the canonical `item_id` after the user picks (or types) a value. Add:

```ts
interface Props {
  name: string;
  kind: EquipmentKind;
  value?: string;
  api?: string;
  /**
   * Called when the user has finalized a value (selected from the
   * dropdown OR blurred a free-typed value). The component performs
   * a resolve-or-create against `POST /api/equipment/items` and emits
   * the canonical row's `id`. Skipped for empty values.
   */
  onCommit?: (item: { id: string; display_name: string } | null) => void;
}
```

Hook into the existing `select()` (call `commit(item)`) and the `onBlur()` handler (if there's a non-empty value that wasn't selected, call `POST /api/equipment/items` with the typed `display_name` and emit the resulting `{ id, display_name }`). Skip for `kind = 'guiding'` — the picker isn't used there in the new flow, but defensive guard against accidental wiring.

- [ ] **Step 3: Type-check**

Run: `cd frontend && pnpm check`
Expected: green.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/lib/components/EquipmentAutocomplete.svelte
git commit -m "feat(frontend): EquipmentAutocomplete supports focal_modifier + onCommit"
```

### Task 14: Shared SetupForm.svelte component

**Files:**
- Create: `frontend/src/lib/components/SetupForm.svelte`

- [ ] **Step 1: Implement the form**

```svelte
<script lang="ts">
  import EquipmentAutocomplete from './EquipmentAutocomplete.svelte';
  import type { SetupDetail } from '$lib/api/SetupDetail';
  import type { SetupInput } from '$lib/api/SetupInput';

  type Committed = { id: string; display_name: string };

  interface Props {
    initial: SetupDetail | null;
    submitLabel?: string;
    onsubmit: (input: SetupInput) => void;
    oncancel?: () => void;
  }
  let { initial, submitLabel = 'Save', onsubmit, oncancel }: Props = $props();

  function pickItem(role: string): Committed | null {
    if (!initial) return null;
    const it = initial.items.find((x) => x.role === role);
    return it ? { id: it.item.id, display_name: it.item.display_name } : null;
  }
  function pickFilters(): Committed[] {
    return (initial?.items ?? [])
      .filter((x) => x.role === 'filter')
      .map((x) => ({ id: x.item.id, display_name: x.item.display_name }));
  }

  let name = $state(initial?.name ?? '');
  let description = $state(initial?.description ?? '');
  let location = $state(initial?.location ?? '');
  let is_remote = $state(initial?.is_remote ?? false);
  let is_default = $state(initial?.is_default ?? false);
  let guiding = $state(initial?.guiding ?? '');

  let optical = $state<Committed | null>(pickItem('optical_tube'));
  let focal = $state<Committed | null>(pickItem('focal_modifier'));
  let camera = $state<Committed | null>(pickItem('main_camera'));
  let mount = $state<Committed | null>(pickItem('mount'));
  let filters = $state<Committed[]>(pickFilters());

  // Free-typed values for the autocomplete fields. Bound separately
  // from the committed canonical id so the user can re-type without
  // losing the existing selection until they re-commit.
  let opticalText = $state(optical?.display_name ?? '');
  let focalText = $state(focal?.display_name ?? '');
  let cameraText = $state(camera?.display_name ?? '');
  let mountText = $state(mount?.display_name ?? '');
  let filterText = $state('');

  function addFilter(c: Committed | null) {
    if (!c) return;
    if (filters.some((f) => f.id === c.id)) return;
    filters = [...filters, c];
    filterText = '';
  }
  function removeFilter(id: string) {
    filters = filters.filter((f) => f.id !== id);
  }

  let error = $state<string | null>(null);
  function submit() {
    if (!name.trim()) {
      error = 'Name is required';
      return;
    }
    error = null;
    const items: SetupInput['items'] = [];
    if (optical) items.push({ role: 'optical_tube', item_id: optical.id });
    if (focal) items.push({ role: 'focal_modifier', item_id: focal.id });
    if (camera) items.push({ role: 'main_camera', item_id: camera.id });
    if (mount) items.push({ role: 'mount', item_id: mount.id });
    for (const f of filters) items.push({ role: 'filter', item_id: f.id });
    onsubmit({
      name: name.trim(),
      description: description.trim() || null,
      location: location.trim() || null,
      is_remote,
      is_default,
      guiding: guiding.trim() || null,
      items
    });
  }
</script>

<form onsubmit={(e) => { e.preventDefault(); submit(); }}>
  <label>Name <input bind:value={name} required /></label>
  <label>Description <textarea bind:value={description}></textarea></label>
  <label>Location <input bind:value={location} /></label>
  <label><input type="checkbox" bind:checked={is_remote} /> Remote</label>
  <label><input type="checkbox" bind:checked={is_default} /> Default — auto-applied to new uploads</label>

  <fieldset>
    <legend>Equipment</legend>

    <EquipmentAutocomplete
      name="optical_tube" kind="telescope"
      bind:value={opticalText}
      onCommit={(c) => (optical = c)}
    />
    <EquipmentAutocomplete
      name="focal_modifier" kind="focal_modifier"
      bind:value={focalText}
      onCommit={(c) => (focal = c)}
    />
    <EquipmentAutocomplete
      name="main_camera" kind="camera"
      bind:value={cameraText}
      onCommit={(c) => (camera = c)}
    />
    <EquipmentAutocomplete
      name="mount" kind="mount"
      bind:value={mountText}
      onCommit={(c) => (mount = c)}
    />

    <div>
      <label>Filters</label>
      <ul>
        {#each filters as f (f.id)}
          <li>
            {f.display_name}
            <button type="button" onclick={() => removeFilter(f.id)}>×</button>
          </li>
        {/each}
      </ul>
      <EquipmentAutocomplete
        name="filter" kind="filter"
        bind:value={filterText}
        onCommit={(c) => addFilter(c)}
      />
    </div>

    <label>Guiding (free text)
      <input bind:value={guiding} placeholder="e.g., ASI120MM Mini + 60mm guide scope" />
    </label>
  </fieldset>

  {#if error}<p class="form-error">{error}</p>{/if}

  <div class="actions">
    {#if oncancel}<button type="button" onclick={() => oncancel?.()}>Cancel</button>{/if}
    <button type="submit">{submitLabel}</button>
  </div>
</form>
```

Submit dispatches via the `onsubmit` callback prop with the built `SetupInput` (matches the backend DTO exactly). Cancel via `oncancel`. This component is used unchanged by both the `new` and `edit` pages.

- [ ] **Step 2: Type-check**

Run: `cd frontend && pnpm check`

- [ ] **Step 3: Commit**

```bash
git add frontend/src/lib/components/SetupForm.svelte
git commit -m "feat(frontend): SetupForm shared component"
```

### Task 15: /settings/equipment list page

**Files:**
- Create: `frontend/src/routes/settings/equipment/+page.server.ts`
- Create: `frontend/src/routes/settings/equipment/+page.svelte`

- [ ] **Step 1: Implement the load**

```ts
// +page.server.ts
import { error, redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';

const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';

export const load: PageServerLoad = async ({ locals, fetch, cookies }) => {
  if (!locals.user) redirect(303, '/signin');
  const cookie = cookies.getAll().map((c) => `${c.name}=${c.value}`).join('; ');
  const r = await fetch(`${API}/api/equipment/setups`, { headers: { Cookie: cookie } });
  if (!r.ok) error(500, 'Backend error');
  return { setups: await r.json() };
};
```

- [ ] **Step 2: Implement the page**

```svelte
<script lang="ts">
  import { invalidateAll } from '$app/navigation';
  import type { SetupSummary } from '$lib/api/SetupSummary';

  let { data } = $props<{ data: { setups: SetupSummary[] } }>();

  const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? '';

  const ROLE_LABEL: Record<string, string> = {
    optical_tube: 'Telescope',
    focal_modifier: 'Focal modifier',
    main_camera: 'Camera',
    mount: 'Mount',
    filter: 'Filter'
  };

  async function setDefault(s: SetupSummary) {
    // PATCH the setup with everything as-is plus is_default=true.
    // We need full items to send back. Fetch detail first.
    const detail = await (await fetch(`${API}/api/equipment/setups/${s.id}`,
      { credentials: 'include' })).json();
    const body = {
      name: detail.name, description: detail.description,
      location: detail.location, is_remote: detail.is_remote,
      is_default: true, guiding: detail.guiding,
      items: detail.items.map((it: { role: string; item: { id: string } }) =>
        ({ role: it.role, item_id: it.item.id }))
    };
    const r = await fetch(`${API}/api/equipment/setups/${s.id}`, {
      method: 'PATCH',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'include',
      body: JSON.stringify(body)
    });
    if (r.ok) await invalidateAll();
  }

  async function del(s: SetupSummary) {
    if (!confirm(`Delete setup "${s.name}"?`)) return;
    const r = await fetch(`${API}/api/equipment/setups/${s.id}`, {
      method: 'DELETE', credentials: 'include'
    });
    if (r.ok) await invalidateAll();
  }
</script>

<header>
  <h1>Equipment setups</h1>
  <a href="/settings/equipment/new" class="button">+ New setup</a>
</header>

{#if data.setups.length === 0}
  <p>No setups yet.</p>
{:else}
  <ul>
    {#each data.setups as s (s.id)}
      <li>
        <h2>{s.name}</h2>
        {#if s.is_default}<span class="badge">Default</span>{/if}
        {#if s.is_remote}<span class="badge">Remote</span>{/if}
        <p class="meta">
          {#each s.item_counts as c (c.role)}
            <span>{ROLE_LABEL[c.role] ?? c.role} · {c.count}</span>
          {/each}
        </p>
        <p class="updated">Updated {new Date(s.updated_at).toLocaleDateString()}</p>
        <div class="actions">
          {#if !s.is_default}
            <button onclick={() => setDefault(s)}>Set as default</button>
          {/if}
          <a href={`/settings/equipment/${s.id}/edit`}>Edit</a>
          <button onclick={() => del(s)} class="danger">Delete</button>
        </div>
      </li>
    {/each}
  </ul>
{/if}
```

- [ ] **Step 3: Smoke test in browser**

Run: `just dev` and navigate to `/settings/equipment`. Empty state should render (no setups yet).

- [ ] **Step 4: Commit**

```bash
git add frontend/src/routes/settings/equipment
git commit -m "feat(frontend): /settings/equipment list page"
```

### Task 16: /settings/equipment/new

**Files:**
- Create: `frontend/src/routes/settings/equipment/new/+page.server.ts`
- Create: `frontend/src/routes/settings/equipment/new/+page.svelte`

- [ ] **Step 1: server load = auth gate**

```ts
import { redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
export const load: PageServerLoad = async ({ locals }) => {
  if (!locals.user) redirect(303, '/signin');
  return {};
};
```

- [ ] **Step 2: page calls SetupForm**

Mounts `<SetupForm initial={null} on:submit={...} on:cancel={...} />`. On submit, POST to the backend, then redirect to `/settings/equipment` (or the edit page of the newly-created setup, picker user-pref). On error, display an inline message.

- [ ] **Step 3: Test end-to-end in browser**

Create a setup with at least one autocomplete-derived item AND one free-typed item (which exercises the resolve-or-create path).

- [ ] **Step 4: Commit**

```bash
git add frontend/src/routes/settings/equipment/new
git commit -m "feat(frontend): /settings/equipment/new create page"
```

### Task 17: /settings/equipment/[id]/edit

**Files:**
- Create: `frontend/src/routes/settings/equipment/[id]/edit/+page.server.ts`
- Create: `frontend/src/routes/settings/equipment/[id]/edit/+page.svelte`

- [ ] **Step 1: server load fetches the setup detail**

```ts
import { error, redirect } from '@sveltejs/kit';
import type { PageServerLoad } from './$types';
const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? 'http://localhost:8080';
export const load: PageServerLoad = async ({ params, locals, fetch, cookies }) => {
  if (!locals.user) redirect(303, '/signin');
  const cookie = cookies.getAll().map((c) => `${c.name}=${c.value}`).join('; ');
  const r = await fetch(`${API}/api/equipment/setups/${params.id}`, { headers: { Cookie: cookie } });
  if (r.status === 404) error(404, 'Setup not found');
  if (!r.ok) error(500, 'Backend error');
  return { setup: await r.json() };
};
```

- [ ] **Step 2: page mounts SetupForm with initial values**

On submit, PATCH the setup; redirect to `/settings/equipment`.

- [ ] **Step 3: Browser test**

Edit a setup, verify changes persist.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/routes/settings/equipment/[id]
git commit -m "feat(frontend): /settings/equipment/[id]/edit page"
```

---

## Phase 7 — Frontend: upload-verify integration

### Task 18: Add focal_modifier field to verify form

**Files:**
- Modify: `frontend/src/routes/upload/[id]/verify/+page.svelte`
- Modify: `frontend/src/routes/upload/[id]/verify/+page.server.ts`

- [ ] **Step 1: Add `focal_modifier` to the inferred-photo type and `$state`**

Mirror the existing `scope` / `mount` plumbing line-for-line.

- [ ] **Step 2: Render the field**

Add `<EquipmentAutocomplete name="focal_modifier" kind="focal_modifier" bind:value={focal_modifier} />` between scope and mount in the equipment-grid div.

- [ ] **Step 3: Add to `collectPatch`**

```ts
focal_modifier: strOrNull('focal_modifier'),
```

- [ ] **Step 4: Browser smoke test**

Upload a photo, fill focal_modifier, save, reload — value persists.

- [ ] **Step 5: Commit**

```bash
git add frontend/src/routes/upload
git commit -m "feat(upload): focal_modifier field in verify form"
```

### Task 19: SetupPicker.svelte component

**Files:**
- Create: `frontend/src/lib/components/SetupPicker.svelte`

- [ ] **Step 1: Implement the picker**

```svelte
<script lang="ts">
  import type { SetupSummary } from '$lib/api/SetupSummary';
  import type { SetupDetail } from '$lib/api/SetupDetail';

  interface Current {
    scope: string;
    focal_modifier: string;
    camera: string;
    mount: string;
    filters: string;
    guiding: string;
  }

  interface Props {
    setups: SetupSummary[];
    currentSetupId: string | null;
    current: Current;
    onapply: (req: { setup_id: string; mode: 'fill_empty' | 'overwrite' }) => void;
    ondetach: () => void;
  }
  let { setups, currentSetupId, current, onapply, ondetach }: Props = $props();

  const API = (import.meta.env.VITE_API_BASE_URL as string | undefined) ?? '';

  // Track the previous selection so we can revert if the user cancels
  // the confirm dialog. svelte's `<select bind:value>` writes before we
  // can intercept, so we keep our own copy.
  let selected = $state(currentSetupId ?? '');
  let lastConfirmed = currentSetupId ?? '';

  function projectFromDetail(d: SetupDetail): Current {
    let scope = '', focal_modifier = '', camera = '', mount = '';
    const filterNames: string[] = [];
    for (const it of d.items) {
      switch (it.role) {
        case 'optical_tube':   scope = it.item.display_name; break;
        case 'focal_modifier': focal_modifier = it.item.display_name; break;
        case 'main_camera':    camera = it.item.display_name; break;
        case 'mount':          mount = it.item.display_name; break;
        case 'filter':         filterNames.push(it.item.display_name); break;
      }
    }
    filterNames.sort((a, b) => a.localeCompare(b));
    return {
      scope, focal_modifier, camera, mount,
      filters: filterNames.join(', '),
      guiding: d.guiding ?? ''
    };
  }

  function conflictCount(target: Current): number {
    const fields: (keyof Current)[] =
      ['scope','focal_modifier','camera','mount','filters','guiding'];
    let n = 0;
    for (const f of fields) {
      const cur = (current[f] ?? '').trim();
      const next = (target[f] ?? '').trim();
      if (cur && next && cur !== next) n++;
    }
    return n;
  }

  async function onChange(e: Event) {
    const target = e.target as HTMLSelectElement;
    const newId = target.value;
    if (!newId) {
      // "(none)" — same as detach.
      selected = '';
      lastConfirmed = '';
      ondetach();
      return;
    }
    const r = await fetch(`${API}/api/equipment/setups/${newId}`,
      { credentials: 'include' });
    if (!r.ok) {
      target.value = lastConfirmed;
      selected = lastConfirmed;
      return;
    }
    const detail: SetupDetail = await r.json();
    const projected = projectFromDetail(detail);
    const n = conflictCount(projected);
    if (n === 0) {
      lastConfirmed = newId;
      selected = newId;
      onapply({ setup_id: newId, mode: 'fill_empty' });
      return;
    }
    if (confirm(`Replace ${n} field${n > 1 ? 's' : ''}?`)) {
      lastConfirmed = newId;
      selected = newId;
      onapply({ setup_id: newId, mode: 'overwrite' });
    } else {
      target.value = lastConfirmed;
      selected = lastConfirmed;
    }
  }

  function detach() {
    selected = '';
    lastConfirmed = '';
    ondetach();
  }
</script>

<div class="setup-picker">
  <label>
    Setup
    <select bind:value={selected} onchange={onChange}>
      <option value="">(none)</option>
      {#each setups as s (s.id)}
        <option value={s.id}>{s.is_default ? '★ ' : ''}{s.name}</option>
      {/each}
    </select>
  </label>
  {#if selected}
    <button type="button" onclick={detach}>Detach</button>
  {/if}
</div>
```

- [ ] **Step 2: Type-check + smoke test in isolation**

Mount it on `/design` (the existing playground route, if accessible) or just ship it directly.

- [ ] **Step 3: Commit**

```bash
git add frontend/src/lib/components/SetupPicker.svelte
git commit -m "feat(frontend): SetupPicker component"
```

### Task 20: Mount SetupPicker on verify, default fill-empty pre-apply

**Files:**
- Modify: `frontend/src/routes/upload/[id]/verify/+page.server.ts`
- Modify: `frontend/src/routes/upload/[id]/verify/+page.svelte`

- [ ] **Step 1: Server-side default pre-apply on load**

Extend the load function to also fetch the user's setups list. Then: if the photo's `setup_id` is null AND there is a setup with `is_default = true`, server-call `POST /api/photos/:id/apply-setup` with `mode: 'fill_empty'` for the default's id, before reading the photo back. Re-fetch the photo after the apply so the form sees the merged values.

```ts
export const load: PageServerLoad = async ({ params, locals, fetch, cookies }) => {
  if (!locals.user) redirect(303, '/signin');
  const cookie = cookies.getAll().map((c) => `${c.name}=${c.value}`).join('; ');

  let r = await fetch(`${API}/api/photos/${params.id}`, { headers: { Cookie: cookie } });
  if (r.status === 404) error(404, 'Photo not found');
  if (!r.ok) error(500, 'Backend error');
  let photo = await r.json();
  if (photo.owner_id !== locals.user.id) error(404, 'Not found');

  const sr = await fetch(`${API}/api/equipment/setups`, { headers: { Cookie: cookie } });
  const setups = sr.ok ? await sr.json() : [];

  if (!photo.setup_id) {
    const def = setups.find((s: { is_default: boolean }) => s.is_default);
    if (def) {
      const ar = await fetch(`${API}/api/photos/${params.id}/apply-setup`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', Cookie: cookie },
        body: JSON.stringify({ setup_id: def.id, mode: 'fill_empty' })
      });
      if (ar.ok) {
        // Re-read so the form has the merged columns and setup_id.
        r = await fetch(`${API}/api/photos/${params.id}`, { headers: { Cookie: cookie } });
        photo = await r.json();
      }
    }
  }

  return { photo, setups };
};
```

- [ ] **Step 2: Mount the picker**

Above the "Row 3: equipment pickers" block in `+page.svelte`:

```svelte
<SetupPicker
  setups={data.setups}
  currentSetupId={data.photo.setup_id}
  current={{ scope, focal_modifier, camera, mount, filters, guiding }}
  on:apply={onApplySetup}
  on:detach={onDetachSetup}
/>
```

Implement `onApplySetup` and `onDetachSetup` in the script: call the backend apply/detach endpoints, then update the local `$state` values from the response (or use `invalidateAll()`). After apply, the form's six bindings reflect the new values; the user can then edit any of them by hand.

- [ ] **Step 3: Manual browser test (chrome-devtools-mcp)**

Drive the flow with `mcp__chrome-devtools__*`:
1. Sign in.
2. Create a default setup at `/settings/equipment/new` with optical_tube + main_camera + 1 filter.
3. Upload a JPEG, navigate to `/upload/<id>/verify`.
4. Confirm the equipment fields are pre-filled and `setup_id` is set on the photo (cross-check via `/api/photos/<id>` in network tab).
5. Pick a different setup with overlapping fields. Confirm a "Replace N fields?" dialog appears, accept it, see the new values.
6. Detach the setup, see fields untouched but `setup_id` cleared.

- [ ] **Step 4: Commit**

```bash
git add frontend/src/routes/upload
git commit -m "feat(upload): mount SetupPicker with default fill-empty pre-apply"
```

---

## Phase 8 — Final checks

### Task 21: Full quality gate

**Files:** none

- [ ] **Step 1: Run `just check`**

Run: `just check`
Expected: zero output (or only known pre-existing warnings).

- [ ] **Step 2: Run the full test suite**

Run: `just test`
Expected: all backend tests pass.

- [ ] **Step 3: Frontend type check**

Run: `cd frontend && pnpm check`
Expected: green.

- [ ] **Step 4: Final manual smoke test**

Repeat Task 20 step 3 end-to-end on a fresh `just db-reset` to confirm the full flow on a clean DB.

### Task 22: Open PR

**Files:** none

- [ ] **Step 1: Push the branch**

```bash
git push -u origin feat/equipment-setups
```

- [ ] **Step 2: Open the PR**

Use `gh pr create` referencing the spec at
`docs/superpowers/specs/2026-05-04-equipment-setups-design.md`.

---

## Out of scope (do not add to this plan)

These are intentionally deferred per the spec's "Out of scope" section. Reject scope creep aggressively:

- Public profile listing of a user's setups.
- Browse-by-setup page.
- Admin merge / rename tooling for `equipment_items`.
- Canonical specs on items (aperture_mm, focal_length_mm, sensor).
- Image attached to a setup.
- Per-item notes inside `setup_items`.
- Sharing or forking another user's setup.
- Additional roles (focuser, filter_wheel, OAG, computer, rotator, power).
- Filter wheel positions / explicit ordering on `setup_items`.
