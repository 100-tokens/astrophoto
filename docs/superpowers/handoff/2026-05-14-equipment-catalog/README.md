# Handoff: Equipment Catalog Enrichment

Implementation of the **2026-05-14 spec** — typed specs per equipment kind, a
photo↔filter junction table, and a structured filter chip input on
`/upload/[id]/verify`. Builds on the per-user setups shipped in the 2026-05-04
spec.

---

## About the design files

The files in this bundle are **design references in HTML/React**. They are
prototypes intended to communicate the look, layout and interactive feel of
the new equipment-catalog surfaces. **Do not ship them as-is.**

Your task is to recreate these designs in the astrophoto.pics codebase
**using its existing conventions**:

- Backend: **Rust** (Axum/Actix per the spec — uses `AppError`, `ts-rs`).
  Implement the SQL migration `0018_equipment_catalog_enriched.sql` exactly
  as written in the spec, plus the new handlers & specs validation module.
- Frontend: **SvelteKit** (per the spec — routes like
  `frontend/src/routes/upload/[id]/verify/+page.svelte`). Port the React
  prototype components to Svelte.
- Use the existing `styles.css` design tokens (already in the codebase per
  the existing Astrophoto Design system — included here for reference).

If a piece of styling appears in `chips.css` (this bundle) and not in
`styles.css`, port it as part of the new feature's CSS.

---

## Fidelity

**Hi-fi.** Pixel-perfect mockups with final colors, typography, spacing, and
interactions. Recreate pixel-perfectly using the codebase's existing
patterns. Where the prototype invents a new sub-component (FilterChip,
FilterChipInput, RoleRow, SpecsPanel) — implement those as new Svelte
components inside the upload-verify / settings-equipment / equip-browse
route folders.

---

## Surfaces

Four screens are mocked. Open `index.html` and pan/zoom the design canvas to
see them side-by-side.

### A · `/upload/[id]/verify` — the hero surface
**File reference:** `screen-verify.jsx` · artboard label "A"

The structured filter chip input replaces the existing free-text **Filters**
field. Drives the new `photo_filters` junction (source of truth). The
legacy `photos.filters` cache string is still written on save, but rebuilt
from the junction in-transaction (not from the user's text).

Layout:
- 4-step stepper at the top (UPLOAD ✓ · VERIFY DATA · EQUIPMENT · CAPTION & PUBLISH).
  Active step has amber top-border (`var(--accent)`); done has ✓ glyph.
- Two-column body: 520 px image+EXIF on the left, form on the right
  (`grid-template-columns: 520px 1fr`, gap 64 px).
- Left: real photo preview, then EXIF table (`<table class="exif">`).
- Right: page subtitle + "11 fields recovered from EXIF" meta, then
  **Setup** chip (collapsible), then 2×2 equipment fields, then
  **FILTERS** spanning full row, then 4×1 acquisition row, then Notes,
  then footer toolbar (Save as draft / Previous / Save & next).

Key new behavior: **FilterChipInput** (see Components below). When the user
saves, the backend writes the structured `filter_item_ids` array to
`PATCH /api/photos/:id` and rebuilds `photos.filters` from the junction in
the same transaction.

### B · `/equip/filter/[slug]` — specs header + photo grid
**File reference:** `screen-equip.jsx`

The existing browse page gains a specs header above the photo grid. For
filters specifically: filter-type chip · bandwidth · size · mounted · brand.
Plus a community-submitted transmission-curve placeholder, an "Other Antlia
narrowband" sidebar, and the catalog item meta table (status, canonical
name, created/approved dates, submitted_by).

The header right rail has three buttons: **Follow** (ghost) · **Edit
specs** (secondary) · **Add to setup** (primary amber).

Photo grid is `repeat(4, 1fr)`, 16 px gap. Each photo card: real image,
italic Source Serif title, mono meta row (`@handle · X.X H · Bortle N`).

### C · `/settings/equipment/new` — setup builder with "Edit specs" panel
**File reference:** `screen-setup.jsx`

Setup form that gains, after each role's autocomplete input, a collapsible
**"Edit specs"** panel. The Telescope panel is shown expanded in the mock
(mode=`edit`, on an existing catalog item) to demonstrate the field set.

The panel header has a warm-amber indicator and the text "● EDITING A
SHARED CATALOG ITEM" — important UX cue that changes affect every other
user with that item. Footer has Discard + "Save to catalog" buttons.

Per-kind spec fields:
- **Telescope:** design (8-option dropdown), aperture_mm, focal_length_mm.
  `focal_ratio_f` is computed (DB-generated) and shown read-only.
- **Camera:** sensor_type (cmos/ccd), color_type (mono/osc), cooled (bool),
  sensor_model (free text), pixel_size_um, sensor_width_px,
  sensor_height_px.
- **Filter:** filter_type (14-option dropdown, labels like "Hα (656 nm)"
  for h_alpha), bandwidth_nm (hidden for broadband/LP/L/UV-IR/other), size
  (1.25in/2in/31mm/36mm/50mm round/50mm square/other), mounted (bool).
- **Mount:** mount_type, payload_kg, goto (bool).
- **Focal modifier:** modifier_type, factor.

The Filters role is multi-select via the same `FilterChipInput`.

### D · `/u/[handle]/p/[short]` — photo fiche
**File reference:** `screen-photo.jsx`

The photo page renders typed chips from the `photo_filters` junction. A
"legacy" orphan-token chip trails after them with a dashed border —
demonstrating that the string-cache fallback still surfaces tokens that
didn't match any catalog item during backfill.

Right rail adds **Integration · per filter** — a stacked bar per filter
with `hours / frames × sub_exposure` meta. This is **placeholder UI** for
phase 3 (`photo_filter_acquisitions` from the spec's "Out of scope" list).
Implement the data path in phase 3; for phase 1, you can hide this card or
mock it from EXIF totals.

---

## Filter type system (the heart of the feature)

The `filter_specs.filter_type` enum has 14 values. Each renders as a
**FilterChip** with a colored badge code + tinted background. Codes and
colors:

| `filter_type` enum value | Badge code | Color (hex) | Notes |
|---|---|---|---|
| `luminance`       | L     | `#f8f1e6` (warm white)  | broadband – no bw shown |
| `red`             | R     | `#c25048`               | warm muted red |
| `green`           | G     | `#7da64a`               | sage |
| `blue`            | B     | `#6b8db8`               | cool slate |
| `h_alpha`         | Hα    | `#b04634`               | deep narrowband red |
| `oiii`            | OIII  | `#4ea0a8`               | teal |
| `sii`             | SII   | `#e8a43a` (var(--accent)) | the brand sodium amber |
| `uv_ir_cut`       | UV/IR | `#8a6a9c`               | violet · no bw shown |
| `dual_band`       | D     | `#7a8fa8`               | Hα+OIII blend tone |
| `tri_band`        | T     | `#7a9588`               | |
| `quad_band`       | Q     | `#8a8a6a`               | |
| `light_pollution` | LP    | `#c98920` (var(--warning)) | broadband – no bw shown |
| `broadband_color` | BB    | `#d6cdba`               | |
| `other`           | ?     | `#6a6358` (var(--fg-faint)) | |

Untyped filter (filter_type null) gets a dashed border, "?" badge, and an
inline **"+ type"** CTA linking to the item edit page.

`bandwidth_nm` is rendered next to the chip name in mono `11px` only when
the type is one of: R, G, B, Hα, OIII, SII, dual_band, tri_band, quad_band.
For broadband, L, UV-IR cut, light_pollution, other — `bandwidth_nm` is
not displayed.

### Three chip style modes (Tweaks)
The prototype exposes three rendering modes for the chip (in the Tweaks
panel, top-right). **Ship `vivid` by default.** The other two are
experimental and can be revisited later:
- `vivid` — tinted background + colored badge (the default)
- `outline` — ring-only chip; no fill
- `mono` — neutral chip; type code is the only signal

To implement: a single CSS data-attribute on the page or app shell
(`data-ap-chip="vivid"`) flips between the three. See `chips.css` end of
file.

---

## Components to build

All four screens reuse five new components. Implement these first; the
screens become straightforward layout once the primitives are in place.

### `<FilterChip filter draggable removable compact />`
A single filter chip. `filter` is the joined row from
`GET /api/equipment/items/:id` (item + filter_specs). Renders:
- Type badge (square, `1px` radius, color from the table above)
- Chip name in Inter 12 px medium
- Bandwidth (`12 nm`) in mono only for narrowband types
- Optional drag-grip dots + remove × button

Styling: `chips.css` `.fchip` and `.fchip.is-<filter_type>`. Use a
`color-mix(in srgb, var(--ft-c) 7%, var(--bg-base))` background and a
`color-mix(in srgb, var(--ft-c) 40%, var(--border-default))` border for
the tinted look.

Untyped state: dashed border, "?" badge, "+ type" link.

### `<FilterChipInput value onChange orphans />`
Multi-select autocomplete chip-input. Mirrors the `filter_item_ids: Vec<Uuid>`
PATCH payload.

Behaviors:
- Search by `display_name` or by typed label ("hydrogen alpha" matches Hα).
- Keyboard: ↑/↓ navigate dropdown, ↵ adds the focused item, Backspace on
  empty input removes the last chip, Esc closes the dropdown.
- Drag-reorder chips (HTML5 drag API). Order maps to `photo_filters.position`.
- Click on dropdown row adds the filter and clears the query.
- When the query has no match, show "Create new filter '...'" footer that
  hits the `POST /api/equipment/items` endpoint with `kind=filter`,
  `display_name=<query>`. Returns the new item; add it to the chips.
- `orphans` prop renders legacy text tokens (from `photos.filters` cache)
  that didn't match any catalog item during backfill. They sit after the
  typed chips with a dashed border + "legacy" label. User can leave them
  or replace them by picking the right filter from the dropdown — on
  save, the orphan disappears from the cache string because it's
  rebuilt from the junction.

Important: the **dropdown should open by default** in the upload-verify
flow when the user first lands on the page (per the prototype). After they
add their three filters and tab away, it stays closed.

### `<RoleRow kind value badge expanded onToggle>{children}</RoleRow>`
A single row in the setup builder, used for Telescope/Camera/Mount/Focal
modifier. Layout: kind label (140 px) + autocomplete input + usage-count
chip + Edit specs toggle button. Children render inside an indented panel
below the row when `expanded`.

### `<SpecsPanel mode={'edit'|'create'} footerNote onSave>{fields}</SpecsPanel>`
The "Edit specs" panel that lives inside an expanded RoleRow. Has a
distinct header strip (warm amber border-left in edit mode, primary amber
in create mode) and a footer with Discard + "Save to catalog" buttons.

When `mode='create'`: shown right after the user picks an autocomplete
suggestion that has no catalog match (i.e., they're creating a new item).
Submit calls `POST /api/equipment/items` with both `display_name` and
`specs` in one round-trip.

When `mode='edit'`: shown when user opens "Edit specs" on an existing
catalog item. Calls `PATCH /api/equipment/items/:id`. Specs body
**fully replaces** the sub-table row — not field-merge.

### `<Field label value mono detected hint />{children}`
The field wrapper used everywhere: t-label + optional "FROM EXIF" /
"YOU FILL" meta on the right (amber if from EXIF, muted if user-fills),
input, optional hint below. `mono` uses JetBrains Mono. Children are
rendered inside if a non-input control is needed (select, textarea, custom).

---

## Design tokens

The codebase already uses the Astrophoto design system tokens. Reference
file: **`styles.css`**. Key tokens used by this feature:

**Colors (sodium-warm dark palette):**
- `--bg-canvas: #0c0a08` (page)
- `--bg-base: #100d0a` (default surface)
- `--bg-raised: #16120e` (cards)
- `--bg-elevated: #1d1812` (hover, popovers)
- `--border-subtle: #221d17`
- `--border-default: #2c2620`
- `--fg-primary: #f8f1e6` (headlines, copy)
- `--fg-secondary: #d6cdba` (body)
- `--fg-muted: #9c9384` (captions, meta)
- `--fg-faint: #6a6358` (disabled)
- `--accent: #e8a43a` (sodium amber — primary CTA, focus, active state)
- `--warning: #c98920` (used for "editing shared catalog" indicator)
- `--success: #6b8e4e`
- `--danger: #a8453a`

**Tinted callout backgrounds:**
- `--bg-accent-tint: rgba(232, 164, 58, 0.07)` (setup-applied chip)
- `--bg-warning-tint: rgba(201, 137, 32, 0.09)` (specs-editing banner)

**Type:**
- `--font-display: "Source Serif 4"` — display titles, photo titles, setup names
- `--font-ui: "Inter"` — body, buttons, inputs
- `--font-mono: "JetBrains Mono"` — labels, metadata, technical values, RA/Dec

**Sizes:**
- Display: 48–64 px (h1), 36 px (h2 in palette ref)
- Body: 14 px base, 13 px copy, 12 px UI, 11 px labels/meta
- Letter-spacing on labels: 0.06–0.16 em

**Radii (sharp):**
- `--r-sm: 2px` (inputs, chips, callouts)
- `--r-md: 4px` (cards)
- `--r-lg: 8px` (rare)
- `--r-pill: 999px` (avatars only)

**Spacing scale:** `--s-1: 4px` through `--s-20: 80px`.

**Sections use 64 px horizontal padding and 40–48 px vertical.**

---

## API surface (from the spec — phase 1)

All routes auth-required, JSON, errors via `AppError`.

### Equipment items
- `GET  /api/equipment/items/:id` — returns item + joined specs (tagged
  union keyed off `kind`).
- `POST /api/equipment/items` — body `{ kind, display_name, specs? }`.
  Resolve-or-create item; upserts specs. Sets `submitted_by`, `status='approved'`.
- `PATCH /api/equipment/items/:id` — body `{ display_name?, specs? }`.
  Specs body **fully replaces** the sub-table row (not field-merge).

### Photo filters
- `PATCH /api/photos/:id` — gains `filter_item_ids: Vec<Uuid>`. When
  present, replaces the photo's `photo_filters` rows (positions = array
  index) and rebuilds `photos.filters` cache string in the same
  transaction. Legacy `filters: Option<String>` still accepted; if both
  present, **structured wins**.
- `GET /api/photos/:id` — response gains `filters: { items: [...] }`
  joined from the junction, alongside the existing `filters: String` cache.

### Apply setup (already exists)
- `POST /api/photos/:id/apply-setup` — gains filter-junction sync (see
  spec §"Apply-setup").

---

## Type generation

After the backend changes, regenerate ts-rs bindings:
```
just types
```

New types:
- `EquipmentItemKind` (existing enum, unchanged)
- `EquipmentItemDetail` — item + optional kind-specific specs (tagged union)
- `EquipmentSpecsPayload` — input shape for create/edit, tagged union
- `FilterType`, `TelescopeDesign`, `CameraSensorType`, `CameraColorType`,
  `FilterSize`, `MountType`, `FocalModifierType` — string enums
- `PhotoFilterChip` — `{ id, display_name, filter_type, bandwidth_nm }`,
  what the fiche-photo renders

---

## Files in this bundle

### Design-canvas runtime (do not ship)
- `index.html` — entry point loading everything below.
- `design-canvas.jsx` — Figma-ish pan/zoom canvas wrapper (DesignCanvas / DCSection / DCArtboard).
- `tweaks-panel.jsx` — the Tweaks runtime + form-control helpers.
- `app.jsx` — composes the four artboards inside a DesignCanvas + Tweaks.

### Design system (already in your codebase as `styles.css`)
- `styles.css` — canonical Astrophoto design tokens. **Reference, not new.**
- `shared.jsx` — existing AppHeader, AppFooter, Photo. **Reference, not new.**
- `logos.jsx` — existing marks (Reticle, etc.). **Reference, not new.**

### New for this feature — port these to Svelte
- `chips.css` — FilterChip + FilterChipInput styles, including the
  per-filter-type color map and the three `data-ap-chip` style modes.
- `chips.jsx` — `<FilterChip>` and `<FilterChipInput>` React components.
- `shell.jsx` — `<Field>`, `<Crumbs>`, `<SubNav>`, `<Callout>`,
  `<PlaceholderPhoto>` helpers. Some of these may already exist in your
  codebase (e.g., `<Field>` is used in the existing ScreenUpload).
- `data.js` — sample catalog data (filter types + items). Useful as a
  test fixture, **not production data**. Real catalog is your DB.

### Screen mocks — recreate as routes
- `screen-verify.jsx` → `frontend/src/routes/upload/[id]/verify/+page.svelte`
- `screen-equip.jsx`  → `frontend/src/routes/equip/[kind]/[slug]/+page.svelte`
- `screen-setup.jsx`  → `frontend/src/routes/settings/equipment/new/+page.svelte`
- `screen-photo.jsx`  → `frontend/src/routes/u/[handle]/p/[short]/+page.svelte`

---

## Notes & follow-up phases (from the spec)

**Phase 1 (this work):** typed specs per kind + photo↔filter junction.
Auto-approved items. Open editability.

**Phase 2 (separate spec, planned next):** admin moderation workflow.
Schema is prepared — `status`, `submitted_by`, `approved_at` already in
`equipment_items` after migration 0018.

**Phase 3 (separate spec):** per-filter integration data
(`photo_filter_acquisitions` table). The photo-fiche "Integration · per
filter" card is placeholder UI for that phase.

**Explicitly out of scope:** range/facet browse pages, EXIF→camera-spec
autopopulate, transmission curves (the one in the mock is a placeholder
SVG), filter-wheel positions, public catalog index pages.

---

## Reference spec

`docs/superpowers/specs/2026-05-14-equipment-catalog-enriched.md`
