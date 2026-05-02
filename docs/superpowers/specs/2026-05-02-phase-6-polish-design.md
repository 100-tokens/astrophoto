# Phase 6 — Polish & UX Design

**Date:** 2026-05-02
**Status:** Approved (sections 1–3) — pending written-spec review
**Author:** Pascal (with Claude)

## Goal

Close the visible UX gaps left by Phase 5 so a fresh user can sign up,
upload a photo, see it surfaced in the gallery and on a profile page
with an EXIF panel that reads sensibly, and sign out cleanly. Also: ship
a small `just seed` flow so a developer cloning the repo can demo the
product without writing photos by hand.

Out of scope for Phase 6: follow / appreciations / comments (this is
the engagement layer — Phase 7), email change, password reset, account
deletion (Phase 8).

## Decisions

| # | Topic                              | Choice                                                                  |
|---|------------------------------------|-------------------------------------------------------------------------|
| 1 | Items in scope                     | a, b, c, d, e, f (skip g — Follow needs a relations table)              |
| 2 | `/api/users/:id` auth              | Public read, no email exposed                                           |
| 3 | EXIF panel for real-no-EXIF photos | Show only available rows; always include ORIGINAL FILE, SIZE, DIMENSIONS |
| 4 | FRAME OF THE WEEK tag              | Real photos: `target` only; placeholder demo: `target · integration`    |
| 5 | Hero / grid dedup                  | Hero = newest; grid = remaining; if only 1 real photo, grid is empty    |
| 6 | Logout pattern                     | Avatar dropdown with Profile, Upload, Sign out                          |
| 7 | Seed approach                      | `backend/seeds/fixtures/` with 1 PD-NASA JPEG bundled + extensible      |

## Item-by-item spec

### a) FRAME OF THE WEEK tag — drop hardcoded "SHO"

**File:** `frontend/src/routes/+page.svelte` lines around the hero corner
overlay.

The overlay currently reads `target · integration` for both real and
placeholder data, but real photos have an empty `integration` and
inherit a hardcoded "SHO" from a fallback. Switch to a conditional
based on the existing `data.isReal` flag:

```svelte
{#if data.isReal}
  <div class="frame-of-the-week">
    <div style="color: var(--accent)">FRAME OF THE WEEK</div>
    <div style="color: var(--fg-primary)">{heroPhoto.target}</div>
  </div>
{:else}
  <div class="frame-of-the-week">
    <div style="color: var(--accent)">FRAME OF THE WEEK</div>
    <div style="color: var(--fg-primary)">
      {heroPhoto.target} · {heroPhoto.integration}
    </div>
    <div style="color: var(--fg-muted)">Marie Dubois · Bortle 4</div>
  </div>
{/if}
```

Smallest possible diff. ~5 min.

### b) EXIF panel for real photos without EXIF

**File:** `frontend/src/routes/photo/[slug]/+page.server.ts` (the load)
and `+page.svelte` (the EXIF section).

Today the load returns `[{ label: "Record", value: "Full record not
available for this placeholder." }]` for real photos in the non-rich
branch. Replace with a row builder that omits null/empty fields:

```ts
function buildExifRows(photo: PhotoSummary, originalName: string, bytes: number): ExifRow[] {
  const rows: ExifRow[] = [];

  // Always-present rows
  rows.push({ label: 'Original file', value: originalName });
  rows.push({ label: 'Size', value: formatBytes(bytes) });
  if (photo.width && photo.height) {
    rows.push({ label: 'Dimensions', value: `${photo.width} × ${photo.height}` });
  }
  if (photo.target) rows.push({ label: 'Target', value: photo.target });

  // EXIF rows — only when present
  if (photo.taken_at) {
    rows.push({ label: 'Captured', value: formatDate(photo.taken_at) });
  }
  if (photo.camera) rows.push({ label: 'Camera', value: photo.camera });
  if (photo.lens) rows.push({ label: 'Lens', value: photo.lens });
  if (photo.iso != null) rows.push({ label: 'ISO', value: String(photo.iso) });
  if (photo.exposure_s != null) {
    rows.push({ label: 'Exposure', value: formatExposure(photo.exposure_s) });
  }
  if (photo.focal_mm != null) {
    rows.push({ label: 'Focal', value: `${photo.focal_mm} mm` });
  }
  return rows;
}

function formatBytes(b: number): string {
  if (b < 1024) return `${b} B`;
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB`;
  return `${(b / 1024 / 1024).toFixed(1)} MB`;
}

function formatExposure(s: number): string {
  if (s >= 1) return `${s} s`;
  return `1/${Math.round(1 / s)} s`;
}

function formatDate(iso: string): string {
  return new Date(iso).toISOString().split('T')[0];  // "2026-05-02"
}
```

The page renders `<ExifTable rows={data.photo.exifRows} />` unchanged;
the empty-records check that triggered the placeholder message goes
away. The canonical NGC 7000 demo branch keeps its rich row builder
verbatim.

### c) `GET /api/users/:id`

**Files:**
- Create `backend/src/users/get.rs`
- Add a query to `backend/src/photos/queries.rs`: `count_by_owner`
- Mount in `backend/src/http/mod.rs`
- Annotate response type with `#[ts(export)]` for codegen

**Response shape:**

```rust
#[derive(Serialize, ts_rs::TS)]
#[ts(export, export_to = "UserPublic.ts")]
pub struct UserPublic {
    pub id: String,
    pub display_name: String,
    pub created_at: String,    // RFC 3339
    pub photo_count: i64,
}
```

**Handler:**

```rust
pub async fn handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<UserPublic>, AppError> {
    let user = users::queries::find_by_id(&state.pool, id).await?
        .ok_or(AppError::NotFound)?;
    let count = photos::queries::count_by_owner(&state.pool, id).await?;
    Ok(Json(UserPublic {
        id: user.id.to_string(),
        display_name: user.display_name,
        created_at: user.created_at.to_rfc3339(),
        photo_count: count,
    }))
}
```

Public endpoint (no `CurrentUser` extractor). Email NOT exposed.

`/u/[uuid]/+page.server.ts` calls `${API}/api/users/${uuid}` to resolve
the display_name, then `${API}/api/photos?owner_id=${uuid}` for the
photo grid. If the user lookup 404s, throw 404.

If the username param is **not** a UUID:
- `marie-dubois` → existing placeholder Marie route.
- Anything else → 404.

### d) Avatar dropdown menu (Sign out)

**Files:**
- Create `frontend/src/lib/components/AvatarMenu.svelte`
- Modify `frontend/src/lib/components/AppHeader.svelte` (use AvatarMenu when authenticated)
- Create `frontend/src/routes/account/logout/+server.ts` (POST handler that forwards to backend, propagates clear-cookie, redirects to `/`)

**`AvatarMenu.svelte` props:**
- `user: { id: string; displayName: string }` — required.

**Behavior:**
- Renders the same 32 px amber avatar circle as today.
- `onclick` toggles a `$state` flag. `aria-haspopup="menu"`, `aria-expanded`.
- Popover: absolute, `top: calc(100% + 8px); right: 0; width: 220px`.
- Surface: `--bg-elevated`, `1px solid --border-default`, `--r-md`,
  `--shadow-md`.
- 4 zones top to bottom:
  1. Mono `t-meta` greeting — "Hi, Marie Dubois". Padding 12px 16px.
  2. Hairline divider — `--border-subtle`.
  3. 3 nav-link items mono uppercase 11px:
     - `<a href="/u/{user.id}">Profile</a>`
     - `<a href="/upload">Upload</a>`
     - Hairline divider
     - Sign out button (renders inside a `<form action="/account/logout" method="POST">` so non-JS users still work)
  4. Closes on:
     - Escape key (`$effect` adds keydown listener)
     - Click outside (effect adds document click listener that checks the menu's containing element)
     - Click on any link/button (the link's natural navigation closes the menu via `$state = false` in onclick)

**Animation:** opacity 0→1 + translateY(-4px)→0 over 150ms ease-out.
Closing: same in reverse.

**`/account/logout/+server.ts`:**

```ts
import { redirect } from '@sveltejs/kit';
import type { RequestHandler } from './$types';

const API = import.meta.env.VITE_API_BASE_URL ?? 'http://localhost:8080';

export const POST: RequestHandler = async ({ fetch, cookies, setHeaders }) => {
  const cookie = cookies.getAll().map((c) => `${c.name}=${c.value}`).join('; ');
  const res = await fetch(`${API}/api/auth/logout`, {
    method: 'POST',
    headers: { Cookie: cookie }
  });
  const setCookie = res.headers.get('set-cookie');
  if (setCookie) setHeaders({ 'set-cookie': setCookie });
  throw redirect(303, '/');
};
```

### e) Hero / grid dedup

**File:** `frontend/src/routes/+page.server.ts`.

Current: `realPhotos` are mapped 1:1 into `photos` AND the first one
appears as `heroPhoto`. Result: same photo twice.

Fix: split into `heroPhoto` (first) and `photos` (slice 1..):

```ts
if (realPhotos.length > 0) {
  const [hero, ...rest] = realPhotos;
  return {
    heroPhoto: { target: hero.target ?? 'Untitled', /* ... */ },
    heroSrc: `${API}/api/photos/${hero.id}/thumb/1200`,
    photos: rest.map(toGalleryCard),
    isReal: true
  };
}
```

The `+page.svelte` updates: hero photo uses `heroSrc`, grid iterates
`photos` only. When `photos.length === 0`, the grid section renders as
an empty wrapper (the page is still complete: header + hero + filter
bar + footer). Don't show "no photos yet" copy — empty grid is calmer.

### f) `just seed` with bundled NASA fixture

**Files:**
- Create `backend/seeds/fixtures/m42-orion.jpg` — a small (~150 KB)
  public-domain NASA image with EXIF.
- Create `backend/seeds/fixtures/LICENSE.txt` — credits and PD-NASA notice.
- Create `backend/src/bin/seed.rs` — binary.
- Refactor `backend/src/photos/upload.rs::process_photo` into a
  reusable function in a new `backend/src/photos/pipeline.rs` so both
  the HTTP handler and the seed binary call the same code.
- Add `seed` recipe to `justfile`.

**`backend/src/photos/pipeline.rs`:**

```rust
pub async fn process(
    pool: &PgPool,
    storage: Arc<dyn Storage>,
    owner_id: Uuid,
    original_name: &str,
    mime: &str,
    target: Option<&str>,
    caption: Option<&str>,
    bytes: Bytes,
) -> Result<Uuid, AppError> {
    let photo_id = Uuid::new_v4();
    let storage_key = format!("originals/{photo_id}");
    storage.put(&storage_key, mime, bytes.clone()).await?;
    queries::insert_processing(
        pool, owner_id, &storage_key, original_name,
        bytes.len() as i64, mime, target, caption,
    ).await?;

    // Synchronous variant: no spawn, runs to completion before returning.
    // The HTTP handler still uses tokio::spawn around this for async behavior.
    let bytes_for_blocking = bytes.clone();
    let parsed = tokio::task::spawn_blocking(move || {
        let exif_data = exif::parse_blocking(&bytes_for_blocking)?;
        let mut generated = Vec::with_capacity(2);
        for size in &[400u32, 1200u32] {
            generated.push(thumbs::generate_blocking(&bytes_for_blocking, *size)?);
        }
        Ok::<_, AppError>((exif_data, generated))
    })
    .await
    .map_err(|e| AppError::Internal(format!("spawn_blocking: {e}")))??;

    let (exif_data, generated) = parsed;
    let (full_w, full_h) = generated.iter().max_by_key(|t| t.size)
        .map(|t| (t.width as i32, t.height as i32))
        .unwrap_or((0, 0));

    for thumb in generated {
        let key = format!("thumbs/{photo_id}/{}", thumb.size);
        let len = thumb.bytes.len() as i64;
        storage.put(&key, "image/jpeg", thumb.bytes).await?;
        queries::insert_thumbnail(pool, photo_id, thumb.size as i32, &key, len).await?;
    }
    queries::mark_ready(pool, photo_id, full_w, full_h, &exif_data).await?;
    Ok(photo_id)
}
```

The HTTP `upload::handler` becomes a thin wrapper that calls
`pipeline::process` inside a `tokio::spawn` (background processing).
The seed binary calls it directly (synchronous, awaits each photo).

**`backend/src/bin/seed.rs`:**

```rust
//! Demo seed: creates a "demo@astrophoto.example" user and uploads any
//! files in backend/seeds/fixtures/*.jpg through the same pipeline as
//! the HTTP upload handler. Idempotent (skips files already imported
//! based on original_name).
//!
//! Run: `just seed`

use std::sync::Arc;
use std::path::PathBuf;
use bytes::Bytes;

use astrophoto::{auth::password, photos::pipeline, storage::S3Storage, users::queries as user_q, Config, db};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = Config::from_env();
    tracing_subscriber::fmt::init();

    let pool = db::connect(&cfg.database_url).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let storage = Arc::new(
        S3Storage::new(
            cfg.s3_endpoint.as_deref(),
            &cfg.s3_region,
            &cfg.s3_bucket,
            &cfg.s3_access_key,
            &cfg.s3_secret_key,
            cfg.s3_path_style,
        )
        .await?,
    );

    let demo_email = "demo@astrophoto.example";
    let demo_user = match user_q::find_by_email(&pool, demo_email).await? {
        Some(u) => {
            tracing::info!(user_id=%u.id, "demo user already exists");
            u
        }
        None => {
            let hash = password::hash("demoaccount".into()).await?;
            let u = user_q::create_with_password(&pool, demo_email, "Demo Astrographer", &hash).await?;
            tracing::info!(user_id=%u.id, "demo user created");
            u
        }
    };

    let fixtures_dir = PathBuf::from("seeds/fixtures");
    let mut entries = match std::fs::read_dir(&fixtures_dir) {
        Ok(it) => it.filter_map(Result::ok).collect::<Vec<_>>(),
        Err(_) => {
            tracing::warn!("no fixtures dir; nothing to seed");
            return Ok(());
        }
    };
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) if n.ends_with(".jpg") || n.ends_with(".png") => n.to_string(),
            _ => continue,
        };

        // Idempotency: skip if a photo with this name already exists for the user.
        if photos_already_imported(&pool, demo_user.id, &name).await? {
            tracing::info!(file=%name, "already imported, skipping");
            continue;
        }

        let bytes = Bytes::from(std::fs::read(&path)?);
        let mime = if name.ends_with(".png") { "image/png" } else { "image/jpeg" };
        let id = pipeline::process(
            &pool,
            storage.clone(),
            demo_user.id,
            &name,
            mime,
            None, None,
            bytes,
        ).await?;
        tracing::info!(file=%name, photo_id=%id, "imported");
    }

    Ok(())
}

async fn photos_already_imported(pool: &sqlx::PgPool, owner_id: uuid::Uuid, name: &str) -> Result<bool, astrophoto::AppError> {
    let row = sqlx::query!(
        "select 1 as one from photos where owner_id = $1 and original_name = $2 limit 1",
        owner_id, name
    ).fetch_optional(pool).await?;
    Ok(row.is_some())
}
```

**`justfile` addition:**

```just
# Seed the dev database with demo content. Idempotent.
seed:
    cd backend && cargo run --bin seed
```

**Sourcing the bundled fixture:** include 1 NASA APOD photo (e.g.
"M42 — Hubble" or similar small public-domain JPEG with real EXIF —
final selection done at implementation time). LICENSE.txt cites NASA
PD as source. Future contributors can drop their own JPEGs into the
fixtures dir and re-run `just seed` to test their EXIF flow.

## Backend testing

- `count_by_owner` query — covered indirectly when the auth integration
  test asserts the user's photo count after upload.
- `pipeline::process` — replaces direct calls to `upload::process_photo`
  in the existing photos integration test; that test still covers the
  full pipeline.
- `users::get::handler` — extend `tests/photos.rs` to GET
  `/api/users/<id>` after the upload and assert `photo_count == 1`.
- No new test for `users::queries::find_by_email` (already exercised by
  signup).

## Frontend testing

No new tests in Phase 6. The 3 visual changes (a, e, d) are smoke-tested
manually via Chrome DevTools after the implementation lands.

## Quality gates

- `just check` clean
- `cargo test` all green (16+ tests)
- `pnpm -C frontend check && pnpm -C frontend build` clean
- Manual browser smoke: signup → upload → see photo without EXIF render
  with sensible row list → click avatar → menu opens → Sign out →
  redirected to `/`, header back to "Sign in / Create account".
- `just seed` imports the bundled NASA fixture; `/u/<demo-id>` shows it.

## Out of scope (deferred)

- Follow / followers (no relations table, no mutation endpoints)
- Appreciations / comments (Phase 7)
- Account settings page (Phase 8)
- Email change / password reset (Phase 8)
- Plate-solving, AI captioning (Vision)

## Implementation order suggestion

1. Backend: `count_by_owner` query + `users::get` handler + route
2. Backend: extract `pipeline::process`, refactor `upload::handler`
3. Backend: `bin/seed.rs` + bundled fixture + `just seed` recipe
4. Frontend: gallery item (a) + (e) — small diff, fast win
5. Frontend: photo detail (b) — EXIF row builder
6. Frontend: AvatarMenu (d) + `/account/logout/+server.ts`
7. Manual smoke + merge as `v0.5.0-polish`

## References

- Existing patterns lifted: `auth::middleware`, `photos::queries`,
  `photos::upload::process_photo`.
- Phase 5 plan, just merged: `docs/superpowers/plans/2026-05-02-phase-5-photos.md`
- Original spec for the bootstrap: `docs/superpowers/specs/2026-05-01-astrophoto-bootstrap-design.md`
