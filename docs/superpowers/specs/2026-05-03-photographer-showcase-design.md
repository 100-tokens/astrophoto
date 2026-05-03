# Photographer Showcase Design — Upload, Hero Page, Discovery

**Date:** 2026-05-03
**Status:** Draft — pending written-spec review
**Author:** Pascal (with Claude)
**Design handoff:** A high-fidelity design built from this spec lives at
`/Users/pleclech/Downloads/design_handoff_astrophoto 3/showcase/`
(`Astrophoto Showcase.html`, `showcase-p1.jsx`, `showcase-p2.jsx`,
`showcase-p3.jsx`, `showcase-cross.jsx`, `styles.css`).

**Dual-canonical rule** — borrowed from the handoff README:

> Where the spec and the design disagree, the **spec is canonical** for
> behavior, schema, URLs, and security; the **design files are canonical**
> for layout, hierarchy, and copy.

Inside this document, dimensions, owner-mode banner copy, empty-state
copy, and component states reflect the design. Endpoints, schema, and
sanitization rules are this document's own decisions.

## Goal

Improve the three surfaces that determine whether astrophoto feels like a
serious place to share work:

1. **Image upload** — multi-file, presigned PUT direct to S3, polished
   3-step wizard with drag-and-drop, real progress, EXIF preflight, and
   client-side hash dedup. Tier-gated size (50 MB free / 200 MB
   subscriber).
2. **Public gallery from a photographer** — `/u/<handle>` becomes a
   polished public profile + portfolio. Cover, identity, rich-text bio,
   equipment, location/sky, auto-stats, **3–6 featured pinned photos**,
   and a justified-rows gallery of all published work.
3. **Discovery** — global Explore feed plus target / tag / equipment /
   category browse pages and a search bar. Designed so that a future
   plate-solve phase can write into the same data model with no schema
   churn.

The work ships as **one spec, three phases**:

- **P1 Foundations** — schema + URL changes + presigned PUT + S3 +
  CloudFront + Lambda image transformer + handles + tier enforcement.
  No visible product shift; old URLs 301 to new shapes.
- **P2 Hero Page** — `/u/<handle>` redesign with profile editor,
  cover picker, featured-photos, justified-rows gallery, lightbox,
  appreciate UI.
- **P3 Discovery** — `/explore`, `/t/<slug>`, `/tag/<slug>`,
  `/equip/<kind>/<slug>`, `/c/<category>`, `/search`, plus
  autocomplete endpoints.

## Decisions

| #   | Topic                                | Choice                                                                                       |
| --- | ------------------------------------ | -------------------------------------------------------------------------------------------- |
| 1   | Platform identity                    | Both: hero pages first-class portfolios + global discovery surface                           |
| 2   | Hero page role                       | Polished profile inside astrophoto; single template; light personalization slots             |
| 3   | Input formats                        | JPEG, PNG, TIFF (16-bit). Drop FITS / RAW for now                                            |
| 4   | Upload size                          | 50 MB free tier / 200 MB subscriber tier                                                     |
| 5   | Subscription scope                   | `users.tier` column + enforcement only. **No Stripe/billing this round.**                    |
| 6   | URL handles                          | `/u/<handle>`, required at signup, mutable with old-handle redirects                         |
| 7   | Photo permalink                      | `/u/<handle>/p/<short-id>` (8-char base62)                                                   |
| 8   | Upload flow                          | Keep 3-step wizard, polish heavily; multi-file with parallel PUTs                            |
| 9   | Client preprocessing                 | Instant preview (`createImageBitmap`), EXIF preflight (`exifr`), SHA-256 hash for dedup      |
| 10  | Storage                              | **AWS S3**, replacing Cloudflare R2 for image storage. CLAUDE.md to be updated.              |
| 11  | CDN / image transforms               | **AWS CloudFront** with **Lambda function URL** (sharp) as origin                            |
| 12  | Display master pattern               | At upload finalize, derive 4096 px JPEG `display/<id>.jpg`; CDN transforms from this         |
| 13  | Upload transport                     | Presigned PUT direct to S3, `Content-Length-Range` baked in based on tier                    |
| 14  | Gallery layout                       | Justified rows (Flickr-style) — both hero and discovery pages                                |
| 15  | Discovery primitives                 | Targets, user tags, equipment, photo categories — all four                                   |
| 16  | Plate-solve forward-compat           | `photo_targets.source ('manual'\|'plate_solve')` shipped day-one; only `'manual'` written now |
| 17  | Hero personalization slots           | Cover, avatar, name, tagline, **rich-text bio (allowlist)**, social links, featured photos, equipment, location/sky, auto-stats |
| 18  | Bio editor                           | Tiptap on client, **server-side ammonia sanitizer is the source of truth**                   |
| 19  | Engagement primitives                | **Appreciations** (existing `appreciations` table from Phase 7). Add `photos.appreciations_count` for popular-sort. Comments and follows already exist (Phase 7) — inherited, not extended in this spec |
| 20  | Equipment model                      | Free-text on photos + `equipment_items` lookup table (autocomplete + browse)                 |
| 21  | Design system                        | Tokens, typography, components live in the handoff `styles.css` — single source of truth. Spec does not redefine them |

## Out of scope

Explicitly **not** built in this spec; deferred to future phases:

- AI "is-this-astronomy" validation at upload
- Astrometry / plate solving (target model designed to accept it cleanly later)
- Creator monetization, Stripe Connect, tip jars, subscription billing UI
- Direct messages between users
- Collections (the Featured Photos slot replaces this for the hero use-case)
- FITS / RAW input formats

## Inherited from earlier phases (not built here, but used)

These features already exist in the project (migrations `0001`–`0004`)
and are surfaced by this spec without modification:

- **Appreciations** — Phase 7. `appreciations(user_id, photo_id, created_at)`.
  Endpoints `POST /api/photos/:id/appreciate` and `DELETE`. The hero page,
  lightbox, photo tile, and discovery sort all read this. P1 of this spec
  adds a denormalized `photos.appreciations_count` column for popular-sort
  performance.
- **Comments** — Phase 7. `comments(...)`. Surfaced on the photo detail page
  per existing design; no changes here.
- **Follows** — Phase 7. `follows(follower_id, followed_id)`. Drives the
  Follow button on the hero page and the `following=true` filter on
  `/explore`.
- **Drafts and Replace** — Phase 8b. `photos.published_at`, `last_step`,
  `replaced_at`. The upload wizard flow respects these states; the hero
  page hides drafts from non-owners.
- **Account security** — Phase 8a/8b. 2FA, password reset, account deletion
  with grace. Settings v2 IA stays as designed.

## Design system — references

Tokens (color, typography, spacing, radii, shadows, motion), button /
input / chip / EXIF-table / nav-link / photo-card / corner-marks CSS, and
the wordmark + Reticle logomark are defined once in:

- **`styles.css`** in the handoff bundle — single source of truth for
  CSS custom properties and base component classes.
- **`shared.jsx`** — `<AppHeader>`, `<AppFooter>`, `Photo` placeholder.
- **Main handoff `README.md`** — narrative guide (brand voice,
  italic-on-emphasized-word headlines, type scale, motion budget).
- **Phase 8 `README.md`** — settings, password reset, 2FA, deletion,
  drafts, replace.
- **`showcase/README.md`** — this spec's screens.

The spec does **not** restate these. Components built in P1/P2/P3
inherit the existing tokens and class library. Sodium-amber accent
(`--accent: #e8a43a`), warm near-black canvas, Source Serif 4 (display)
+ Inter (UI) + JetBrains Mono (eyebrows / EXIF / handles) — all already
in the project's stylesheet. Sharp corners (no rounded surfaces beyond
chips and avatars) are intentional.

## Architecture

### Storage

New AWS S3 bucket `astrophoto-images-<env>`. Two object prefixes per
photo:

- `originals/<photo-id>.<ext>` — file as uploaded (JPEG/PNG/TIFF, ≤ 200 MB)
- `display/<photo-id>.jpg` — **display master**: max 4096 px on the long
  edge, q=85 baseline JPEG, sRGB, ICC and EXIF stripped. This is what the
  CDN transforms on demand.

The display master decouples the resizer from heavy/exotic source
formats. A 200 MB 16-bit TIFF gets decoded once at upload; every view
afterwards fetches a small JPEG. Cuts Lambda memory/cold-start cost and
keeps the resizer simple.

### CDN

AWS CloudFront. URL shape:

```
https://cdn.astrophoto.pics/img/<photo-id>?w=&h=&fit=&q=&fm=
```

Origin is a Lambda function URL (Node + sharp). Lambda fetches
`display/<photo-id>.jpg` from S3, resizes per query params, returns.
CloudFront caches by full URL.

The Lambda is ported from the previous project's `image-transformer/`
and adapted from Lambda@Edge to the function-URL pattern (regional, not
edge — simpler deploys, 10 s timeout, up to 10 GB memory).

**Dev**: no CloudFront. Backend exposes `GET /cdn/img/<photo-id>` that
performs the same transforms locally with the `image` crate. Frontend
uses one env-driven `IMG_BASE_URL` constant.

### Upload pipeline (presigned PUT + finalize)

```
Client                        Backend                          S3
  │                              │                              │
  │  POST /api/uploads/init      │                              │
  │   {files:[{name,size,hash}]} │                              │
  │ ────────────────────────────>│                              │
  │                              │  enforce tier upload limit;  │
  │                              │  reject duplicate hashes     │
  │                              │  (per owner);                │
  │                              │  insert photos rows          │
  │                              │  (status='pending');         │
  │                              │  sign PUT with               │
  │                              │  Content-Length-Range        │
  │ <────────────────────────────│                              │
  │  [{photo_id,                 │                              │
  │   short_id,                  │                              │
  │   presigned_put_url}, ...]   │                              │
  │                              │                              │
  │  PUT (file)  ─ ─ ─ ─ ─ ─ ─ ─ │ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─ ─>│
  │  (xhr.upload progress)       │                              │
  │                              │                              │
  │  POST /api/uploads/<id>/     │                              │
  │       finalize               │                              │
  │ ────────────────────────────>│  HEAD object;                │
  │                              │  validate magic bytes;       │
  │                              │  spawn_blocking:             │
  │                              │    - decode (image crate)    │
  │                              │    - extract EXIF            │
  │                              │    - generate blurhash       │
  │                              │    - derive display master   │
  │                              │    - PUT display/<id>.jpg    │
  │                              │  set status='ready'          │
  │ <────────────────────────────│                              │
```

**Critical invariants**:

1. **Client pre-flight, then tier-gated size on the wire**:
   1. Client checks `file.size > tier_max` **before** calling
      `/api/uploads/init`. If exceeded, the upload row is set to the
      `disabled (over-quota)` state and `<TierUpgradeModal>` opens. No
      network call is wasted.
   2. The backend signs the presigned URL with
      `Content-Length-Range: 0,<tier_max>` regardless. Backend reads
      `users.tier` at sign time. A free user PUT of 200 MB returns 400
      from S3 — bandwidth never reaches the file.

   The pre-flight check is the user-friendly path; the wire enforcement
   is the security backstop. The client can be wrong about `tier_max`
   (e.g. stale tier value, tampering); S3 cannot be.
2. **Idempotent finalize**: returns current state if already
   `status='ready'`. Safe to retry.
3. **Magic-byte sniff**: backend reads the first ~16 bytes of the object
   via S3 ranged GET; rejects if MIME doesn't match the declared
   extension. Client `Content-Type` is untrusted.
4. **Display master derivation runs in `spawn_blocking`** — same rule as
   today's thumbnail pipeline. 4096 px long edge, q=85, sRGB, strip ICC
   and EXIF (originals keep them).
5. **Per-owner SHA-256 dedup** (not global). Two photographers may
   legitimately upload the same Hubble crop.
6. **Orphan reaper**: photos in `status='pending'` for more than 2
   hours have their S3 originals deleted and rows hard-deleted. Reuses
   the existing `photo_pending_deletes` mechanism.
7. **`short_id` collisions**: 8-char base62 keyspace (~10^14); on
   collision, retry up to 5 times then raise.

### Component reuse across phases

P1 ships only schema + plumbing + signup/handle UI + the rebuilt upload
wizard. The hero-page redesign in P2 introduces the visual components
(`<HeroCover>`, `<PhotoGrid>`, `<PhotoTile>`, `<Lightbox>`,
`<DiscoveryHeader>`); P3 reuses every one of those without
modification, switching `<PhotoTile>` to `mode="cross-author"` to also
render `<AuthorChip>`.

## Data model

### `users` — new columns

| Column                                                                                            | Type                                                | Notes                                                          |
| ------------------------------------------------------------------------------------------------- | --------------------------------------------------- | -------------------------------------------------------------- |
| `handle`                                                                                          | `citext UNIQUE NOT NULL`                            | `[a-z0-9_-]{3,30}`; reserved-list checked; case-insensitive    |
| `tier`                                                                                            | `text NOT NULL DEFAULT 'free'`                      | check (`'free'`,`'subscriber'`)                                |
| `tagline`                                                                                         | `text`                                              | one-line bio under the name                                    |
| `bio_html`                                                                                        | `text`                                              | sanitized HTML output of rich-text editor                      |
| `cover_photo_id`                                                                                  | `uuid REFERENCES photos(id) ON DELETE SET NULL`     | hero banner; user picks one of their own photos                |
| `equipment_telescope`, `equipment_camera`, `equipment_mount`, `equipment_filters`, `equipment_guiding` | `text`                                              | free-text; autocomplete-fed                                    |
| `location_text`                                                                                   | `text`                                              | city / region only — never coordinates                         |
| `bortle_class`                                                                                    | `smallint`                                          | 1–9, optional, check `between 1 and 9`                         |
| `sqm`                                                                                             | `numeric(4,2)`                                      | optional                                                       |
| `social_links`                                                                                    | `jsonb NOT NULL DEFAULT '[]'`                       | `[{kind, url}]`; kind ∈ small enum (twitter, instagram, astrobin, mastodon, youtube, flickr, website) |

### `photos` — new columns

| Column              | Type                              | Notes                                                                |
| ------------------- | --------------------------------- | -------------------------------------------------------------------- |
| `short_id`          | `text UNIQUE NOT NULL`            | 8-char base62; used in `/u/<handle>/p/<short-id>`                    |
| `display_key`       | `text`                            | S3 key for display master, e.g. `display/<id>.jpg`                   |
| `original_hash`     | `text`                            | SHA-256 of original; dedup per owner                                 |
| `blurhash`          | `text`                            | LQIP placeholder                                                     |
| `category`          | `text`                            | `dso \| planetary \| lunar \| solar \| wide_field \| nightscape \| other` |
| `scope`             | `text`                            | telescope used, free-text + autocomplete (no EXIF source)            |
| `mount`             | `text`                            | mount used, free-text + autocomplete                                 |
| `filters`           | `text`                            | filters used, free-text + autocomplete                               |
| `guiding`           | `text`                            | guiding setup, free-text + autocomplete                              |
| `featured_at`       | `timestamptz NULL`                | non-null = pinned to hero                                            |
| `featured_position` | `smallint NULL`                   | 1–6, NOT NULL when `featured_at` is set                              |
| `appreciations_count` | `integer NOT NULL DEFAULT 0`    | denormalized counter on existing `appreciations` table; updated transactionally |

The existing `photos.camera` and `photos.lens` columns (populated from
EXIF, user-overridable in upload verify) plus the four new columns above
form the per-photo equipment record. `users.equipment_*` is the
**default loadout** that pre-fills the upload-verify form; it does **not**
drive discovery.

Featured-pin invariant enforced by partial unique index:
```sql
CREATE UNIQUE INDEX photos_featured_per_user_idx
  ON photos (owner_id, featured_position)
  WHERE featured_at IS NOT NULL;
ALTER TABLE photos ADD CONSTRAINT photos_featured_position_range_chk
  CHECK (featured_position IS NULL OR featured_position BETWEEN 1 AND 6);
```

### New tables

```sql
-- Old-handle redirects (90-day reuse cooldown)
CREATE TABLE handle_redirects (
  old_handle citext PRIMARY KEY,
  user_id    uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  released_at timestamptz NOT NULL
);
CREATE INDEX handle_redirects_user_idx ON handle_redirects(user_id);

-- Astronomical targets (Messier, NGC, IC, Caldwell, common, other)
CREATE TABLE targets (
  id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  slug            text UNIQUE NOT NULL,           -- 'm31', 'ngc-7000'
  canonical_name  text NOT NULL,                  -- 'Andromeda Galaxy'
  aliases         text[] NOT NULL DEFAULT '{}',   -- ['M31', 'NGC 224']
  kind            text NOT NULL                   -- 'messier'|'ngc'|'ic'|'caldwell'|'common'|'other'
);
CREATE INDEX targets_aliases_gin_idx ON targets USING gin (aliases);

-- Photo ↔ targets, designed to accept future plate-solve writes
CREATE TABLE photo_targets (
  photo_id   uuid NOT NULL REFERENCES photos(id) ON DELETE CASCADE,
  target_id  uuid NOT NULL REFERENCES targets(id) ON DELETE CASCADE,
  source     text NOT NULL,                       -- 'manual' | 'plate_solve'
  confidence numeric,                             -- NULL when source='manual'
  is_primary boolean NOT NULL DEFAULT false,
  created_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (photo_id, target_id),
  CHECK (source IN ('manual','plate_solve'))
);
CREATE INDEX photo_targets_target_idx ON photo_targets(target_id, photo_id);

-- Free-form user tags
CREATE TABLE tags (
  id   uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  slug text UNIQUE NOT NULL,
  name text NOT NULL
);
CREATE TABLE photo_tags (
  photo_id uuid NOT NULL REFERENCES photos(id) ON DELETE CASCADE,
  tag_id   uuid NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
  PRIMARY KEY (photo_id, tag_id)
);
CREATE INDEX photo_tags_tag_idx ON photo_tags(tag_id, photo_id);

-- (No new likes/appreciations table: `appreciations` already exists from
--  Phase 7 migration 0002. We only add a denormalized counter on photos.)

-- Equipment dictionary, populated by upsert on photo save
CREATE TABLE equipment_items (
  id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  kind            text NOT NULL,                  -- 'telescope'|'camera'|'mount'|'filter'|'guiding'
  canonical_name  text NOT NULL,                  -- normalized lowercase
  display_name    text NOT NULL,                  -- as first seen
  usage_count     integer NOT NULL DEFAULT 0,
  UNIQUE(kind, canonical_name)
);
CREATE INDEX equipment_items_kind_count_idx
  ON equipment_items(kind, usage_count DESC);
```

### Indexes added on `photos` for discovery

```sql
CREATE INDEX photos_published_newest_idx
  ON photos (published_at DESC, id DESC) WHERE published_at IS NOT NULL;
CREATE INDEX photos_published_popular_idx
  ON photos (appreciations_count DESC, published_at DESC, id DESC)
  WHERE published_at IS NOT NULL;
CREATE INDEX photos_category_published_idx
  ON photos (category, published_at DESC, id DESC) WHERE published_at IS NOT NULL;
CREATE INDEX photos_camera_lower_idx
  ON photos (lower(camera)) WHERE published_at IS NOT NULL;
CREATE INDEX photos_scope_lower_idx
  ON photos (lower(scope)) WHERE published_at IS NOT NULL;
CREATE INDEX photos_mount_lower_idx
  ON photos (lower(mount)) WHERE published_at IS NOT NULL;
CREATE INDEX photos_filters_lower_idx
  ON photos (lower(filters)) WHERE published_at IS NOT NULL;
CREATE INDEX photos_guiding_lower_idx
  ON photos (lower(guiding)) WHERE published_at IS NOT NULL;
```

### Existing data preservation

`photos.target` (free-text) is kept as a denormalized "primary target
string." When a `target` matches a known `targets.slug` at save time, a
row is also written into `photo_targets` with `is_primary=true,
source='manual'`.

### Migrations (append-only, numbered; current latest is `0004`)

| # | Migration                              | Concern                                                                              |
| - | -------------------------------------- | ------------------------------------------------------------------------------------ |
| 0005 | `add_handles_and_redirects`         | `users.handle`, reserved list, `handle_redirects`. Backfill placeholder handles for existing users. |
| 0006 | `add_user_tier`                     | `users.tier` + check constraint                                                      |
| 0007 | `add_photo_short_id_and_display`    | `short_id`, `display_key`, `original_hash`, `blurhash`. Backfill `short_id` for existing rows. |
| 0008 | `add_user_profile_fields`           | `tagline`, `bio_html`, `cover_photo_id`, equipment, location, sky, social_links      |
| 0009 | `add_photo_featured_and_category`   | `featured_at`, `featured_position`, `category`, `scope`, `mount`, `filters`, `guiding`, partial unique index, check |
| 0010 | `add_targets_tags_categories`       | `targets`, `photo_targets` (incl. `source` enum), `tags`, `photo_tags` + seed targets (Messier M1–M110, Caldwell C1–C109, popular NGC/IC ~200, common-name aliases) |
| 0011 | `add_appreciations_count`           | `photos.appreciations_count` column + backfill from existing `appreciations` table; transactional update on appreciate/unappreciate |
| 0012 | `add_equipment_items_and_indexes`   | `equipment_items` + discovery indexes (per-column lower() indexes on photos)         |

Existing users at deploy time get auto-generated placeholder handles
(`u-<short-uuid>`); a banner on next login prompts them to pick a real
handle.

---

## Phase 1 — Foundations

### Goal

Ship infrastructure + schema + URL changes without a visible product
shift. After P1 the site looks the same, but: handles work, tiers are
enforced, uploads go through presigned PUT, CloudFront serves
transformed images, the upload-verify step captures targets/tags/
category for later discovery.

### Backend deliverables

| File / module                                | Purpose                                                                       |
| -------------------------------------------- | ----------------------------------------------------------------------------- |
| `backend/migrations/0005..0012_*.sql`        | Schema from above; one concern per migration                                  |
| `backend/src/auth/handle.rs`                 | Handle validation regex, reserved-list check, availability lookup             |
| `backend/src/auth/signup.rs`                 | Add handle field; reject if taken/reserved                                    |
| `backend/src/users/handle.rs`                | Rename + write to `handle_redirects`; 90-day reuse cooldown                   |
| `backend/src/users/profile.rs`               | Bio HTML sanitization via `ammonia` (allowlist below); profile-update endpoint|
| `backend/src/photos/upload.rs`               | Split into: `POST /api/uploads/init`, `POST /api/uploads/:id/finalize`. Old `POST /api/photos` multipart route removed |
| `backend/src/photos/pipeline.rs`             | Extend: display-master derivation, blurhash, magic-byte sniff, hash check     |
| `backend/src/photos/permalink.rs`            | `short_id` base62 generator + collision retry; lookup by `(handle, short_id)` |
| `backend/src/photos/cdn.rs`                  | URL builder: `cdn_url(photo_id, {w,h,fit,q,fm})` — env-driven base            |
| `backend/src/photos/targets.rs`              | Manual target write (P1 scope); plate-solve write deferred                    |
| `backend/src/photos/tags.rs`                 | Tag write + slug normalize                                                    |
| `backend/src/storage/presign.rs`             | Presigned PUT signing with `Content-Length-Range` baked from `users.tier`     |
| `backend/src/storage/cdn_dev.rs`             | `/cdn/img/:photo_id` route for MinIO dev — same params, served by `image` crate |
| `backend/src/jobs/orphan_reaper.rs`          | Periodic task: photos `status='pending'` for >2 h → delete S3 originals + row |
| `backend/src/middleware/handle_redirect.rs`  | 301 from `/u/<old>/...` and `/photo/<uuid>` to current canonical URL          |
| `backend/src/equipment/autocomplete.rs`      | `GET /api/equipment/autocomplete?kind=&q=` (used by upload + profile editor)  |
| `backend/src/equipment/upsert.rs`            | Increment `equipment_items.usage_count` on photo save                         |

### Frontend deliverables

| File / route                                              | Purpose                                                                |
| --------------------------------------------------------- | ---------------------------------------------------------------------- |
| `frontend/src/routes/auth/signup/+page.svelte`            | Add `<HandlePicker>` with debounced availability check                 |
| `frontend/src/routes/upload/+page.svelte`                 | Replace single-file picker with `<UploadDropzone>` + `<UploadFileRow>` × N |
| `frontend/src/routes/upload/[id]/verify/+page.svelte`     | EXIF preflight prefilled; **new fields**: `<TargetPicker>`, `<TagInput>`, `<CategorySelect>`, `<EquipmentAutocomplete>` × 5 |
| `frontend/src/routes/u/[handle]/+page.server.ts`          | Resolve handle → user → fall back to `handle_redirects` (301)          |
| `frontend/src/routes/u/[handle]/p/[shortid]/+page.server.ts` | Photo detail under owner                                            |
| `frontend/src/routes/photo/[id]/+page.server.ts`          | Becomes 301 redirect to canonical URL                                  |
| `frontend/src/lib/upload/presigned.ts`                    | Parallel PUT uploader with `xhr.upload` progress, concurrency 3        |
| `frontend/src/lib/upload/preflight.ts`                    | `exifr` extraction + file SHA-256 + `createImageBitmap` thumb          |
| `frontend/src/lib/cdn.ts`                                 | Mirror of backend URL builder; consumed by `<Img>` component           |
| `frontend/src/lib/components/Img.svelte`                  | Builds `srcset` for `1x/2x/3x` at standard widths; blurhash placeholder while loading |

### Bio sanitizer allowlist (`ammonia`, server-side)

```
Tags:   p, br, strong, em, u, h2, h3, h4, ul, ol, li, blockquote, code, a
Attrs:  a[href, rel] only — rel forced to "nofollow noopener", target stripped
URLs:   http(s):// only; mailto allowed
```

The Tiptap client editor (P2) is configured with the same set; the
server sanitizer is the source of truth for `bio_html`.

### Upload dropzone wireframe (desktop)

```
┌─────────────────────────────────────────────────────────────┐
│  Upload photos                                              │
├─────────────────────────────────────────────────────────────┤
│ ┌─────────────────────────────────────────────────────────┐ │
│ │                                                         │ │
│ │            ↑   Drop photos here, or click               │ │
│ │                                                         │ │
│ │       JPEG · PNG · TIFF · up to 50 MB (free)            │ │
│ │       Subscribers: up to 200 MB                         │ │
│ │                                                         │ │
│ └─────────────────────────────────────────────────────────┘ │
│                                                             │
│  ▾ Files (3)                                                │
│  ┌────┐  m31_2024_v3.tif         12.4 MB  ████████░░  82%  │
│  │ 🖼 │  Camera: ASI2600MC · ISO — · 240s × 30 frames       │
│  └────┘  ✏  Edit details                                    │
│                                                             │
│  ┌────┐  m51_lrgb.jpg              4.2 MB  ██████████ 100% │
│  │ 🖼 │  ✓ Saved as draft · target: M51                     │
│  └────┘  ✏  Continue to caption                             │
│                                                             │
│  ┌────┐  ngc7000_h_alpha.tif      48 MB    ✗ Failed         │
│  │ ⚠  │  Too large for free tier (max 50 MB)                │
│  └────┘  Upgrade · Replace file                             │
└─────────────────────────────────────────────────────────────┘
```

### Component states (named for downstream design)

| Component             | States                                                                |
| --------------------- | --------------------------------------------------------------------- |
| `<UploadDropzone>`    | `idle` · `drag-over` · `disabled (over-quota)` · `error`              |
| `<UploadFileRow>`     | `queued` · `hashing` · `uploading (with %)` · `finalizing` · `ready` · `failed (with reason chip)` |
| `<HandlePicker>`      | `empty` · `checking` · `available` · `taken` · `invalid` · `reserved` |
| `<TargetPicker>`      | `idle` · `searching` · `with-suggestions` · `free-entry` · `selected` |
| `<TagInput>`          | `idle` · `with-suggestions` · `over-limit` (max 8 tags)               |
| `<CategorySelect>`    | `default` · `selected`                                                |
| `<EquipmentAutocomplete>` | `idle` · `with-suggestions` · `free-entry` · `selected`           |

### Copy hooks

- `<UploadDropzone>` empty: "Drop photos here, or click."
- `<UploadDropzone>` over-quota: "You've used your free 50 MB. Upgrade for 200 MB."
- `<UploadFileRow>` failed: dynamic reason chip, e.g. "Too large for free tier (max 50 MB)" / "Unsupported format" / "Upload failed — retry?"
- `<HandlePicker>` available: "Available."
- `<HandlePicker>` taken: "Already taken."
- `<HandlePicker>` invalid: "Use 3–30 lowercase letters, numbers, `-`, or `_`."
- `<HandlePicker>` reserved: "Reserved — please choose another."

### AWS infrastructure (out-of-band)

- S3 bucket `astrophoto-images-<env>` with CORS allowing `PUT` from app
  origin and `x-amz-meta-*` headers.
- CloudFront distribution. Origin: Lambda function URL (Node, sharp).
  Cache key includes full query string. TTL: 30 days.
- Lambda is ported from previous project's `image-transformer/`,
  switched from Lambda@Edge to function URL pattern; reads
  `display/<id>.jpg` from S3.
- IAM: Lambda gets `s3:GetObject` on `display/*` only.
- Backend role: `s3:PutObject`, `s3:GetObject`, `s3:DeleteObject` on
  both prefixes.

### Phase 1 acceptance

- Existing functionality continues to work via 301 redirects.
- New uploads go through presigned PUT, are visible at
  `/u/<handle>/p/<short-id>`.
- A free user is physically prevented from PUT-ing >50 MB; subscriber
  up to 200 MB.
- Display masters generated and served via CDN URL.
- Backend test suite passes (testcontainers Postgres + LocalStack/MinIO
  for S3 surface).
- XSS fuzz cases against ammonia bio sanitizer pass.

---

## Phase 2 — Hero Page

### Goal

Redesign `/u/<handle>` into a polished public profile that doubles as a
portfolio. Adds the profile editor, cover picker, featured-photos pin
mechanism, justified-rows gallery, lightbox, and the appreciate button.

### Page composition

```
<HeroPage viewMode="visitor|owner|admin">
  [owner-mode banner]   — accent-tinted strip, only when viewMode=owner
                          "● VIEWING YOUR OWN PROFILE · OWNER MODE"
                          [ Edit profile ] right-aligned
  <HeroCover>           — full-bleed banner, 480 px desktop / 28 vh mobile
                          renders users.cover_photo_id as background
                          bottom gradient fades to --bg-canvas
                          top-right credit line:
                          "● COVER · <target> · <integration> · <when>"
                          (omitted entirely when empty for visitors)
  <HeroIdentity>        — three-column grid (avatar / name+tagline+socials / actions)
    <HeroAvatar>        — 144×144 SQUARE, marginTop: -80 (overlaps cover),
                          4 px solid --bg-canvas border
    <HeroName>          — Source Serif 4, italic on surname (e.g. "Marie *Dubois*")
    <HeroTagline>       — users.tagline
    <HeroSocialLinks>   — icons row, max 6
    <HeroActions>       — Follow / (owner: Edit profile)
  <HeroAbout>           — bio_html (sanitized server-side); collapses past N lines
  <HeroEquipmentStrip>  — 5-cell strip (scope/camera/mount/filters/guiding);
                          cells with no value are hidden, not stubbed
  <HeroLocationBadge>   — "Lyon · Bortle 6 · SQM 19.8" mono row
  <HeroStatsRow>        — 5 inline stats:
                          published frames · total integration · followers
                          · appreciations (in --accent) · targets shot
  <FeaturedRow>         — exactly 6 portrait tiles, 3:4 aspect, with
                          rank badge top-left, target/appreciations label
                          on gradient-bottom overlay
                          owner-mode: empty slots render placeholder tiles
                          "SLOT 01..06" mono labels, the first carries
                          "+ Pin a photo" in --accent
  <GalleryToolbar>      — sort selector + (later P3) filter pills
  <PhotoGrid layout="justified-rows">
                        — Flickr's justified-layout package
                        — target row height 220 px desktop / 140 px mobile
                        — gap 8 px
    <PhotoTile>         — blurhash placeholder + lazy <Img>; on hover
                          shows caption + appreciations pill in overlay
  <Lightbox>            — route-mounted at /u/<handle>/p/<short-id>
                          two-column: full-bleed black image left,
                          380 px panel right
```

### Desktop wireframe

```
┌─────────────────────────────────────────────────────────────┐
│ [navbar]                                                    │
├─────────────────────────────────────────────────────────────┤
│ ░░░░░░░░░░░░░░░░░░░░ Cover (40vh) ░░░░░░░░░░░░░░░░░░░░░░░  │
│ ░░░░░░░░░░░░░░░░░░░░ NGC 7000 wide-field ░░░░░░░░░░░░░░░░  │
│                                            ┌─────────────┐  │
│                                            │ Change cover│ ← owner only
│                                            └─────────────┘  │
├─────────────────────────────────────────────────────────────┤
│  ╭──────╮                                  ┌─────────────┐  │
│  │  👤  │   Marie Dubois                   │   Follow    │  │
│  │ avatar│   Hunting deep-sky from a       └─────────────┘  │
│  ╰──────╯   Bortle 6 backyard                               │
│              𝕏  IG  AB  🌐                                   │
├─────────────────────────────────────────────────────────────┤
│  About               Equipment              Location & Sky │
│  ──────              ─────────               ──────────────│
│  3-paragraph bio    Scope: RedCat 51         📍 Lyon, FR   │
│  rendered from      Camera: ASI2600MC        🌙 Bortle 6   │
│  sanitized HTML     Mount: ZWO AM5           ✨ SQM 19.8   │
│                     Filters: L-Pro                         │
│                     Guiding: ASI120MM                       │
├─────────────────────────────────────────────────────────────┤
│  📊 412 frames · 86h integration · 1.2k followers ·         │
│      284 appreciations · 47 targets shot                    │
│      Member since Mar 2024                                   │
├─────────────────────────────────────────────────────────────┤
│  Featured  (6 portrait tiles, 3:4, rank badge top-left)     │
│  ┌──────┐┌──────┐┌──────┐┌──────┐┌──────┐┌──────┐           │
│  │ #01  ││ #02  ││ #03  ││ #04  ││ #05  ││ #06  │           │
│  │ tile ││ tile ││ tile ││ tile ││ tile ││ tile │           │
│  │      ││      ││      ││      ││      ││      │           │
│  └──────┘└──────┘└──────┘└──────┘└──────┘└──────┘           │
│  owner-mode placeholders: SLOT 01..06 mono labels;          │
│  first slot carries [+ Pin a photo] in --accent             │
├─────────────────────────────────────────────────────────────┤
│  Frames                                ▾ Newest             │
│  justified rows · row height 220 px desktop / 140 px mobile │
│  · gap 8 px · last row left-aligned                         │
│  ┌────┐┌──────────┐┌──┐┌──────────┐                         │
│  │ 3:2││   16:9   ││1:1│   16:9   │                         │
│  └────┘└──────────┘└──┘└──────────┘                         │
│  ┌──────────┐┌────┐┌────────────────┐                       │
│  │   16:9   ││ 3:2││   21:9 panorama│                       │
│  └──────────┘└────┘└────────────────┘                       │
│  ↻ load more (cursor) / infinite scroll                     │
└─────────────────────────────────────────────────────────────┘
```

### Mobile breakpoint (≤640 px)

- Cover height collapses to 28 vh.
- Identity stacks vertically below cover (no overlap).
- About / Equipment / Location stack into a single column.
- Stats wrap to two lines.
- Featured row becomes 2-column grid (3 rows × 2 cols max).
- Justified-rows algorithm receives the new container width and
  reflows.
- Lightbox EXIF panel hides behind a swipe-up sheet.

### Component states

| Component                | States                                                                 |
| ------------------------ | ---------------------------------------------------------------------- |
| `<HeroCover>`            | `with-cover` · `empty (owner-prompt)` · `empty (visitor-blank)` · `editing` |
| `<HeroAvatar>`           | `with-avatar` · `initials-fallback` · `editing`                        |
| `<HeroBio>`              | `present` · `present-clamped (>N lines)` · `empty (owner-prompt)` · `empty (visitor-hidden)` |
| `<HeroEquipmentStrip>`   | `full` · `partial (some cells hidden)` · `empty-hidden` · `editing-inline` |
| `<HeroLocationBadge>`    | `full` · `location-only` · `empty-hidden`                              |
| `<FeaturedRow>`          | `full (6)` · `partial (1–5)` · `empty (owner-prompt)` · `empty (visitor-hidden)` · `reordering` |
| `<PhotoTile>`            | `idle` · `hover (caption + ❤ appreciate pill)` · `appreciating-optimistic` · `unavailable (status≠ready)` |
| `<Lightbox>`             | `loading` · `ready` · `panel-collapsed` · `panel-expanded`             |

### Owner-only empty-state copy hooks

Tone: warm, single sentence. Visitor side **never** sees these prompts;
the section is hidden when empty for visitors.

| Empty slot                    | Owner copy                                  |
| ----------------------------- | ------------------------------------------- |
| `<HeroCover>` empty           | "Pick a cover from your gallery →"          |
| `<HeroTagline>` empty         | "Add a tagline"                             |
| `<HeroBio>` empty             | "Tell visitors about your astrophotography" |
| `<HeroEquipmentStrip>` empty  | "Add the gear behind your shots"            |
| `<HeroLocationBadge>` empty   | "Where do you observe from?"                |
| `<FeaturedRow>` empty         | "Pin 3–6 of your best photos"               |

### Profile editor (modal/drawer)

Triggered by `[ Edit profile ]` (owner only). Sectioned, save-on-blur
per section:

```
┌─ Edit profile ─────────────────────────┐
│  Identity                              │
│   • Avatar [ upload ]                  │
│   • Display name [______________]      │
│   • Tagline [_______________________]  │
│                                        │
│  About                                 │
│   ┌──── rich text editor ──────────┐   │
│   │ B  I  U  H₂ H₃ ⌃ • 1. " <> 🔗  │   │
│   │                                │   │
│   │ Lorem ipsum…                   │   │
│   └────────────────────────────────┘   │
│                                        │
│  Equipment                             │
│   Scope    [______________ ▾ suggest]  │
│   Camera   [______________ ▾ suggest]  │
│   Mount    [______________ ▾ suggest]  │
│   Filters  [______________ ▾ suggest]  │
│   Guiding  [______________ ▾ suggest]  │
│                                        │
│  Location & sky                        │
│   City/region [_______________]        │
│   Bortle (ladder, 9-cell segmented)    │
│   [ 1 ][ 2 ][ 3 ][ 4 ][●5 ][ 6 ][ 7 ][ 8 ][ 9 ]│
│   each cell uppercase mono label;       │
│   selected: --accent bg, --accent-ink fg│
│   SQM (opt)   [_____.__]               │
│                                        │
│  Social links                          │
│   [+ Add link]                         │
│   ▢ Twitter  [https://...] [×]         │
│   ▢ Instagram[https://...] [×]         │
└────────────────────────────────────────┘
```

Editor lib: **Tiptap** (Svelte bindings) configured with only the marks
and nodes that match the ammonia allowlist. Pasted HTML from external
sources is sanitized server-side regardless of client state.

### Cover picker

Modal: grid of the user's published photos; click to select; preview
with crop indicator (16:9 area used as banner). Persists
`users.cover_photo_id`.

### Featured-photo controls (owner only)

- `<FeaturedRow>` shows pinned photos plus a `[+ Pin a photo]`
  placeholder tile if `count < 6`.
- Click placeholder → photo picker (same component as cover picker,
  multiselect-disabled, filters out already-pinned).
- Each pinned tile, on hover: `↕` drag handle + `✕` unpin button.
- Drag-and-drop reorder (Svelte 5 + `@neodrag/svelte`); persists on
  drop.
- Server enforces `featured_position` 1–6 + partial unique index per
  user.

### Lightbox

- Mounts as a route, not just a modal: `/u/<handle>/p/<short-id>`
  deeplinks open the lightbox over the gallery via SvelteKit overlay
  routing.
- Direct visit (no referrer) renders a full photo-detail page, not a
  modal.
- Keyboard: `←` / `→` prev/next, `Esc` close, `i` toggle EXIF panel,
  `a` appreciate.
- EXIF panel sticky right, 320 px on desktop; bottom sheet on mobile.
- Right panel order: title/target → caption → appreciate button →
  EXIF table → equipment used → "More from <photographer>" thumbnail
  strip.
- Image rendered at CDN URL with `?w=<viewportPx>&q=85&fm=auto`,
  blurhash placeholder while loading.

### Appreciations

The user-facing verb is **"appreciate"** (the AppreciateButton terminology
inherited from Phase 7 / 8). Endpoints already exist:

- `POST /api/photos/:id/appreciate` (auth required) — inserts a row in
  the existing `appreciations` table; in the same transaction, increments
  the **new** `photos.appreciations_count` column.
- `DELETE /api/photos/:id/appreciate` — symmetric.
- Anonymous click → redirect to login with return URL.
- UI: heart icon with count; optimistic toggle; 300 ms debounce against
  rapid double-clicks.
- Lightbox keyboard shortcut: `a` (appreciate) — replaces the `l` (like)
  binding referenced in earlier drafts.

### Phase 2 acceptance

- Owner can fully personalize their `/u/<handle>` page: cover, avatar,
  name, tagline, bio, equipment, location, social, featured photos.
- Visitor lands on a page that visually communicates the photographer's
  work; never sees owner-only prompts.
- Justified-rows gallery handles mixed aspect ratios end-to-end.
- Lightbox deep-links and back-button work both ways.
- Appreciate toggle works; `photos.appreciations_count` stays consistent
  under concurrent insert/delete.
- All editor inputs sanitized server-side; XSS test cases pass.

---

## Phase 3 — Discovery

### Goal

Surface the corpus across photographers. Adds the global Explore page,
target / tag / equipment / category pages, and search. Reuses
`<PhotoGrid>`, `<PhotoTile>`, `<Lightbox>` from P2.

> One small adjustment to earlier phases: the **inputs** that feed
> discovery (target picker, tag input, category select on the upload
> verify step) ship in **P1** alongside the schema. P3 reads what P1
> captures.

### Routes

| Route                       | Purpose                                             |
| --------------------------- | --------------------------------------------------- |
| `/explore`                  | Global feed — newest published across all users     |
| `/t/<slug>`                 | Target page — every photo of `m31`, `ngc-7000`, etc. |
| `/tag/<slug>`               | User-tag page — free-form tag aggregator            |
| `/equip/<kind>/<slug>`      | Equipment page; `kind ∈ telescope|camera|mount|filter|guiding` |
| `/c/<category>`             | Category page; `dso \| planetary \| lunar \| solar \| wide_field \| nightscape \| other` |
| `/search?q=`                | Combined results (targets + photographers + photos) |

### Component composition

```
<ExplorePage>
<TargetPage>
<TagPage>
<EquipmentPage>
<CategoryPage>
<SearchPage>

<DiscoveryHeader variant="...">  — single component, variant per page kind
<GalleryToolbar>                 — sort selector + (page-specific) filter pills
<PhotoGrid layout="justified-rows" mode="cross-author">
  <PhotoTile>                    — same as P2 but renders <AuthorChip>
<SearchBar>                      — global; sits in navbar
```

### Explore wireframe (desktop)

```
┌─────────────────────────────────────────────────────────────┐
│ [navbar  · 🔍 search…                                  ]    │
├─────────────────────────────────────────────────────────────┤
│  Explore                                                    │
│                                                             │
│  ▾ Newest  │  ▾ Past week        ← two groups split by      │
│            │                       hairline divider          │
│  [DSO] [Planetary] [Lunar] [Solar] [Wide] [Nightscape]      │
│  [✓ Following only]                       [✕ Clear filters] │
│                                                             │
│  ┌──────┐ ┌──────────┐ ┌────┐ ┌──────────┐                 │
│  │ 3:2  │ │   16:9   │ │1:1 │ │   16:9   │                 │
│  │@sue  │ │ @marie   │ │@tom│ │ @ada     │  justified rows │
│  └──────┘ └──────────┘ └────┘ └──────────┘                 │
│  …infinite scroll, cursor-paginated                         │
└─────────────────────────────────────────────────────────────┘
```

### Target page wireframe

```
┌─────────────────────────────────────────────────────────────┐
│ Target                                                      │
│ ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━           │
│ M31  ·  Andromeda Galaxy                                    │
│ Also known as: NGC 224                                      │
│ 🌌 Galaxy   ·   482 photos   ·   213 contributors           │
│                                                             │
│ ▾ Newest                  ▾ Most liked                      │
│ ┌──────┐┌──────────┐┌────┐...  justified rows               │
└─────────────────────────────────────────────────────────────┘
```

### Equipment page wireframe

```
┌─────────────────────────────────────────────────────────────┐
│ Camera   ·   ZWO ASI2600MC Pro                              │
│ 138 photos                                                  │
│                                                             │
│ ▾ Newest                                                    │
│ ┌──────┐┌──────────┐...                                     │
│                                                             │
│ ─────────────────────────────────────────                   │
│ OFTEN PAIRED WITH                                           │
│ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐         │
│ │ Scope    │ │ Mount    │ │ Filter   │ │ Guiding  │         │
│ │ RedCat51 │ │ ZWO AM5  │ │ L-eXtreme│ │ ASI120MM │         │
│ │ 84 shared│ │ 71 shared│ │ 52 shared│ │ 38 shared│         │
│ └──────────┘ └──────────┘ └──────────┘ └──────────┘         │
└─────────────────────────────────────────────────────────────┘
```

Query for the rail: top N other `equipment_items` with the highest
co-occurrence on photos that also use this item. Cap N=4 across all
kinds (one per kind preferred when available).

### Search results wireframe

```
┌─────────────────────────────────────────────────────────────┐
│ Search · "andromeda"                                        │
├─────────────────────────────────────────────────────────────┤
│  Targets                                                    │
│   M31 — Andromeda Galaxy           482 photos               │
│   M32                                87 photos              │
│   NGC 891                            14 photos              │
│                                                             │
│  Photographers                                              │
│   @andromeda_aficionado    avatar                           │
│   @nightsky_andy           avatar                           │
│                                                             │
│  Photos (124)                                               │
│   ┌──────┐┌──────────┐...                                   │
└─────────────────────────────────────────────────────────────┘
```

### Backend endpoints

| Endpoint                                          | Notes                                                          |
| ------------------------------------------------- | -------------------------------------------------------------- |
| `GET /api/explore`                                | Cursor-paginated; `sort=newest\|most-appreciated`, `since=24h\|7d\|30d\|all`, optional `category`, `following=true` |
| `GET /api/targets/:slug`                          | Target metadata + photos page                                  |
| `GET /api/tags/:slug`                             | Tag metadata + photos page                                     |
| `GET /api/equipment/:kind/:slug`                  | Equipment record + photos page                                 |
| `GET /api/categories/:cat`                        | Photos in category                                             |
| `GET /api/search?q=`                              | Returns `{targets[], users[], photos[]}` — each capped (5/5/24)|
| `GET /api/targets/autocomplete?q=`                | Upload verify target picker; also search bar                   |
| `GET /api/tags/autocomplete?q=`                   | Caption tag input                                              |
| `GET /api/equipment/autocomplete?kind=&q=`        | Profile editor + upload verify                                 |

### Sort & cursor pagination

- `newest` cursor: `(published_at DESC, id DESC)`. Where:
  `(published_at, id) < ($t, $id)`.
- `most-appreciated` cursor:
  `(appreciations_count DESC, published_at DESC, id DESC)`. Three-tuple
  comparison.
- Required indexes are added in P1 migration `0012`.

### Search v1 (cheap)

- ILIKE across `users.handle`, `users.display_name`,
  `targets.canonical_name`, `targets.aliases`, `tags.name`,
  `photos.target`, `photos.caption`.
- Three small queries (one per group), unioned in handler.
- Cap each group; total round-trip stays under ~30 ms with the indexes
  added in P1.
- v2 (future, **not in P3**): Postgres `tsvector` materialized column +
  GIN.

### Discovery filter pills (per page)

| Page           | Pills                                                       |
| -------------- | ----------------------------------------------------------- |
| `/explore`     | Time (24h/7d/30d/all), category chips, following-only       |
| `/t/<slug>`    | Category chips                                              |
| `/tag/<slug>`  | Category chips                                              |
| `/equip/<...>` | Category chips                                              |
| `/c/<cat>`     | Time window                                                 |

### Component states

| Component                     | States                                                        |
| ----------------------------- | ------------------------------------------------------------- |
| `<DiscoveryHeader>`           | `loaded` · `loading` · `not-found`                            |
| `<GalleryToolbar>`            | `default` · `applied (filter chips)` · `empty-results`        |
| `<PhotoGrid>` (cross-author)  | `loading` · `loaded` · `empty (with copy hook)` · `error` · `loading-more` |
| `<SearchBar>`                 | `idle` · `focused` · `typing` · `with-suggestions` · `no-results` |
| `<CrossAuthorTile>`           | `idle` · `hover (caption + @handle chip + appreciate pill)` · `unavailable` |

`<SearchBar>` lives in the navbar with a **⌘K** hotkey (per the
existing `<AppHeader>` design). Pressing ⌘K from any page opens it
focused; Esc closes; Enter on a suggestion navigates.

### Empty-state copy hooks

| Page             | Empty copy                                                   |
| ---------------- | ------------------------------------------------------------ |
| `/explore`       | "No photos yet — be the first to upload."                    |
| `/t/m31`         | "No M31 photos yet. Upload yours to start the page."         |
| `/tag/<slug>`    | "Nothing tagged '<name>' yet."                               |
| `/equip/<...>`   | "No photos with this equipment yet."                         |
| `/c/<cat>`       | "No photos in this category yet."                            |
| `/search?q=`     | "Nothing matched '<q>' — try a target name or photographer handle." |

### Equipment slug normalization

- Free-text `users.equipment_camera = "ZWO ASI2600MC Pro"` → on save,
  upsert `equipment_items(kind='camera',
  canonical_name='zwo-asi2600mc-pro', display_name='ZWO ASI2600MC
  Pro')`, increment `usage_count`.
- Browse URL: `/equip/camera/zwo-asi2600mc-pro`.
- Photos query: join via `LOWER(photos.equipment_camera) =
  equipment_items.canonical_name`.

### Phase 3 acceptance

- Visiting `/explore`, `/t/m31`, `/tag/widefield`,
  `/equip/camera/asi2600mc`, `/c/dso` returns the expected photo grids
  in <300 ms p95.
- Search across handles, targets, tags, photos returns combined
  results.
- Autocomplete endpoints power upload + profile editing.
- Cursor pagination correct under concurrent inserts (no skipped or
  duplicate rows across pages).
- **Plate-solve readiness**: nothing in P3 schema or queries assumes
  `photo_targets.source = 'manual'`. When the future astrometry phase
  writes `'plate_solve'` rows, target pages just include those photos.

---

## Components to build (Svelte)

Non-exhaustive checklist matching the showcase artboards. Each entry
maps 1:1 to a JSX component in `showcase-p1.jsx`, `showcase-p2.jsx`,
`showcase-p3.jsx`, or `showcase-cross.jsx`.

**Phase 1**

- `<UploadDropzone>` · `<UploadFileRow>` · `<UploadProgress>`
- `<HandlePicker>` (debounce 300 ms)
- `<TargetPicker>` · `<TagInput>` (cap 8 tags) · `<CategorySegmented>`
- `<EquipmentAutocomplete>`
- `<TierUpgradeModal>` (pre-flight when `file.size > tier_max`)

**Phase 2**

- `<HeroCover>` · `<HeroIdentity>` · `<HeroAvatar>` · `<HeroName>`
- `<HeroAbout>` · `<HeroEquipmentStrip>` · `<HeroLocationBadge>`
- `<HeroStatsRow>` · `<FeaturedRow>` (drag-reorder via `@neodrag/svelte`)
- `<PhotoGrid>` (justified-rows wrapper; `mode="single-author"` here)
- `<PhotoTile>` · `<Lightbox>` (route-mounted overlay)
- `<ProfileEditor>` (drawer) · Tiptap binding · `<BortleLadder>`
  (9-cell segmented control) · `<SocialLinksEditor>`

**Phase 3**

- `<DiscoveryHeader>` (variants: explore / target / equipment /
  category / tag)
- `<GalleryToolbar>` (sort + time + chips)
- `<PhotoGrid mode="cross-author">` · `<CrossAuthorTile>` ·
  `<AuthorChip>`
- `<SearchBar>` (⌘K) · `<SearchSuggestions>` · `<SearchResultsPage>`
- `<EmptyState>` · `<ErrorChip>`

## Cross-cutting

### Error model

Extend `AppError` (in `backend/src/error.rs`) with:

- `RateLimited` — 429
- `QuotaExceeded` — for tier-gated upload size pre-flight; 413
- `Conflict` — handle taken / dup hash; 409
- `PayloadTooLarge` — file exceeds tier limit at S3; 413
- `MagicByteMismatch` — declared MIME doesn't match sniffed type; 400
- `PendingFinalizeStuck` — finalize never followed init; 408
- `UnsupportedFormat` — FITS / RAW / unsupported; 400

Each variant renders via `<ErrorChip>` in the showcase artboard
`ScreenErrorStates` — coloured chip (warning / danger / muted),
italic single-sentence message, one or more recoverable actions
with the primary action accent-coloured. `IntoResponse` returns a
non-leaky body.

### Observability

- `tracing` spans tagged with `photo_id`, `user_id`, `phase`
  (`init`, `put`, `finalize`, `render`).
- CloudFront access logs to S3 → Athena for slow-query analysis.
- Lambda function URL emits structured JSON logs to CloudWatch with
  `photo_id`, transform params, and timing.
- Backend prometheus counters: `uploads_total{result}`,
  `display_master_seconds`, `presign_total`.

### Testing

- Backend: `testcontainers` for Postgres + LocalStack for S3.
- Specific test surfaces:
  - presigned-PUT happy path + tier-cap rejection at S3
  - finalize idempotency (call twice → single state transition)
  - magic-byte sniff rejects mismatched MIME
  - orphan reaper sweeps stale `pending` rows
  - bio sanitizer fuzz: XSS payloads (script, on*, javascript:, data:)
  - cursor pagination correctness under concurrent inserts
  - handle change → old URL 301 → new URL
- Frontend: Playwright E2E for upload flow (multi-file), hero-page
  edit flow, lightbox keyboard nav, signup-with-handle.

### Accessibility

- Lightbox: keyboard map, focus trap, `aria-modal="true"`, restore
  focus on close.
- Cover image: `alt` defaults to caption of the cover photo.
- Gallery tiles: `aria-label="<target or filename> by @<handle>"`.
- Color contrast checked at AA on all owner-prompt copy and on
  `--fg-secondary` over `--bg-accent-tint`.
- `<BortleLadder>`: each of the 9 cells carries a visible label and an
  `aria-label` describing the class (e.g. "Class 6 — Bright Suburban").
- Owner-mode banner is announced via `aria-live="polite"` on first
  render.
- ⌘K opens `<SearchBar>` from anywhere; the search input has
  `aria-label="Search the archive"` and announces result count via
  `aria-live`.

### SSR / SEO

- The hero page (`/u/<handle>`), photo detail (`/u/<handle>/p/<short>`),
  target page (`/t/<slug>`), and equipment / tag / category pages render
  full content at first paint without JS — they are the primary SEO
  surface. `+page.server.ts` for each, no client-only logic for the
  initial render.
- `<meta property="og:image">` uses a CDN URL like
  `cdn.astrophoto.pics/img/<photo-id>?w=1200&fm=jpg&q=85`. Open Graph
  card template renders photo + Astrophoto wordmark + uploader handle.
- Theme cookie (`theme=dark|light`) read in the layout's `load` so the
  initial HTML carries `<html data-theme="...">` — no FOUC.
- The `theme` and `density` cookies are inherited from earlier phases;
  this spec does not change their semantics.

### Migration & rollout

- All migrations are append-only and numbered. None touch shipped
  files.
- Existing users at deploy time get auto-generated placeholder handles
  (`u-<short-uuid>`) via `0005`. A banner on next login prompts them to
  pick a real handle. A grace endpoint `/account/handle/setup` accepts
  the choice.
- Old `/photo/<uuid>` URLs 301 to `/u/<handle>/p/<short-id>` via
  middleware. Backfill `short_id` for existing rows in `0007`.
- Photo storage migration from R2 to S3: a one-shot job copies existing
  originals from R2 into S3, generates display masters, populates
  `display_key`. Old R2 bucket is decommissioned only after the job
  completes and CDN serves real traffic from S3.

### Documentation updates

- `CLAUDE.md` — update the "Storage" line from "Cloudflare R2" to
  "AWS S3 + CloudFront". Add a section on the display-master /
  CDN-URL convention.
- `README.md` — add `/explore`, `/t/<slug>`, `/u/<handle>` to the route
  list.

## References

### Spec & inheritance

- This spec — `docs/superpowers/specs/2026-05-03-photographer-showcase-design.md`
- Phase 7 engagement (appreciations, comments, follows) —
  `docs/superpowers/specs/2026-05-02-phase-7-engagement-design.md`
- Phase 8a (security/account) —
  `docs/superpowers/specs/2026-05-02-phase-8a-security-account-design.md`
- Phase 8b (drafts, replace, polish) —
  `docs/superpowers/specs/2026-05-02-phase-8b-photos-polish-design.md`

### Design handoff (canonical for layout / hierarchy / copy)

Located in `/Users/pleclech/Downloads/design_handoff_astrophoto 3/`:

- `README.md` — main brand/system handoff (Phases 1–7 baseline)
- `README - Phase 8.md` — Phase 8 addendum (settings, password reset,
  2FA, deletion, drafts, replace, polish)
- `showcase/README.md` — **the design pass for this spec** (P1/P2/P3)
- `showcase/styles.css` — design tokens (single source of truth)
- `showcase/shared.jsx` — `<AppHeader>`, `<AppFooter>`, `<Photo>` placeholder
- `showcase/showcase-p1.jsx` — Upload, HandlePicker, verify-step pickers
- `showcase/showcase-p2.jsx` — Hero page (visitor + owner), profile editor, lightbox
- `showcase/showcase-p3.jsx` — Explore, target, equipment, search, empty
- `showcase/showcase-cross.jsx` — TierUpgrade, ErrorStates, PlateSolveNote, RouteMap

### External libraries

- Old project's Lambda image transformer (port + adapt to function-URL
  pattern): `/Volumes/Pascal4Tb/Projects/claude/astrophoto/dev`
- AWS Lambda + sharp pattern: AWS Solutions Library "Serverless Image Handler"
- ammonia HTML sanitizer — https://github.com/rust-ammonia/ammonia
- Tiptap editor — https://tiptap.dev/
- justified-layout algorithm — https://github.com/flickr/justified-layout
- `@neodrag/svelte` — drag-reorder for `<FeaturedRow>`
