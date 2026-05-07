# Upload polish — design

**Date:** 2026-05-07
**Status:** Draft, pending implementation plan
**Spec author:** brainstorming session, Pascal Le Clech

## Summary

Polish the multi-file upload flow into a coherent batch experience, add
resilience (retry, cancel, reload recovery, autosave), and close several
known UX gaps in verify/caption (tags loading, tier wiring, drafts
discoverability, processing flicker).

The current flow accepts multiple files in step 1 but funnels into a
single-photo verify/caption pair in steps 2 and 3, leaving N–1 drafts
stranded with no UI to find them. This spec replaces steps 2 and 3 with a
batch-aware two-step flow and adds the supporting backend endpoints.

Speed/feedback work (web-worker hashing, real-time progress) and
plate-solve are explicitly **out of scope**.

## Motivation

Pain points addressed:

- Step 2 only verifies one photo; the other N–1 sit as drafts with no
  visible surface.
- A failed upload row offers no retry; an in-flight upload can't be
  cancelled.
- Closing the tab loses both the metadata being edited and the link
  back to the batch.
- `TIER_MAX` is hardcoded at 50 MB — subscribers see incorrect quota.
- Tags don't load when editing a published photo (P2 TODO).
- Verify pane shows broken-image flicker while `display/<id>.jpg` is
  still being generated.
- No drafts list page; users have no way to find unpublished work.

## Non-goals

- Web-worker hashing / faster perceived completion.
- Replacing the 2-second poll on processing photos with realtime push.
- Plate-solve worker (remains placeholder).
- Drag-to-reorder photos within a batch.
- Sharing exposure/equipment metadata across the batch. Only target
  and tags are applied to all in step 2a; everything else (ISO,
  exposure, equipment, RA/Dec, sensor temp, gain, sessions, caption)
  stays per-photo in step 2b.

## User flow

### Single photo (N=1)

1. Drop one file on `/upload`.
2. Hash + thumb client-side, init, presigned PUT, finalize.
3. Auto-redirect to `/upload/<id>/verify` (existing route, with caption
   field merged in).
4. Click **Publish** → goes to `/u/<handle>/p/<short-id>`.

The N=1 case **skips step 2a** entirely: applying target+tags to a
single photo is no different from filling those fields on the verify
pane.

### Multi-file (N≥2)

1. Drop N files on `/upload`. Each row shows hashing → uploading →
   finalizing.
2. When **all rows are at least `ready` or `failed`** (i.e. nothing
   in-flight), the page shows a **"Continue with N frames →"** CTA. The
   user can also continue earlier — failed/cancelled slots are simply
   excluded from the batch URL.
3. Click → `/upload/batch?ids=<csv>` (step 2a, "apply to all"): set
   target + tags shared across the batch.
4. Click **Continue** → `/upload/batch/edit?ids=<csv>` (step 2b): photo
   ribbon, per-photo verify pane with caption, autosave.
5. Click **Publish all** → publishes ready+draft photos in the batch,
   redirects to `/u/<handle>` showing the published count. Skipped
   photos (still processing or failed) remain drafts.

### Reload recovery

If a user reloads `/upload` while drafts exist (created within the last
24 h), a banner appears above the dropzone:

```
● N DRAFTS IN PROGRESS
Continue verifying frames from <relative time>
                          [ Discard ]   [ Resume ]
```

`Resume` → `/upload/batch/edit?ids=<all-recent-draft-ids>`.
`Discard` → DELETE each (with confirm).

For older drafts (>24 h), no banner; users find them via `/me/drafts`.

## Routes

| route | role | change |
|---|---|---|
| `/upload` | Step 1. Multi-file dropzone. | Adds retry/cancel/remove per slot, reload-recovery banner, tier-aware quota. |
| `/upload/batch?ids=` | Step 2a. Apply target + tags to all. | **New.** Skipped if N=1. |
| `/upload/batch/edit?ids=` | Step 2b. Ribbon + per-photo verify pane (with caption) + bulk publish. | **New.** |
| `/upload/<id>/verify` | Single-photo edit. | Adds caption field; gains tag loading; processing-state placeholder. |
| `/upload/<id>/caption` | Removed. | **Deleted.** Functionality folded into `/verify`. |
| `/me/drafts` | Drafts list. | **New.** Auth-scoped (not handle-scoped). |

## Components

### Step 1 — `/upload`

**Existing components changed:**

- `UploadDropzone` — copy reflects user tier; quota max comes from
  `data.user.tier`.
- `UploadFileRow` — gains two icon buttons in the right gutter:
  - **× cancel/remove** — visible during `hashing | queued | uploading
    | finalizing`. Behaviour:
    - `hashing | queued`: drop from local list (no server work yet).
    - `uploading`: `xhr.abort()` (requires `AbortController` per slot
      in `presigned.ts`), then `DELETE /api/uploads/<id>`.
    - `finalizing`: gray out the × button (UX choice — the window is
      too short to be worth surfacing). The DELETE endpoint still
      accepts processing+is_draft state, so a fast user could call it
      programmatically; we just don't expose the button.
    - Confirm-on-click only when `uploading` and `pct > 50`.
  - **↻ retry** — visible only when `state === 'failed'`. Reissues the
    appropriate phase (PUT or finalize), no re-hash.
- `+page.svelte` upload page — adds a **resume banner** above the
  dropzone when `data.recentDrafts.length > 0`. Adds a sticky
  "Continue with N frames" CTA at the bottom of the file list once
  every slot is `ready` or `failed`.

**New file:** `frontend/src/lib/upload/pump.ts` — replaces the
`uploadAll()` queue model with a slot-by-slot pump and a shared
3-permit semaphore. Each slot's lifecycle (init → PUT → finalize) is an
independent async fn that takes a permit. New slots dropped mid-batch
are picked up automatically.

`init` becomes per-slot rather than batched: one POST per slot. This
costs one extra round-trip per slot but vastly simplifies retry/cancel
and concurrent-add semantics. (The pre-validation in
`/api/uploads/init` still happens, and the per-owner-hash dedup still
works.) Alternatively, batch init can stay the same and the cancel
endpoint just deletes a single id; we'll let the implementation plan
decide based on actual code shape.

### Step 2a — `/upload/batch`

**New components:** `BatchApplyForm` (or just inline in `+page.svelte`).

Layout (~720 px centered):

```
[stepper: ✓ UPLOAD · ② APPLY TO ALL · ③ PER-PHOTO]

▦ APPLY TO ALL · 5 frames
"These will be set on every frame. You can override per-photo on the next step."

  TARGET     [ TargetPicker ............ ]
  TAGS       [ TagInput ................ ]

  thumb strip (read-only, 72×72 each):
  ▣ ▣ ▣ ▣ ▣

                        [ Skip ]   [ Continue → ]
```

- **Skip** → `/upload/batch/edit?ids=<csv>` without writing.
- **Continue** → `POST /api/photos/batch/apply { ids, target?, tags? }`,
  then redirect.
- Empty fields are ignored (no overwrite).

**Server load:** validate ownership and that every id is `is_draft=true`,
not yet published. If any id fails the check → 400 with the offending
id. (No silent filtering — fail loud.)

### Step 2b — `/upload/batch/edit`

**New extracted component:** `VerifyPane.svelte` — encapsulates the body
of the existing `/upload/[id]/verify` route (preview + form + caption).
Used by both `/upload/batch/edit` and the standalone `/upload/<id>/verify`
route, so single-photo and batch flows share the same edit surface.

**New components:**

- `BatchRibbon.svelte` — sticky horizontal strip, 64×64 thumbs, status
  pip per thumb (`✓ verified` / current / `—` pending / `⟳ processing`
  / `✗ failed`), prev/next arrows, `?selected=<id>` URL param. Keyboard
  `←/→` cycles.
- `useAutosave.ts` (helper, not a component) — debounced 800 ms
  `PATCH /api/photos/<id>/metadata` from a Svelte 5 reactive store. The
  pane reads/writes the store; the helper handles the HTTP calls,
  retries, and the "● Saved Xs ago" indicator state.

Layout:

```
[stepper: ✓ UPLOAD · ✓ APPLY TO ALL · ③ PER-PHOTO]

▦ PER-PHOTO · 5 frames                ● Saved 2s ago

[ ribbon: 5 thumbs, current highlighted ]
prev   next                                          3 of 5

[ VerifyPane for the selected photo ]

footer:
[ ← Back to apply-to-all ]                  [ Publish all ]
```

**Caption** — moves into `VerifyPane` as a `<Textarea name="caption"
rows={6}>`. The standalone `/upload/<id>/verify` form action that
previously redirected to `/caption` now stays on the page. In the
batch flow, navigation between photos uses the ribbon's prev/next or
clicking a thumb; saves are continuous via autosave (no "Save & next"
button needed).

**Autosave details:**

- Triggers on field-change, debounced 800 ms.
- Sends only the diff (changed fields).
- Server endpoint must accept `null` for "clear this field" vs absent
  for "leave alone" — confirm the existing PATCH handler does this.
- Indicator states: `idle` ("● Saved 2s ago"), `saving` ("● Saving…"),
  `error` ("● Save failed — retry") with a manual retry button.
- Form-action paths (`?/save_continue`, `?/save_changes`,
  `?/save_changes_published`) **stay** for non-JS fallback and as the
  canonical save mechanism for `/verify` published-photo edits;
  autosave is layered on top.

**Processing flicker fix** — `VerifyPane` checks `photo.status` before
rendering `<Img>`:

- `processing`: full-frame placeholder reusing the existing processing
  overlay style.
- `failed`: error frame with the pipeline_error string and "Retry
  upload" CTA (current behaviour, kept).
- `ready`: `<Img photoId={photo.id} w={1200}>` as today.

Polling stays at 2 s via `invalidateAll()` while `status='processing'`
on the verify route. Speed/realtime is out of scope.

**Tags loading** — `GET /api/photos/<id>` adds `tags: string[]`. The
load function passes it to the page; `VerifyPane` seeds the `TagInput`
from it.

**Publish all** — `POST /api/photos/batch/publish { ids }`:

- Server filters to ids the caller owns and that are
  `is_draft=true AND status='ready'`. Strict 403 if any id in the
  request body isn't owned by the caller (don't silently filter
  ownership errors; ownership = security boundary; we do silently skip
  not-yet-ready, which is a UX choice not a security one).
- Returns `{ published: [{ id, short_id }], skipped: [{ id, reason }] }`
  where `reason ∈ "still_processing" | "failed" | "already_published"`.
- UI shows a toast ("Published 3 of 5 — 2 still processing") and
  redirects to `/u/<handle>`. Skipped photos remain drafts (visible on
  `/me/drafts`).

### Drafts page — `/me/drafts`

**New components:** `DraftsGrid.svelte`, `DraftTile.svelte` (or inline).

- Auth-scoped: redirect to `/signin` if unauthenticated.
- Server load fetches `GET /api/photos/me/drafts` (paginated 24 per
  page).
- Each tile shows thumb, target (or "untitled"), relative upload time,
  status pip.
- Per-tile actions: **Resume** → `/upload/batch/edit?ids=<this-id>`.
  **Discard** → `DELETE /api/photos/<id>` with confirm.
- Top-level "Resume all recent" CTA groups drafts created within the
  same hour into a batch URL — convenience for "I uploaded 5 frames
  yesterday and didn't publish".

## Data flow

### Step 1 (per slot)

```
client                             server                    S3
  │                                  │                         │
  │  preflight (hash, thumb)         │                         │
  │  POST /api/uploads/init [1 file] │                         │
  │ ───────────────────────────────► │                         │
  │                                  │  insert photos pending  │
  │ ◄──────────────────────────────  │                         │
  │  { photo_id, presigned_put_url } │                         │
  │                                                            │
  │  XHR PUT presigned_put_url      ────────────────────────► │
  │  (with AbortController)                                    │
  │                                                            │
  │  POST /api/uploads/<id>/finalize                           │
  │ ───────────────────────────────► │                         │
  │                                  │  set status=processing  │
  │                                  │  spawn_blocking          │
  │                                  │  generate display master│
  │                                  │  set status=ready        │
  │ ◄──────────────────────────────  │                         │
```

### Cancel mid-upload

```
client                             server                    S3
  │  xhr.abort()                                               │
  │  DELETE /api/uploads/<id>        │                         │
  │ ───────────────────────────────► │                         │
  │                                  │  authz: owner+pending   │
  │                                  │  delete photo row        │
  │                                  │  delete originals/<id>   │
  │                                  │ ──────────────────────► │
  │ ◄──────────────────────────────  │                         │
  │  { ok }                          │                         │
```

### Step 2b autosave

```
client (VerifyPane)                server
  │  user types in field            │
  │  store update                   │
  │  debounce 800ms                 │
  │  PATCH /api/photos/<id>/metadata│
  │ ──────────────────────────────► │
  │                                 │  validate, update photos
  │                                 │  replace photo_tags if tags
  │                                 │  in body
  │ ◄──────────────────────────────  │
  │  { ok, updated_at }             │
  │  indicator: "● Saved 2s ago"    │
```

## API endpoints

### New

#### `POST /api/photos/batch/apply`

Apply shared metadata to N photos in one transaction.

```json
// request
{
  "ids": ["uuid1", "uuid2", "uuid3"],
  "target": "M31 Andromeda Galaxy",
  "tags": ["andromeda", "galaxy"]
}
// response
{ "applied": 3 }
```

- Auth: current user.
- Validation: every id must be owned by caller, `is_draft=true`,
  `is_published=false`. Strict 403 on any ownership failure (don't
  partially apply).
- `target` and `tags` are both optional. Semantics:
  - Absent or `null` `target` → leaves existing target alone.
  - Empty string `target` → not allowed (400). Use the verify pane to
    clear a target on a single photo.
  - Absent `tags` → leaves existing tags alone.
  - `tags: []` → explicitly replaces all photos' tags with empty.
  - `tags: ["a", "b"]` → replaces all photos' tags with `["a", "b"]`
    (set semantics, not append).

#### `POST /api/photos/batch/publish`

```json
// request
{ "ids": ["uuid1", "uuid2", "uuid3", "uuid4", "uuid5"] }
// response
{
  "published": [{ "id": "uuid1", "short_id": "abc123" }, …],
  "skipped":   [{ "id": "uuid4", "reason": "still_processing" }, …]
}
```

- Auth: current user. Strict 403 on any ownership failure.
- Filters to `is_draft=true AND status='ready'`. Skipped reasons:
  `still_processing | failed | already_published`.
- Each publish runs the same publish path as the existing single-photo
  publish (sets `is_draft=false`, sets `published_at`, etc.).

#### `GET /api/photos/me/drafts`

Paginated list of caller's drafts.

Query params: `cursor`, `limit` (default 24, max 50).

```json
{
  "items": [
    {
      "id": "uuid",
      "short_id": "abc123",
      "original_name": "light_001.fits",
      "target": "M31",
      "status": "ready",
      "created_at": "2026-05-07T14:32:00Z",
      "thumb_url": "/cdn/img/uuid?w=320"
    }
  ],
  "next_cursor": "..." | null
}
```

- Auth: current user.
- Filters: `owner_id = current_user AND is_draft = true`.
- Used by reload banner (filtered to last 24 h) and `/me/drafts` page.

#### `DELETE /api/uploads/<id>`

Cancel a pending upload. Distinct from `DELETE /api/photos/<id>`
(which exists for published deletion) because the semantics differ:
this endpoint also deletes any partial S3 object at `originals/<id>`
and only succeeds while the photo is still in `pending` state.

- Auth: current user.
- Validation: photo must be owned and in `status='pending'` OR
  `is_draft=true AND status='processing'`.
- Effects: deletes photo row, deletes `originals/<id>` from S3 (best
  effort — log warning if S3 delete fails but DB delete succeeds).

#### Pending-photo cleanup cron

Periodic task (hourly) deleting `photos` rows where
`status='pending' AND created_at < now() - interval '24 hours'`. Best
effort S3 cleanup. Logs orphaned-id count per run.

24 h chosen over 1 h to allow for very slow uploads and intentional
wait (user opens upload, takes lunch, finishes after).

Implementation: a tokio task spawned from `main.rs` on a 1 h interval.
No new dependency.

### Changed

#### `GET /api/auth/me`

Add `tier: "free" | "subscriber"` to the response. Source: `users.tier`
column (already exists). `users.tier` defaults to `'free'` and gets
upgraded by Stripe webhooks (out of scope here, already wired
elsewhere).

#### `GET /api/photos/<id>`

Add `tags: string[]` to `PhotoDetail`. Source: existing `photo_tags`
join.

#### `PATCH /api/photos/<id>/metadata`

Confirm (don't change unless needed) that:

- Partial bodies are accepted (each field is optional).
- `null` clears, absent leaves alone.
- Tolerates `status='processing'` (we want autosave to work while the
  display master is being generated).

If today's behaviour doesn't match, fix it as part of this work.

## Types (ts-rs)

New Rust types exported via ts-rs:

```rust
#[derive(Serialize, Deserialize, TS)]
pub struct BatchApplyRequest { pub ids: Vec<Uuid>, pub target: Option<String>, pub tags: Option<Vec<String>> }
#[derive(Serialize, TS)]
pub struct BatchApplyResponse { pub applied: u32 }

#[derive(Serialize, Deserialize, TS)]
pub struct BatchPublishRequest { pub ids: Vec<Uuid> }
#[derive(Serialize, TS)]
pub struct BatchPublishResponse {
    pub published: Vec<PublishedItem>,
    pub skipped: Vec<SkippedItem>,
}
#[derive(Serialize, TS)]
pub struct PublishedItem { pub id: Uuid, pub short_id: String }
#[derive(Serialize, TS)]
pub struct SkippedItem { pub id: Uuid, pub reason: SkipReason }
#[derive(Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason { StillProcessing, Failed, AlreadyPublished }

#[derive(Serialize, TS)]
pub struct DraftsListItem { /* ... */ }
#[derive(Serialize, TS)]
pub struct DraftsListResponse { pub items: Vec<DraftsListItem>, pub next_cursor: Option<String> }
```

Plus additions to existing types:

- `MeResponse.tier: UserTier` (new enum: `Free | Subscriber`).
- `PhotoDetail.tags: Vec<String>`.

Run `just types` after backend changes. Commit the diff.

## Schema

**No migrations.** All required state is on the existing `photos` table
(`is_draft`, `status`, `original_hash`, etc.) and `users.tier`.

## Testing

**Backend** (testcontainers, real Postgres + S3-compatible mock):

- `POST /api/photos/batch/apply` — happy path; ownership 403; mixed
  states 400; empty tags clears, absent tags leaves alone.
- `POST /api/photos/batch/publish` — publishes ready ones, skips
  processing/failed; ownership 403; idempotent on already-published.
- `DELETE /api/uploads/<id>` — happy path; not-owned 403; not-pending
  conflict 409; S3 object gets removed.
- `GET /api/photos/me/drafts` — pagination, ordering by created_at
  desc, only owner's drafts visible.
- Pending cleanup cron — fixture pending row older than 24 h gets
  deleted; younger row is kept.

**Frontend** (svelte-check + targeted unit tests):

- `useAutosave` debounce + diff logic (vitest).
- `pump.ts` semaphore behaviour (vitest).
- `preflight.test.ts` extended for retry path.

**E2E** (chrome-devtools-mcp, interactive — per project convention,
no Playwright authoring):

- Single-photo flow (regression).
- 3-photo batch happy path: drop → 2a → 2b → publish all.
- Cancel mid-upload removes the slot and the S3 object.
- Retry after simulated failure publishes successfully.
- Reload mid-2b lands on `/upload` with banner, click Resume returns
  to 2b with the same photos.
- `/me/drafts` shows drafts and Discard works.

## Risk & rollout

- **Backwards compatibility**: `/upload/<id>/caption` route deletion is
  the only breaking URL change. Add a 301 redirect from
  `/upload/<id>/caption` → `/upload/<id>/verify` for any users with
  bookmarks or in-flight tabs.
- **Form-action regressions**: the standalone `/verify` route still
  uses form actions (publish/draft/save). Autosave is additive; if the
  patch endpoint regresses, manual save still works. Test with
  JavaScript disabled.
- **Pending cleanup**: 24 h TTL must not delete rows that are still in
  active flow. Drafts (`is_draft=true`) are never touched — only
  `status='pending'` rows that never reached `processing`.
- **Per-slot init round-trips**: if we go with per-slot init instead
  of batched, init becomes N HTTP calls instead of 1. Each is small
  (one DB tx, one presign), but if N=10 on a slow connection that's a
  visible delay. Monitor; fall back to batched init if needed. The
  spec leaves this to the implementation plan.

## Build order

Each phase ships independently; the existing single-photo flow keeps
working until phase 5.

1. **Backend foundation** — `MeResponse.tier`, `PhotoDetail.tags`,
   `GET /api/photos/me/drafts`, `DELETE /api/uploads/<id>`, pending
   cleanup cron. Testable in isolation.
2. **Step 1 polish** — retry/cancel/remove buttons, pump/semaphore,
   reload-recovery banner, tier wiring on dropzone.
3. **Step 2a** — `/upload/batch` page + `POST /api/photos/batch/apply`
   endpoint.
4. **Step 2b** — `/upload/batch/edit` page (extracted `VerifyPane` +
   `BatchRibbon`), `useAutosave`, processing placeholder,
   `POST /api/photos/batch/publish`. Caption merged into `VerifyPane`.
5. **Verify-route migration** — delete `/upload/<id>/caption`, add 301,
   update form actions on `/upload/<id>/verify` to do publish from
   that page directly.
6. **Drafts page** — `/me/drafts`.

## Open issues for implementation plan

- Per-slot vs batched init (Risk section): defer decision to plan
  author, who can read the actual `presigned.ts` and decide.
- Exact column projection for `DraftsListItem` (need owner-side fields
  only; should mirror the explore-feed photo card or be slimmer).
- Whether autosave should send `null` to clear vs require an explicit
  "clear field" UI affordance.
