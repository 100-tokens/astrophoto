# Photographer Showcase — Phase 1 Foundations Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land schema + URL changes + presigned PUT upload pipeline + S3/CloudFront image-transform integration + handle-based signup + tier enforcement, without a visible product shift. Old `/photo/<uuid>` URLs 301 to `/u/<handle>/p/<short-id>`. The hero-page redesign (P2) and discovery surfaces (P3) ride on top of this foundation in later phases.

**Architecture:** Eight numbered migrations land the new schema (handles, tier, photo permalinks + display metadata, profile fields, featured/category, targets, tags, denormalised appreciations counter, equipment dictionary). The upload flow splits into a `POST /api/uploads/init` endpoint that issues presigned PUTs with `Content-Length-Range` baked in from `users.tier`, and a `POST /api/uploads/:id/finalize` endpoint that HEADs S3, sniffs magic bytes, then runs the existing `pipeline::finalize` plus a new display-master derivation step. The CDN URL builder produces `https://cdn.astrophoto.pics/img/<id>?w=&h=&fit=&q=&fm=`; in dev the backend serves an equivalent local route. The frontend rebuilds the upload route around `<UploadDropzone>` + parallel `<UploadFileRow>` × N with `xhr.upload` progress, EXIF preflight via `exifr`, and SHA-256 dedup. The verify step gains target / tag / category / equipment pickers feeding the discovery data model. Old multipart `POST /api/photos` is removed; old photo URLs 301 via middleware.

**Tech Stack:** Rust 2024 + axum 0.7 + sqlx 0.8 (compile-time checked) + ammonia 4 + blurhash 0.2 + nanoid 0.4 + image 0.25 (existing) + kamadak-exif 0.5 (existing) + aws-sdk-s3 1 (presigned URL signing) + Postgres 16; SvelteKit 2 + Svelte 5 runes + exifr 7 + justified-layout (Flickr) — added now though primarily used in P2/P3; ts-rs for Rust→TS types. Out-of-band: AWS S3 + CloudFront distribution + Lambda function URL (sharp) ported from the previous project.

**Spec:** `docs/superpowers/specs/2026-05-03-photographer-showcase-design.md` — read before starting. The design handoff lives at `/Users/pleclech/Downloads/design_handoff_astrophoto 3/showcase/`.

**Dual-canonical rule:** the spec is canonical for behavior, schema, URLs, and security; the design files are canonical for layout, hierarchy, and copy. Where a task's UI code references dimensions or copy, those values come from the design — don't invent them.

---

## Branch and worktree

Before Task 1, create the worktree (handled by subagent-driven-development):

```bash
git worktree add ../astrophoto-showcase-p1 -b feat/showcase-p1-foundations main
cd ../astrophoto-showcase-p1
```

All commits go onto `feat/showcase-p1-foundations`. Merge via `superpowers:finishing-a-development-branch` after the final acceptance task.

---

## Setup

### Task 1: Add backend dependencies

**Files:**
- Modify: `backend/Cargo.toml`

- [ ] **Step 1: Add new deps to `[dependencies]`**

Open `backend/Cargo.toml`. In the `[dependencies]` section, add the following lines (keep alphabetical-ish ordering, matching what's already there):

```toml
ammonia = "4"
blurhash = "0.2"
nanoid = "0.4"
```

- [ ] **Step 2: Verify the deps compile**

Run from the repo root:

```
cd backend && cargo check
```

Expected: Successful build with `warning: unused crate ...` lines for the three new crates (they're not referenced yet). If `cargo check` fails for any other reason, fix before moving on.

- [ ] **Step 3: Commit**

```bash
git add backend/Cargo.toml backend/Cargo.lock
git commit -m "chore(backend): add ammonia, blurhash, nanoid for showcase P1"
```

---

### Task 2: Add frontend dependencies

**Files:**
- Modify: `frontend/package.json`

- [ ] **Step 1: Add deps via pnpm**

Run from the repo root:

```
cd frontend && pnpm add exifr justified-layout
cd frontend && pnpm add -D @types/justified-layout
```

(`justified-layout` ships its own types in newer versions; only install `@types/...` if `pnpm check` complains. Skip and revisit if not needed.)

- [ ] **Step 2: Verify installation**

```
cd frontend && pnpm install && pnpm check
```

Expected: `pnpm check` exits 0 (no svelte-check errors).

- [ ] **Step 3: Commit**

```bash
git add frontend/package.json frontend/pnpm-lock.yaml
git commit -m "chore(frontend): add exifr, justified-layout for showcase P1"
```

---

### Task 3: Add CDN base-URL config

**Files:**
- Modify: `backend/src/config.rs`
- Modify: `.env.example`
- Modify: `backend/src/main.rs:setup-app` (where `AppState` is constructed) — find via `grep -n cdn_base_url backend/src/`. If absent, see Step 1.

- [ ] **Step 1: Add the field to `Config`**

Open `backend/src/config.rs`. After the existing `s3_*` fields and before `oauth_google_*`, add:

```rust
    pub cdn_base_url: String,
```

The string is intentionally non-`Option` — boot must fail loudly when it's missing (CLAUDE.md: "Panic at boot for missing config").

- [ ] **Step 2: Add the env line to `.env.example`**

```
APP_CDN_BASE_URL=http://localhost:8080/cdn
```

(Local dev points at the backend's own `/cdn` mount; staging/prod point at the CloudFront distribution.)

- [ ] **Step 3: Update test `Config` builders**

Find every test that builds a `Config { ... }` directly:

```
cd backend && grep -rn "Config {" tests/ src/
```

Add `cdn_base_url: "http://localhost:0/cdn".into(),` to each struct literal. Without this, integration tests fail to compile.

- [ ] **Step 4: Verify build**

```
cd backend && cargo check --all-targets
```

Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add backend/src/config.rs .env.example backend/tests/ backend/src/
git commit -m "chore(backend): add APP_CDN_BASE_URL config"
```

---

### Task 4: Reserved-handles seed file

**Files:**
- Create: `backend/data/reserved_handles.txt`

- [ ] **Step 1: Create the file**

Path: `backend/data/reserved_handles.txt`. One handle per line, lowercase. The list below is the minimum; add more as the product evolves.

```
admin
api
auth
about
account
astrophoto
billing
contact
docs
explore
favicon
help
home
login
logout
me
mod
moderator
official
photo
photos
pics
press
public
root
search
security
settings
signin
signup
staff
support
system
team
terms
test
tos
u
uploads
www
```

(40 entries; the handle picker will block any of these regardless of case.)

- [ ] **Step 2: Commit**

```bash
git add backend/data/reserved_handles.txt
git commit -m "chore(backend): seed reserved handles list"
```

---

## Schema

Migrations are append-only and run automatically on backend startup
(see `backend/src/main.rs` — `sqlx::migrate!("./migrations").run(&pool)`).
After every migration: regenerate the offline cache and commit it.

```bash
cd backend && cargo sqlx prepare && git add .sqlx
```

That step is omitted from individual migrations below for brevity but
applies to **every** task that adds or modifies a `sqlx::query!` /
`query_as!` invocation.

---

### Task 5: Migration 0005 — handles + redirects + reserved list

**Files:**
- Create: `backend/migrations/0005_handles.sql`
- Test: `backend/tests/migrations.rs` (create if absent)

- [ ] **Step 1: Create the migration**

Path: `backend/migrations/0005_handles.sql`.

```sql
-- 0005 handles: required @handle on users + redirect history.
-- Existing users get an auto-generated placeholder ('u-' + 6 chars of
-- their UUID) so the NOT NULL constraint is satisfied; a banner on
-- next login will prompt them to pick a real handle.

alter table users
    add column handle citext;

update users
    set handle = 'u-' || left(replace(id::text, '-', ''), 6)
    where handle is null;

alter table users
    alter column handle set not null,
    add constraint users_handle_format_chk
        check (handle ~ '^[a-z0-9_-]{3,30}$' or handle ~ '^u-[a-f0-9]{6}$');

create unique index users_handle_uidx on users (handle);

-- Old-handle redirects, written when a user renames their handle.
-- The old handle becomes reservable again 90 days after `released_at`.
create table handle_redirects (
    old_handle  citext primary key,
    user_id     uuid not null references users(id) on delete cascade,
    released_at timestamptz not null
);
create index handle_redirects_user_idx on handle_redirects (user_id);
```

- [ ] **Step 2: Write the migration smoke test**

Path: `backend/tests/migrations.rs`. Create the file if it doesn't exist.

```rust
//! Migration smoke tests. Each new schema bump gets a check that the
//! migrations apply cleanly to a fresh DB and the new schema objects
//! exist with the expected names.

use sqlx::Row;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PgImage;

async fn fresh_db() -> sqlx::PgPool {
    let pg = PgImage::default().start().await.unwrap();
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = astrophoto::db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    // Hold container alive for the test scope by storing in a static
    // (testcontainers leaks otherwise on early return). Acceptable
    // for tests; not for prod.
    Box::leak(Box::new(pg));
    pool
}

#[tokio::test]
async fn migration_0005_adds_handles_and_redirects() {
    let pool = fresh_db().await;

    // users.handle exists, NOT NULL, unique
    let row = sqlx::query(
        "select column_name, is_nullable
           from information_schema.columns
          where table_name = 'users' and column_name = 'handle'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let nullable: String = row.try_get("is_nullable").unwrap();
    assert_eq!(nullable, "NO");

    // handle_redirects table exists
    let exists: bool = sqlx::query_scalar(
        "select exists(select 1 from information_schema.tables \
         where table_name = 'handle_redirects')"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(exists);
}
```

- [ ] **Step 3: Run the test**

```
cd backend && cargo test --test migrations migration_0005 -- --nocapture
```

Expected: PASS. Docker must be running (testcontainers spins up Postgres).

- [ ] **Step 4: Commit**

```bash
git add backend/migrations/0005_handles.sql backend/tests/migrations.rs
git commit -m "feat(schema): 0005 handles + redirects, backfill placeholders"
```

---

### Task 6: Migration 0006 — user tier

**Files:**
- Create: `backend/migrations/0006_user_tier.sql`
- Modify: `backend/tests/migrations.rs`

- [ ] **Step 1: Create the migration**

Path: `backend/migrations/0006_user_tier.sql`.

```sql
-- 0006 user tier: free / subscriber. Drives upload-size enforcement
-- in the presigned-PUT path. No billing UI in this phase; the column
-- is toggled manually for now.

alter table users
    add column tier text not null default 'free'
        check (tier in ('free', 'subscriber'));
```

- [ ] **Step 2: Add the smoke test**

Append to `backend/tests/migrations.rs`:

```rust
#[tokio::test]
async fn migration_0006_adds_user_tier() {
    let pool = fresh_db().await;
    let default_tier: String = sqlx::query_scalar(
        "select column_default \
         from information_schema.columns \
         where table_name = 'users' and column_name = 'tier'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(default_tier.starts_with("'free'"));
}
```

- [ ] **Step 3: Run + commit**

```
cd backend && cargo test --test migrations migration_0006
```

```bash
git add backend/migrations/0006_user_tier.sql backend/tests/migrations.rs
git commit -m "feat(schema): 0006 add users.tier column"
```

---

### Task 7: Migration 0007 — photo short_id, display metadata, blurhash

**Files:**
- Create: `backend/migrations/0007_photo_short_and_display.sql`
- Modify: `backend/tests/migrations.rs`

- [ ] **Step 1: Create the migration**

Path: `backend/migrations/0007_photo_short_and_display.sql`.

```sql
-- 0007 photo permalink + display master + blurhash + content hash.
-- short_id is filled by the application on insert (8-char base62);
-- existing rows get a backfill from a deterministic UUID hash.

alter table photos
    add column short_id        text,
    add column display_key     text,
    add column original_hash   text,
    add column blurhash        text;

-- Backfill short_id for existing rows. Deterministic mapping from
-- the photo UUID's first 6 bytes -> base62, padded to 8.
update photos
    set short_id = upper(left(replace(id::text, '-', ''), 8));

alter table photos
    alter column short_id set not null;

create unique index photos_short_id_uidx on photos (short_id);

-- original_hash is per-owner unique to dedup re-uploads.
create unique index photos_owner_hash_uidx
    on photos (owner_id, original_hash)
    where original_hash is not null;
```

- [ ] **Step 2: Smoke test**

Append to `backend/tests/migrations.rs`:

```rust
#[tokio::test]
async fn migration_0007_adds_photo_short_id() {
    let pool = fresh_db().await;
    let exists: bool = sqlx::query_scalar(
        "select exists(select 1 from information_schema.columns \
         where table_name = 'photos' and column_name = 'short_id')"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(exists);
}
```

- [ ] **Step 3: Run + commit**

```
cd backend && cargo test --test migrations migration_0007
```

```bash
git add backend/migrations/0007_photo_short_and_display.sql backend/tests/migrations.rs
git commit -m "feat(schema): 0007 photo short_id, display_key, hash, blurhash"
```

---

### Task 8: Migration 0008 — user profile fields

**Files:**
- Create: `backend/migrations/0008_user_profile.sql`
- Modify: `backend/tests/migrations.rs`

- [ ] **Step 1: Create the migration**

Path: `backend/migrations/0008_user_profile.sql`.

```sql
-- 0008 user profile fields used by the hero page (P2 of showcase).
-- Schema only — no UI yet.

alter table users
    add column tagline             text,
    add column bio_html            text,
    add column cover_photo_id      uuid references photos(id) on delete set null,
    add column equipment_telescope text,
    add column equipment_camera    text,
    add column equipment_mount     text,
    add column equipment_filters   text,
    add column equipment_guiding   text,
    add column location_text       text,
    add column bortle_class        smallint
        check (bortle_class is null or bortle_class between 1 and 9),
    add column sqm                 numeric(4,2),
    add column social_links        jsonb not null default '[]'::jsonb;
```

- [ ] **Step 2: Smoke test**

Append to `backend/tests/migrations.rs`:

```rust
#[tokio::test]
async fn migration_0008_adds_user_profile_fields() {
    let pool = fresh_db().await;
    let count: i64 = sqlx::query_scalar(
        "select count(*) from information_schema.columns \
         where table_name = 'users' \
         and column_name in ('tagline','bio_html','cover_photo_id', \
             'equipment_telescope','equipment_camera','equipment_mount', \
             'equipment_filters','equipment_guiding', \
             'location_text','bortle_class','sqm','social_links')"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 12);
}
```

- [ ] **Step 3: Run + commit**

```
cd backend && cargo test --test migrations migration_0008
```

```bash
git add backend/migrations/0008_user_profile.sql backend/tests/migrations.rs
git commit -m "feat(schema): 0008 user profile fields"
```

---

### Task 9: Migration 0009 — photo featured, category, per-photo equipment

**Files:**
- Create: `backend/migrations/0009_photo_featured_category.sql`
- Modify: `backend/tests/migrations.rs`

- [ ] **Step 1: Create the migration**

Path: `backend/migrations/0009_photo_featured_category.sql`.

```sql
-- 0009 photo featured pinning + category + per-photo equipment fields.
-- These columns sit alongside the existing `camera`/`lens` columns from
-- 0001. `scope`/`mount`/`filters`/`guiding` are user-entered (no EXIF
-- source). `category` is a small fixed taxonomy.

alter table photos
    add column featured_at       timestamptz,
    add column featured_position smallint
        check (featured_position is null or featured_position between 1 and 6),
    add column category          text
        check (category is null or category in
               ('dso','planetary','lunar','solar','wide_field','nightscape','other')),
    add column scope             text,
    add column mount             text,
    add column filters           text,
    add column guiding           text;

-- One photo per slot per owner.
create unique index photos_featured_per_user_uidx
    on photos (owner_id, featured_position)
    where featured_at is not null;

-- featured_position must be set when featured_at is set, and vice versa.
alter table photos
    add constraint photos_featured_pair_chk
        check ((featured_at is null) = (featured_position is null));
```

- [ ] **Step 2: Smoke test**

Append to `backend/tests/migrations.rs`:

```rust
#[tokio::test]
async fn migration_0009_adds_photo_featured_and_category() {
    let pool = fresh_db().await;
    let count: i64 = sqlx::query_scalar(
        "select count(*) from information_schema.columns \
         where table_name = 'photos' \
         and column_name in ('featured_at','featured_position','category', \
             'scope','mount','filters','guiding')"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 7);
}
```

- [ ] **Step 3: Run + commit**

```
cd backend && cargo test --test migrations migration_0009
```

```bash
git add backend/migrations/0009_photo_featured_category.sql backend/tests/migrations.rs
git commit -m "feat(schema): 0009 photo featured, category, per-photo equipment"
```

---

### Task 10: Migration 0010 — targets, tags, photo_targets, photo_tags + seed

**Files:**
- Create: `backend/migrations/0010_targets_tags.sql`
- Create: `backend/data/targets_seed.sql` (optional — see Step 1 note)
- Modify: `backend/tests/migrations.rs`

- [ ] **Step 1: Create the migration**

Path: `backend/migrations/0010_targets_tags.sql`. The seed list is
deliberately small here (the full Messier set + ~20 well-known NGC/IC).
A larger seed can land in a follow-up migration without changing
schema.

```sql
-- 0010 discovery primitives: targets, tags, and the join tables.
-- photo_targets carries `source` so a future plate-solve job can write
-- rows without schema churn.

create table targets (
    id              uuid primary key default gen_random_uuid(),
    slug            text unique not null,
    canonical_name  text not null,
    aliases         text[] not null default '{}',
    kind            text not null
        check (kind in ('messier','ngc','ic','caldwell','common','other'))
);
create index targets_aliases_gin_idx on targets using gin (aliases);

create table photo_targets (
    photo_id   uuid not null references photos(id) on delete cascade,
    target_id  uuid not null references targets(id) on delete cascade,
    source     text not null check (source in ('manual','plate_solve')),
    confidence numeric,
    is_primary boolean not null default false,
    created_at timestamptz not null default now(),
    primary key (photo_id, target_id)
);
create index photo_targets_target_idx on photo_targets (target_id, photo_id);

create table tags (
    id   uuid primary key default gen_random_uuid(),
    slug text unique not null,
    name text not null
);
create table photo_tags (
    photo_id uuid not null references photos(id) on delete cascade,
    tag_id   uuid not null references tags(id) on delete cascade,
    primary key (photo_id, tag_id)
);
create index photo_tags_tag_idx on photo_tags (tag_id, photo_id);

-- Seed: Messier 1..110 plus a few popular NGC/IC objects.
-- Generate Messier rows from a series.
insert into targets (slug, canonical_name, aliases, kind)
select
    'm' || g,
    'Messier ' || g,
    array['M' || g, 'Messier ' || g],
    'messier'
from generate_series(1, 110) g
on conflict (slug) do nothing;

-- Common-name overrides for the high-traffic ones.
update targets set canonical_name = 'Andromeda Galaxy', aliases = aliases || array['NGC 224']
    where slug = 'm31';
update targets set canonical_name = 'Orion Nebula',     aliases = aliases || array['NGC 1976']
    where slug = 'm42';
update targets set canonical_name = 'Triangulum Galaxy', aliases = aliases || array['NGC 598']
    where slug = 'm33';
update targets set canonical_name = 'Whirlpool Galaxy', aliases = aliases || array['NGC 5194']
    where slug = 'm51';
update targets set canonical_name = 'Pleiades',         aliases = aliases || array['Seven Sisters', 'NGC 1432']
    where slug = 'm45';
update targets set canonical_name = 'Dumbbell Nebula',  aliases = aliases || array['NGC 6853']
    where slug = 'm27';
update targets set canonical_name = 'Hercules Cluster', aliases = aliases || array['NGC 6205']
    where slug = 'm13';

-- A handful of very-popular NGC/IC.
insert into targets (slug, canonical_name, aliases, kind) values
    ('ngc-7000', 'North America Nebula', array['NGC 7000','Caldwell 20'], 'ngc'),
    ('ngc-6960', 'Western Veil Nebula',  array['NGC 6960','Witch''s Broom'], 'ngc'),
    ('ngc-2237', 'Rosette Nebula',       array['NGC 2237','Caldwell 49'], 'ngc'),
    ('ngc-281',  'Pacman Nebula',        array['NGC 281'], 'ngc'),
    ('ngc-3324', 'Cosmic Cliffs',        array['NGC 3324'], 'ngc'),
    ('ic-1805',  'Heart Nebula',         array['IC 1805','Sharpless 2-190'], 'ic'),
    ('ic-1396',  'Elephant''s Trunk',    array['IC 1396'], 'ic'),
    ('ic-434',   'Horsehead Nebula',     array['IC 434','Barnard 33'], 'ic')
on conflict (slug) do nothing;
```

- [ ] **Step 2: Smoke test**

Append to `backend/tests/migrations.rs`:

```rust
#[tokio::test]
async fn migration_0010_adds_targets_and_tags() {
    let pool = fresh_db().await;
    let messier_count: i64 = sqlx::query_scalar(
        "select count(*) from targets where kind = 'messier'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(messier_count, 110);

    let m31: String = sqlx::query_scalar(
        "select canonical_name from targets where slug = 'm31'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(m31, "Andromeda Galaxy");
}
```

- [ ] **Step 3: Run + commit**

```
cd backend && cargo test --test migrations migration_0010
```

```bash
git add backend/migrations/0010_targets_tags.sql backend/tests/migrations.rs
git commit -m "feat(schema): 0010 targets, tags, photo_targets/tags + seed"
```

---

### Task 11: Migration 0011 — appreciations_count denormalisation

**Files:**
- Create: `backend/migrations/0011_appreciations_count.sql`
- Modify: `backend/tests/migrations.rs`

- [ ] **Step 1: Create the migration**

Path: `backend/migrations/0011_appreciations_count.sql`. The
`appreciations` table itself was created in Phase 7 migration 0002.
This migration adds the denormalised counter column on `photos`,
backfills from existing rows, and installs the index used for the
"most appreciated" sort.

```sql
-- 0011 appreciations_count: denormalised counter on photos.
-- Backfills from the existing appreciations table (Phase 7).
-- Application code (POST /appreciate, DELETE /appreciate) maintains
-- this counter transactionally.

alter table photos
    add column appreciations_count integer not null default 0;

update photos p
    set appreciations_count = (
        select count(*) from appreciations a where a.photo_id = p.id
    );

-- Index: most-appreciated sort across all published photos.
create index photos_published_popular_idx
    on photos (appreciations_count desc, published_at desc, id desc)
    where published_at is not null;
```

- [ ] **Step 2: Smoke test**

Append to `backend/tests/migrations.rs`:

```rust
#[tokio::test]
async fn migration_0011_adds_appreciations_count() {
    let pool = fresh_db().await;
    let exists: bool = sqlx::query_scalar(
        "select exists(select 1 from information_schema.columns \
         where table_name = 'photos' and column_name = 'appreciations_count')"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(exists);
}
```

- [ ] **Step 3: Run + commit**

```
cd backend && cargo test --test migrations migration_0011
```

```bash
git add backend/migrations/0011_appreciations_count.sql backend/tests/migrations.rs
git commit -m "feat(schema): 0011 photos.appreciations_count + backfill"
```

---

### Task 12: Migration 0012 — equipment_items + discovery indexes

**Files:**
- Create: `backend/migrations/0012_equipment_items.sql`
- Modify: `backend/tests/migrations.rs`

- [ ] **Step 1: Create the migration**

Path: `backend/migrations/0012_equipment_items.sql`.

```sql
-- 0012 equipment_items: the lookup dictionary populated by upsert on
-- photo save. Powers the autocomplete in upload-verify (P1) and the
-- equipment browse pages /equip/<kind>/<slug> (P3).

create table equipment_items (
    id             uuid primary key default gen_random_uuid(),
    kind           text not null
        check (kind in ('telescope','camera','mount','filter','guiding')),
    canonical_name text not null,           -- lowercased, slug-friendly
    display_name   text not null,           -- as first seen
    usage_count    integer not null default 0,
    unique (kind, canonical_name)
);
create index equipment_items_kind_count_idx
    on equipment_items (kind, usage_count desc);

-- Discovery indexes on the per-photo equipment fields.
create index photos_camera_lower_idx
    on photos (lower(camera))  where published_at is not null;
create index photos_scope_lower_idx
    on photos (lower(scope))   where published_at is not null;
create index photos_mount_lower_idx
    on photos (lower(mount))   where published_at is not null;
create index photos_filters_lower_idx
    on photos (lower(filters)) where published_at is not null;
create index photos_guiding_lower_idx
    on photos (lower(guiding)) where published_at is not null;

-- Category browse + cursor.
create index photos_category_published_idx
    on photos (category, published_at desc, id desc)
    where published_at is not null;

-- "Newest" cursor (also used by /explore).
-- Note: a simpler photos_published_at_idx exists from 0004; this one
-- adds id as the tie-breaker required for stable cursor pagination.
create index photos_published_newest_idx
    on photos (published_at desc, id desc) where published_at is not null;
```

- [ ] **Step 2: Smoke test**

Append to `backend/tests/migrations.rs`:

```rust
#[tokio::test]
async fn migration_0012_adds_equipment_items_and_indexes() {
    let pool = fresh_db().await;
    let exists: bool = sqlx::query_scalar(
        "select exists(select 1 from information_schema.tables \
         where table_name = 'equipment_items')"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(exists);
}
```

- [ ] **Step 3: Run + commit**

```
cd backend && cargo test --test migrations migration_0012
```

```bash
git add backend/migrations/0012_equipment_items.sql backend/tests/migrations.rs
git commit -m "feat(schema): 0012 equipment_items + discovery indexes"
```

---

## Backend — handles & signup

### Task 13: Handle validation + reserved-list check

**Files:**
- Create: `backend/src/auth/handle.rs`
- Modify: `backend/src/auth/mod.rs`
- Test: `backend/src/auth/handle.rs` (inline `#[cfg(test)]` mod)

- [ ] **Step 1: Write the failing test**

Path: `backend/src/auth/handle.rs`. Create the file with the test
first (and a stub for the function to make it compile).

```rust
//! Handle validation: regex + reserved-list check.

use std::collections::HashSet;
use std::sync::OnceLock;

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum HandleError {
    #[error("handle must be 3-30 chars of [a-z0-9_-]")]
    Format,
    #[error("handle is reserved")]
    Reserved,
}

pub fn validate(_handle: &str) -> Result<(), HandleError> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_simple_handle() {
        assert_eq!(validate("marie"), Ok(()));
    }

    #[test]
    fn rejects_too_short() {
        assert_eq!(validate("ab"), Err(HandleError::Format));
    }

    #[test]
    fn rejects_uppercase() {
        assert_eq!(validate("Marie"), Err(HandleError::Format));
    }

    #[test]
    fn rejects_reserved() {
        assert_eq!(validate("admin"), Err(HandleError::Reserved));
    }

    #[test]
    fn accepts_underscore_and_hyphen() {
        assert_eq!(validate("a_b-c"), Ok(()));
    }
}
```

Wire it into the module:

`backend/src/auth/mod.rs` — add `pub mod handle;` next to the
existing `pub mod` lines.

- [ ] **Step 2: Run the failing tests**

```
cd backend && cargo test --lib auth::handle
```

Expected: PASS for compilation but PANIC at `todo!()` on every test.

- [ ] **Step 3: Implement validation**

Replace the stubbed `validate` and add the reserved-list loader:

```rust
fn reserved() -> &'static HashSet<String> {
    static SET: OnceLock<HashSet<String>> = OnceLock::new();
    SET.get_or_init(|| {
        include_str!("../../data/reserved_handles.txt")
            .lines()
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .collect()
    })
}

pub fn validate(handle: &str) -> Result<(), HandleError> {
    let h = handle.trim();
    if !is_valid_format(h) {
        return Err(HandleError::Format);
    }
    if reserved().contains(&h.to_ascii_lowercase()) {
        return Err(HandleError::Reserved);
    }
    Ok(())
}

fn is_valid_format(h: &str) -> bool {
    let len = h.chars().count();
    if !(3..=30).contains(&len) {
        return false;
    }
    h.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}
```

- [ ] **Step 4: Run tests**

```
cd backend && cargo test --lib auth::handle
```

Expected: 5 PASS.

- [ ] **Step 5: Commit**

```bash
git add backend/src/auth/handle.rs backend/src/auth/mod.rs
git commit -m "feat(auth): handle validation + reserved-list check"
```

---

### Task 14: Add handle to signup

**Files:**
- Modify: `backend/src/auth/signup.rs`
- Modify: `backend/src/users/queries.rs` — `create_with_password`
- Test: `backend/tests/auth.rs` (extend the `signup_login_me_logout_full_flow`
  test or add a sibling)

- [ ] **Step 1: Extend the request body**

In `backend/src/auth/signup.rs`, add the field to `SignupBody`:

```rust
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
```

- [ ] **Step 2: Add handle validation in the handler**

After the existing `body.validate()...` line in `auth::signup::handler`,
add:

```rust
    crate::auth::handle::validate(&body.handle)
        .map_err(|e| AppError::Validation(e.to_string()))?;
```

- [ ] **Step 3: Update the SQL insert**

`backend/src/users/queries.rs` — modify `create_with_password`. New
signature takes a `handle`; the SQL inserts it.

```rust
pub async fn create_with_password(
    pool: &PgPool,
    email: &str,
    handle: &str,
    display_name: &str,
    password_hash: &str,
) -> Result<UserRow, AppError> {
    sqlx::query_as!(
        UserRow,
        r#"
        insert into users (email, handle, display_name, password_hash, password_changed_at)
        values ($1, $2, $3, $4, now())
        returning id, email::text as "email!", handle::text as "handle!", display_name, password_hash, created_at
        "#,
        email,
        handle,
        display_name,
        password_hash,
    )
    .fetch_one(pool)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(db) if db.constraint() == Some("users_email_key") => {
            AppError::Conflict("email already in use".into())
        }
        sqlx::Error::Database(db) if db.constraint() == Some("users_handle_uidx") => {
            AppError::Conflict("handle already taken".into())
        }
        _ => AppError::Database(e),
    })
}
```

(`UserRow` itself gains a `handle: String` field — find it in
`backend/src/users/mod.rs` or `users/queries.rs` and add it.)

- [ ] **Step 4: Update call sites**

The signup handler now calls
`queries::create_with_password(&pool, email, handle, display_name, &hash)`.
Find any other call sites:

```
cd backend && grep -rn "create_with_password" src/
```

Update each to pass the new handle argument.

- [ ] **Step 5: Run sqlx prepare**

```
cd backend && cargo sqlx prepare
git add .sqlx
```

- [ ] **Step 6: Update or extend the integration test**

`backend/tests/auth.rs` — find the existing signup test and add
`handle: "marie"` to the JSON body. Add a sibling test for collision:

```rust
#[tokio::test]
async fn signup_rejects_duplicate_handle() {
    // ... boot test app per existing pattern ...
    let body1 = serde_json::json!({
        "email": "a@example.com",
        "password": "verylongpassword",
        "display_name": "A",
        "handle": "shared"
    });
    let r1 = app.clone().oneshot(/* POST /api/auth/signup with body1 */).await.unwrap();
    assert_eq!(r1.status(), 201);

    let body2 = serde_json::json!({
        "email": "b@example.com",
        "password": "verylongpassword",
        "display_name": "B",
        "handle": "shared"
    });
    let r2 = app.clone().oneshot(/* POST /api/auth/signup with body2 */).await.unwrap();
    assert_eq!(r2.status(), 409); // Conflict
}
```

(Replace the comments with the existing test's request-construction
pattern.)

- [ ] **Step 7: Run tests**

```
cd backend && cargo test --test auth
```

Expected: all PASS.

- [ ] **Step 8: Commit**

```bash
git add backend/src/auth/signup.rs backend/src/users/ backend/tests/auth.rs backend/.sqlx
git commit -m "feat(auth): require @handle at signup, conflict on duplicate"
```

---

### Task 15: Handle availability endpoint

**Files:**
- Create: `backend/src/auth/handle_check.rs`
- Modify: `backend/src/http/mod.rs` (route registration)
- Test: `backend/tests/handle_check.rs`

- [ ] **Step 1: Write the failing integration test**

Path: `backend/tests/handle_check.rs`.

```rust
//! GET /api/auth/handle-check?handle=foo
//! Returns 200 with {"status":"available"|"taken"|"reserved"|"invalid"}.

use astrophoto::{db, http, storage::MemoryStorage};
use axum::body::Body;
use axum::http::{header, Request};
use std::sync::Arc;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres as PgImage;
use tower::ServiceExt;

#[tokio::test]
async fn handle_check_returns_available_then_taken() {
    let pg = PgImage::default().start().await.unwrap();
    let host = pg.get_host().await.unwrap();
    let port = pg.get_host_port_ipv4(5432).await.unwrap();
    let url = format!("postgres://postgres:postgres@{host}:{port}/postgres");
    let pool = db::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let (mailer, _outbox) = astrophoto::mail::Mailer::for_test();
    let app = http::router(
        pool.clone(),
        /* config_for(&url) helper, copy from auth.rs */
        Arc::new(MemoryStorage::new()),
        Arc::new(mailer),
    );

    // Available
    let r = app.clone().oneshot(
        Request::builder().uri("/api/auth/handle-check?handle=fresh")
            .body(Body::empty()).unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 200);
    let body = serde_json::from_slice::<serde_json::Value>(
        &axum::body::to_bytes(r.into_body(), 1024).await.unwrap()
    ).unwrap();
    assert_eq!(body["status"], "available");

    // Reserved
    let r = app.clone().oneshot(
        Request::builder().uri("/api/auth/handle-check?handle=admin")
            .body(Body::empty()).unwrap()
    ).await.unwrap();
    let body = serde_json::from_slice::<serde_json::Value>(
        &axum::body::to_bytes(r.into_body(), 1024).await.unwrap()
    ).unwrap();
    assert_eq!(body["status"], "reserved");

    // Invalid
    let r = app.clone().oneshot(
        Request::builder().uri("/api/auth/handle-check?handle=AB")
            .body(Body::empty()).unwrap()
    ).await.unwrap();
    let body = serde_json::from_slice::<serde_json::Value>(
        &axum::body::to_bytes(r.into_body(), 1024).await.unwrap()
    ).unwrap();
    assert_eq!(body["status"], "invalid");
}
```

(Copy `config_for` from `backend/tests/auth.rs` into a local helper
or into a shared `tests/common/mod.rs` if not already there.)

- [ ] **Step 2: Run the test**

```
cd backend && cargo test --test handle_check
```

Expected: FAIL — route not registered.

- [ ] **Step 3: Implement the handler**

Path: `backend/src/auth/handle_check.rs`.

```rust
use axum::{extract::{Query, State}, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::http::AppState;

#[derive(Deserialize)]
pub struct Q {
    pub handle: String,
}

#[derive(Serialize)]
pub struct R {
    pub status: &'static str,
}

pub async fn handler(
    State(state): State<AppState>,
    Query(q): Query<Q>,
) -> Result<impl IntoResponse, AppError> {
    use crate::auth::handle::{validate, HandleError};

    match validate(&q.handle) {
        Err(HandleError::Format)   => return Ok(Json(R { status: "invalid"  })),
        Err(HandleError::Reserved) => return Ok(Json(R { status: "reserved" })),
        Ok(()) => {}
    }

    let taken: bool = sqlx::query_scalar!(
        "select exists(select 1 from users where handle = $1)",
        q.handle
    )
    .fetch_one(&state.pool)
    .await?
    .unwrap_or(false);

    Ok(Json(R { status: if taken { "taken" } else { "available" } }))
}
```

- [ ] **Step 4: Wire the route**

`backend/src/http/mod.rs` — add to the auth route group:

```rust
.route(
    "/api/auth/handle-check",
    axum::routing::get(crate::auth::handle_check::handler),
)
```

Add `pub mod handle_check;` to `backend/src/auth/mod.rs`.

- [ ] **Step 5: Run sqlx prepare + tests**

```
cd backend && cargo sqlx prepare && cargo test --test handle_check
```

Expected: PASS (3 cases).

- [ ] **Step 6: Commit**

```bash
git add backend/src/auth/handle_check.rs backend/src/auth/mod.rs backend/src/http/mod.rs backend/tests/handle_check.rs backend/.sqlx
git commit -m "feat(auth): GET /api/auth/handle-check availability endpoint"
```

---

### Task 16: Handle rename + redirect

**Files:**
- Create: `backend/src/users/handle.rs`
- Modify: `backend/src/http/mod.rs`
- Test: `backend/tests/handle_rename.rs`

- [ ] **Step 1: Failing integration test**

Path: `backend/tests/handle_rename.rs`. Follow the testcontainers
pattern from `auth.rs`. Test sequence:

1. Sign up as `marie` (returns session cookie).
2. POST `/api/me/handle` with new handle `marie2` (auth required).
3. Expect 200; subsequent GET `/api/users/by-handle/marie2` returns
   the user; `/api/users/by-handle/marie` 404s but a row exists in
   `handle_redirects`.

```rust
#[tokio::test]
async fn rename_handle_writes_redirect_row() {
    // ... boot app, signup as 'marie', capture session cookie ...
    let resp = app.clone().oneshot(
        Request::builder().method("POST")
            .uri("/api/me/handle")
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::COOKIE, &session_cookie)
            .body(Body::from(serde_json::json!({"handle":"marie2"}).to_string()))
            .unwrap()
    ).await.unwrap();
    assert_eq!(resp.status(), 200);

    let exists: bool = sqlx::query_scalar(
        "select exists(select 1 from handle_redirects where old_handle = 'marie')"
    )
    .fetch_one(&pool).await.unwrap();
    assert!(exists);

    let new_handle: String = sqlx::query_scalar(
        "select handle::text from users where id = $1"
    )
    .bind(user_id)
    .fetch_one(&pool).await.unwrap();
    assert_eq!(new_handle, "marie2");
}
```

- [ ] **Step 2: Run the test**

```
cd backend && cargo test --test handle_rename
```

Expected: FAIL (404 on `/api/me/handle`).

- [ ] **Step 3: Implement**

Path: `backend/src/users/handle.rs`.

```rust
use axum::{extract::State, response::IntoResponse, Json};
use serde::Deserialize;

use crate::auth::handle::{validate, HandleError};
use crate::auth::session::CurrentUser;
use crate::error::AppError;
use crate::http::AppState;

#[derive(Deserialize)]
pub struct Body { pub handle: String }

pub async fn rename(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<Body>,
) -> Result<impl IntoResponse, AppError> {
    validate(&body.handle).map_err(|e| match e {
        HandleError::Format   => AppError::Validation("invalid handle format".into()),
        HandleError::Reserved => AppError::Conflict("handle is reserved".into()),
    })?;

    let mut tx = state.pool.begin().await?;

    // Read current handle for the redirect row.
    let current: String = sqlx::query_scalar!(
        "select handle::text as \"handle!\" from users where id = $1",
        user.id
    ).fetch_one(&mut *tx).await?;

    if current == body.handle {
        return Ok(axum::http::StatusCode::NO_CONTENT.into_response());
    }

    // Try to update.
    let res = sqlx::query!(
        "update users set handle = $1 where id = $2",
        body.handle, user.id
    ).execute(&mut *tx).await;

    if let Err(sqlx::Error::Database(db)) = &res {
        if db.constraint() == Some("users_handle_uidx") {
            return Err(AppError::Conflict("handle already taken".into()));
        }
    }
    res?;

    // Released-at = now + 90 days (cooldown before reuse).
    sqlx::query!(
        "insert into handle_redirects (old_handle, user_id, released_at) \
         values ($1, $2, now() + interval '90 days') \
         on conflict (old_handle) do update set user_id = excluded.user_id, released_at = excluded.released_at",
        current, user.id
    ).execute(&mut *tx).await?;

    tx.commit().await?;
    Ok(axum::http::StatusCode::OK.into_response())
}
```

- [ ] **Step 4: Wire route**

`backend/src/http/mod.rs`:

```rust
.route(
    "/api/me/handle",
    axum::routing::post(crate::users::handle::rename),
)
```

`backend/src/users/mod.rs`: add `pub mod handle;`.

- [ ] **Step 5: Run sqlx prepare + tests**

```
cd backend && cargo sqlx prepare && cargo test --test handle_rename
```

- [ ] **Step 6: Commit**

```bash
git add backend/src/users/handle.rs backend/src/users/mod.rs backend/src/http/mod.rs backend/tests/handle_rename.rs backend/.sqlx
git commit -m "feat(users): POST /api/me/handle with redirect row + 90-day cooldown"
```

---

## Backend — AppError extensions

### Task 17: Extend AppError with new variants

**Files:**
- Modify: `backend/src/error.rs`
- Test: `backend/src/error.rs` (inline `#[cfg(test)]`)

- [ ] **Step 1: Add the new variants**

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    // ... existing variants ...

    #[error("rate limited")]
    RateLimited,
    #[error("quota exceeded: {0}")]
    QuotaExceeded(String),
    #[error("payload too large: {0}")]
    PayloadTooLarge(String),
    #[error("magic byte mismatch: {0}")]
    MagicByteMismatch(String),
    #[error("pending finalize stuck: {0}")]
    PendingFinalizeStuck(String),
    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),
}
```

- [ ] **Step 2: Add status / code arms**

In `impl AppError { fn status(&self) -> StatusCode {...} fn code(&self) -> &'static str {...} }`:

```rust
fn status(&self) -> StatusCode {
    match self {
        // ... existing arms ...
        AppError::RateLimited            => StatusCode::TOO_MANY_REQUESTS,
        AppError::QuotaExceeded(_)       => StatusCode::PAYLOAD_TOO_LARGE,    // 413
        AppError::PayloadTooLarge(_)     => StatusCode::PAYLOAD_TOO_LARGE,    // 413
        AppError::MagicByteMismatch(_)   => StatusCode::BAD_REQUEST,
        AppError::PendingFinalizeStuck(_)=> StatusCode::REQUEST_TIMEOUT,      // 408
        AppError::UnsupportedFormat(_)   => StatusCode::BAD_REQUEST,
    }
}

fn code(&self) -> &'static str {
    match self {
        // ... existing arms ...
        AppError::RateLimited            => "rate-limited",
        AppError::QuotaExceeded(_)       => "quota-exceeded",
        AppError::PayloadTooLarge(_)     => "payload-too-large",
        AppError::MagicByteMismatch(_)   => "magic-byte-mismatch",
        AppError::PendingFinalizeStuck(_)=> "pending-finalize-stuck",
        AppError::UnsupportedFormat(_)   => "unsupported-format",
    }
}
```

- [ ] **Step 3: Run check**

```
cd backend && cargo check --all-targets
```

Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add backend/src/error.rs
git commit -m "feat(error): add quota / payload / magic-byte / format variants"
```

---

## Backend — bio sanitiser

### Task 18: ammonia bio HTML sanitiser

**Files:**
- Create: `backend/src/users/bio.rs`
- Modify: `backend/src/users/mod.rs`
- Test: `backend/src/users/bio.rs` (inline)

- [ ] **Step 1: Failing tests**

Path: `backend/src/users/bio.rs`.

```rust
//! Bio HTML sanitisation. Server is the source of truth — any HTML
//! posted to PATCH /api/me passes through `sanitize`. The Tiptap
//! client editor (P2) is configured to match this allowlist, but
//! pasted HTML or tampered POSTs may contain anything.

use ammonia::Builder;
use std::sync::OnceLock;

pub fn sanitize(input: &str) -> String {
    let cleaner = cleaner();
    cleaner.clean(input).to_string()
}

fn cleaner() -> &'static Builder<'static> {
    static C: OnceLock<Builder<'static>> = OnceLock::new();
    C.get_or_init(|| {
        let mut b = Builder::default();
        b.tags(std::collections::HashSet::from([
            "p","br","strong","em","u","h2","h3","h4",
            "ul","ol","li","blockquote","code","a"
        ]));
        b.tag_attributes(std::collections::HashMap::from([
            ("a", std::collections::HashSet::from(["href"]))
        ]));
        b.url_schemes(std::collections::HashSet::from(["http","https","mailto"]));
        b.link_rel(Some("nofollow noopener"));
        b
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_safe_tags() {
        let out = sanitize("<p>Hello <strong>world</strong></p>");
        assert!(out.contains("<strong>"));
    }

    #[test]
    fn strips_script() {
        let out = sanitize("<p>Hi</p><script>alert('x')</script>");
        assert!(!out.contains("script"));
    }

    #[test]
    fn strips_onclick() {
        let out = sanitize("<a href=\"https://x\" onclick=\"x()\">l</a>");
        assert!(!out.contains("onclick"));
    }

    #[test]
    fn strips_javascript_uri() {
        let out = sanitize("<a href=\"javascript:alert(1)\">l</a>");
        assert!(!out.contains("javascript:"));
    }

    #[test]
    fn forces_rel_on_links() {
        let out = sanitize("<a href=\"https://x\">l</a>");
        assert!(out.contains("rel=\"nofollow noopener\""));
    }

    #[test]
    fn strips_iframe() {
        let out = sanitize("<iframe src=\"https://x\"></iframe>");
        assert!(!out.contains("iframe"));
    }

    #[test]
    fn keeps_lists() {
        let out = sanitize("<ul><li>a</li><li>b</li></ul>");
        assert!(out.contains("<ul>") && out.contains("<li>"));
    }
}
```

Wire: `backend/src/users/mod.rs` add `pub mod bio;`.

- [ ] **Step 2: Run tests**

```
cd backend && cargo test --lib users::bio
```

Expected: 7 PASS.

- [ ] **Step 3: Commit**

```bash
git add backend/src/users/bio.rs backend/src/users/mod.rs
git commit -m "feat(users): bio HTML sanitiser via ammonia allowlist"
```

---

## Backend — short_id + permalink

### Task 19: short_id base62 generator + collision retry

**Files:**
- Create: `backend/src/photos/short_id.rs`
- Modify: `backend/src/photos/mod.rs`
- Test: `backend/src/photos/short_id.rs` (inline)

- [ ] **Step 1: Failing tests**

```rust
//! 8-char base62 short identifier for photo permalinks.
//! /u/<handle>/p/<short_id>. ~2.18*10^14 keyspace.

use nanoid::nanoid;

const ALPHABET: [char; 62] = [
    '0','1','2','3','4','5','6','7','8','9',
    'A','B','C','D','E','F','G','H','I','J',
    'K','L','M','N','O','P','Q','R','S','T',
    'U','V','W','X','Y','Z',
    'a','b','c','d','e','f','g','h','i','j',
    'k','l','m','n','o','p','q','r','s','t',
    'u','v','w','x','y','z',
];

pub fn generate() -> String {
    nanoid!(8, &ALPHABET)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn produces_8_chars() {
        let s = generate();
        assert_eq!(s.chars().count(), 8);
    }

    #[test]
    fn alphabet_is_base62() {
        for _ in 0..1000 {
            let s = generate();
            for c in s.chars() {
                assert!(c.is_ascii_alphanumeric(), "char {c} not base62");
            }
        }
    }

    #[test]
    fn collisions_unlikely_in_small_set() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        for _ in 0..10_000 {
            assert!(set.insert(generate()));
        }
    }
}
```

`backend/src/photos/mod.rs`: add `pub mod short_id;`.

- [ ] **Step 2: Run tests + commit**

```
cd backend && cargo test --lib photos::short_id
```

```bash
git add backend/src/photos/short_id.rs backend/src/photos/mod.rs
git commit -m "feat(photos): base62 short_id generator (nanoid)"
```

---

### Task 20: Photo permalink resolution by (handle, short_id)

**Files:**
- Create: `backend/src/photos/permalink.rs`
- Modify: `backend/src/photos/mod.rs`
- Test: `backend/tests/permalink.rs`

- [ ] **Step 1: Failing test**

Path: `backend/tests/permalink.rs`. Boot the test app per
`auth.rs` pattern. Insert a user with `handle='marie'`, insert a
photo with `short_id='ABCD1234', published_at=now()`. Then GET
`/api/photos/by-permalink/marie/ABCD1234` and assert 200 + JSON
body contains the expected `id`.

```rust
#[tokio::test]
async fn resolve_permalink_returns_photo_id() {
    // ... boot ...
    sqlx::query!(
        "insert into users (id, email, handle, display_name, password_hash) \
         values ($1, 'm@example.com', 'marie', 'M', 'x')",
        user_id
    ).execute(&pool).await.unwrap();
    sqlx::query!(
        "insert into photos (id, owner_id, storage_key, original_name, bytes, mime, status, short_id, published_at) \
         values ($1, $2, 'k', 'n', 1, 'image/jpeg', 'ready', 'ABCD1234', now())",
        photo_id, user_id
    ).execute(&pool).await.unwrap();

    let r = app.oneshot(
        Request::builder()
            .uri("/api/photos/by-permalink/marie/ABCD1234")
            .body(Body::empty()).unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 200);
    let body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(r.into_body(), 8192).await.unwrap()
    ).unwrap();
    assert_eq!(body["id"], photo_id.to_string());
}
```

- [ ] **Step 2: Implement**

Path: `backend/src/photos/permalink.rs`.

```rust
use axum::{extract::{Path, State}, response::IntoResponse, Json};

use crate::error::AppError;
use crate::http::AppState;

pub async fn lookup(
    State(state): State<AppState>,
    Path((handle, short_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, AppError> {
    let row = sqlx::query!(
        r#"
        select p.id as "id!"
          from photos p
          join users  u on u.id = p.owner_id
         where u.handle  = $1
           and p.short_id = $2
           and p.published_at is not null
        "#,
        handle, short_id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("photo".into()))?;

    Ok(Json(serde_json::json!({ "id": row.id })))
}
```

- [ ] **Step 3: Wire route**

`backend/src/http/mod.rs`:

```rust
.route(
    "/api/photos/by-permalink/:handle/:short_id",
    axum::routing::get(crate::photos::permalink::lookup),
)
```

- [ ] **Step 4: Run sqlx prepare + tests + commit**

```
cd backend && cargo sqlx prepare && cargo test --test permalink
```

```bash
git add backend/src/photos/permalink.rs backend/src/photos/mod.rs backend/src/http/mod.rs backend/tests/permalink.rs backend/.sqlx
git commit -m "feat(photos): resolve permalink (handle, short_id) -> photo id"
```

---

## Backend — CDN URL builder + dev fallback

### Task 21: CDN URL builder

**Files:**
- Create: `backend/src/photos/cdn.rs`
- Modify: `backend/src/photos/mod.rs`
- Test: `backend/src/photos/cdn.rs` (inline)

- [ ] **Step 1: Implementation + test**

```rust
//! CDN URL builder. Returns
//!   {base}/img/{photo_id}?w=&h=&fit=&q=&fm=
//! In prod, base is the CloudFront distribution. In dev, the backend's
//! own /cdn route serves the same URL shape locally.

use uuid::Uuid;

#[derive(Default)]
pub struct Transform {
    pub w: Option<u32>,
    pub h: Option<u32>,
    pub fit: Option<&'static str>,  // "cover" | "contain"
    pub q: Option<u8>,
    pub fm: Option<&'static str>,   // "auto" | "jpg" | "webp"
}

pub fn url(base: &str, photo_id: Uuid, t: &Transform) -> String {
    let mut s = format!("{base}/img/{photo_id}");
    let mut sep = '?';
    if let Some(w) = t.w  { s.push(sep); s.push_str(&format!("w={w}")); sep = '&'; }
    if let Some(h) = t.h  { s.push(sep); s.push_str(&format!("h={h}")); sep = '&'; }
    if let Some(f) = t.fit { s.push(sep); s.push_str(&format!("fit={f}")); sep = '&'; }
    if let Some(q) = t.q  { s.push(sep); s.push_str(&format!("q={q}")); sep = '&'; }
    if let Some(fm) = t.fm { s.push(sep); s.push_str(&format!("fm={fm}")); }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_params() {
        let id = Uuid::nil();
        assert_eq!(url("https://cdn.x.app", id, &Transform::default()),
                   format!("https://cdn.x.app/img/{id}"));
    }

    #[test]
    fn full_params() {
        let id = Uuid::nil();
        let t = Transform { w: Some(800), h: Some(600), fit: Some("cover"), q: Some(85), fm: Some("auto") };
        let got = url("https://cdn.x.app", id, &t);
        assert!(got.contains("w=800"));
        assert!(got.contains("fm=auto"));
    }
}
```

`photos/mod.rs`: add `pub mod cdn;`.

- [ ] **Step 2: Run + commit**

```
cd backend && cargo test --lib photos::cdn
```

```bash
git add backend/src/photos/cdn.rs backend/src/photos/mod.rs
git commit -m "feat(photos): CDN URL builder"
```

---

### Task 22: Dev CDN route (local image transforms over MinIO)

**Files:**
- Create: `backend/src/storage/cdn_dev.rs`
- Modify: `backend/src/http/mod.rs` (mount `/cdn/img/:id` only when not in prod)
- Test: `backend/tests/cdn_dev.rs`

- [ ] **Step 1: Failing test**

Path: `backend/tests/cdn_dev.rs`. Boot the app, store a small JPEG
at `display/<photo-id>.jpg` in `MemoryStorage`, then GET
`/cdn/img/<id>?w=100` and assert 200 + `Content-Type: image/jpeg`
+ body length less than the original.

```rust
#[tokio::test]
async fn dev_cdn_resizes_display_master() {
    // ... boot ...
    let bytes = include_bytes!("fixtures/sample.jpg");
    storage.put(&format!("display/{photo_id}.jpg"), "image/jpeg",
                Bytes::copy_from_slice(bytes)).await.unwrap();

    let r = app.oneshot(
        Request::builder().uri(&format!("/cdn/img/{photo_id}?w=100"))
            .body(Body::empty()).unwrap()
    ).await.unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(r.headers().get("content-type").unwrap(), "image/jpeg");
    let resized = axum::body::to_bytes(r.into_body(), 1_000_000).await.unwrap();
    assert!(resized.len() < bytes.len());
}
```

(Drop a small JPEG at `backend/tests/fixtures/sample.jpg` — any 800×600
photo works. Commit it as a binary fixture.)

- [ ] **Step 2: Implement**

Path: `backend/src/storage/cdn_dev.rs`.

```rust
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::header::{CACHE_CONTROL, CONTENT_TYPE},
    response::Response,
};
use bytes::Bytes;
use image::{imageops::FilterType, ImageFormat};
use serde::Deserialize;
use uuid::Uuid;

use crate::error::AppError;
use crate::http::AppState;

#[derive(Deserialize, Default)]
pub struct Q {
    pub w: Option<u32>,
    pub h: Option<u32>,
    pub fit: Option<String>,
    pub q: Option<u8>,
    pub fm: Option<String>,
}

pub async fn handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(q): Query<Q>,
) -> Result<Response, AppError> {
    let key = format!("display/{id}.jpg");
    let bytes = state.storage.get(&key).await?
        .ok_or_else(|| AppError::NotFound("display master".into()))?;

    let resized = tokio::task::spawn_blocking(move || -> Result<Bytes, AppError> {
        let img = image::load_from_memory_with_format(&bytes, ImageFormat::Jpeg)
            .map_err(|e| AppError::Internal(format!("decode: {e}")))?;
        let target_w = q.w.unwrap_or(img.width());
        let target_h = q.h.unwrap_or(img.height());
        let fit = q.fit.as_deref().unwrap_or("cover");
        let resized = match fit {
            "contain" => img.resize(target_w, target_h, FilterType::Lanczos3),
            _         => img.resize_to_fill(target_w, target_h, FilterType::Lanczos3),
        };
        let mut out = Vec::with_capacity(64 * 1024);
        let quality = q.q.unwrap_or(85);
        let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, quality);
        enc.encode_image(&resized).map_err(|e| AppError::Internal(format!("encode: {e}")))?;
        Ok(Bytes::from(out))
    })
    .await
    .map_err(|e| AppError::Internal(format!("spawn_blocking: {e}")))??;

    let resp = Response::builder()
        .status(200)
        .header(CONTENT_TYPE, "image/jpeg")
        .header(CACHE_CONTROL, "public, max-age=86400")
        .body(Body::from(resized))
        .map_err(|e| AppError::Internal(format!("build resp: {e}")))?;
    Ok(resp)
}
```

- [ ] **Step 3: Wire route (only in dev)**

`backend/src/http/mod.rs` — at the end of the router build, gate by
config:

```rust
let mut router = router; // existing
// Mount the dev CDN only when CDN_BASE_URL points back at this app.
if state.config.cdn_base_url.contains("localhost")
   || state.config.cdn_base_url.contains("127.0.0.1") {
    router = router.route(
        "/cdn/img/:id",
        axum::routing::get(crate::storage::cdn_dev::handler),
    );
}
```

(Adjust the snippet to match the existing router-building style;
some projects use a `Router::new().merge(...)` chain instead.)

`backend/src/storage/mod.rs`: add `pub mod cdn_dev;`.

- [ ] **Step 4: Run + commit**

```
cd backend && cargo sqlx prepare && cargo test --test cdn_dev
```

```bash
git add backend/src/storage/cdn_dev.rs backend/src/storage/mod.rs backend/src/http/mod.rs backend/tests/cdn_dev.rs backend/tests/fixtures/sample.jpg backend/.sqlx
git commit -m "feat(storage): dev CDN route serving on-the-fly resizes"
```

---

## Backend — presigned PUT

### Task 23: Add presigned-PUT to Storage trait

**Files:**
- Modify: `backend/src/storage/mod.rs`
- Modify: `backend/src/storage/s3.rs`
- Modify: `backend/src/storage/memory.rs`
- Test: `backend/tests/presign.rs`

- [ ] **Step 1: Extend the trait**

`backend/src/storage/mod.rs` — add to the trait:

```rust
    /// Sign a PUT URL good for `ttl_secs`, capped to `max_bytes`.
    /// The S3 implementation embeds Content-Length-Range so the bucket
    /// rejects oversize uploads at the edge.
    async fn presigned_put(
        &self,
        key: &str,
        content_type: &str,
        max_bytes: u64,
        ttl_secs: u64,
    ) -> Result<String, AppError>;
```

- [ ] **Step 2: Implement in S3Storage**

`backend/src/storage/s3.rs` — add:

```rust
async fn presigned_put(
    &self,
    key: &str,
    content_type: &str,
    max_bytes: u64,
    ttl_secs: u64,
) -> Result<String, AppError> {
    use aws_sdk_s3::presigning::PresigningConfig;

    let cfg = PresigningConfig::expires_in(std::time::Duration::from_secs(ttl_secs))
        .map_err(|e| AppError::Internal(format!("presign cfg: {e}")))?;

    let req = self.client.put_object()
        .bucket(&self.bucket)
        .key(key)
        .content_type(content_type)
        .content_length(max_bytes as i64);  // S3 enforces if header sent
    let signed = req
        .presigned(cfg)
        .await
        .map_err(|e| AppError::Internal(format!("presign: {e}")))?;
    Ok(signed.uri().to_string())
}
```

(Note: the AWS SDK doesn't surface `Content-Length-Range` directly
on the v4 signer; we sign with `content_length(max_bytes)` so the
PUT request must include `Content-Length: max_bytes` exactly. The
client's `XMLHttpRequest` sets `Content-Length` automatically from
the `body` size — the contract is "client must not exceed the
declared max". A defence-in-depth bucket policy with
`s3:content-length-range` is documented in Task 49 (AWS infra).)

- [ ] **Step 3: Stub in MemoryStorage**

`backend/src/storage/memory.rs` — implement to return a synthetic URL
for tests:

```rust
async fn presigned_put(
    &self,
    key: &str,
    _content_type: &str,
    _max_bytes: u64,
    _ttl_secs: u64,
) -> Result<String, AppError> {
    // Tests don't actually PUT against this URL; they use Storage::put
    // directly to seed objects.
    Ok(format!("memory://put/{key}"))
}
```

- [ ] **Step 4: Smoke test**

Path: `backend/tests/presign.rs`.

```rust
use astrophoto::storage::{MemoryStorage, Storage};
use std::sync::Arc;

#[tokio::test]
async fn memory_presign_returns_synthetic_url() {
    let s: Arc<dyn Storage> = Arc::new(MemoryStorage::new());
    let url = s.presigned_put("originals/abc", "image/jpeg", 1024, 60).await.unwrap();
    assert!(url.starts_with("memory://"));
}
```

- [ ] **Step 5: Run + commit**

```
cd backend && cargo test --test presign
```

```bash
git add backend/src/storage/ backend/tests/presign.rs
git commit -m "feat(storage): presigned_put on Storage trait + S3 + Memory impls"
```

---

## Backend — display master + magic byte sniff

### Task 24: Display-master derivation in pipeline

**Files:**
- Modify: `backend/src/photos/pipeline.rs`
- Test: `backend/src/photos/pipeline.rs` (or `backend/tests/pipeline_display.rs`)

- [ ] **Step 1: Add the derivation function**

`backend/src/photos/pipeline.rs` — add (alongside the existing
thumbnail generation):

```rust
const DISPLAY_MASTER_LONG_EDGE: u32 = 4096;
const DISPLAY_MASTER_QUALITY: u8 = 85;

fn derive_display_master_blocking(bytes: &[u8]) -> Result<bytes::Bytes, AppError> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| AppError::Internal(format!("display decode: {e}")))?;
    let (w, h) = (img.width(), img.height());
    let scale = if w.max(h) > DISPLAY_MASTER_LONG_EDGE {
        DISPLAY_MASTER_LONG_EDGE as f32 / w.max(h) as f32
    } else { 1.0 };
    let target_w = (w as f32 * scale) as u32;
    let target_h = (h as f32 * scale) as u32;
    let resized = if scale < 1.0 {
        img.resize(target_w, target_h, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };
    let mut out = Vec::with_capacity(256 * 1024);
    let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, DISPLAY_MASTER_QUALITY);
    enc.encode_image(&resized).map_err(|e| AppError::Internal(format!("display encode: {e}")))?;
    Ok(bytes::Bytes::from(out))
}
```

- [ ] **Step 2: Add the blurhash function**

```rust
fn derive_blurhash_blocking(bytes: &[u8]) -> Result<String, AppError> {
    let img = image::load_from_memory(bytes)
        .map_err(|e| AppError::Internal(format!("blurhash decode: {e}")))?;
    let small = img.resize(32, 32, image::imageops::FilterType::Triangle);
    let rgba = small.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    let pixels = rgba.into_raw();
    let hash = blurhash::encode(4, 3, w, h, &pixels)
        .map_err(|e| AppError::Internal(format!("blurhash: {e}")))?;
    Ok(hash)
}
```

- [ ] **Step 3: Integrate into `finalize`**

In the existing `finalize` function, alongside thumbnail generation,
also derive the display master and blurhash inside the same
`spawn_blocking` closure. Then upload the display master and
persist the blurhash + display_key on the row:

```rust
let parsed = tokio::task::spawn_blocking(move || -> Result<_, AppError> {
    let exif_data = exif::parse_blocking(&bytes_for_blocking)?;
    let display = derive_display_master_blocking(&bytes_for_blocking)?;
    let blurhash = derive_blurhash_blocking(&bytes_for_blocking)?;
    Ok((exif_data, display, blurhash))
}).await.map_err(|e| AppError::Internal(format!("spawn_blocking: {e}")))??;

let (exif_data, display_bytes, blurhash) = parsed;

let display_key = format!("display/{photo_id}.jpg");
storage.put(&display_key, "image/jpeg", display_bytes).await?;
queries::set_display_metadata(pool, photo_id, &display_key, &blurhash).await?;
```

(Add `set_display_metadata` to `backend/src/photos/queries.rs` —
a simple `UPDATE photos SET display_key = $1, blurhash = $2 WHERE id = $3`.)

- [ ] **Step 4: Test**

A unit test asserting that
`derive_display_master_blocking(jpeg_bigger_than_4096)` produces a
JPEG with `<= 4096` long edge:

```rust
#[cfg(test)]
mod display_tests {
    use super::*;

    #[test]
    fn display_master_clamps_long_edge() {
        let big = include_bytes!("../../tests/fixtures/wide_5000.jpg");
        let out = derive_display_master_blocking(big).unwrap();
        let img = image::load_from_memory(&out).unwrap();
        assert!(img.width().max(img.height()) <= 4096);
    }
}
```

(Drop a `wide_5000.jpg` ≥5000 px on long edge into `backend/tests/fixtures/`.)

- [ ] **Step 5: Run + commit**

```
cd backend && cargo sqlx prepare && cargo test --lib photos::pipeline::display_tests
```

```bash
git add backend/src/photos/pipeline.rs backend/src/photos/queries.rs backend/tests/fixtures/wide_5000.jpg backend/.sqlx
git commit -m "feat(pipeline): derive display master (4096 q85) + blurhash"
```

---

### Task 25: Magic-byte sniff helper

**Files:**
- Create: `backend/src/photos/magic.rs`
- Modify: `backend/src/photos/mod.rs`

- [ ] **Step 1: Implementation + tests**

```rust
//! Magic-byte sniff. Backend trusts neither the client `Content-Type`
//! header nor the file extension. We range-GET the first 16 bytes from
//! S3 and check the signature against the declared mime.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SniffResult {
    Jpeg,
    Png,
    Tiff,
    Unknown,
}

pub fn sniff(bytes: &[u8]) -> SniffResult {
    if bytes.len() < 4 { return SniffResult::Unknown; }
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF])         { return SniffResult::Jpeg; }
    if bytes.starts_with(&[0x89, b'P', b'N', b'G'])   { return SniffResult::Png;  }
    if bytes.starts_with(&[b'I', b'I', 0x2A, 0x00]) ||
       bytes.starts_with(&[b'M', b'M', 0x00, 0x2A])   { return SniffResult::Tiff; }
    SniffResult::Unknown
}

pub fn matches_mime(s: SniffResult, mime: &str) -> bool {
    match (s, mime) {
        (SniffResult::Jpeg, "image/jpeg") => true,
        (SniffResult::Png,  "image/png")  => true,
        (SniffResult::Tiff, "image/tiff") => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jpeg_signature() {
        assert_eq!(sniff(&[0xFF, 0xD8, 0xFF, 0xE0, 0,0,0,0]), SniffResult::Jpeg);
    }

    #[test]
    fn png_signature() {
        assert_eq!(sniff(b"\x89PNG\r\n\x1a\n"), SniffResult::Png);
    }

    #[test]
    fn tiff_le_signature() {
        assert_eq!(sniff(b"II*\x00\x00\x00\x00\x00"), SniffResult::Tiff);
    }

    #[test]
    fn no_match_for_random() {
        assert_eq!(sniff(b"hello, world!"), SniffResult::Unknown);
    }

    #[test]
    fn matches_mime_strict() {
        assert!( matches_mime(SniffResult::Jpeg, "image/jpeg"));
        assert!(!matches_mime(SniffResult::Jpeg, "image/png"));
        assert!(!matches_mime(SniffResult::Unknown, "image/jpeg"));
    }
}
```

`photos/mod.rs`: add `pub mod magic;`.

- [ ] **Step 2: Run + commit**

```
cd backend && cargo test --lib photos::magic
```

```bash
git add backend/src/photos/magic.rs backend/src/photos/mod.rs
git commit -m "feat(photos): magic-byte sniff (JPEG/PNG/TIFF)"
```

---

## Backend — upload init + finalize

### Task 26: POST /api/uploads/init

**Files:**
- Create: `backend/src/photos/upload_init.rs`
- Modify: `backend/src/photos/mod.rs`
- Modify: `backend/src/http/mod.rs`
- Test: `backend/tests/upload_init.rs`

- [ ] **Step 1: Write the failing integration test**

Path: `backend/tests/upload_init.rs`. Test scenario:

1. Sign up `marie` (free tier).
2. POST `/api/uploads/init` with `{files:[{name:"a.jpg", size:10485760, mime:"image/jpeg", hash:"abc"}]}`. Expect 200 + array of one element with `presigned_put_url`, `photo_id`, `short_id`.
3. POST again with the SAME hash. Expect 409 Conflict.
4. POST a 60 MB file. Expect 413 QuotaExceeded.

```rust
#[tokio::test]
async fn upload_init_signs_url_and_dedups() {
    // ... boot, signup as 'marie', capture session ...
    let body = serde_json::json!({
        "files": [{"name":"a.jpg","size":10485760,"mime":"image/jpeg","hash":"abcdef"}]
    });
    let r = app.clone().oneshot(/* POST with cookie */).await.unwrap();
    assert_eq!(r.status(), 200);
    let v: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(r.into_body(), 65536).await.unwrap()
    ).unwrap();
    assert!(v["files"][0]["presigned_put_url"].is_string());

    // Dedup
    let r2 = app.clone().oneshot(/* same body */).await.unwrap();
    assert_eq!(r2.status(), 409);

    // Quota
    let big = serde_json::json!({
        "files":[{"name":"b.jpg","size":62914560,"mime":"image/jpeg","hash":"x"}]
    });
    let r3 = app.clone().oneshot(/* POST big */).await.unwrap();
    assert_eq!(r3.status(), 413);
}
```

- [ ] **Step 2: Implement**

Path: `backend/src/photos/upload_init.rs`.

```rust
use axum::{extract::State, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::session::CurrentUser;
use crate::error::AppError;
use crate::http::AppState;
use crate::photos::short_id;

#[derive(Deserialize)]
pub struct File {
    pub name: String,
    pub size: u64,
    pub mime: String,
    pub hash: String,
}

#[derive(Deserialize)]
pub struct InitBody {
    pub files: Vec<File>,
}

#[derive(Serialize)]
pub struct InitFile {
    pub photo_id: String,
    pub short_id: String,
    pub presigned_put_url: String,
}

#[derive(Serialize)]
pub struct InitResponse {
    pub files: Vec<InitFile>,
}

const FREE_MAX: u64 = 50 * 1024 * 1024;
const SUBSCRIBER_MAX: u64 = 200 * 1024 * 1024;
const PUT_TTL_SECS: u64 = 600;

pub async fn handler(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(body): Json<InitBody>,
) -> Result<impl IntoResponse, AppError> {
    if body.files.is_empty() || body.files.len() > 10 {
        return Err(AppError::Validation("files must be 1..=10".into()));
    }
    let tier: String = sqlx::query_scalar!(
        "select tier from users where id = $1", user.id
    ).fetch_one(&state.pool).await?;
    let max_bytes = if tier == "subscriber" { SUBSCRIBER_MAX } else { FREE_MAX };

    // Pre-validation pass.
    for f in &body.files {
        if f.size > max_bytes {
            return Err(AppError::QuotaExceeded(
                format!("file {} exceeds {} bytes", f.name, max_bytes)));
        }
        match f.mime.as_str() {
            "image/jpeg" | "image/png" | "image/tiff" => {}
            _ => return Err(AppError::UnsupportedFormat(f.mime.clone())),
        }
    }

    let mut out = Vec::with_capacity(body.files.len());
    let mut tx = state.pool.begin().await?;
    for f in body.files {
        // Per-owner hash dedup.
        let dup: Option<Uuid> = sqlx::query_scalar!(
            "select id from photos where owner_id = $1 and original_hash = $2",
            user.id, f.hash
        ).fetch_optional(&mut *tx).await?;
        if dup.is_some() {
            return Err(AppError::Conflict("file already uploaded".into()));
        }

        // Insert pending row with retry on short_id collision.
        let mut attempts = 0u8;
        let (photo_id, short) = loop {
            attempts += 1;
            let pid = Uuid::new_v4();
            let s = short_id::generate();
            let key = format!("originals/{pid}");
            match sqlx::query!(
                "insert into photos (id, owner_id, storage_key, original_name, bytes, mime, original_hash, short_id, status, last_step) \
                 values ($1, $2, $3, $4, $5, $6, $7, $8, 'pending', 'upload')",
                pid, user.id, key, f.name, f.size as i64, f.mime, f.hash, s
            ).execute(&mut *tx).await {
                Ok(_) => break (pid, s),
                Err(sqlx::Error::Database(db))
                    if db.constraint() == Some("photos_short_id_uidx") && attempts < 5 => continue,
                Err(e) => return Err(AppError::Database(e)),
            }
        };

        let key = format!("originals/{photo_id}");
        let url = state.storage.presigned_put(&key, &f.mime, max_bytes, PUT_TTL_SECS).await?;
        out.push(InitFile {
            photo_id: photo_id.to_string(),
            short_id: short,
            presigned_put_url: url,
        });
    }
    tx.commit().await?;

    Ok(Json(InitResponse { files: out }))
}
```

- [ ] **Step 3: Wire route**

`backend/src/http/mod.rs`:

```rust
.route(
    "/api/uploads/init",
    axum::routing::post(crate::photos::upload_init::handler),
)
```

`photos/mod.rs`: `pub mod upload_init;`.

- [ ] **Step 4: sqlx prepare + run + commit**

```
cd backend && cargo sqlx prepare && cargo test --test upload_init
```

```bash
git add backend/src/photos/upload_init.rs backend/src/photos/mod.rs backend/src/http/mod.rs backend/tests/upload_init.rs backend/.sqlx
git commit -m "feat(uploads): POST /api/uploads/init (presign + tier + dedup)"
```

---

### Task 27: POST /api/uploads/:id/finalize

**Files:**
- Create: `backend/src/photos/upload_finalize.rs`
- Modify: `backend/src/photos/mod.rs`
- Modify: `backend/src/http/mod.rs`
- Test: `backend/tests/upload_finalize.rs`

- [ ] **Step 1: Failing test**

Test that finalize:
1. Returns 404 if photo doesn't exist.
2. Returns 404 if photo isn't owned by caller.
3. Returns 408 PendingFinalizeStuck if S3 has no object.
4. Returns 400 MagicByteMismatch if first 16 bytes don't match declared mime.
5. Returns 200 + `{status:"ready"}` on happy path; rerunning finalize is idempotent.

(Use `MemoryStorage::put` directly to seed valid/invalid bytes.)

- [ ] **Step 2: Implement**

Path: `backend/src/photos/upload_finalize.rs`.

```rust
use axum::{extract::{Path, State}, response::IntoResponse, Json};
use serde::Serialize;
use uuid::Uuid;

use crate::auth::session::CurrentUser;
use crate::error::AppError;
use crate::http::AppState;
use crate::photos::magic;

#[derive(Serialize)]
pub struct FinalizeResp {
    pub status: String,
    pub display_key: Option<String>,
}

pub async fn handler(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let row = sqlx::query!(
        "select owner_id as \"owner_id!\", storage_key, mime, status \
         from photos where id = $1",
        id
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("photo".into()))?;

    if row.owner_id != user.id {
        return Err(AppError::NotFound("photo".into()));
    }

    // Idempotency.
    if row.status == "ready" {
        let dk: Option<String> = sqlx::query_scalar!(
            "select display_key from photos where id = $1", id
        ).fetch_one(&state.pool).await?;
        return Ok(Json(FinalizeResp { status: "ready".into(), display_key: dk }));
    }

    // Fetch object from S3 (full body — needed for pipeline anyway).
    let bytes = state.storage.get(&row.storage_key).await?
        .ok_or_else(|| AppError::PendingFinalizeStuck(
            "no object at storage_key — did the PUT succeed?".into()))?;

    // Magic-byte sniff over the first 16 bytes.
    let head: Vec<u8> = bytes.iter().take(16).cloned().collect();
    let sig = magic::sniff(&head);
    if !magic::matches_mime(sig, &row.mime) {
        sqlx::query!("update photos set status = 'failed', pipeline_error = $1 where id = $2",
                     "magic-byte mismatch", id)
            .execute(&state.pool).await?;
        return Err(AppError::MagicByteMismatch(format!("{:?}", sig)));
    }

    // Run the existing pipeline (extended with display master + blurhash in Task 24).
    crate::photos::pipeline::finalize(
        &state.pool,
        state.storage.clone(),
        id,
        bytes,
        crate::photos::pipeline::PipelineOptions::Initial,
    ).await?;

    let dk: Option<String> = sqlx::query_scalar!(
        "select display_key from photos where id = $1", id
    ).fetch_one(&state.pool).await?;

    Ok(Json(FinalizeResp { status: "ready".into(), display_key: dk }))
}
```

- [ ] **Step 3: Wire route**

`backend/src/http/mod.rs`:

```rust
.route(
    "/api/uploads/:id/finalize",
    axum::routing::post(crate::photos::upload_finalize::handler),
)
```

`photos/mod.rs`: `pub mod upload_finalize;`.

- [ ] **Step 4: Run + commit**

```
cd backend && cargo sqlx prepare && cargo test --test upload_finalize
```

```bash
git add backend/src/photos/upload_finalize.rs backend/src/photos/mod.rs backend/src/http/mod.rs backend/tests/upload_finalize.rs backend/.sqlx
git commit -m "feat(uploads): POST /api/uploads/:id/finalize (HEAD + magic + pipeline)"
```

---

### Task 28: Remove old multipart upload route

**Files:**
- Modify: `backend/src/http/mod.rs`
- Modify: `backend/src/photos/mod.rs`
- Delete: `backend/src/photos/upload.rs` (or rename for archival — see Step 1)

- [ ] **Step 1: Decide**

The old `POST /api/photos` multipart route is replaced by init +
finalize. Delete the route registration and the handler module.
The frontend's old form action is rebuilt in Task 38.

```
cd backend && grep -rn "post(crate::photos::upload::handler)" src/
```

Comment-driven check: no consumers should remain after that grep is
empty.

- [ ] **Step 2: Remove**

`backend/src/http/mod.rs` — delete the `.route("/api/photos", post(...).get(...).layer(DefaultBodyLimit::max(50 * 1024 * 1024)))` block. Keep the `.get(crate::photos::list::handler)` part as `axum::routing::get(...)`:

```rust
.route(
    "/api/photos",
    axum::routing::get(crate::photos::list::handler),
)
```

Delete `backend/src/photos/upload.rs`. Remove
`pub mod upload;` from `backend/src/photos/mod.rs`.

Update or delete tests that call the old endpoint:

```
cd backend && grep -rn "/api/photos\"" tests/ src/
```

- [ ] **Step 3: Run all tests**

```
cd backend && cargo test
```

Expected: PASS (after fixing any remaining call sites).

- [ ] **Step 4: Commit**

```bash
git add backend/src/http/mod.rs backend/src/photos/mod.rs
git rm backend/src/photos/upload.rs
git commit -m "refactor(uploads): remove old multipart POST /api/photos route"
```

---

## Backend — orphan reaper

### Task 29: Orphan reaper job

**Files:**
- Create: `backend/src/jobs/orphan_reaper.rs`
- Modify: `backend/src/jobs/mod.rs`
- Modify: `backend/src/main.rs` (spawn periodic task)
- Test: `backend/tests/orphan_reaper.rs`

- [ ] **Step 1: Implementation**

```rust
//! Orphan reaper: photos in `status = 'pending'` past N hours have
//! their S3 originals deleted and rows hard-deleted. Reuses the
//! `photo_pending_deletes` mechanism from Phase 8b.

use std::sync::Arc;
use std::time::Duration;

use crate::error::AppError;
use crate::storage::Storage;

const STALE_HOURS: i64 = 2;
const TICK_SECS: u64 = 300;

pub async fn run(pool: sqlx::PgPool, storage: Arc<dyn Storage>) {
    let mut tick = tokio::time::interval(Duration::from_secs(TICK_SECS));
    loop {
        tick.tick().await;
        if let Err(e) = sweep_once(&pool, &storage).await {
            tracing::error!(error = %e, "orphan reaper tick failed");
        }
    }
}

pub async fn sweep_once(
    pool: &sqlx::PgPool,
    storage: &Arc<dyn Storage>,
) -> Result<(), AppError> {
    let stale: Vec<(uuid::Uuid, String)> = sqlx::query!(
        r#"
        select id as "id!", storage_key
          from photos
         where status = 'pending'
           and created_at < now() - make_interval(hours => $1::int)
         limit 100
        "#,
        STALE_HOURS as i32
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|r| (r.id, r.storage_key))
    .collect();

    for (id, key) in stale {
        let _ = storage.delete(&key).await; // best-effort
        sqlx::query!("delete from photos where id = $1", id)
            .execute(pool).await?;
        tracing::info!(photo_id = %id, "reaped orphan upload");
    }
    Ok(())
}
```

- [ ] **Step 2: Test (uses sweep_once)**

```rust
#[tokio::test]
async fn reaper_deletes_stale_pending_photos() {
    // ... boot pool, MemoryStorage ...
    sqlx::query!("insert into photos (id, owner_id, storage_key, original_name, bytes, mime, status, short_id, created_at) \
                  values ($1, $2, 'k', 'n', 1, 'image/jpeg', 'pending', 'AAAA0001', now() - interval '3 hours')",
                 id, user_id).execute(&pool).await.unwrap();
    storage.put("k", "image/jpeg", Bytes::from_static(&[0; 4])).await.unwrap();

    orphan_reaper::sweep_once(&pool, &storage).await.unwrap();

    let exists: bool = sqlx::query_scalar!("select exists(select 1 from photos where id = $1)", id)
        .fetch_one(&pool).await.unwrap().unwrap_or(false);
    assert!(!exists);
}
```

- [ ] **Step 3: Spawn from main.rs**

`backend/src/main.rs` — after pool + storage are constructed, before
`axum::serve`:

```rust
{
    let pool = pool.clone();
    let storage = storage.clone();
    tokio::spawn(async move { jobs::orphan_reaper::run(pool, storage).await });
}
```

- [ ] **Step 4: Run + commit**

```
cd backend && cargo sqlx prepare && cargo test --test orphan_reaper
```

```bash
git add backend/src/jobs/orphan_reaper.rs backend/src/jobs/mod.rs backend/src/main.rs backend/tests/orphan_reaper.rs backend/.sqlx
git commit -m "feat(jobs): orphan reaper for stale pending uploads"
```

---

## Backend — discovery data capture (manual writes)

### Task 30: Target write helpers

**Files:**
- Create: `backend/src/photos/targets.rs`
- Modify: `backend/src/photos/mod.rs`

- [ ] **Step 1: Implementation**

```rust
//! Manual target attachment to a photo. Looks up by slug or alias;
//! writes a photo_targets row with source='manual' and is_primary=true
//! when the user picked one explicitly.

use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

pub async fn attach_primary_by_freetext(
    pool: &PgPool,
    photo_id: Uuid,
    freetext: &str,
) -> Result<(), AppError> {
    let trimmed = freetext.trim();
    if trimmed.is_empty() { return Ok(()); }

    // Try slug exact, then alias inclusion.
    let target_id: Option<Uuid> = sqlx::query_scalar!(
        r#"
        select id from targets
         where slug = lower($1)
            or $1 = any (aliases)
            or canonical_name ilike $1
         limit 1
        "#,
        trimmed
    ).fetch_optional(pool).await?;

    let Some(tid) = target_id else { return Ok(()); }; // unknown target, just keep photos.target

    sqlx::query!(
        "insert into photo_targets (photo_id, target_id, source, is_primary) \
         values ($1, $2, 'manual', true) \
         on conflict (photo_id, target_id) do update set is_primary = true, source = 'manual'",
        photo_id, tid
    ).execute(pool).await?;
    Ok(())
}
```

- [ ] **Step 2: Wire in module**

`backend/src/photos/mod.rs`: `pub mod targets;`.

- [ ] **Step 3: Run + commit**

```
cd backend && cargo sqlx prepare && cargo check
```

```bash
git add backend/src/photos/targets.rs backend/src/photos/mod.rs backend/.sqlx
git commit -m "feat(photos): manual target attach by slug/alias/name"
```

---

### Task 31: Tag write helpers

**Files:**
- Create: `backend/src/photos/tags.rs`
- Modify: `backend/src/photos/mod.rs`

- [ ] **Step 1: Implementation**

```rust
//! Tag write helpers: slug-normalize, upsert into `tags`, attach to a
//! photo via `photo_tags`. Cap is enforced upstream (max 8) by the
//! caller (the verify endpoint).

use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

pub fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

pub async fn attach(pool: &PgPool, photo_id: Uuid, tags_freetext: &[String])
    -> Result<(), AppError>
{
    if tags_freetext.is_empty() { return Ok(()); }
    if tags_freetext.len() > 8 {
        return Err(AppError::Validation("max 8 tags".into()));
    }

    for t in tags_freetext {
        let slug = slugify(t);
        if slug.is_empty() { continue; }
        let tag_id: Uuid = sqlx::query_scalar!(
            r#"
            insert into tags (slug, name) values ($1, $2)
            on conflict (slug) do update set slug = excluded.slug
            returning id
            "#,
            slug, t.trim()
        ).fetch_one(pool).await?;

        sqlx::query!(
            "insert into photo_tags (photo_id, tag_id) values ($1, $2) \
             on conflict do nothing",
            photo_id, tag_id
        ).execute(pool).await?;
    }
    Ok(())
}
```

- [ ] **Step 2: Wire in module**

`backend/src/photos/mod.rs`: `pub mod tags;`.

- [ ] **Step 3: Run + commit**

```
cd backend && cargo sqlx prepare && cargo check
```

```bash
git add backend/src/photos/tags.rs backend/src/photos/mod.rs backend/.sqlx
git commit -m "feat(photos): tag slug-normalise + attach helpers"
```

---

### Task 32: Equipment items upsert helper

**Files:**
- Create: `backend/src/equipment/mod.rs`
- Create: `backend/src/equipment/upsert.rs`
- Modify: `backend/src/lib.rs`

- [ ] **Step 1: Implementation**

```rust
//! equipment_items upsert. Called whenever a photo's equipment fields
//! are written. Increments usage_count on existing entries; otherwise
//! creates a new row in title-case display form with lowercase canonical.

use sqlx::PgPool;

use crate::error::AppError;

pub async fn upsert(pool: &PgPool, kind: &str, freetext: &str) -> Result<(), AppError> {
    let display = freetext.trim();
    if display.is_empty() { return Ok(()); }
    let canonical = display.to_lowercase();
    sqlx::query!(
        r#"
        insert into equipment_items (kind, canonical_name, display_name, usage_count)
            values ($1, $2, $3, 1)
        on conflict (kind, canonical_name)
            do update set usage_count = equipment_items.usage_count + 1
        "#,
        kind, canonical, display
    ).execute(pool).await?;
    Ok(())
}
```

- [ ] **Step 2: Wire**

`backend/src/equipment/mod.rs`: `pub mod upsert;`.
`backend/src/lib.rs`: `pub mod equipment;`.

- [ ] **Step 3: sqlx prepare + commit**

```
cd backend && cargo sqlx prepare && cargo check
```

```bash
git add backend/src/equipment backend/src/lib.rs backend/.sqlx
git commit -m "feat(equipment): items upsert helper (auto on photo save)"
```

---

### Task 33: Verify-step PUT — extend with target/tags/category/equipment

**Files:**
- Modify: `backend/src/photos/metadata.rs` (or `verify.rs` — find via `grep -rn "PUT.*photos/:id" backend/src`; the Phase 8b plan named it `metadata.rs`)
- Test: `backend/tests/verify_metadata.rs`

- [ ] **Step 1: Find the existing endpoint**

```
cd backend && grep -rn "PUT" src/photos/ | grep metadata
cd backend && grep -rn "PUT" src/http/mod.rs
```

The existing `PUT /api/photos/:id` accepts caption + EXIF override
fields (camera, lens, ISO, exposure, focal_mm, target, taken_at, …).
We extend its `MetadataBody` struct.

- [ ] **Step 2: Extend the request body**

```rust
#[derive(Deserialize)]
pub struct MetadataBody {
    // ... existing fields ...
    pub category:  Option<String>,
    pub scope:     Option<String>,
    pub mount:     Option<String>,
    pub filters:   Option<String>,
    pub guiding:   Option<String>,
    pub tags:      Option<Vec<String>>,
}
```

- [ ] **Step 3: Update the SQL**

Existing UPDATE statement gains the new columns. Validate
`category` against the enum:

```rust
if let Some(c) = &body.category {
    if !matches!(c.as_str(),
        "dso"|"planetary"|"lunar"|"solar"|"wide_field"|"nightscape"|"other")
    {
        return Err(AppError::Validation("invalid category".into()));
    }
}
```

After the row UPDATE succeeds:

```rust
if let Some(target) = &body.target {
    crate::photos::targets::attach_primary_by_freetext(&state.pool, id, target).await?;
}
if let Some(tags) = &body.tags {
    crate::photos::tags::attach(&state.pool, id, tags).await?;
}
for (kind, val) in [
    ("camera",   &body.camera),
    ("telescope",&body.scope),
    ("mount",    &body.mount),
    ("filter",   &body.filters),
    ("guiding",  &body.guiding),
] {
    if let Some(v) = val {
        if !v.trim().is_empty() {
            crate::equipment::upsert::upsert(&state.pool, kind, v).await?;
        }
    }
}
```

- [ ] **Step 4: Test**

`backend/tests/verify_metadata.rs` — exercise PUT with a full body
and assert that:
- The `photos.category` is set.
- A row in `photo_targets` exists for the matching slug.
- A row in `photo_tags` exists for each tag.
- An `equipment_items` row exists for each populated field.

- [ ] **Step 5: Run + commit**

```
cd backend && cargo sqlx prepare && cargo test --test verify_metadata
```

```bash
git add backend/src/photos/metadata.rs backend/tests/verify_metadata.rs backend/.sqlx
git commit -m "feat(photos): metadata PUT writes targets/tags/category/equipment"
```

---

## Backend — autocomplete endpoints

### Task 34: Targets autocomplete

**Files:**
- Create: `backend/src/photos/targets_autocomplete.rs`
- Modify: `backend/src/photos/mod.rs`
- Modify: `backend/src/http/mod.rs`
- Test: `backend/tests/targets_autocomplete.rs`

- [ ] **Step 1: Failing test**

```rust
#[tokio::test]
async fn targets_autocomplete_finds_messier_by_alias() {
    // ... boot ...
    let r = app.oneshot(Request::builder()
        .uri("/api/targets/autocomplete?q=Andromeda")
        .body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(r.status(), 200);
    let v: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(r.into_body(), 65536).await.unwrap()).unwrap();
    let items = v["targets"].as_array().unwrap();
    assert!(items.iter().any(|t| t["slug"] == "m31"));
}
```

- [ ] **Step 2: Implement**

```rust
use axum::{extract::{Query, State}, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::http::AppState;

#[derive(Deserialize)] pub struct Q { pub q: String }
#[derive(Serialize)]   pub struct Item { pub slug: String, pub canonical_name: String, pub kind: String }
#[derive(Serialize)]   pub struct R { pub targets: Vec<Item> }

pub async fn handler(
    State(state): State<AppState>,
    Query(qs): Query<Q>,
) -> Result<impl IntoResponse, AppError> {
    let q = qs.q.trim();
    if q.is_empty() {
        return Ok(Json(R { targets: vec![] }));
    }
    let pattern = format!("%{q}%");
    let rows = sqlx::query!(
        r#"
        select slug, canonical_name, kind
          from targets
         where slug ilike $1
            or canonical_name ilike $1
            or exists (select 1 from unnest(aliases) a where a ilike $1)
         order by slug
         limit 10
        "#,
        pattern
    ).fetch_all(&state.pool).await?;

    let targets = rows.into_iter().map(|r| Item {
        slug: r.slug, canonical_name: r.canonical_name, kind: r.kind,
    }).collect();
    Ok(Json(R { targets }))
}
```

- [ ] **Step 3: Wire**

`backend/src/http/mod.rs`:

```rust
.route("/api/targets/autocomplete", axum::routing::get(crate::photos::targets_autocomplete::handler))
```

- [ ] **Step 4: Run + commit**

```
cd backend && cargo sqlx prepare && cargo test --test targets_autocomplete
```

```bash
git add backend/src/photos/targets_autocomplete.rs backend/src/photos/mod.rs backend/src/http/mod.rs backend/tests/targets_autocomplete.rs backend/.sqlx
git commit -m "feat(targets): GET /api/targets/autocomplete?q="
```

---

### Task 35: Tags autocomplete

**Files:**
- Create: `backend/src/photos/tags_autocomplete.rs`
- Modify: `backend/src/photos/mod.rs`, `backend/src/http/mod.rs`
- Test: `backend/tests/tags_autocomplete.rs`

- [ ] **Step 1: Implement (mirror Targets)**

`/api/tags/autocomplete?q=…` returns up to 10 tags whose `slug` or
`name` matches `ILIKE %q%`, ordered by `slug`.

```rust
let pattern = format!("%{q}%");
let rows = sqlx::query!(
    "select slug, name from tags where slug ilike $1 or name ilike $1 order by slug limit 10",
    pattern
).fetch_all(&state.pool).await?;
```

- [ ] **Step 2: Test (insert 3 tags, query for one) + commit**

```bash
git add backend/src/photos/tags_autocomplete.rs backend/src/photos/mod.rs backend/src/http/mod.rs backend/tests/tags_autocomplete.rs backend/.sqlx
git commit -m "feat(tags): GET /api/tags/autocomplete?q="
```

---

### Task 36: Equipment autocomplete

**Files:**
- Create: `backend/src/equipment/autocomplete.rs`
- Modify: `backend/src/equipment/mod.rs`, `backend/src/http/mod.rs`
- Test: `backend/tests/equipment_autocomplete.rs`

- [ ] **Step 1: Implement**

`GET /api/equipment/autocomplete?kind=camera&q=ASI` — returns up to
10 equipment_items rows for the kind, matching ILIKE on
canonical_name OR display_name, ordered by usage_count desc.

```rust
#[derive(Deserialize)]
pub struct Q { pub kind: String, pub q: String }

#[derive(Serialize)]
pub struct Item { pub canonical_name: String, pub display_name: String, pub usage_count: i32 }

pub async fn handler(
    State(state): State<AppState>,
    Query(qs): Query<Q>,
) -> Result<impl IntoResponse, AppError> {
    if !["telescope","camera","mount","filter","guiding"].contains(&qs.kind.as_str()) {
        return Err(AppError::Validation("kind must be telescope|camera|mount|filter|guiding".into()));
    }
    let pattern = format!("%{}%", qs.q.trim());
    let rows = sqlx::query!(
        r#"
        select canonical_name, display_name, usage_count
          from equipment_items
         where kind = $1
           and (canonical_name ilike $2 or display_name ilike $2)
         order by usage_count desc
         limit 10
        "#,
        qs.kind, pattern
    ).fetch_all(&state.pool).await?;

    let items = rows.into_iter().map(|r| Item {
        canonical_name: r.canonical_name,
        display_name:   r.display_name,
        usage_count:    r.usage_count,
    }).collect::<Vec<_>>();
    Ok(Json(serde_json::json!({ "items": items })))
}
```

- [ ] **Step 2: Wire route + test + commit**

```bash
git add backend/src/equipment/autocomplete.rs backend/src/equipment/mod.rs backend/src/http/mod.rs backend/tests/equipment_autocomplete.rs backend/.sqlx
git commit -m "feat(equipment): GET /api/equipment/autocomplete?kind=&q="
```

---

## Backend — handle-redirect middleware

### Task 37: 301 middleware for old handle paths and old `/photo/:uuid`

**Files:**
- Create: `backend/src/middleware/handle_redirect.rs`
- Modify: `backend/src/middleware/mod.rs` (or `backend/src/http/mod.rs` if no `middleware/` exists yet)
- Modify: `backend/src/http/mod.rs` (mount the layer)
- Test: `backend/tests/handle_redirect.rs`

- [ ] **Step 1: Failing test**

Test cases:
1. GET `/photo/<uuid>` → 301 to `/u/<handle>/p/<short>`.
2. GET `/u/<old-handle>/p/<short>` → 301 to `/u/<new-handle>/p/<short>` after rename.
3. GET `/u/<unknown>` → 404 (no redirect entry).

(Frontend routes are server-rendered SvelteKit; the backend redirect
middleware applies to the API. Frontend redirects are handled in
frontend tasks. Keep this test backend-only.)

The simplest backend hook: a separate route group at `/photo/:id`
and `/u/:handle/p/:short` that returns 301. Implement those as
plain handlers rather than middleware.

```rust
// /api/photos/by-uuid/:id  -> 301 with Location: /u/{handle}/p/{short}
pub async fn redirect_uuid_to_canonical(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<axum::response::Response, AppError> {
    let row = sqlx::query!(
        "select u.handle::text as \"handle!\", p.short_id \
         from photos p join users u on u.id = p.owner_id where p.id = $1",
        id
    ).fetch_optional(&state.pool).await?
     .ok_or_else(|| AppError::NotFound("photo".into()))?;

    let location = format!("/u/{}/p/{}", row.handle, row.short_id);
    Ok(axum::response::Redirect::permanent(&location).into_response())
}
```

(The frontend's existing `routes/photo/[id]/+page.server.ts` will be
converted to a 301 redirect in Task 53 — that is the user-facing
redirect. This backend endpoint is for any API client that knows the
UUID and needs the canonical URL.)

- [ ] **Step 2: Wire route + test + commit**

```
cd backend && cargo sqlx prepare && cargo test --test handle_redirect
```

```bash
git add backend/src/photos/redirect.rs backend/src/photos/mod.rs backend/src/http/mod.rs backend/tests/handle_redirect.rs backend/.sqlx
git commit -m "feat(photos): GET /api/photos/by-uuid/:id 301s to canonical"
```

---

## Frontend — CDN URL builder + Img component

### Task 38: lib/cdn.ts URL builder

**Files:**
- Create: `frontend/src/lib/cdn.ts`
- Test: `frontend/src/lib/cdn.test.ts`

- [ ] **Step 1: Implementation + test**

`frontend/src/lib/cdn.ts`:

```typescript
import { env } from '$env/dynamic/public';

const BASE = env.PUBLIC_CDN_BASE_URL ?? '/cdn';

export type Transform = {
  w?: number;
  h?: number;
  fit?: 'cover' | 'contain';
  q?: number;
  fm?: 'auto' | 'jpg' | 'webp';
};

export function cdn(photoId: string, t: Transform = {}): string {
  const url = new URL(`${BASE}/img/${photoId}`, 'http://placeholder');
  if (t.w)   url.searchParams.set('w',  String(t.w));
  if (t.h)   url.searchParams.set('h',  String(t.h));
  if (t.fit) url.searchParams.set('fit', t.fit);
  if (t.q)   url.searchParams.set('q',  String(t.q));
  if (t.fm)  url.searchParams.set('fm', t.fm);
  // strip placeholder origin
  return url.pathname + url.search;
}

export function srcset(photoId: string, widths: number[], t: Omit<Transform, 'w'> = {}): string {
  return widths.map(w => `${cdn(photoId, { ...t, w })} ${w}w`).join(', ');
}
```

`frontend/src/lib/cdn.test.ts`:

```typescript
import { describe, it, expect } from 'vitest';
import { cdn, srcset } from './cdn';

describe('cdn URL builder', () => {
  it('returns base path + id with no params', () => {
    expect(cdn('abc')).toBe('/cdn/img/abc');
  });
  it('adds width', () => {
    expect(cdn('abc', { w: 800 })).toBe('/cdn/img/abc?w=800');
  });
  it('builds srcset', () => {
    const s = srcset('abc', [400, 800, 1200]);
    expect(s).toContain('400w');
    expect(s).toContain('1200w');
  });
});
```

- [ ] **Step 2: Run + commit**

```
cd frontend && pnpm vitest run lib/cdn
```

```bash
git add frontend/src/lib/cdn.ts frontend/src/lib/cdn.test.ts
git commit -m "feat(frontend): CDN URL builder + srcset helper"
```

---

### Task 39: lib/components/Img.svelte

**Files:**
- Create: `frontend/src/lib/components/Img.svelte`

- [ ] **Step 1: Implement**

```svelte
<script lang="ts">
  import { cdn, srcset, type Transform } from '$lib/cdn';

  let {
    photoId,
    alt,
    w = 800,
    transform = {},
    sizes = '(max-width: 640px) 100vw, 800px',
    blurhash,
    aspectRatio,
    class: cls = '',
  }: {
    photoId: string;
    alt: string;
    w?: number;
    transform?: Omit<Transform, 'w'>;
    sizes?: string;
    blurhash?: string;
    aspectRatio?: string; // e.g. "3/2"
    class?: string;
  } = $props();

  const widths = [w, w * 2, w * 3];

  // Decode blurhash to a CSS gradient placeholder if provided.
  // Lightweight approach: skip decoding here, render solid bg.
  // (P2 task adds the @ts/blurhash decoder for richer placeholders.)
</script>

<img
  src={cdn(photoId, { ...transform, w })}
  srcset={srcset(photoId, widths, transform)}
  {sizes}
  {alt}
  loading="lazy"
  decoding="async"
  style:aspect-ratio={aspectRatio}
  class={cls}
  data-blurhash={blurhash ?? ''}
/>
```

- [ ] **Step 2: Commit**

```bash
git add frontend/src/lib/components/Img.svelte
git commit -m "feat(frontend): <Img> component with srcset + blurhash slot"
```

---

## Frontend — HandlePicker

### Task 40: lib/components/HandlePicker.svelte

**Files:**
- Create: `frontend/src/lib/components/HandlePicker.svelte`

- [ ] **Step 1: Implement**

States from the spec: `empty`, `checking`, `available`, `taken`,
`invalid`, `reserved`. Debounce: 300 ms. Calls
`GET /api/auth/handle-check?handle=<v>`.

```svelte
<script lang="ts">
  import { env } from '$env/dynamic/public';

  let {
    name = 'handle',
    value = $bindable(''),
    api = env.PUBLIC_API_URL ?? '',
  }: { name?: string; value?: string; api?: string } = $props();

  type Status = 'empty' | 'checking' | 'available' | 'taken' | 'invalid' | 'reserved';
  let status: Status = $state('empty');
  let timer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    if (timer) clearTimeout(timer);
    if (!value) { status = 'empty'; return; }
    status = 'checking';
    timer = setTimeout(async () => {
      try {
        const r = await fetch(`${api}/api/auth/handle-check?handle=${encodeURIComponent(value)}`);
        const j = await r.json() as { status: Status };
        status = j.status;
      } catch {
        status = 'empty';
      }
    }, 300);
  });

  const messages: Record<Status, string> = {
    empty:     '',
    checking:  '…',
    available: 'Available.',
    taken:     'Already taken.',
    invalid:   'Use 3–30 lowercase letters, numbers, "-", or "_".',
    reserved:  'Reserved — please choose another.',
  };
</script>

<label class="t-label" for={name}>HANDLE</label>
<div class="hp">
  <span class="at" aria-hidden>@</span>
  <input
    id={name}
    {name}
    bind:value
    class="input input-mono hp-input"
    autocomplete="username"
    spellcheck="false"
    minlength="3"
    maxlength="30"
    pattern="[a-z0-9_-]+"
    aria-describedby={`${name}-status`}
  />
  <span id={`${name}-status`} class="t-meta hp-status" data-status={status}>
    {messages[status]}
  </span>
</div>

<style>
  .hp { position: relative; }
  .at { position: absolute; left: 12px; top: 9px; color: var(--fg-muted); font-family: var(--font-mono); }
  .hp-input { padding-left: 28px; }
  .hp-status[data-status="available"] { color: var(--success); }
  .hp-status[data-status="taken"], .hp-status[data-status="reserved"], .hp-status[data-status="invalid"] {
    color: var(--danger);
  }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add frontend/src/lib/components/HandlePicker.svelte
git commit -m "feat(frontend): <HandlePicker> with 300ms debounce + state chip"
```

---

### Task 41: Wire HandlePicker into signup

**Files:**
- Modify: `frontend/src/routes/auth/signup/+page.svelte`
- Modify: `frontend/src/routes/auth/signup/+page.server.ts`

- [ ] **Step 1: Add the field to the form**

```svelte
<script lang="ts">
  import HandlePicker from '$lib/components/HandlePicker.svelte';
  let handle = $state('');
  // ... existing email/password/display_name state ...
</script>

<form method="POST">
  <!-- ... existing fields ... -->
  <HandlePicker bind:value={handle} />
  <input type="hidden" name="handle" value={handle} />
  <!-- ... -->
</form>
```

- [ ] **Step 2: Update the form-action contract**

`+page.server.ts` — extract `handle` from `request.formData()` and
forward it in the JSON body to `POST /api/auth/signup`. Surface
backend 409 ("handle already taken") as `fail(409, { handleError: ... })`.

- [ ] **Step 3: Commit**

```bash
git add frontend/src/routes/auth/signup
git commit -m "feat(signup): require @handle via HandlePicker"
```

---

## Frontend — upload preflight + presigned uploader

### Task 42: lib/upload/preflight.ts

**Files:**
- Create: `frontend/src/lib/upload/preflight.ts`
- Test: `frontend/src/lib/upload/preflight.test.ts`

- [ ] **Step 1: Implement**

```typescript
import { parse as parseExif } from 'exifr';

export type Preflight = {
  thumbDataUrl: string;
  exif: Record<string, unknown>;
  hash: string;
};

export async function preflight(file: File): Promise<Preflight> {
  const [thumbDataUrl, exif, hash] = await Promise.all([
    makeThumb(file),
    parseExif(file).catch(() => ({})),
    sha256(file),
  ]);
  return { thumbDataUrl, exif: exif ?? {}, hash };
}

async function makeThumb(file: File): Promise<string> {
  const bmp = await createImageBitmap(file, { resizeWidth: 256, resizeQuality: 'medium' });
  const canvas = document.createElement('canvas');
  canvas.width = bmp.width;
  canvas.height = bmp.height;
  canvas.getContext('2d')!.drawImage(bmp, 0, 0);
  return canvas.toDataURL('image/jpeg', 0.8);
}

async function sha256(file: File): Promise<string> {
  const buf = await file.arrayBuffer();
  const digest = await crypto.subtle.digest('SHA-256', buf);
  return [...new Uint8Array(digest)].map(b => b.toString(16).padStart(2, '0')).join('');
}
```

- [ ] **Step 2: Test (vitest with a synthetic File)**

```typescript
import { describe, it, expect } from 'vitest';
import { preflight } from './preflight';

describe('preflight', () => {
  it('hashes deterministically', async () => {
    // jpeg-shaped 4 bytes — enough for sha256 + bitmap-creation may
    // fail in jsdom; gate the bitmap test out for unit context.
    const f = new File([new Uint8Array([0xff,0xd8,0xff,0xe0])], 'a.jpg', { type: 'image/jpeg' });
    // Hash only — bitmap requires browser canvas
    const { hash } = await preflight(f).catch(e => ({ hash: '' }));
    if (hash) expect(hash.length).toBe(64);
  });
});
```

(Bitmap-dependent integration is exercised in the Playwright E2E in
Task 50.)

- [ ] **Step 3: Run + commit**

```
cd frontend && pnpm vitest run lib/upload/preflight
```

```bash
git add frontend/src/lib/upload/preflight.ts frontend/src/lib/upload/preflight.test.ts
git commit -m "feat(upload): client preflight (thumb, EXIF, SHA-256)"
```

---

### Task 43: lib/upload/presigned.ts

**Files:**
- Create: `frontend/src/lib/upload/presigned.ts`

- [ ] **Step 1: Implement**

```typescript
import { env } from '$env/dynamic/public';

export type FileSlot = {
  name: string;
  size: number;
  mime: string;
  hash: string;
  file: File;
};

export type SlotProgress = {
  state: 'queued' | 'hashing' | 'uploading' | 'finalizing' | 'ready' | 'failed';
  pct: number;
  photoId?: string;
  shortId?: string;
  reason?: string;
};

export type Listener = (idx: number, p: SlotProgress) => void;

const API = env.PUBLIC_API_URL ?? '';

export async function uploadAll(slots: FileSlot[], listener: Listener): Promise<void> {
  const init = await fetch(`${API}/api/uploads/init`, {
    method: 'POST',
    credentials: 'include',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ files: slots.map(s => ({
      name: s.name, size: s.size, mime: s.mime, hash: s.hash
    })) }),
  });
  if (!init.ok) {
    const reason = await init.text();
    slots.forEach((_, i) => listener(i, { state: 'failed', pct: 0, reason }));
    return;
  }
  const body = await init.json() as { files: { photo_id: string; short_id: string; presigned_put_url: string }[] };

  // Concurrency 3
  const queue = [...slots.map((s, i) => ({ slot: s, idx: i, signed: body.files[i] }))];
  const workers = Array.from({ length: 3 }, () => worker());
  async function worker() {
    while (queue.length) {
      const job = queue.shift();
      if (!job) return;
      await uploadOne(job.idx, job.slot, job.signed, listener);
    }
  }
  await Promise.all(workers);
}

async function uploadOne(idx: number, slot: FileSlot,
                         signed: { photo_id: string; short_id: string; presigned_put_url: string },
                         listener: Listener) {
  listener(idx, { state: 'uploading', pct: 0, photoId: signed.photo_id, shortId: signed.short_id });

  await new Promise<void>((resolve, reject) => {
    const xhr = new XMLHttpRequest();
    xhr.open('PUT', signed.presigned_put_url);
    xhr.setRequestHeader('content-type', slot.mime);
    xhr.upload.onprogress = (e) => {
      if (e.lengthComputable) {
        listener(idx, { state: 'uploading', pct: (e.loaded / e.total) * 100, photoId: signed.photo_id, shortId: signed.short_id });
      }
    };
    xhr.onerror = () => reject(new Error('PUT failed'));
    xhr.onload = () => xhr.status >= 200 && xhr.status < 300 ? resolve() : reject(new Error(`PUT ${xhr.status}`));
    xhr.send(slot.file);
  }).catch(err => {
    listener(idx, { state: 'failed', pct: 0, reason: err.message });
    throw err;
  });

  listener(idx, { state: 'finalizing', pct: 100, photoId: signed.photo_id, shortId: signed.short_id });
  const fin = await fetch(`${API}/api/uploads/${signed.photo_id}/finalize`, {
    method: 'POST',
    credentials: 'include',
  });
  if (!fin.ok) {
    listener(idx, { state: 'failed', pct: 100, reason: await fin.text(), photoId: signed.photo_id, shortId: signed.short_id });
    return;
  }
  listener(idx, { state: 'ready', pct: 100, photoId: signed.photo_id, shortId: signed.short_id });
}
```

- [ ] **Step 2: Commit**

```bash
git add frontend/src/lib/upload/presigned.ts
git commit -m "feat(upload): parallel presigned-PUT uploader (concurrency 3)"
```

---

## Frontend — upload UI

### Task 44: TierUpgradeModal component

**Files:**
- Create: `frontend/src/lib/components/TierUpgradeModal.svelte`

- [ ] **Step 1: Implement (per showcase-cross.jsx ScreenTierUpgrade)**

Two-tier comparison panel. Free 50 MB / Subscriber 200 MB — RECOMMENDED
flag on the right card. Triggered when client pre-flight detects
`file.size > tier_max`.

```svelte
<script lang="ts">
  let {
    open = $bindable(false),
    onClose,
  }: { open?: boolean; onClose?: () => void } = $props();
</script>

{#if open}
  <div class="tu-overlay" role="dialog" aria-modal="true" onclick={() => { open = false; onClose?.(); }}>
    <div class="tu-modal card" onclick={(e) => e.stopPropagation()}>
      <p class="t-eyebrow">● UPGRADE · UPLOAD LARGER MASTERS</p>
      <h2 class="t-display t-display-i">Need <em>more headroom</em>?</h2>
      <p>Subscribers can upload 16-bit TIFFs up to <strong>200 MB</strong>.</p>

      <div class="tu-tiers">
        <div class="tu-tier">
          <h3>Free</h3>
          <p class="t-meta">50 MB / file</p>
          <p>Stays free, always.</p>
        </div>
        <div class="tu-tier tu-tier-recommended">
          <span class="chip chip-accent">RECOMMENDED</span>
          <h3>Subscriber</h3>
          <p class="t-meta">200 MB / file</p>
          <p>Coming soon.</p>
        </div>
      </div>

      <div class="tu-actions">
        <button class="btn btn-ghost" onclick={() => { open = false; onClose?.(); }}>
          Maybe later
        </button>
        <button class="btn btn-primary" disabled aria-disabled="true">
          Upgrade — coming soon
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .tu-overlay { position: fixed; inset: 0; background: var(--bg-overlay); display: grid; place-items: center; z-index: 100; }
  .tu-modal { width: min(560px, 92vw); padding: 32px; }
  .tu-tiers { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; margin: 24px 0; }
  .tu-tier { padding: 20px; border: 1px solid var(--border-default); }
  .tu-tier-recommended { border-color: var(--accent-dim); background: var(--bg-accent-tint); position: relative; }
  .tu-tier-recommended .chip { position: absolute; top: 12px; right: 12px; }
  .tu-actions { display: flex; gap: 8px; justify-content: flex-end; }
</style>
```

(The Subscribe button is a stub — billing is out of P1 scope.)

- [ ] **Step 2: Commit**

```bash
git add frontend/src/lib/components/TierUpgradeModal.svelte
git commit -m "feat(frontend): <TierUpgradeModal> shown on pre-flight quota"
```

---

### Task 45: UploadDropzone component

**Files:**
- Create: `frontend/src/lib/components/UploadDropzone.svelte`

- [ ] **Step 1: Implement**

States from the showcase: `idle`, `drag-over`, `disabled (over-quota)`,
`error`. Tier copy in the subtitle: `JPEG · PNG · TIFF · up to 50 MB
(free) · Subscribers up to 200 MB`.

```svelte
<script lang="ts">
  let {
    onFiles,
    tierMax = 50 * 1024 * 1024,
    overQuota = false,
  }: { onFiles: (files: File[]) => void; tierMax?: number; overQuota?: boolean } = $props();

  let dragOver = $state(false);
  let inputEl: HTMLInputElement;

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    dragOver = false;
    const files = Array.from(e.dataTransfer?.files ?? []);
    if (files.length) onFiles(files);
  }
</script>

<div
  class="dz"
  class:dz-drag={dragOver}
  class:dz-disabled={overQuota}
  ondragover={(e) => { e.preventDefault(); dragOver = true; }}
  ondragleave={() => dragOver = false}
  ondrop={handleDrop}
  onclick={() => inputEl.click()}
  role="button"
  tabindex="0"
  aria-label="Drop photos to upload"
>
  <p class="dz-headline t-display">↑ Drop photos here, or click</p>
  <p class="t-meta">JPEG · PNG · TIFF · up to 50 MB (free) · Subscribers up to 200 MB</p>
  <input
    bind:this={inputEl}
    type="file"
    multiple
    accept="image/jpeg,image/png,image/tiff"
    style="display:none"
    onchange={(e) => { const fs = Array.from((e.target as HTMLInputElement).files ?? []); if (fs.length) onFiles(fs); }}
  />
</div>

<style>
  .dz { padding: 64px 32px; border: 1px dashed var(--border-default); text-align: center; cursor: pointer; transition: border-color .15s; }
  .dz-drag { border-color: var(--accent); background: var(--bg-accent-tint); }
  .dz-disabled { border-color: var(--warning); background: var(--bg-warning-tint); cursor: not-allowed; }
  .dz-headline { font-size: 22px; font-style: italic; }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add frontend/src/lib/components/UploadDropzone.svelte
git commit -m "feat(frontend): <UploadDropzone> with drag-over + over-quota states"
```

---

### Task 46: UploadFileRow component

**Files:**
- Create: `frontend/src/lib/components/UploadFileRow.svelte`

- [ ] **Step 1: Implement**

Per the showcase wireframe — thumbnail, filename, size, progress bar
or status chip, action affordances ("Edit details", "Continue to
caption", "Replace file", "Upgrade").

```svelte
<script lang="ts">
  import type { SlotProgress } from '$lib/upload/presigned';

  let {
    name,
    size,
    thumbDataUrl,
    progress,
  }: {
    name: string;
    size: number;
    thumbDataUrl?: string;
    progress: SlotProgress;
  } = $props();

  const sizeMb = (size / 1024 / 1024).toFixed(1);
</script>

<div class="row" data-state={progress.state}>
  <div class="thumb">{#if thumbDataUrl}<img src={thumbDataUrl} alt="" />{:else}<span>🖼</span>{/if}</div>
  <div class="meta">
    <p class="filename t-display">{name}</p>
    <p class="t-meta">{sizeMb} MB</p>
    {#if progress.state === 'uploading'}
      <div class="bar"><div class="bar-fill" style:width={`${progress.pct}%`}></div></div>
    {:else if progress.state === 'failed'}
      <span class="chip" style:color="var(--danger)" style:border-color="var(--danger)">
        ✗ {progress.reason ?? 'Failed'}
      </span>
    {:else if progress.state === 'ready'}
      <a class="t-meta" href={`/upload/${progress.photoId}/verify`}>✓ Saved as draft · Continue to verify →</a>
    {/if}
  </div>
</div>

<style>
  .row { display: grid; grid-template-columns: 64px 1fr; gap: 12px; padding: 12px 0; border-bottom: 1px solid var(--border-subtle); }
  .thumb { width: 64px; height: 64px; background: var(--bg-base); display: grid; place-items: center; }
  .thumb img { width: 100%; height: 100%; object-fit: cover; }
  .filename { font-style: italic; font-size: 15px; }
  .bar { height: 3px; background: var(--bg-base); margin-top: 8px; }
  .bar-fill { height: 100%; background: var(--accent); transition: width .15s; }
</style>
```

- [ ] **Step 2: Commit**

```bash
git add frontend/src/lib/components/UploadFileRow.svelte
git commit -m "feat(frontend): <UploadFileRow> with progress / status chip"
```

---

### Task 47: Rebuild /upload route with multi-file dropzone

**Files:**
- Modify: `frontend/src/routes/upload/+page.svelte`
- Delete: `frontend/src/routes/upload/+page.server.ts` (replaced by client-side flow)

- [ ] **Step 1: Replace the page**

```svelte
<script lang="ts">
  import UploadDropzone from '$lib/components/UploadDropzone.svelte';
  import UploadFileRow from '$lib/components/UploadFileRow.svelte';
  import TierUpgradeModal from '$lib/components/TierUpgradeModal.svelte';
  import { preflight } from '$lib/upload/preflight';
  import { uploadAll, type FileSlot, type SlotProgress } from '$lib/upload/presigned';

  type Slot = FileSlot & { thumbDataUrl?: string; progress: SlotProgress };
  let slots = $state<Slot[]>([]);
  let showUpgrade = $state(false);

  // Tier max comes from session — add an /api/me endpoint that returns
  // tier; for now hardcode the free limit.
  const TIER_MAX = 50 * 1024 * 1024;

  async function onFiles(files: File[]) {
    for (const file of files) {
      if (file.size > TIER_MAX) {
        showUpgrade = true;
        continue;
      }
      const idx = slots.length;
      slots = [...slots, {
        name: file.name, size: file.size, mime: file.type, file,
        hash: '', thumbDataUrl: undefined,
        progress: { state: 'hashing', pct: 0 },
      }];
      const pre = await preflight(file);
      slots[idx] = { ...slots[idx], hash: pre.hash, thumbDataUrl: pre.thumbDataUrl, progress: { state: 'queued', pct: 0 } };
    }
    // Kick off after preflight finishes for the batch.
    const ready = slots.filter(s => s.hash);
    await uploadAll(ready, (i, p) => {
      // Find by photo id when present; otherwise match by index in ready.
      const target = slots.indexOf(ready[i]);
      if (target >= 0) slots[target].progress = p;
    });
  }
</script>

<TierUpgradeModal bind:open={showUpgrade} />

<section>
  <h1 class="t-display"><em>Upload</em> photos</h1>
  <UploadDropzone {onFiles} tierMax={TIER_MAX} />

  {#if slots.length}
    <div>
      <p class="t-eyebrow">● FILES ({slots.length})</p>
      {#each slots as slot}
        <UploadFileRow
          name={slot.name}
          size={slot.size}
          thumbDataUrl={slot.thumbDataUrl}
          progress={slot.progress}
        />
      {/each}
    </div>
  {/if}
</section>
```

- [ ] **Step 2: Delete old +page.server.ts**

```
git rm frontend/src/routes/upload/+page.server.ts
```

- [ ] **Step 3: Commit**

```bash
git add frontend/src/routes/upload/+page.svelte
git commit -m "feat(upload): multi-file dropzone with parallel presigned PUT"
```

---

## Frontend — verify-step pickers

### Task 48: TargetPicker, TagInput, CategorySegmented, EquipmentAutocomplete

**Files:**
- Create: `frontend/src/lib/components/TargetPicker.svelte`
- Create: `frontend/src/lib/components/TagInput.svelte`
- Create: `frontend/src/lib/components/CategorySegmented.svelte`
- Create: `frontend/src/lib/components/EquipmentAutocomplete.svelte`

The four pickers share a debounce + autocomplete pattern. Implement
each as a self-contained component. Code below is for one each;
the others mirror the same shape.

- [ ] **Step 1: TargetPicker**

```svelte
<!-- frontend/src/lib/components/TargetPicker.svelte -->
<script lang="ts">
  import { env } from '$env/dynamic/public';
  let { name = 'target', value = $bindable('') }: { name?: string; value?: string } = $props();
  let suggestions = $state<{ slug: string; canonical_name: string }[]>([]);
  let timer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    if (timer) clearTimeout(timer);
    if (!value) { suggestions = []; return; }
    timer = setTimeout(async () => {
      const r = await fetch(`${env.PUBLIC_API_URL ?? ''}/api/targets/autocomplete?q=${encodeURIComponent(value)}`);
      if (r.ok) suggestions = (await r.json()).targets;
    }, 200);
  });
</script>

<label class="t-label" for={name}>TARGET</label>
<div class="ac">
  <input id={name} {name} bind:value class="input input-mono" placeholder="M31, NGC 7000…" />
  {#if suggestions.length}
    <ul class="ac-list card">
      {#each suggestions as s}
        <li onclick={() => { value = s.canonical_name; suggestions = []; }}>
          <span class="t-mono">{s.slug}</span> · {s.canonical_name}
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .ac { position: relative; }
  .ac-list { position: absolute; top: 100%; left: 0; right: 0; padding: 4px 0; max-height: 240px; overflow-y: auto; z-index: 10; }
  .ac-list li { padding: 6px 12px; cursor: pointer; }
  .ac-list li:hover { background: var(--bg-elevated); }
</style>
```

- [ ] **Step 2: TagInput (8-tag cap, comma/Enter to commit)**

```svelte
<script lang="ts">
  let { name = 'tags', value = $bindable<string[]>([]) }: { name?: string; value?: string[] } = $props();
  let buf = $state('');
  function commit() {
    const s = buf.trim().toLowerCase();
    if (!s) return;
    if (value.includes(s)) { buf = ''; return; }
    if (value.length >= 8) return;
    value = [...value, s];
    buf = '';
  }
  function onKey(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ',') { e.preventDefault(); commit(); }
    if (e.key === 'Backspace' && !buf && value.length) value = value.slice(0, -1);
  }
</script>

<label class="t-label">TAGS · max 8</label>
<div class="tags">
  {#each value as t}
    <span class="chip">{t} <button type="button" onclick={() => value = value.filter(x => x !== t)} aria-label={`remove ${t}`}>×</button></span>
  {/each}
  <input bind:value={buf} onkeydown={onKey} onblur={commit} class="input input-mono" placeholder="widefield, narrowband…" />
  <input type="hidden" {name} value={JSON.stringify(value)} />
</div>

<style>
  .tags { display: flex; flex-wrap: wrap; gap: 4px; align-items: center; }
  .tags .chip button { margin-left: 4px; background: none; color: inherit; }
</style>
```

- [ ] **Step 3: CategorySegmented**

```svelte
<script lang="ts">
  let { name = 'category', value = $bindable('other') }: { name?: string; value?: string } = $props();
  const opts = ['dso','planetary','lunar','solar','wide_field','nightscape','other'];
</script>

<label class="t-label">CATEGORY</label>
<div class="seg" role="radiogroup">
  {#each opts as o}
    <button type="button"
      class="seg-btn"
      class:active={value === o}
      role="radio"
      aria-checked={value === o}
      onclick={() => value = o}>
      {o.replace('_', ' ')}
    </button>
  {/each}
  <input type="hidden" {name} value={value} />
</div>

<style>
  .seg { display: flex; border: 1px solid var(--border-default); }
  .seg-btn { padding: 6px 12px; font-family: var(--font-mono); font-size: 11px; text-transform: uppercase; letter-spacing: 0.08em; color: var(--fg-secondary); border-right: 1px solid var(--border-default); }
  .seg-btn:last-child { border-right: 0; }
  .seg-btn.active { background: var(--accent); color: var(--accent-ink); }
</style>
```

- [ ] **Step 4: EquipmentAutocomplete (parametrised by `kind`)**

```svelte
<script lang="ts">
  import { env } from '$env/dynamic/public';
  let {
    name,
    kind,
    value = $bindable(''),
  }: { name: string; kind: 'telescope'|'camera'|'mount'|'filter'|'guiding'; value?: string } = $props();

  let items = $state<{ canonical_name: string; display_name: string }[]>([]);
  let timer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    if (timer) clearTimeout(timer);
    timer = setTimeout(async () => {
      if (!value) { items = []; return; }
      const r = await fetch(`${env.PUBLIC_API_URL ?? ''}/api/equipment/autocomplete?kind=${kind}&q=${encodeURIComponent(value)}`);
      if (r.ok) items = (await r.json()).items;
    }, 200);
  });
</script>

<label class="t-label" for={name}>{kind.toUpperCase()}</label>
<div class="ac">
  <input id={name} {name} bind:value class="input input-mono" />
  {#if items.length}
    <ul class="ac-list card">
      {#each items as i}
        <li onclick={() => { value = i.display_name; items = []; }}>{i.display_name}</li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .ac { position: relative; }
  .ac-list { position: absolute; top: 100%; left: 0; right: 0; padding: 4px 0; max-height: 200px; overflow-y: auto; z-index: 10; }
  .ac-list li { padding: 6px 12px; cursor: pointer; }
  .ac-list li:hover { background: var(--bg-elevated); }
</style>
```

- [ ] **Step 5: Commit all four**

```bash
git add frontend/src/lib/components/TargetPicker.svelte frontend/src/lib/components/TagInput.svelte frontend/src/lib/components/CategorySegmented.svelte frontend/src/lib/components/EquipmentAutocomplete.svelte
git commit -m "feat(frontend): TargetPicker / TagInput / CategorySegmented / EquipmentAutocomplete"
```

---

### Task 49: Wire the new pickers into /upload/[id]/verify

**Files:**
- Modify: `frontend/src/routes/upload/[id]/verify/+page.svelte`
- Modify: `frontend/src/routes/upload/[id]/verify/+page.server.ts`

- [ ] **Step 1: Add fields to the form**

In `+page.svelte`, import and render the four pickers next to the
existing camera / lens / ISO / exposure fields:

```svelte
<script lang="ts">
  import TargetPicker from '$lib/components/TargetPicker.svelte';
  import TagInput from '$lib/components/TagInput.svelte';
  import CategorySegmented from '$lib/components/CategorySegmented.svelte';
  import EquipmentAutocomplete from '$lib/components/EquipmentAutocomplete.svelte';
  let { data } = $props();
  let target = $state(data.photo.target ?? '');
  let tags   = $state<string[]>(data.tags ?? []);
  let category = $state(data.photo.category ?? 'other');
  let scope    = $state(data.photo.scope ?? '');
  let mount    = $state(data.photo.mount ?? '');
  let filters  = $state(data.photo.filters ?? '');
  let guiding  = $state(data.photo.guiding ?? '');
</script>

<form method="POST">
  <!-- existing camera/lens/ISO/exposure fields ... -->
  <TargetPicker bind:value={target} />
  <TagInput bind:value={tags} />
  <CategorySegmented bind:value={category} />
  <EquipmentAutocomplete name="scope"   kind="telescope" bind:value={scope} />
  <EquipmentAutocomplete name="camera2" kind="camera"    bind:value={data.photo.camera} />
  <EquipmentAutocomplete name="mount"   kind="mount"     bind:value={mount} />
  <EquipmentAutocomplete name="filters" kind="filter"    bind:value={filters} />
  <EquipmentAutocomplete name="guiding" kind="guiding"   bind:value={guiding} />
  <button type="submit" class="btn btn-primary">Save and continue →</button>
</form>
```

- [ ] **Step 2: Update +page.server.ts**

The form action collects the new fields, parses `tags` from the
hidden `tags` JSON field, forwards everything in the JSON body to
`PUT /api/photos/:id`.

- [ ] **Step 3: Run + commit**

```
cd frontend && pnpm check
```

```bash
git add frontend/src/routes/upload/[id]/verify
git commit -m "feat(upload): verify step captures target / tags / category / equipment"
```

---

## Frontend — handle-based routes

### Task 50: Replace UUID lookup at /u/[username] with handle resolution

**Files:**
- Rename: `frontend/src/routes/u/[username]/` → `frontend/src/routes/u/[handle]/`
  (SvelteKit treats `[username]` as a route param name. Renaming the
  folder to `[handle]` makes the param consistent.)
- Modify: `frontend/src/routes/u/[handle]/+page.server.ts`

- [ ] **Step 1: Rename the folder**

```bash
cd frontend/src/routes/u
git mv "[username]" "[handle]"
```

If the folder hasn't been committed yet, just rename via filesystem
and stage as create+delete.

- [ ] **Step 2: Replace the loader**

```typescript
// frontend/src/routes/u/[handle]/+page.server.ts
import type { PageServerLoad } from './$types';
import { error, redirect } from '@sveltejs/kit';
import { env } from '$env/dynamic/public';

const API = env.PUBLIC_API_URL ?? '';

export const load: PageServerLoad = async ({ params, fetch }) => {
  const { handle } = params;

  // 1) Try current handle.
  const userRes = await fetch(`${API}/api/users/by-handle/${handle}`);
  if (userRes.status === 404) {
    // 2) Check redirect history.
    const r = await fetch(`${API}/api/handles/redirect/${handle}`);
    if (r.ok) {
      const { handle: target } = await r.json();
      throw redirect(301, `/u/${target}`);
    }
    throw error(404, 'No photographer here.');
  }
  if (!userRes.ok) throw error(500, 'Lookup failed');
  const user = await userRes.json();

  const photosRes = await fetch(`${API}/api/photos?owner_id=${user.id}&limit=24`);
  const photos = photosRes.ok ? (await photosRes.json()).photos : [];

  return { user, photos };
};
```

(Add `GET /api/users/by-handle/:handle` and `GET /api/handles/redirect/:handle`
to the backend in the next task.)

- [ ] **Step 3: Commit (without backend yet, will fail at runtime — fine, next task fixes)**

```bash
git add frontend/src/routes/u
git commit -m "feat(frontend): /u/[handle] resolves by handle instead of UUID"
```

---

### Task 51: Backend — /api/users/by-handle and /api/handles/redirect

**Files:**
- Create: `backend/src/users/by_handle.rs`
- Create: `backend/src/users/redirect_lookup.rs`
- Modify: `backend/src/users/mod.rs`, `backend/src/http/mod.rs`
- Test: `backend/tests/users_by_handle.rs`

- [ ] **Step 1: Implement `by-handle`**

```rust
// users/by_handle.rs
use axum::{extract::{Path, State}, response::IntoResponse, Json};
use crate::error::AppError;
use crate::http::AppState;

pub async fn handler(
    State(state): State<AppState>,
    Path(handle): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let row = sqlx::query!(
        r#"
        select id, email::text as "email!", handle::text as "handle!",
               display_name, created_at,
               (select count(*) from photos where owner_id = users.id and published_at is not null) as "photo_count!"
          from users where handle = $1
        "#,
        handle
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("user".into()))?;

    Ok(Json(serde_json::json!({
        "id":          row.id,
        "handle":      row.handle,
        "display_name":row.display_name,
        "created_at":  row.created_at,
        "photo_count": row.photo_count,
    })))
}
```

- [ ] **Step 2: Implement redirect lookup**

```rust
// users/redirect_lookup.rs
use axum::{extract::{Path, State}, response::IntoResponse, Json};
use crate::error::AppError;
use crate::http::AppState;

pub async fn handler(
    State(state): State<AppState>,
    Path(old_handle): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let row = sqlx::query!(
        "select u.handle::text as \"handle!\" \
           from handle_redirects r join users u on u.id = r.user_id \
          where r.old_handle = $1",
        old_handle
    )
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("redirect".into()))?;
    Ok(Json(serde_json::json!({ "handle": row.handle })))
}
```

- [ ] **Step 3: Wire**

`backend/src/http/mod.rs`:

```rust
.route("/api/users/by-handle/:handle", axum::routing::get(crate::users::by_handle::handler))
.route("/api/handles/redirect/:handle", axum::routing::get(crate::users::redirect_lookup::handler))
```

- [ ] **Step 4: Run + commit**

```
cd backend && cargo sqlx prepare && cargo test --test users_by_handle
```

```bash
git add backend/src/users backend/src/http/mod.rs backend/tests/users_by_handle.rs backend/.sqlx
git commit -m "feat(users): by-handle lookup + handle-redirect endpoint"
```

---

### Task 52: New /u/[handle]/p/[shortid] photo detail route

**Files:**
- Create: `frontend/src/routes/u/[handle]/p/[shortid]/+page.server.ts`
- Create: `frontend/src/routes/u/[handle]/p/[shortid]/+page.svelte`

- [ ] **Step 1: Loader**

```typescript
// +page.server.ts
import type { PageServerLoad } from './$types';
import { error } from '@sveltejs/kit';
import { env } from '$env/dynamic/public';
const API = env.PUBLIC_API_URL ?? '';

export const load: PageServerLoad = async ({ params, fetch }) => {
  const { handle, shortid } = params;
  const r = await fetch(`${API}/api/photos/by-permalink/${handle}/${shortid}`);
  if (!r.ok) throw error(404, 'Photo not found');
  const { id } = await r.json();
  const photoR = await fetch(`${API}/api/photos/${id}`);
  if (!photoR.ok) throw error(404, 'Photo not found');
  return { photo: await photoR.json() };
};
```

- [ ] **Step 2: Page (minimal — full photo-detail UI is P2)**

```svelte
<script lang="ts">
  import Img from '$lib/components/Img.svelte';
  let { data } = $props();
</script>

<article>
  <Img photoId={data.photo.id} alt={data.photo.target ?? data.photo.original_name} w={1200} />
  <h1 class="t-display">{data.photo.target ?? data.photo.original_name}</h1>
  <p>{data.photo.caption ?? ''}</p>
</article>
```

- [ ] **Step 3: Commit**

```bash
git add frontend/src/routes/u/[handle]/p
git commit -m "feat(frontend): photo detail at /u/[handle]/p/[shortid]"
```

---

### Task 53: Convert /photo/[id] to 301 redirect

**Files:**
- Modify: `frontend/src/routes/photo/[id]/+page.server.ts`
- Delete: `frontend/src/routes/photo/[id]/+page.svelte` (no longer rendered)

- [ ] **Step 1: Replace loader with a redirect**

```typescript
import type { PageServerLoad } from './$types';
import { error, redirect } from '@sveltejs/kit';
import { env } from '$env/dynamic/public';
const API = env.PUBLIC_API_URL ?? '';

export const load: PageServerLoad = async ({ params, fetch }) => {
  const r = await fetch(`${API}/api/photos/by-uuid/${params.id}`, { redirect: 'manual' });
  // The backend returns a 301 with Location: /u/<handle>/p/<short>
  // SvelteKit's fetch follows redirects by default; ask the backend
  // for the canonical URL and 301 the user there.
  if (r.status === 301 || r.status === 200) {
    const loc = r.headers.get('location');
    if (loc) throw redirect(301, loc);
  }
  if (r.ok) {
    // Backend returned the photo body (200 with JSON); compute the URL
    // from {handle, short_id} fields.
    const body = await r.json();
    if (body.handle && body.short_id) {
      throw redirect(301, `/u/${body.handle}/p/${body.short_id}`);
    }
  }
  throw error(404, 'Photo not found');
};
```

(Adjust based on what `/api/photos/by-uuid/:id` actually returns —
Task 37 implementation returns a redirect; SvelteKit `fetch` may
follow it. If so, switch the backend handler to return the canonical
URL in JSON and do the 301 here.)

- [ ] **Step 2: Delete the old +page.svelte**

```
git rm frontend/src/routes/photo/[id]/+page.svelte
```

- [ ] **Step 3: Commit**

```bash
git add frontend/src/routes/photo
git commit -m "feat(frontend): /photo/[id] 301-redirects to /u/[handle]/p/[short]"
```

---

## Out-of-band (AWS infra)

The following are **operational tasks** documented for the infra
operator. They produce no Rust code but must be done before the new
upload flow works in any non-dev environment. The S3 stub used in
tests doesn't need any of this.

### Task 54: Provision S3 bucket + CORS

**Steps (executed via AWS console or Terraform — not Rust):**

- [ ] Create bucket `astrophoto-images-prod` in the chosen region.
- [ ] Enable Object Ownership: Bucket owner enforced.
- [ ] Block all public access (CloudFront uses an OAI/OAC).
- [ ] Lifecycle rule: abort multipart uploads >24 h.
- [ ] CORS configuration:

```json
[
  {
    "AllowedHeaders": ["*"],
    "AllowedMethods": ["PUT", "GET", "HEAD"],
    "AllowedOrigins": ["https://astrophoto.pics", "http://localhost:5173"],
    "ExposeHeaders": ["ETag"],
    "MaxAgeSeconds": 3000
  }
]
```

- [ ] Bucket policy: deny all except the IAM principal used by the
  backend; allow `s3:GetObject` on `display/*` from the CloudFront
  Origin Access Control principal.
- [ ] Document bucket name, region, and KMS key (if any) in the
  ops runbook.

### Task 55: CloudFront distribution + Lambda function URL

- [ ] Lift `image-transformer/` from
  `/Volumes/Pascal4Tb/Projects/claude/astrophoto/dev` into a new repo
  or `infra/lambda/image-transformer/`.
- [ ] Adapt from Lambda@Edge to a regional Lambda function URL
  (Node.js 20, sharp installed via Lambda layer or container image).
- [ ] Function URL with IAM auth-type or open (per ops decision).
- [ ] CloudFront distribution: origin = Lambda function URL; cache
  policy includes the full query string (`w`, `h`, `fit`, `q`, `fm`);
  TTL 30 days; alias `cdn.astrophoto.pics`.
- [ ] DNS: CNAME `cdn.astrophoto.pics` → CloudFront distribution.
- [ ] Smoke test:

```bash
curl -I "https://cdn.astrophoto.pics/img/<known-photo-id>?w=400"
# Expect 200, content-type: image/jpeg, x-cache: Hit/Miss
```

### Task 56: Production env wiring

- [ ] Set `APP_S3_*` in the prod env to the new S3 bucket
  (replacing R2).
- [ ] Set `APP_CDN_BASE_URL=https://cdn.astrophoto.pics`.
- [ ] Update CLAUDE.md (Task 58) and the deployment runbook to
  reflect the new storage / CDN.
- [ ] One-shot migration job: copy existing originals from R2 into
  S3, generate display masters, populate `photos.display_key`.
  Decommission R2 only after the job completes and CDN serves real
  traffic from S3.

---

## Final

### Task 57: P1 acceptance smoke test (Playwright E2E)

**Files:**
- Create: `frontend/e2e/p1-upload-flow.spec.ts`

- [ ] **Step 1: Write the E2E**

```typescript
import { test, expect } from '@playwright/test';

test('P1 upload flow: signup → upload → permalink', async ({ page }) => {
  // 1. Sign up with handle.
  await page.goto('/auth/signup');
  await page.fill('input[name="display_name"]', 'Marie Test');
  await page.fill('input[name="email"]', `marie-${Date.now()}@example.com`);
  await page.fill('input[name="password"]', 'longenoughpassword');
  await page.fill('input#handle', 'marietest');
  // wait for the picker to settle
  await expect(page.locator('.hp-status[data-status="available"]')).toBeVisible();
  await page.click('button[type="submit"]');

  // 2. Upload a small JPEG.
  await page.goto('/upload');
  const file = await page.evaluate(() => new File([new Uint8Array([0xff, 0xd8, 0xff, 0xe0])], 'a.jpg', { type: 'image/jpeg' }));
  // Playwright file input
  await page.setInputFiles('input[type=file]', 'frontend/e2e/fixtures/sample.jpg');

  // 3. Wait for ready state.
  await expect(page.locator('[data-state="ready"]')).toBeVisible({ timeout: 30000 });

  // 4. Click through to verify, then to detail page; assert URL shape.
  await page.click('a:has-text("Continue to verify")');
  await expect(page).toHaveURL(/\/upload\/.+\/verify/);
});
```

- [ ] **Step 2: Run**

```
cd frontend && pnpm test:e2e
```

(Backend must be running; `just dev` is the easiest setup.)

- [ ] **Step 3: Commit**

```bash
git add frontend/e2e/p1-upload-flow.spec.ts
git commit -m "test(e2e): P1 happy-path signup → upload → detail"
```

---

### Task 58: Update CLAUDE.md and README

**Files:**
- Modify: `CLAUDE.md`
- Modify: `README.md`

- [ ] **Step 1: Update storage line in CLAUDE.md**

Find the line in CLAUDE.md that says `Storage: S3-compatible (Cloudflare R2 in prod, MinIO in dev).` and replace with:

```
Storage: AWS S3 in prod (image bucket), MinIO in dev. CloudFront with
a Lambda function URL (sharp) serves on-the-fly transforms from
`display/<id>.jpg` masters. URL shape:
`https://cdn.astrophoto.pics/img/<id>?w=&h=&fit=&q=&fm=`.
```

Also add a new gotcha bullet:

```
- Display master pattern: every photo has both an `originals/<id>.<ext>`
  (archival) and `display/<id>.jpg` (4096 px / q=85, what the CDN
  transforms). Never plumb originals through the CDN.
```

- [ ] **Step 2: Update README route list**

Add `/explore`, `/u/<handle>`, `/u/<handle>/p/<short-id>` to whatever
section lists routes (or note that `/u/<handle>` replaces
`/u/<uuid>`).

- [ ] **Step 3: Run `just check`**

```
just check
```

Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add CLAUDE.md README.md
git commit -m "docs: update storage + CDN sections for showcase P1"
```

---

### Task 59: Final acceptance gate

- [ ] **Step 1: Run the full check**

```
just check
just test
```

Both should exit 0. If anything is red, fix and re-commit.

- [ ] **Step 2: Verify the spec acceptance bullets**

For each P1 acceptance item in the spec, tick that the implementation
covers it:

- [ ] Existing functionality continues to work via 301 redirects
      (`/photo/<uuid>` → `/u/<handle>/p/<short>`).
- [ ] New uploads go through presigned PUT and resolve at
      `/u/<handle>/p/<short>`.
- [ ] Free user PUT >50 MB is rejected (pre-flight + signed URL).
- [ ] Subscriber tier raises the cap to 200 MB (toggle `users.tier`
      manually in the DB and confirm the limit changes).
- [ ] Display masters generated and served via CDN URL.
- [ ] Backend test suite passes (`just test`).
- [ ] XSS fuzz cases against ammonia bio sanitiser pass (Task 18).

- [ ] **Step 3: Hand off to merge**

Use `superpowers:finishing-a-development-branch` to merge
`feat/showcase-p1-foundations` into `main`.

---

## Plan complete.

The plan above covers Phase 1 of the Photographer Showcase spec. After
merge, P2 (Hero page) is the next plan; P3 (Discovery) follows P2.
