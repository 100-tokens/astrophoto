# Equipment Catalog Enriched — Typed Specs per Kind + Photo↔Filter Junction

**Date:** 2026-05-14
**Status:** Draft — pending written-spec review
**Author:** Pascal (with Claude)
**Builds on:**
- `docs/superpowers/specs/2026-05-04-equipment-setups-design.md`
  (per-user setups, `equipment_items` lookup dict, apply-setup flow).

## Goal

Promote `equipment_items` from a name-only dictionary into a typed
**equipment catalog** with structured specs per kind, and reshape the
photo↔filter relation into a proper junction table so filters can be
displayed, browsed, and (in a later phase) annotated with per-filter
integration data.

Concretely, this phase delivers:

1. **Typed specs per kind.** New sub-tables `telescope_specs`,
   `camera_specs`, `filter_specs`, `mount_specs`,
   `focal_modifier_specs`, each in 1-1 with an `equipment_items` row.
2. **Filter as a first-class typed item.** Every filter carries a
   `filter_type` (L / R / G / B / Hα / OIII / SII / UV-IR / dual-band /
   tri-band / quad-band / light-pollution / broadband-color / other),
   a `bandwidth_nm`, a `size`, and a `mounted` flag.
3. **Photo→filter junction.** New table `photo_filters` joins photos
   to filter items with stable ordering. The legacy `photos.filters`
   text column survives as a denormalized cache so existing browse
   indexes keep working.
4. **Catalog metadata for moderation phase 2.** `equipment_items`
   gains `status`, `submitted_by`, `approved_at`, `created_at`. In
   phase 1 every new item is auto-`approved`; phase 2 (a separate
   spec, see "Out of scope") introduces an admin-driven `pending` →
   `approved | rejected | merged` workflow without touching the
   schema again.

This is **one spec, one implementation plan**. Public range/facet
browse (e.g., "all refractors 400–600 mm focal") and the moderation
workflow are explicitly deferred.

## Why now

Today every photo carries five free-text equipment fields and an
`equipment_items` dictionary upserted from those fields. After
`docs/superpowers/specs/2026-05-04-equipment-setups-design.md` shipped
the per-user **setup** layer, two limits surface:

- A filter named `"L"` in one user's setup means **luminance** for
  one imager and **UV/IR-cut** for another, with no way to express
  that distinction. The filter is the most semantically loaded
  equipment class in deep-sky astrophotography (LRGB vs SHO vs OSC
  dual-band), and the current model is text-only.
- The browse page `/equip/<kind>/<slug>` shows a name and a count of
  photos. It cannot show "200 mm aperture f/5 Newtonian" because no
  such data exists in the catalog. Users expect a community catalog,
  not a list of strings.

This work creates the structured foundation. A follow-up phase adds
moderation; another follow-up adds per-filter integration data.

## Decisions

| #  | Topic                                       | Choice                                                                                              |
| -- | ------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| 1  | Scope                                       | Catalog enrichment (typed specs per kind) + photo↔filter junction. Per-filter integration deferred. |
| 2  | Curation model                              | Phase 1: auto-approved. Phase 2 (separate spec): admin-driven moderation. Schema prepared in phase 1.|
| 3  | Specs storage                               | One sub-table per kind, FK 1-1 to `equipment_items`. No JSONB, no EAV.                              |
| 4  | Item identity                               | Unchanged: unique `(kind, canonical_name)`. Variants (focal length, bandwidth) encode in name.      |
| 5  | Specs editability (phase 1)                 | Any authenticated user can create or edit specs of any item. Phase 2 will restrict.                 |
| 6  | Brand                                       | Encoded in `display_name`. No dedicated column.                                                     |
| 7  | Photo↔filter modeling                       | Junction `photo_filters` (source of truth) + `photos.filters` string cache (rebuilt in code).       |
| 8  | Junction kind enforcement                   | At code level (handler checks `item.kind = 'filter'`). No composite FK on kind.                     |
| 9  | Legacy photos backfill                      | Best-effort SQL inside the migration: tokenize `photos.filters`, match by lowercased canonical.     |
| 10 | Legacy text PATCH on photo                  | Preserved (`PATCH /api/photos/:id` body `filters: Option<String>`). Verify form switches to new structured PATCH. |
| 11 | Specs required                              | All optional. Existing items keep NULL specs. UI encourages filling but does not block.             |
| 12 | Browse with specs                           | `/equip/<kind>/<slug>` shows specs alongside photo grid. Range/facet browse out of scope.           |
| 13 | Public catalog index pages                  | Out of scope phase 1. Defer with moderation phase or a dedicated browse spec.                       |

## Glossary

- **Item** — a row in `equipment_items`. The catalog entry for a
  telescope, camera, mount, filter, or focal modifier.
- **Specs** — the typed attributes of an item, stored in the per-kind
  sub-table. Optional in phase 1.
- **Setup** — a user-owned bundle of items playing roles (defined by
  `2026-05-04-equipment-setups-design.md`). Unchanged here except
  filter list now also drives the junction.
- **Junction** — `photo_filters`. Many-to-many between `photos` and
  `equipment_items` (kind=filter) with stable ordering.
- **Cache string** — `photos.filters`. Comma-space-joined display
  names, rebuilt from the junction in the same transaction as any
  mutation. Authoritative for legacy / orphaned tokens only.

## Data model

### Migration `0018_equipment_catalog_enriched.sql`

```sql
-- 0018 equipment catalog enriched
--
-- 1. Catalog metadata on equipment_items (status pipeline prep for
--    phase 2 moderation, plus submission audit).
alter table equipment_items
    add column status        text        not null default 'approved'
        check (status in ('pending','approved','rejected','merged')),
    add column submitted_by  uuid        references users(id) on delete set null,
    add column approved_at   timestamptz,
    add column created_at    timestamptz not null default now();

-- Existing rows: stamp approved_at = now() to give them a coherent
-- audit footprint without a meaningful submitted_by.
update equipment_items
   set approved_at = coalesce(approved_at, now())
 where status = 'approved';

create index equipment_items_status_idx
    on equipment_items (kind, status, usage_count desc)
    where status = 'approved';

-- 2. telescope_specs.
create table telescope_specs (
    item_id         uuid primary key references equipment_items(id) on delete cascade,
    design          text       check (design in (
                       'refractor_apo','refractor_achro','sct','rc',
                       'newtonian','maksutov_cassegrain','maksutov_newtonian',
                       'dall_kirkham','other')),
    aperture_mm     int        check (aperture_mm between 30 and 1500),
    focal_length_mm int        check (focal_length_mm between 100 and 15000),
    focal_ratio_f   numeric(4,2) generated always as
                        ((focal_length_mm::numeric) / nullif(aperture_mm, 0)) stored
);
create index telescope_specs_aperture_idx on telescope_specs (aperture_mm);
create index telescope_specs_focal_idx    on telescope_specs (focal_length_mm);

-- 3. camera_specs.
create table camera_specs (
    item_id           uuid primary key references equipment_items(id) on delete cascade,
    sensor_type       text  check (sensor_type in ('cmos','ccd')),
    color_type        text  check (color_type in ('mono','osc')),
    cooled            boolean,
    sensor_model      text,
    pixel_size_um     numeric(4,2) check (pixel_size_um between 0.5 and 25),
    sensor_width_px   int   check (sensor_width_px > 0),
    sensor_height_px  int   check (sensor_height_px > 0)
);

-- 4. filter_specs. The heart of this feature: every filter is typed.
create table filter_specs (
    item_id        uuid primary key references equipment_items(id) on delete cascade,
    filter_type    text  check (filter_type in (
                      'luminance','red','green','blue',
                      'h_alpha','oiii','sii','uv_ir_cut',
                      'dual_band','tri_band','quad_band',
                      'light_pollution','broadband_color','other')),
    bandwidth_nm   numeric(5,2) check (bandwidth_nm > 0 and bandwidth_nm <= 200),
    size           text  check (size in (
                      '1_25in','2in','31mm','36mm','50mm_round','50mm_square','other')),
    mounted        boolean
);
create index filter_specs_type_idx on filter_specs (filter_type);

-- 5. mount_specs.
create table mount_specs (
    item_id     uuid primary key references equipment_items(id) on delete cascade,
    mount_type  text  check (mount_type in (
                   'equatorial_german','equatorial_fork','alt_az',
                   'harmonic_drive','strain_wave','other')),
    payload_kg  numeric(4,1) check (payload_kg > 0 and payload_kg <= 200),
    goto        boolean
);

-- 6. focal_modifier_specs.
create table focal_modifier_specs (
    item_id        uuid primary key references equipment_items(id) on delete cascade,
    modifier_type  text  check (modifier_type in (
                      'reducer','flattener','reducer_flattener',
                      'barlow','extender','corrector')),
    factor         numeric(3,2) check (factor > 0 and factor <= 5)
);

-- 7. photo_filters junction.
create table photo_filters (
    photo_id  uuid     not null references photos(id) on delete cascade,
    item_id   uuid     not null references equipment_items(id) on delete restrict,
    position  smallint not null default 0,
    primary key (photo_id, item_id)
);
create index photo_filters_item_idx      on photo_filters (item_id);
create index photo_filters_photo_pos_idx on photo_filters (photo_id, position);

-- 8. Backfill: tokenize photos.filters into junction rows. Best-effort
--    join against canonical_name. Orphan tokens are silently dropped;
--    photos.filters string cache stays as-is for them.
insert into photo_filters (photo_id, item_id, position)
select s.photo_id, e.id, s.position::smallint
  from (
    select p.id as photo_id,
           btrim(t.token) as token,
           t.ord - 1 as position
      from photos p,
           unnest(string_to_array(p.filters, ',')) with ordinality as t(token, ord)
     where p.filters is not null
       and length(btrim(p.filters)) > 0
  ) s
  join equipment_items e
    on e.kind = 'filter'
   and e.canonical_name = lower(s.token)
on conflict do nothing;
```

### FK delete semantics — recap

| Edge                                  | Behavior   | Why                                                                |
| ------------------------------------- | ---------- | ------------------------------------------------------------------ |
| `*_specs.item_id`                     | `cascade`  | Specs are part of the item. Deleting an item drops its specs.      |
| `photo_filters.photo_id`              | `cascade`  | Photo deletion takes its filter links.                             |
| `photo_filters.item_id`               | `restrict` | Catalog items must not vanish under a referenced junction.         |
| `equipment_items.submitted_by`        | `set null` | User deletion preserves the catalog row; submitter audit goes null.|

### Identity and uniqueness

Item identity remains `(kind, canonical_name)` unique. Two items with
the same name but different bandwidths or focal lengths are
intentionally separate rows. The display name encodes the
distinguishing variant:

- `Antlia Ha 3nm` vs `Antlia Ha 6nm` — two filter items.
- `Sky-Watcher Esprit 100 ED` vs `Sky-Watcher Esprit 120 ED` — two
  telescope items.

The specs sub-table describes the variant; it never disambiguates
items. If two rows accidentally describe the same product (e.g.,
typo: `Sky-Watcher` vs `SkyWatcher`), they are duplicates to be
merged by a future admin tool (phase 2 of the moderation spec).

### Cache string rebuild rules

A helper `rebuild_photo_filters_cache(tx, photo_id)` is called by
every writer of `photo_filters`. It runs in the same transaction:

1. `select e.display_name from photo_filters pf join equipment_items e on e.id = pf.item_id where pf.photo_id = $1 order by pf.position, e.display_name`.
2. Join with `", "`. Empty result → `NULL`.
3. `update photos set filters = $2 where id = $1`.

The cache is therefore **eventually consistent at end of transaction**
with the junction. Reads (browse, fiche photo) can pick whichever
source fits their query shape:

- `/explore` and `/equip/filter/<slug>` use the existing string-cache
  index (`photos_filters_lower_idx`). Unchanged.
- The fiche photo (`/u/<handle>/p/<short>`) joins through
  `photo_filters` to render typed chips.

### Photos with unmatched filter tokens after backfill

Some legacy `photos.filters` strings will contain tokens that did not
match any `equipment_items` row. Those rows are intentionally not in
`photo_filters` after backfill. The fiche photo renders no typed chip
for them but the string cache still displays the raw text below the
chip strip ("plus: Astronomik CLS"). On the next photo edit (user
visits verify or the edit form), the new structured PATCH replaces
the junction; the user is prompted to re-pick filters from the
autocomplete, which resolves the orphan.

## API surface

All routes auth-required. JSON. Errors via `AppError`.

### Item create & edit (phase 1: open)

| Method | Path                                  | Purpose                                                                 |
| ------ | ------------------------------------- | ----------------------------------------------------------------------- |
| POST   | `/api/equipment/items`                | **Extended**. Body: `{ kind, display_name, specs?: <kind-specific> }`. Resolve-or-create item, then upsert specs (insert on miss, update on hit). `submitted_by` = current user on first creation. `status` = `'approved'` (phase 1 auto). |
| PATCH  | `/api/equipment/items/:id`            | New. Body: `{ display_name?, specs?: <kind-specific> }`. Updates item display_name and/or specs. Specs body **replaces** the sub-table row (full-object replace, not field-merge).        |
| GET    | `/api/equipment/items/:id`            | New. Returns item + specs joined. Public to authenticated users.        |

Specs body shape is a tagged union, validated by Rust enum
`EquipmentSpecsPayload` (see Type generation). Sending the wrong
shape for the item's kind → 422.

### Photo filters

| Method | Path                                  | Purpose                                                                 |
| ------ | ------------------------------------- | ----------------------------------------------------------------------- |
| PATCH  | `/api/photos/:id`                     | **Extended**. Body adds optional `filter_item_ids: Vec<Uuid>`. When present, replaces the photo's junction set; positions = array index. Cache string rebuilt in same tx. Legacy `filters: Option<String>` still accepted for clients that haven't migrated; if both present, structured wins. |
| GET    | `/api/photos/:id`                     | **Extended**. Response includes `filters: { items: [{id, display_name, filter_type, bandwidth_nm}] }` joined from the junction, alongside the existing `filters: String` cache. |

### Apply-setup (existing — extended)

`POST /api/photos/:id/apply-setup` (defined in 2026-05-04 spec) gains:

1. Builds the filter id list from `setup_items` where `role='filter'`,
   ordered alphabetically by display_name (same order the cache string
   already uses).
2. Mode `overwrite`: `delete from photo_filters where photo_id=$1`,
   re-insert with positions matching the order.
3. Mode `fill_empty`: only insert when `photo_filters` for that photo
   is empty AND the photo's `filters` string cache is null/empty.
4. Cache string rebuilt at end of tx.

This keeps the apply-setup contract intact: cache string and junction
move together, `setup_id` is set regardless.

### Specs validation

Per-kind validation lives in `backend/src/equipment/specs.rs` (new
module). Each kind has a deserialize-and-validate function that
checks enum membership, numeric ranges, and consistency (e.g., for
filter: if `filter_type` is `dual_band`/`tri_band`/`quad_band` and
`bandwidth_nm` is null, accept; if it's `luminance` or `uv_ir_cut`,
bandwidth is conventionally null but not enforced as a hard rule —
only the check constraints in the DB are hard rules).

## Frontend

### `/settings/equipment` setup form — spec fields

The existing setup builder (`/settings/equipment/new` and `/settings/equipment/[id]/edit`)
gains, **after** the autocomplete input for each role, a collapsible
**"Edit specs"** panel that appears when the user has just created a
new item via the "create-if-missing" path. The panel:

- Shows fields specific to the kind (see "Specs per kind UI" below).
- Submit calls `POST /api/equipment/items` with both `display_name` and
  `specs` — single round-trip, the item is created with its specs.
- If the user picked an **existing** item from autocomplete, the panel
  reads the current specs and renders them as **editable**, with an
  inline "Save changes to catalog" button calling `PATCH /api/equipment/items/:id`.
  An info text reminds them that edits affect the shared catalog.

Specs are **not stored on the setup row** — they belong to the item,
not to the user's bundle.

#### Specs per kind UI

- **Telescope:** `design` (dropdown), `aperture_mm` (number), `focal_length_mm` (number). `focal_ratio_f` is shown read-only, computed.
- **Camera:** `sensor_type` + `color_type` (radios), `cooled` (checkbox), `sensor_model` (text), `pixel_size_um` (number), `sensor_width_px` + `sensor_height_px` (number pair).
- **Filter:** `filter_type` (dropdown with the 14 enum values, labeled e.g., "Hα (656 nm)" for `h_alpha`), `bandwidth_nm` (number, hidden when type is broadband_color / light_pollution / luminance / uv_ir_cut / other), `size` (dropdown), `mounted` (checkbox).
- **Mount:** `mount_type` (dropdown), `payload_kg` (number), `goto` (checkbox).
- **Focal modifier:** `modifier_type` (dropdown), `factor` (number).

All fields are optional. A subtle "Specs help others find your photos
and characterize your gear" hint lives at the top of each panel.

### Upload-verify form — filter chip input

`/upload/[id]/verify/+page.svelte` replaces the free-text **Filters**
input with a multi-select autocomplete chip input bound to
`equipment_items` of kind=filter:

- Each chip shows `display_name` and a small typed badge derived from
  `filter_type` (`L`, `R`, `G`, `B`, `Hα`, `OIII`, `SII`, `UV/IR`,
  `D`, `T`, `Q`, `LP`, `BB`, `?`). Color tokens: red filters in red,
  green in green, blue in blue, narrowband-Hα in deep red,
  narrowband-OIII in cyan, etc. Chips without typed specs render
  uncolored with a `?` badge plus an inline "Add type" link going to
  the item's edit panel.
- Drag to reorder. Order persists via the new structured PATCH.
- "Detach setup" (existing) does not clear the chips — it clears
  `setup_id` only, per the 2026-05-04 spec.

### `/equip/[kind]/[slug]` — specs in the header

The existing page (photos using a given item) gains a **specs header**
above the photo grid:

- Telescope: "Newton 200/1000 (f/5)" formatted line + design badge.
- Camera: "Sony IMX571, mono, cooled, 3.76 µm".
- Filter: "Hα — 3 nm — 2 inch — mounted".
- Mount: "EQ German, 20 kg, GoTo".
- Focal modifier: "Reducer 0.79×".

Items with NULL specs render an "Add specs" inline CTA (auth-only)
that links to a one-shot edit page or opens an inline form.

### Photo page — filter chips

`/u/<handle>/p/<short>` and the explore card both render the typed
filter chips next to the existing "Filters: L, R, G, B" line. If
the junction has more chips than the string cache, the junction wins
visually. If the string cache has tokens not in the junction (legacy
orphans), they appear after the chips as plain-text suffix.

## EXIF interaction (unchanged)

EXIF parsing still populates `camera`, `focal_mm`, `aperture_f`, etc.
This feature doesn't change EXIF behavior. The new `camera_specs` row,
if present for the EXIF-derived camera item, can in principle override
the EXIF values for display, but **not in phase 1** — the EXIF columns
on `photos` remain authoritative for the photo's recorded metadata.

## Type generation

The following types are exposed via `ts-rs`:

- `EquipmentItemKind` (existing enum, unchanged).
- `EquipmentItemDetail`: item + optional kind-specific specs (tagged
  union).
- `EquipmentSpecsPayload`: input shape for create/edit, tagged union
  per kind.
- `FilterType`, `TelescopeDesign`, `CameraSensorType`,
  `CameraColorType`, `FilterSize`, `MountType`, `FocalModifierType`:
  string-enum types for the controlled vocabularies.
- `PhotoFilterChip`: `{ id, display_name, filter_type, bandwidth_nm }`
  — what the fiche-photo renders.

After backend changes: `just types` regenerates
`frontend/src/lib/api/types.ts`. Diff committed.

## Testing

Backend (Rust, testcontainers, fresh DB per test):

- **Migration**: applying `0018` to a DB with legacy `photos.filters`
  strings backfills `photo_filters` rows for tokens that match
  `equipment_items.canonical_name`; non-matching tokens are silently
  dropped; the string cache is unchanged.
- **Item create with specs**: `POST /api/equipment/items` with
  `kind=telescope` and a full specs payload returns the item id and
  inserts both the `equipment_items` row and the `telescope_specs`
  row. Re-posting the same name updates the existing specs (PUT
  semantics on specs body).
- **Specs validation**: invalid enum → 422; out-of-range numeric →
  422; wrong-kind specs payload → 422.
- **GET item**: returns item + specs joined. Items without a specs
  row return `specs: null`.
- **PATCH item**: updates display_name and specs in one tx;
  ownership is open in phase 1 (any authenticated user). Specs body
  fully replaces the sub-table row.
- **Photo filter PATCH (structured)**: passing
  `filter_item_ids: [a, b, c]` replaces the junction, sets positions
  0/1/2, rebuilds the cache string in the same tx.
- **Photo filter PATCH (legacy text)**: passing `filters: "L, R, G"`
  still works; junction is **not** auto-populated from text (legacy
  behavior preserved); cache string is set verbatim.
- **Photo filter PATCH (both fields present)**: structured wins,
  cache rebuilt from junction.
- **apply-setup overwrite**: junction replaced; cache string matches
  junction order.
- **apply-setup fill-empty**: junction unchanged when photo already
  has filters in junction OR a non-empty cache string.
- **Cache rebuild ordering**: filter A position 0, filter B position
  1; assert cache string = "A, B" (joined by `", "`).
- **FK delete**: deleting a filter item with junction rows is
  blocked by `restrict` (sanity check); deleting a photo cascades the
  junction; deleting a user nulls `submitted_by` on items they created.
- **Kind constraint at junction insert**: handler refuses
  `filter_item_ids` containing an item of non-filter kind → 422.

Frontend (Playwright, against running backend):

- Create a new telescope item via the setup form's "Edit specs"
  panel; submit; assert the item appears with specs in
  `/equip/telescope/<slug>`.
- Upload a photo, pick three filter chips (mix of typed and untyped),
  publish; fiche photo renders three chips with correct badges.
- Edit a published photo: drag-reorder chips; verify order persists.
- Visit `/equip/filter/<slug>` for a filter with specs: header shows
  type/bandwidth/size; verify CTA "Add specs" appears for items
  without.

## Out of scope (deferred)

- **Moderation phase 2** (separate spec, planned next): admin role
  introduction, pending queue UI, merge & rename tooling. Schema is
  prepared (`status`, `submitted_by`, `approved_at`) so this phase
  ships without migrating again.
- **Per-filter integration data** on photos (subs × sub_time per
  filter, total integration table à la AstroBin). Will hang off
  `photo_filters` with additional columns or a separate
  `photo_filter_acquisitions` table.
- **Range / facet browse** (`/equipment/<kind>` index with filters by
  aperture range, bandwidth range, etc.). Possible later spec without
  schema impact.
- **EXIF→camera-spec autopopulate.** EXIF stays photo-level only.
- **Specs on guiding equipment, focuser, filter wheel, OAG, rotator,
  computer, power.** Their roles aren't first-class on setups (per
  2026-05-04 spec), so specs are premature.
- **Filter transmission curves, MTF charts, datasheets.** Way beyond
  catalog identity.
- **Filter wheel positions** (slot 1 = L, slot 2 = R…). Hardware
  configuration, not photo metadata.
- **Public catalog index pages** (browse all telescopes alphabetical,
  newest, etc.). Discoverability concern; defer.

## Open questions

None blocking. To confirm during implementation:

- Whether the filter chip color tokens (red / green / blue / Hα /
  OIII / SII) live in `frontend/src/lib/styles/filters.css` or
  inline. Pure presentation choice.
- Whether `filter_type='broadband_color'` is necessary alongside the
  R/G/B trio — it covers "the whole RGB triplet sold as one set" but
  may never appear in practice. Keep for now; revisit on first user
  feedback.
- Whether to short-circuit the "Edit specs" inline form to a
  dedicated `/equip/[kind]/[slug]/edit` page if the panel grows
  beyond a few fields. UI choice, not a schema choice.

## References

- `backend/migrations/0012_equipment_items.sql` — original `equipment_items` dictionary.
- `backend/migrations/0017_equipment_setups.sql` — setups + setup_items + photo.setup_id.
- `docs/superpowers/specs/2026-05-04-equipment-setups-design.md` — setups design that this spec extends.
- `backend/src/equipment/items_create.rs` — current resolve-or-create handler, will gain the specs path.
- `backend/src/equipment/setups/` — setup CRUD, unchanged except apply-setup integration.
- `backend/src/photos/metadata.rs` — `photos.filters` write path, gains `filter_item_ids` branch.
- `backend/src/photos/apply_setup.rs` — gains junction sync.
- `frontend/src/routes/equip/[kind]/[slug]/` — existing browse page, gains specs header.
- `frontend/src/routes/upload/[id]/verify/` — existing verify form, gains chip input.
- AstroBin equipment model (reference, not a hard requirement): https://welcome.astrobin.com/features/equipment-database.
