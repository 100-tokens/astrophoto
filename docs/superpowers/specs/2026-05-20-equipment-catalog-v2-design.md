# Equipment catalog v2 — design

**Date:** 2026-05-20
**Status:** Spec — awaiting scope decision
**Author:** post-deploy audit (catalog coherence Pt. 2)

## Context

The catalog ships today (migration 0012, enriched by 0018) with:

- `equipment_items` keyed on `(kind, lower(name))` — 6 kinds.
- Five typed sub-tables: `telescope_specs`, `camera_specs`, `mount_specs`,
  `filter_specs`, `focal_modifier_specs`. Each ON DELETE CASCADE from the
  item. Constraints already in place (enums, value ranges).
- A `SetupForm` (`/settings/equipment/new`) with per-role "Edit specs"
  buttons that open the typed sub-form.
- Catalog browse routes `/equip/[kind]` and `/equip/[kind]/[slug]` exist
  as scaffolding — minimal content because no users have entered specs.

What works: shape is right (typed sub-tables per kind, FK enforced),
filter creation now forces filter_type + bandwidth at create time
(`fix/filter-specs-required`).

What doesn't: only **2 spec rows / 12 catalog items** on staging.
`display_name` is a single freetext blob (`"Sky-Watcher Esprit 100 ED"`)
that mixes brand + model + variant. No `brand` / `model` separation. No
self-weight for any kind. No catalog page worth browsing. The result is
a "shared catalog" that's not actually shared.

## Goals — v2

1. **Brand + model as first-class structured columns** on every catalog
   item. Authoritative source for filtering, sorting, and merge-safety.
2. **Per-kind property completeness** so the catalog entry carries
   enough metadata to be useful in a buying-guide / setup-comparator
   context (the gradual long-term north star).
3. **Self-weight everywhere** (the user's "poids"), distinct from
   payload capacity (which is a mount property only).
4. **Saisie forcée** on the fields that make the difference between an
   indexable catalog row and dead text — same pattern as filter specs.
5. **Browse-and-edit UX** at `/equip/[kind]` and `/equip/[kind]/[slug]`
   so users can discover, compare, and improve the catalog without going
   through their own setup.

## Non-goals (v2 — defer to later)

- Brand-level entity (`equipment_brands` table with logos, websites).
  Nice-to-have. Adds a join + a populate-once migration. Punt to v3.
- Photo count per catalog item beyond what `usage_count` already gives
  (which counts photo references, not unique-photo discovery).
- Pricing, used-market data, retailer affiliate links. Out of scope —
  Astrophoto is editorial, not commerce.
- Tagging items as "discontinued" / "vintage" / "production". Useful
  later; out for v2.

## Schema gap audit, per kind

Existing columns are bolded; **gaps for v2** are italic.

### `equipment_items` (shared header)
- **id, kind, canonical_name, display_name, usage_count, status, submitted_by, approved_at, created_at**
- *brand* `text not null` (denormalized, until v3 introduces equipment_brands)
- *model* `text not null`
- *variant* `text null` — optional sub-model tag ("Pro", "Mk II", "v3")

`canonical_name` and `display_name` become derived (regenerated on
write) but kept as columns so the unique index and the legacy callers
keep working.

### `telescope_specs`
- **design, aperture_mm, focal_length_mm, focal_ratio_f** *(computed)*
- *self_weight_kg* `numeric(5,2) check (self_weight_kg > 0 and self_weight_kg <= 200)`
- *optical_length_mm* `int` — useful for travel + dew shield clearance
- *backfocus_mm* `numeric(4,1)` — knowing this saves photographers a buy

### `camera_specs`
- **sensor_type, color_type, cooled, sensor_model, pixel_size_um, sensor_width_px, sensor_height_px**
- *self_weight_g* `int check (self_weight_g > 0 and self_weight_g <= 5000)`
  — grams; cameras span 100 g (planetary) to 2 kg (large-format CCD)
- *full_well_capacity_e* `int` — sensor depth
- *read_noise_e* `numeric(4,2)` — sensor noise at unity gain
- *mount_thread* `text` (T2 / M48 / M54 / EF / RF / Z / E / X / other)
- *backfocus_mm* `numeric(4,1)` — typically 17.5 (T2) but varies

### `mount_specs`
- **mount_type, payload_kg, goto**
- *self_weight_kg* `numeric(5,2)` — distinct from payload; for travel
- *periodic_error_arcsec* `numeric(4,1)` — for tracking quality
- *tripod_included* `bool` — affects total weight conversation
- *control_protocol* `text` (synscan / nexstar / onstep / ascom_native / other)

### `filter_specs`
- **filter_type, bandwidth_nm**
- *mounted_diameter_mm* `int` (1.25" / 2" / 36mm / 50mm / 50.8mm / Eos / other)
- *thickness_mm* `numeric(3,2)` — affects backfocus
- *peak_transmission_pct* `numeric(4,1)`

### `focal_modifier_specs`
- **modifier_type, factor**
- *self_weight_g* `int`
- *backfocus_mm* `numeric(4,1)` — required for stack calculation
- *image_circle_mm* `numeric(4,1)` — vignetting envelope

### Guiding — new sub-table `guiding_specs`
Currently `guiding` is freetext only (no specs row). For v2, model it
like the others: a guiding *system* (camera + scope / OAG / OAG-prism).

```sql
create table guiding_specs (
  item_id           uuid primary key references equipment_items(id) on delete cascade,
  setup_kind        text not null check (setup_kind in ('oag','guidescope','oag_prism','other')),
  guide_focal_mm    int check (guide_focal_mm > 0),
  guide_aperture_mm int check (guide_aperture_mm > 0),
  guide_camera      text -- denormalized; alternatively link to a camera item via FK
);
```

## Migration plan

Single migration `0022_equipment_catalog_v2.sql`. One transaction:

1. `alter table equipment_items add column brand text, add column model text, add column variant text;`
2. Backfill via heuristic — first-word = brand for known brands (whitelist:
   ZWO / Sky-Watcher / Celestron / Takahashi / Vixen / iOptron / Astronomik /
   Optolong / Antlia / Baader / Player One / Touptek / QHY / etc), rest = model.
   Items with unknown brand → `brand = ''`, `model = display_name`, flag for
   moderation.
3. `alter table equipment_items alter column brand set not null;` after backfill
   reports zero NULLs.
4. Per spec table: add the new columns described above (all nullable so
   pre-existing rows stay valid; the saisie-forcée at the create-form
   level fills them going forward).
5. `create table guiding_specs (...)`.

No `drop` operations. No data loss. Reversible by `drop column` + the
five `alter`s, except the backfill is one-way (re-running it on
already-split rows is a no-op if we guard on `brand is null`).

## UI implications

- **SetupForm "Edit specs" sub-form** — already opens a typed form per
  role. Add the new columns to each form. Make `brand`, `model`,
  `self_weight_*` required.
- **FilterChipInput create sub-form** (already shipped) — add a brand
  field (filters need it too: Astronomik / Optolong / Baader / Antlia).
- **Verify form's `EquipmentAutocomplete`** — augment the autocomplete
  popup row to display `brand · model` with a small spec summary
  ("Esprit 100 ED · 100/550 f/5.5"). The pattern's already there for
  filter chips.
- **`/equip/[kind]`** — gridded catalog page with filter sidebar
  (brand checkbox list, aperture range slider for telescopes, payload
  range for mounts, narrowband/broadband toggle for filters, etc).
  Sort by usage_count (popular), aperture (descending), or alphabetical.
- **`/equip/[kind]/[slug]`** — item detail with spec table, photos
  using it (already wired via `usage_count` recompute), edit affordance
  for the submitter or any signed-in user (Phase 1 — moderation queue
  ships in v3).
- **Search box** — `?q=esprit 100` should match `display_name` AND
  `brand || ' ' || model` substring. Currently only canonical_name +
  display_name.

## Three implementation options

### Option A — Brand + model only (~1 day)

- Migration: add `brand`, `model`, `variant` to `equipment_items`. Backfill.
- Update `equipment::normalize_canonical` to recompute from `brand + ' ' + model + ' ' + variant`.
- Update create-form + autocomplete UI to take/show 2-3 fields instead of 1.
- Don't touch spec tables. Don't add weight. Don't build the browse page.

Cheap, fast, unlocks deduplication-by-brand later. Doesn't deliver on
the user's "vraie database" wording but is the smallest correct step.

### Option B — Brand + model + weight + saisie forcée (~3 days)

- Option A, plus:
- Add `self_weight_*` columns to telescope/mount/camera/focal_modifier
  specs (grams or kg per kind, validated).
- Make every spec field required at item create time, per kind. Pattern:
  same as filter PR #40 — popover sub-form before insert.
- Update the SetupForm "Edit specs" sub-forms to include the new fields.

Materially better catalog quality without committing to the browse UX
or extra fields like read_noise / backfocus. Good middle ground.

### Option C — Full v2 (~1-2 weeks)

- Everything in Options A + B, plus:
- All the gap-audit columns above (backfocus, mount_thread, periodic_error, etc.).
- New `guiding_specs` table.
- Catalog browse page at `/equip/[kind]` with filters and sort.
- Item detail page `/equip/[kind]/[slug]` with full spec table + photo count.
- Brand search and per-brand cluster view (still no `equipment_brands`
  table — that's v3; brand here stays denormalized text).

This is the "real database" answer. Lots of UI surface to design.
Probably wants the same handoff/design treatment as the verify page
refonte.

## Sequencing recommendation

If we're going to ship anything, **B** is the right step. A is too
small to feel like a real change; C is too big to land without a
design pass.

Stage gate: after B ships, watch:
- Do users actually fill the new fields? Saisie forcée pushes them to,
  but quality matters more than completeness.
- Does the catalog feel useful enough to browse, or does it still feel
  like a private list per user?

If yes to both, ship C in a quiet sprint. If no, the gap is in
acquisition, not in schema — pivot to seeding the catalog with the
top 200 commonly-used items (one-time data import from a curated CSV).

## Open questions

- **Self-weight unit** — grams for camera/focal_modifier (small) and kg
  for telescope/mount (large)? Or unify on grams to avoid mistakes?
  Decision: keep the natural unit per kind (grams for &lt;5 kg items, kg
  otherwise). Constraints in DB anchor the unit.
- **Brand normalization** — should "Sky-Watcher" and "SkyWatcher" auto-merge?
  Same merge tool discussion from `2026-05-20-catalog-merge-and-junctions-design.md`.
  Don't fight the same battle twice — defer to the merge tool spec.
- **Guiding kind** — keep it as a "system" with optional camera FK,
  or split into `guide_camera` + `guide_scope` separately? The latter
  is cleaner but doubles the catalog rows per setup. v2 = first
  approach; v3 can split if the bundle entries become a problem.

## Out of this spec

- `equipment_brands` table with logos and websites — v3.
- Photo-to-item back-reference UI (clicking a brand on a frame shows all
  photos using it) — depends on #7 from the merge-and-junctions spec.
- Compatibility hints ("this reducer is designed for this scope") —
  v3 or later.
- Pricing data — never.
