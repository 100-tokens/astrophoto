# Handoff: Astrophoto Showcase — Upload, Hero Page, Discovery

## Overview

This handoff covers three product surfaces for **astrophoto.pics**, an
astrophotography-sharing site:

1. **Phase 1 — Foundations**: a polished multi-file upload wizard
   (presigned PUT direct to S3) plus required `@handle` signup and the
   verify step that captures target / tags / category / equipment for
   discovery.
2. **Phase 2 — Hero Page**: `/u/<handle>` rebuilt as a public profile
   + portfolio (cover, identity, bio, equipment, location, featured
   pinned photos, justified-rows gallery, lightbox).
3. **Phase 3 — Discovery**: `/explore`, `/t/<slug>`,
   `/equip/<kind>/<slug>`, `/search`, navbar autocomplete, empty
   states.

The product spec these designs implement is the document
**`docs/superpowers/specs/2026-05-03-photographer-showcase-design.md`**
in the backend repo. Where the spec and the design disagree, the **spec
is canonical** for behavior, schema, URLs, and security; the design
files are canonical for layout, hierarchy, and copy.

## About the design files

Everything inside `showcase/` is a **design reference** rendered in
HTML/JSX. It is not production code to ship as-is. The task is to
**recreate the screens in the target codebase's existing environment**
(SvelteKit on the frontend, Rust/Axum on the backend, Postgres,
S3+CloudFront on the edge) using the codebase's established patterns
and libraries.

Open `Astrophoto Showcase.html` in a browser to navigate the design
canvas. Each artboard inside corresponds to one screen, modal, or
component card.

## Fidelity

**High-fidelity.** Final colors, typography, spacing, and copy. The
developer should reproduce these screens pixel-close using the existing
Svelte component library, the design tokens in `styles.css`, and the
copy verbatim. Photo placeholders in the canvas (the `PHOTOS` fixture)
are stand-ins; real photos come from S3/CloudFront in production.

## Source files in this bundle

| File                                       | Contents                                                                     |
| ------------------------------------------ | ---------------------------------------------------------------------------- |
| `showcase/Astrophoto Showcase.html`        | Canvas entry point — wires every artboard into one document                  |
| `showcase/showcase-p1.jsx`                 | Phase 1: `<UploadDropzone>`, `<UploadFileRow>`, upload verify, `<HandlePicker>` |
| `showcase/showcase-p2.jsx`                 | Phase 2: hero page (visitor + owner), profile editor, lightbox               |
| `showcase/showcase-p3.jsx`                 | Phase 3: explore, target page, equipment page, search, autocomplete, empty   |
| `showcase/showcase-cross.jsx`              | Cross-cutting: tier upgrade, plate-solve note, error states, URL map         |
| `styles.css` (project root)                | Design tokens + utilities — single source of truth                           |
| `shared.jsx` (project root)                | `<AppHeader>`, `<Photo>`, `PHOTOS` fixture, `<AstroMarks>`                   |

## Design tokens

All tokens are defined in `styles.css` as CSS custom properties. **Do
not invent new tokens** — pull from this list:

### Color

| Token                      | Use                                            |
| -------------------------- | ---------------------------------------------- |
| `--bg-canvas`              | Page background                                |
| `--bg-base`                | Section / panel background                     |
| `--bg-raised`              | Card background                                |
| `--bg-elevated`            | Dropdown / popover background                  |
| `--bg-overlay`             | Sticky bar (with `backdrop-filter: blur(12px)`)|
| `--bg-accent-tint`         | Tinted-accent surface (selected, focus, hint)  |
| `--bg-warning-tint`        | Warning callout surface                        |
| `--bg-danger-tint`         | Destructive callout surface                    |
| `--fg-primary`             | Primary text                                   |
| `--fg-secondary`           | Body / secondary text                          |
| `--fg-muted`               | Captions, meta text                            |
| `--fg-faint`               | Disabled / placeholder                         |
| `--accent`                 | Primary brand accent                           |
| `--accent-dim`             | Accent at lower emphasis (borders, hovers)     |
| `--accent-ink`             | Foreground over `--accent` (e.g. avatar text)  |
| `--warning`                | Drafts, quotas, soft warnings                  |
| `--danger`                 | Destructive actions, hard errors               |
| `--border-default`         | Default 1px border                             |
| `--border-subtle`          | Hairline border / dashed dividers              |

### Typography

- `--font-display` — Source Serif 4, used italic for headings, names,
  and emphasized inline words. **Italic is the brand signature** —
  every page-level title has at least one italic word.
- `--font-body` — Inter 400/500/600/700, body and form text.
- `--font-mono` — JetBrains Mono, all eyebrows, meta strings, labels,
  EXIF tables, code-shaped UI.

Type roles already in `styles.css`:

- `.t-eyebrow` — uppercase mono, 11px, letter-spacing 0.12em.
  Optional leading `●` glyph for accent.
- `.t-label` — uppercase mono, 11px, used as form labels.
- `.t-meta` — uppercase mono, 10–11px, captions/timestamps.
- `.t-mono` — inline mono for handles, slugs, code identifiers.

### Spacing & rhythm

- Page horizontal padding: **64px** desktop, **20px** mobile.
- Section vertical padding: 32–48px between sections, separated by
  `1px solid var(--border-subtle)`.
- Inner card padding: 24px.
- Grid gaps: 8px (gallery tiles), 12–16px (cards), 32–48px (column
  layouts).

### Elevation

- `--shadow-lg` for floating panels (autocomplete dropdowns,
  drawers).
- Almost everything else is flat with hairline borders — avoid
  invented shadows.

### Radii

The design is **flat-cornered intentionally**. Most surfaces are 0
radius; chips use sharp corners too (2px / `var(--r-sm)`), and avatars
are square not circular. Do not introduce rounding.
_(reconciled 2026-05-30: sharp wins, per original handoff + shipped code)_

## Phase 1 — Foundations

### `<UploadDropzone>` and `<UploadFileRow>`

Open `showcase-p1.jsx` → `ScreenUploadDropzone`. Implementation notes:

- **Three-step wizard preserved.** Step 01 is the dropzone +
  per-file queue. Steps 02 and 03 (verify, publish) are reached
  per-file once each row reaches `ready`.
- **States** must match the spec exactly:
  - `<UploadDropzone>`: `idle`, `drag-over`, `disabled (over-quota)`,
    `error`.
  - `<UploadFileRow>`: `queued`, `hashing`, `uploading (with %)`,
    `finalizing`, `ready`, `failed (with reason chip)`.
- **Parallel PUT, concurrency 3.** Use `XMLHttpRequest` (not `fetch`)
  for accurate `xhr.upload.onprogress`. The presigned URL is signed
  with `Content-Length-Range: 0,<tier_max>` server-side; if the user
  exceeds it, S3 returns 400 — surface that as the over-quota row state
  with copy `"Too large for free tier (max 50 MB)"` and an inline
  Upgrade CTA.
- **Client preflight, in this order, per file:**
  1. `createImageBitmap` for instant thumbnail (so the row can show
     the image immediately).
  2. `exifr` extraction → cache for the verify step.
  3. SHA-256 over the file (Web Crypto `subtle.digest`) for dedup.
- **POST `/api/uploads/init`** returns `[{photo_id, short_id,
  presigned_put_url}]`. **POST `/api/uploads/<id>/finalize`** is
  idempotent; the client must call it after each successful PUT.
- **Tier copy** lives in the dropzone subtitle:
  `JPEG · PNG · TIFF · up to 50 MB (free) · Subscribers up to 200 MB`.

### `<UploadVerify>` step

Open `ScreenUploadVerify`. New fields beyond the existing verify form:

- `<TargetPicker>` — autocomplete-fed from `/api/targets/autocomplete`.
  Matches against `targets.canonical_name` and `targets.aliases`. Free
  entry allowed; on save, if the typed string matches a known slug, a
  `photo_targets` row is written with `is_primary=true,
  source='manual'`.
- `<TagInput>` — tokenized, comma/Enter to commit. Cap **8 tags**.
  Slug-normalize on save; upsert into `tags`.
- `<CategorySelect>` — segmented control with seven options:
  `dso · planetary · lunar · solar · wide_field · nightscape · other`.
- `<EquipmentAutocomplete>` × 5 (scope, camera, mount, filters,
  guiding) — pre-filled from `users.equipment_*`. On save, upsert into
  `equipment_items` with `kind`, `canonical_name` (lowercased),
  `display_name`, increment `usage_count`.

### `<HandlePicker>`

Open `ScreenHandlePicker`. Required at signup. Validation regex:
`/^[a-z0-9_-]{3,30}$/`. States visible in artboard: `empty`,
`checking`, `available`, `taken`, `invalid`, `reserved`. Debounce
the availability lookup at **300ms** of idle typing.

Existing accounts that pre-date this work get auto-generated handles
(`u-<short-uuid>`); on next sign-in show a banner that links to
`/account/handle/setup` with the same component embedded.

## Phase 2 — Hero page

### Page composition (visitor view)

Open `ScreenHeroPage`. Top-down:

1. **`<HeroCover>`** — full-bleed banner, 480px tall on desktop, 28vh
   on mobile. Renders `users.cover_photo_id` as background. Bottom
   gradient fades to `--bg-canvas`. Top-right corner shows a small mono
   credit line (`● COVER · <target> · <integration> · <when>`). When
   empty for visitors, the section is **omitted**, not stubbed.
2. **`<HeroIdentity>`** — three-column grid (avatar 144×144 / name +
   tagline + socials / actions), avatar overlaps cover with `marginTop:
   -80` and a 4px `--bg-canvas` border.
3. **`<HeroAbout>` + `<HeroEquipment>` + `<HeroLocation>`** — a
   1.4fr / 1fr / 1fr grid below identity. Bio rendered from
   `users.bio_html` (already sanitized server-side). Equipment column
   uses dashed-bottom rows; location uses a stacked stat list.
4. **`<HeroStatsRow>`** — 5 stats inline: published frames, total
   integration, followers, appreciations (accent-coloured), targets
   shot.
5. **`<FeaturedRow>`** — 6 portrait tiles (3:4) with rank badge in
   top-left and target/likes label in a gradient-bottom overlay.
6. **`<GalleryToolbar>` + `<PhotoGrid>`** — justified rows.
   Implementation: use Flickr's `justified-layout` package
   (https://github.com/flickr/justified-layout). Target row height
   220px desktop, 140px mobile. Gap 8px.

### Owner-mode

Open `ScreenHeroOwner`. The page **never** shows owner prompts to
visitors. When the visitor is the owner:

- An accent-tinted banner runs across the top with copy
  `● VIEWING YOUR OWN PROFILE · OWNER MODE` and an `Edit profile`
  button on the right.
- Empty slots render dashed-bordered prompts with the copy in the
  spec's table (`"Pick a cover from your gallery →"`,
  `"Add a tagline"`, etc.).
- Featured slots show 6 placeholder tiles with `SLOT 01..06` mono
  labels; the first one carries `+ Pin a photo` in the accent colour.

### `<ProfileEditor>` (drawer)

Open `ScreenProfileEditor`. Sectioned drawer (720px wide), saves on
section blur. Sections:

1. **Identity** — avatar upload, display name, tagline, handle change
   (with a 90-day-cooldown caveat under the field).
2. **About · rich text** — Tiptap editor configured with marks/nodes
   that match the ammonia allowlist (`p, br, strong, em, u, h2, h3,
   h4, ul, ol, li, blockquote, code, a`). The toolbar is shown literally
   in the artboard. **Server-side `ammonia` is the source of truth** —
   send raw HTML, server returns sanitized `bio_html`.
3. **Equipment** — five rows; each is an `<EquipmentAutocomplete>`.
4. **Location & sky** — city, SQM, **Bortle ladder** (9-cell segmented
   control 1..9; selected cell uses `--accent` background).
5. **Social links** — list of `{kind, url}` rows; kinds restricted to
   the spec's small enum. `+ Add link` adds a row with a kind picker.

### `<Lightbox>`

Open `ScreenLightbox`. Route-mounted at
`/u/<handle>/p/<short-id>` via SvelteKit overlay routing. Two-column
layout: image left (full-bleed black), 380px panel right with
title/caption/like/EXIF table/equipment/more-from-this-author strip.
Keyboard: ←/→ prev/next, Esc close, `i` toggle EXIF panel, `l` like.

## Phase 3 — Discovery

### `/explore`

Open `ScreenExplore`. Filter rail has **two groups separated by a
hairline divider**: sort (newest / most appreciated / most discussed),
then time window (24h / 7d / 30d / all). Below: category chips and a
`Following only` toggle. Cross-author tile (`<CrossAuthorTile>`)
includes an `@HANDLE` chip in the bottom-left of the gradient overlay.

Cursor pagination on `(published_at DESC, id DESC)`. The footer below
the grid shows the current cursor in mono for development confidence
(`CURSOR · (PUBLISHED_AT, ID) < (…)`); remove for production.

### `/t/<slug>` — target page

Open `ScreenTargetPage`. Header is a 1.4fr/1fr grid: left side has
the slug (`M31`) at 64px mono in `--accent`, with the canonical name
beside it as italic display, then aliases as outlined chips, then a
short factual blurb. Right side is a 4-stat panel inside a bordered
card. The target page is **always inclusive of plate-solve rows when
they arrive** — the SQL block on the plate-solve note artboard shows
the canonical query.

### `/equip/<kind>/<slug>` — equipment page

Open `ScreenEquipmentPage`. Structure mirrors target page. Adds an
`OFTEN PAIRED WITH` rail at the bottom showing four other
equipment_items with high co-occurrence counts.

### `/search?q=`

Open `ScreenSearch`. Three-bucket layout (Targets · Photographers ·
Photos), capped 5/5/24 per group. Left rail has bucket counts as
filter pills (selecting one filters the page to that bucket only).
The search-bar artboard `ScreenSearchBar` shows the autocomplete
dropdown shape (used in the navbar, ⌘K from anywhere).

### Empty states

Open `ScreenDiscoveryEmpty`. Six empty-state cards, copy verbatim
from the spec. Every card carries a `+ Upload a frame` CTA — the
empty page is an invitation to publish.

## Cross-cutting

### Tier upgrade prompt

Open `ScreenTierUpgrade`. Triggered when a free user attempts to
upload >50 MB. The prompt **must be shown before the PUT** is
attempted (the presign call returns the cap; client checks
`file.size > tier_max` first and short-circuits). Two-tier compare
(Free 50 MB / Subscriber 200 MB), `RECOMMENDED` flag on the right card.

### Error states

Open `ScreenErrorStates`. Inventory of `AppError` variants and how
they render. Each variant maps to:

- A coloured chip (warning, danger, or muted)
- A single-sentence italic message
- One or more recoverable actions, primary one accent-coloured

Variants documented:
- `QuotaExceeded · 413` — pre-flight tier check
- `PayloadTooLarge · 413` — S3 rejected at PUT
- `Conflict · 409` — handle taken
- `Conflict · 409 · DUP HASH` — same file already uploaded by this
  user
- `RateLimited · 429`
- `Magic-byte mismatch · 400`
- `PendingFinalizeStuck · 408`
- `UnsupportedFormat · 400` — FITS / RAW are out of scope

### Plate-solve forward-compat

Open `ScreenPlateSolveNote`. The discovery queries **must** read
`photo_targets` regardless of `source`. P1 only writes
`source='manual'`; the future astrometry job writes
`source='plate_solve'` with `confidence` populated. **No schema or
query changes when astrometry ships** — that's the whole point.

### URL map

Open `ScreenRouteMap` for the table of every URL change and what
301s. Old `/photo/<uuid>` routes 301 to `/u/<handle>/p/<short-id>`;
old handles 301 for 90 days post-rename.

## Components to build (Svelte)

A non-exhaustive checklist matching the artboards:

- `<UploadDropzone>` · `<UploadFileRow>` · `<UploadProgress>`
- `<HandlePicker>` (input with state chip + 300ms debounce)
- `<TargetPicker>` · `<TagInput>` · `<CategorySegmented>` ·
  `<EquipmentAutocomplete>`
- `<HeroCover>` · `<HeroIdentity>` · `<HeroAvatar>` ·
  `<HeroEquipmentStrip>` · `<HeroLocationBadge>` · `<HeroStatsRow>` ·
  `<FeaturedRow>` (drag-reorder via `@neodrag/svelte`)
- `<PhotoGrid>` (justified-rows wrapper) · `<PhotoTile>` (modes:
  `single-author` / `cross-author`) · `<AuthorChip>`
- `<Lightbox>` (route-mounted overlay)
- `<ProfileEditor>` (drawer) · Tiptap binding · `<BortleLadder>` ·
  `<SocialLinksEditor>`
- `<DiscoveryHeader>` (variants: explore / target / equipment /
  category / tag) · `<GalleryToolbar>` · `<SearchBar>`
- `<TierUpgradeModal>` · `<ErrorChip>` · `<EmptyState>`

## Backend contracts

For each backend route the spec already names the file. Quick
reminder of what the frontend assumes:

- `POST /api/uploads/init { files: [{ name, size, hash }] }` → array
  of `{ photo_id, short_id, presigned_put_url }`.
- `POST /api/uploads/:id/finalize` → idempotent; returns `status` and
  the `display_key`.
- `POST /api/photos/:id/like` and `DELETE` (mirror).
- `GET /api/explore?sort=&since=&category=&following=&cursor=`.
- `GET /api/targets/:slug` · `GET /api/tags/:slug` ·
  `GET /api/equipment/:kind/:slug` · `GET /api/categories/:cat`.
- `GET /api/search?q=` → `{ targets[5], users[5], photos[24] }`.
- `GET /api/targets/autocomplete?q=` ·
  `GET /api/tags/autocomplete?q=` ·
  `GET /api/equipment/autocomplete?kind=&q=`.
- `PATCH /api/users/me` accepts `bio_html` raw, returns sanitized.

## Accessibility

- Lightbox: `aria-modal="true"`, focus trap, restore focus on close.
- All gallery tiles: `aria-label="<target> by @<handle>"`.
- Owner-prompt copy verified at AA contrast against
  `--bg-accent-tint`.
- The Bortle ladder has discrete tick marks; each cell carries a
  visible label and an `aria-label` describing the class
  (e.g. `Class 6 — Bright Suburban`).
- Keyboard map for the lightbox is documented in the side panel
  under the like row.

## Out of scope (do not build)

- Comments on photos
- View counts or bookmarks
- Direct messages
- Stripe/billing UI (the `users.tier` flag and enforcement ship,
  but checkout does not)
- AI "is this astronomy" validation
- Plate-solving itself (only the data shape is reserved)
- FITS / RAW input formats

## Where to start

1. Land migrations 0005–0012 in order.
2. Build the presigned PUT pipeline + `<UploadDropzone>` end-to-end
   for one file. Tier-cap rejection via S3 `Content-Length-Range` is
   the load-bearing test.
3. Add `<HandlePicker>` and the signup change. Backfill placeholder
   handles for existing users in the same migration.
4. Ship Phase 2: `<HeroPage>` visitor first, then owner mode + editor.
5. Ship Phase 3: `<ExplorePage>` first, then `<TargetPage>` (it's the
   most-used after explore), then equipment/tag/category/search.
