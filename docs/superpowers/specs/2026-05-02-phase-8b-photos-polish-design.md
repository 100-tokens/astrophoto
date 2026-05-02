# Phase 8b — Photos, Drafts, Replace & Polish Design

**Date:** 2026-05-02
**Status:** Approved (sections 1–6) — pending written-spec review
**Author:** Pascal (with Claude)

## Goal

Land the photo-management surface from the Phase 8 design handoff that
Phase 8a deferred. Phase 8b covers four cohesive sub-projects:

1. **My Photos page** at `/account/frames` — the per-owner dashboard that
   never existed in Phase 5 despite being referenced in the design brief.
   Stats row, filter chips, table.
2. **Drafts surfaced** — separate "uploaded but not yet published" from
   "published". Add a `published_at` timestamp; rework the upload flow into
   3 steps; expose drafts in My Photos with a callout band; render a DRAFT
   strip on the photo detail for the owner; hide drafts everywhere public.
3. **Replace image** — a new `POST /api/photos/:id/replace` that swaps the
   binary master in place while preserving caption, target, EXIF (manually
   edited fields), comments, appreciations, and follows. Triggered from a
   ⋯ menu on the photo detail page via a Modal.
4. **Polish 8.5** — four micro-fixes: context-aware home eyebrow,
   FollowButton 3-state hover, untitled-photo fallback everywhere, and
   the mobile sticky AppreciateButton bottom bar.

Phase 8b also rounds out one ergonomics gap surfaced during the design:
**editing metadata on an already-published photo** is allowed via the same
`PUT /api/photos/:id` used by the draft flow. No new endpoint; the
"Publish" button just becomes "Save changes" when the photo is already
published.

## Decisions

| #  | Topic                                  | Choice                                                                       |
|----|----------------------------------------|------------------------------------------------------------------------------|
| 1  | Phase 8b scope                         | Single phase: My Photos + Drafts + Replace + Polish 8.5 + edit metadata     |
| 2  | Draft state representation             | `published_at timestamptz` nullable; separate from pipeline `status`         |
| 3  | Upload flow                            | 3-step frontend (`upload → verify → caption/publish`); single backend POST   |
| 4  | Backend draft creation                 | `POST /api/photos` always sets `published_at = NULL` (draft auto)            |
| 5  | Track upload progress                  | `last_step text` column (`'upload' \| 'verify' \| 'caption'`)                 |
| 6  | Publish endpoint                       | `POST /api/photos/:id/publish` — idempotent, owner-only                       |
| 7  | Edit published metadata                | Same `PUT /api/photos/:id` used by drafts; published photos editable too      |
| 8  | Replace endpoint scope                 | Pure binary swap; preserves caption/target/EXIF/comments/appreciations       |
| 9  | EXIF re-extraction on replace          | Off by default; toggle deferred                                              |
| 10 | Replace tracking                       | `replaced_at` + public "REPROCESSED · DD MMM → DD MMM YYYY" on detail        |
| 11 | My Photos route                        | `/account/frames` (`/account/frames/drafts` is a thin redirect)              |
| 12 | Draft visibility — non-owner           | 404 on detail; invisible in gallery/feed/profile; appreciate/comment 404      |
| 13 | Draft visibility — owner               | 200 + warning strip "● DRAFT · ONLY YOU CAN SEE THIS"                        |
| 14 | Discard draft                          | Hard `DELETE` (row + S3 binary). No soft-delete                              |
| 15 | Polish #1 — eyebrow                    | MVP: `● FROM THE N PHOTOGRAPHERS YOU FOLLOW` (no "M NEW since" — deferred)   |
| 16 | Polish #4 — mobile sticky              | Sticky bar yes; long-press appreciators sheet deferred                       |
| 17 | New shared components                  | `ReplaceModal`, `DraftCard`, `PhotosTable`, `StatsRow`, `FilterChips`, `PhotoTitle` |
| 18 | Migration                              | `0004_drafts_replace.sql` — `published_at` + `replaced_at` + `original_uploaded_at` + `last_step` |

## Module map

```
backend/src/
├─ photos/
│  ├─ upload.rs            (existing — modified: insert with published_at = NULL, last_step = 'upload')
│  ├─ get.rs               (existing — modified: 404 on draft for non-owner; expose `is_draft`, `last_step`)
│  ├─ list.rs              (existing — modified: add `published_at IS NOT NULL` everywhere; add `?drafts=true`)
│  ├─ count.rs             (existing — modified: per-owner draft count branch)
│  ├─ queries.rs           (existing — modified: visibility filter on every public SELECT)
│  ├─ pipeline.rs          (existing — modified: skip user-edited EXIF fields on replace)
│  ├─ publish.rs           (NEW — POST /api/photos/:id/publish, idempotent)
│  ├─ replace.rs           (NEW — POST /api/photos/:id/replace, multipart)
│  └─ metadata.rs          (NEW — PUT /api/photos/:id, target/caption/EXIF/last_step partial update)
└─ me/
   └─ stats.rs             (NEW — GET /api/me/stats, per-owner aggregate)

frontend/src/
├─ lib/components/photos/   (NEW directory)
│  ├─ ReplaceModal.svelte
│  ├─ DraftCard.svelte
│  ├─ DraftsCallout.svelte
│  ├─ PhotosTable.svelte
│  ├─ StatsRow.svelte
│  ├─ FilterChips.svelte
│  └─ PhotoTitle.svelte
└─ routes/
   ├─ +page.svelte                       (existing — modified: context-aware eyebrow when authed)
   ├─ +page.server.ts                    (existing — modified: load following_count when authed)
   ├─ photo/[slug]/
   │  ├─ +page.svelte                    (existing — modified: DRAFT strip + ⋯ menu + REPROCESSED label + mobile sticky bar)
   │  └─ +page.server.ts                 (existing — modified: load is_draft, owner check)
   ├─ upload/
   │  ├─ +page.svelte                    (existing — modified: Step 01 picker only, redirects to /upload/[id]/verify)
   │  ├─ +page.server.ts                 (existing — modified: POST returns id; redirect handler)
   │  └─ [id]/
   │     ├─ verify/+page.svelte          (NEW — Step 02 EXIF + target form)
   │     ├─ verify/+page.server.ts       (NEW — load photo + actions: save metadata, save-as-draft)
   │     ├─ caption/+page.svelte         (NEW — Step 03 caption + publish/save-changes)
   │     └─ caption/+page.server.ts      (NEW — load + actions: save caption, publish, save-as-draft)
   └─ account/
      └─ frames/
         ├─ +page.svelte                 (NEW — dashboard with stats + drafts callout + table)
         ├─ +page.server.ts              (NEW — load stats + photos with filter/sort/view)
         └─ drafts/+page.server.ts       (NEW — thin redirect to /account/frames?filter=drafts)
```

Components touched by Polish 8.5 (no new files):
- `frontend/src/lib/components/FollowButton.svelte` — 3-state hover
- `frontend/src/lib/components/AppreciateButton.svelte` — `variant: 'inline' | 'mobile-sticky'` prop

## Migration `0004_drafts_replace.sql`

```sql
-- Drafts: published_at NULL = draft, NOT NULL = published.
-- Pipeline state (status) and publish state stay separate concerns.
alter table photos
  add column published_at timestamptz;

create index photos_published_at_idx on photos (published_at desc)
  where published_at is not null;

create index photos_drafts_owner_idx on photos (owner_id, created_at desc)
  where published_at is null;

-- Backfill: every existing 'ready' photo is considered published at its
-- creation time. 'processing' / 'failed' rows stay draft (NULL).
update photos set published_at = created_at where status = 'ready';

-- Replace tracking: when a photo is replaced, we record the swap so the
-- public detail page can show "REPROCESSED · 14 MAR → 02 MAY 2026".
alter table photos
  add column replaced_at timestamptz,
  add column original_uploaded_at timestamptz;

update photos set original_uploaded_at = created_at;
alter table photos alter column original_uploaded_at set not null;

-- Track upload progress so the draft card can render "STEP 02 · VERIFYING DATA".
alter table photos
  add column last_step text
    check (last_step in ('upload', 'verify', 'caption'));

update photos set last_step = 'caption'
  where status = 'ready' and published_at is not null;
update photos set last_step = 'upload'
  where status in ('processing', 'failed');
```

## API surface

| Method | Route                                  | Auth         | Effect                                                                      |
|--------|----------------------------------------|--------------|-----------------------------------------------------------------------------|
| POST   | `/api/photos`                          | session      | Multipart; creates row with `published_at=NULL, status='processing', last_step='upload'`. Returns `{id, status}`. (existing — semantic change) |
| GET    | `/api/photos/:id`                      | optional     | 404 for non-owner if draft. Returns `is_draft`, `last_step`, `replaced_at`. (existing — extended) |
| PUT    | `/api/photos/:id`                      | session+owner | Partial update of `target/caption/exif_json/last_step`. Works for both drafts and published. |
| POST   | `/api/photos/:id/publish`              | session+owner | Idempotent; sets `published_at = now()`, `last_step = 'caption'`. 400 if `status != 'ready'`. |
| POST   | `/api/photos/:id/replace`              | session+owner | Multipart; swap binary, regenerate thumbs, set `replaced_at`. 202. Preserves metadata. |
| DELETE | `/api/photos/:id`                      | session+owner | Hard delete; works for drafts ("Discard") and published ("Delete"). (existing) |
| GET    | `/api/photos?drafts=true`              | session       | Returns current user's drafts (cross-user `?owner_id=` + `drafts=true` is rejected with 403). |
| GET    | `/api/me/stats`                        | session       | `{published_count, draft_count, integration_secs, appreciations_received}`.   |

## Upload flow — 3 steps

### Step 01 — `/upload`

Existing route, simplified to file picker / drop zone only. Submitting
posts to `POST /api/photos` (multipart). Backend creates the row with
`published_at = NULL`, `status = 'processing'`, `last_step = 'upload'`,
and returns `{id, status: 'processing'}`. Frontend `+page.server.ts`
extracts the id and `redirect(303, '/upload/{id}/verify')`.

### Step 02 — `/upload/[id]/verify`

Loads the photo (which may still be `status = 'processing'` if the pipeline
is fast — display a "● PROCESSING THUMBNAILS" overlay until ready, polling
`GET /api/photos/[id]` every 2s).

Form fields: `target`, `taken_at`, `camera`, `lens`, `iso`, `exposure_s`,
`focal_mm`, `ra_deg`, `dec_deg` (existing dedicated columns) plus
free-form astro fields (`telescope`, `mount`, `filters`, `aperture`,
`sessions`, `sensor_temp`, `gain`) which round-trip through `exif_json`
as a sub-object — pre-filled from EXIF where present.

Two actions:
- **Save as draft** (ghost button) — `PUT /api/photos/:id` with all current
  field values + `last_step = 'verify'`. Redirect to `/account/frames`.
- **Continue →** (primary) — `PUT /api/photos/:id` with field values +
  `last_step = 'caption'`. Navigate to `/upload/[id]/caption`.

### Step 03 — `/upload/[id]/caption`

Caption textarea + a small recap of target/key EXIF (read-only).

Three actions, depending on whether the photo is already published:

| Photo state               | Primary button   | Effect                                                       |
|---------------------------|------------------|--------------------------------------------------------------|
| `published_at = NULL`     | **Publish**       | `PUT /api/photos/:id` (caption) + `POST /api/photos/:id/publish`; redirect to `/photo/[slug]` |
| `published_at != NULL`    | **Save changes** | `PUT /api/photos/:id` (caption only); redirect to `/photo/[slug]` (no publish call) |

Plus, when `published_at = NULL`, a secondary **Save as draft** ghost button:
`PUT` only, redirect to `/account/frames`. Hidden when already published
(no need — Save changes preserves the publish state).

## Replace endpoint

`POST /api/photos/:id/replace` (multipart, owner-only):

1. Verify ownership. Reject if `status = 'processing'` (pipeline busy).
2. Read uploaded file (single field). Validate size ≤ 64 MB and content-type.
3. Generate new storage key `photos/{photo_id}/{uuid}`. Upload to S3.
4. Collect old storage_key + thumbnail keys for deletion.
5. UPDATE photos SET `storage_key`, `original_name`, `mime`, `bytes`,
   `status='processing'`, `replaced_at = now()`. (Width/height regenerate
   when pipeline finishes.)
6. DELETE thumbnails rows. (Pipeline regenerates.)
7. Best-effort `storage.delete_objects(&[old_key, ...thumb_keys])` — log and
   swallow individual failures (consistent with the deletion-purge worker
   pattern from Phase 8a).
8. Spawn pipeline; return 202 Accepted.

The pipeline runs the same decode + thumbnail generation as upload. **Skip
user-edited EXIF**: when `replaced_at IS NOT NULL`, do not write
`exif_json`, `target`, `camera`, etc. — those are user-controlled now.
Only width/height/bytes (file-derived) are refreshed. Implementation
detail: pass a `pipeline_options` enum to the pipeline runner.

## Visibility filter — public surfaces

Every existing public SELECT gains `AND published_at IS NOT NULL`:

- `photos/list.rs` — gallery, profile feed, following feed, target page
  listings (when added)
- `photos/get.rs` — photo detail. Returns 404 unless `published_at IS NOT NULL`
  OR `owner_id = current_user`. When owner views own draft, response
  includes `is_draft: true` so the UI renders the warning strip.
- `photos/count.rs` — public counters. `/api/me/stats` separately reports
  draft_count for the owner.
- Engagement: `appreciations` and `comments` reject any action targeting a
  draft photo with 404. Implementation: an existence check before the
  INSERT/DELETE, scoped to `published_at IS NOT NULL`.

## Photo detail page — owner draft state

When `is_draft = true` AND requester is owner, the page renders a 44 px
warning strip directly under `<AppHeader>` (same pattern as the Phase 8a
grace banner):

```
● DRAFT · ONLY YOU CAN SEE THIS
[Continue editing →]   [Discard]
```

- **Continue editing →** navigates to `/upload/{id}/verify` if
  `last_step IN ('upload', 'verify')`, otherwise `/upload/{id}/caption`.
  (`last_step = 'upload'` means the user only ever submitted the file —
  the pipeline may still be running, but Step 02 is the natural landing.)
- **Discard** opens an inline confirm (no modal — just toggle a state and
  show a second button), then `DELETE /api/photos/:id`.

Below the strip, the normal photo detail layout renders. The ⋯ action menu
in the photo column gains owner-only entries:

| Photo state | ⋯ menu entries                                                      |
|-------------|---------------------------------------------------------------------|
| Draft       | Edit metadata · Replace image… · Discard draft                       |
| Published   | Edit metadata · Replace image… · Delete photo                        |

"Edit metadata" navigates to `/upload/{id}/verify` (the Step 02 route used
by the upload flow — same form, same behaviour).

## REPROCESSED display

When `replaced_at IS NOT NULL` and the viewer is on a published detail
page, the sidebar shows under the published-date eyebrow:

```
● REPROCESSED · 14 MAR → 02 MAY 2026
```

Format: `DD MMM` for both dates if same calendar year; `DD MMM YYYY` for
the right side if different year. Mono, `--fg-muted`. Same year inference
keeps the line short — the design's "14 MAR → 02 MAY 2026" example is
the expected shape.

## My Photos page — `/account/frames`

Auth-required (`+page.server.ts` redirects to `/signin?next=` if no user).

Layout (1280 px content max-width):

```
AppHeader
─────────────────────────────────────────────────────────
Title row : h1 "My frames"
            STATS · 4 cells right-aligned :
              PUBLISHED · {n}
              DRAFTS · {n}        (--accent if > 0)
              TOTAL INTEGRATION · {h} h {m} m
              APPRECIATIONS · {n}
─────────────────────────────────────────────────────────
{#if drafts > 0}
  Drafts callout band — `--bg-warning-tint`, padding 24/64
    eyebrow ● {n} DRAFTS · NOT YET PUBLISHED   |   SEE ALL DRAFTS →
    3-up grid of <DraftCard /> (truncated to first 3)
{/if}
─────────────────────────────────────────────────────────
<FilterChips active={filter} counts={…} sort={sort} view={view} />
─────────────────────────────────────────────────────────
<PhotosTable rows={photos.rows} />
```

URL params control state:
- `?filter=all` (default) | `published` | `drafts`
- `?sort=newest` (default) | `oldest`
- `?view=list` (default) | `grid`

`/account/frames/drafts` is a `+page.server.ts` only — `redirect(303,
'/account/frames?filter=drafts')`. Single source of truth for the listing.

### Empty states

- **Zero photos** (new user): replaces the entire body with the design's
  Empty state screen — Atlas medallion, "An empty plate, waiting for first
  light." headline, "Upload a frame" primary CTA. No stats row, no chips.
- **Zero drafts but published photos exist**: drafts callout entirely
  hidden; table renders only published rows.
- **Filter = drafts but zero drafts**: small friendly message — "No drafts.
  Every frame you upload is published." + "Upload a frame" link.

### `PhotosTable` rows

Each row:

| Column         | Render                                                                            |
|----------------|-----------------------------------------------------------------------------------|
| Thumb (60×60)  | `<img>` with 1px dashed `--warning` border + 40% black overlay if `is_draft`      |
| Target         | `<PhotoTitle photo={p} size="sm" />` — handles untitled fallback                  |
| Captured       | Date or `—`                                                                       |
| Integration    | `{exposure_s formatted}` or `—`                                                   |
| Status         | `chip-accent "PUBLISHED"` or `chip-warning "DRAFT"`                                |
| ♡              | Count or `—` (drafts have no appreciations)                                       |
| ⋯              | Open action menu: Edit / Replace / Delete or Discard                              |

Draft rows get `opacity: 0.78` (CSS scoped on `.row.is-draft`).

## Polish 8.5 — micro-fixes

### 1. Context-aware eyebrow on logged-in home

In `frontend/src/routes/+page.server.ts` load, when `locals.user`:

- Compute `following_count = locals.user.following_ids?.length ?? 0`.
- Pass to page.

In `frontend/src/routes/+page.svelte`:

```svelte
{#if data.user && data.following_count > 0}
  <span class="t-eyebrow accent">
    ● FROM THE {data.following_count} PHOTOGRAPHERS YOU FOLLOW
  </span>
{:else}
  <span class="t-eyebrow">● {dateString} · {weekday}</span>
{/if}
```

The "M NEW since last visit" suffix is **out of scope** (requires a
`users.last_seen_at` column not in the schema; documented as deferred).

### 2. FollowButton — 3 states with hover transition

`frontend/src/lib/components/FollowButton.svelte` extended:

| State                  | Classes                                              | Label          |
|------------------------|------------------------------------------------------|----------------|
| Not following          | `.btn .btn-primary .btn-sm`                          | `Follow`       |
| Following · default    | `.btn .btn-secondary .btn-sm`, accent border + text  | `✓ Following`  |
| Following · hover      | same shape, `--danger` border + text                 | `Unfollow?`    |

CSS-only hover transition on color/border. On click-to-follow, briefly fill
to primary for 150 ms, then settle into `✓ Following` over 240 ms — a
small CSS class toggle on optimistic update.

The avatar + display name **remain a separate link** to `/u/[username]`.
Clicking the FollowButton does not navigate.

### 3. Untitled photo fallback

A new shared component `frontend/src/lib/components/photos/PhotoTitle.svelte`:

```svelte
<script lang="ts">
  let { photo, size = 'md' }: {
    photo: { target?: string | null; original_name: string };
    size?: 'sm' | 'md' | 'lg';
  } = $props();
</script>

{#if photo.target}
  <span class="title size-{size}">{photo.target}</span>
{:else}
  <span class="title untitled size-{size}">{photo.original_name}</span>
  <em class="chip chip-dashed">UNTITLED</em>
{/if}
```

Used in:
- `/photo/[slug]/+page.svelte` — header (size=lg)
- Profile cards `/u/[username]/+page.svelte` — (size=md)
- Gallery cards `/+page.svelte` — (size=md)
- `<PhotosTable>` row target column — (size=sm)
- `<DraftCard>` — (size=sm)

### 4. Mobile AppreciateButton — sticky bottom bar

`frontend/src/lib/components/AppreciateButton.svelte` gains a
`variant: 'inline' | 'mobile-sticky'` prop. Mobile sticky bar is rendered
in `/photo/[slug]/+page.svelte` only on viewport ≤ 640 px (CSS media query
on the wrapping `<div>`).

Bar:
- 64 px tall, `background: var(--bg-overlay); backdrop-filter: blur(12px);`
- `border-top: 1px solid var(--border-subtle); padding-bottom: env(safe-area-inset-bottom);`
- 3 pills inside, 44 px tall: heart + count, comment + count, share icon.

Active state (current user has appreciated): pill background
`--bg-accent-tint`, border `--accent`, heart filled, count in `--accent`.
Tap toggles. Increment animates +1 over 240 ms (`@keyframes count-up`).

**Long-press appreciators sheet is deferred** (requires its own endpoint
+ bottom-sheet UI primitive).

## Tests

### Backend integration tests

`backend/tests/photos_phase8b.rs` (NEW file; Phase 5 tests stay in
`backend/tests/photos.rs`):

```
upload_creates_draft_with_null_published_at
publish_endpoint_sets_published_at_and_last_step_caption
publish_idempotent_on_already_published
publish_403_for_non_owner
publish_400_when_status_processing

draft_invisible_in_public_gallery
draft_invisible_in_following_feed
draft_invisible_in_profile_feed
draft_returns_404_for_non_owner_on_detail
draft_returns_200_with_is_draft_for_owner_on_detail

appreciate_a_draft_returns_404
comment_on_draft_returns_404
drafts_list_only_returns_current_user_drafts
list_with_drafts_query_rejects_cross_user_owner_id

put_metadata_works_on_draft_and_published
put_metadata_403_for_non_owner

replace_swaps_storage_key_keeps_metadata
replace_preserves_caption_target_exif_appreciations_comments
replace_404_for_non_owner
replace_400_when_pipeline_busy
replace_400_when_no_file
replace_deletes_old_s3_objects
replace_sets_replaced_at
replace_regenerates_thumbnails
appreciation_count_unchanged_after_replace

me_stats_returns_published_draft_integration_appreciations
me_stats_excludes_drafts_from_appreciations
me_stats_excludes_drafts_from_integration_sum

discard_draft_deletes_row_and_s3
delete_published_returns_204
```

### Frontend e2e (Playwright)

`frontend/tests/e2e/photos_phase8b.spec.ts`:

```
upload a draft, leave, find it in /account/frames, continue and publish
edit metadata of a published photo via ⋯ menu, save changes, no republish
replace a published photo, REPROCESSED label appears on detail
FollowButton toggles through 3 states with correct copy
untitled photo on home gallery shows UNTITLED chip
mobile viewport: sticky AppreciateButton bar appears, tap toggles state
```

## Out of scope

- **2FA** — still deferred (see Phase 8a Out of scope).
- **Equipment library and Notifications system** — settings nav keeps the
  SOON chips.
- **EXIF re-extraction toggle on Replace** — added later if requested.
- **Long-press appreciators sheet on mobile** — separate feature; needs an
  endpoint + bottom-sheet primitive.
- **`M NEW since last visit` counter on the home eyebrow** — needs a
  `users.last_seen_at` column not introduced here.
- **Auto-save draft via beforeunload + localStorage** — too sophisticated
  for MVP; the explicit "Save as draft" button covers the use case.
- **Side-by-side compare before / after Replace** — deferred (the design's
  "dedicated page alternative" to the modal).
- **Drafts public preview link** — sharing a draft to a peer for feedback
  before publication; future feature.
- **Per-EXIF-field "edited manually" tracking on Replace** — currently the
  whole `exif_json` is preserved as-is on replace. If users want to
  selectively re-extract some fields, the toggle ships later.

## References

- Phase 8 design handoff: `~/Downloads/design_handoff_astrophoto 2/`
  (`README.md` sections 9 / 10 / 11 ; `README - Phase 8.md` sections 05 /
  06 / 07).
- Phase 8a spec (deferred items): `docs/superpowers/specs/2026-05-02-phase-8a-security-account-design.md`.
- Phase 5 plan (where `ScreenMyPhotos` was promised but not built):
  `docs/superpowers/plans/2026-05-02-phase-5-photos.md`.
- Existing photo modules: `backend/src/photos/`,
  `frontend/src/routes/{photo,upload,u}/`.
- `lettre`, `woothee`, AWS SES — unchanged from Phase 8a.
