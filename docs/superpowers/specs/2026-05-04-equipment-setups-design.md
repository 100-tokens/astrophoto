# Equipment Setups Design — Reusable Gear Bundles for Upload

**Date:** 2026-05-04
**Status:** Draft — pending written-spec review
**Author:** Pascal (with Claude)

## Goal

Let users save reusable named **equipment setups** (telescope + camera +
mount + filters + focal modifier) and apply one to a photo at upload
time in a single click, instead of retyping the five equipment fields
every time.

The feature ships as **one spec, one implementation plan**. Public
discovery surfaces (browse-by-setup, public listing of a user's setups
on their profile) are explicitly deferred — see "Out of scope" below.

## Why now

Today, every photo carries five free-text equipment fields (`scope`,
`camera`, `mount`, `filters`, `guiding`) that are upserted into the
`equipment_items` dictionary at save time. This works for autocomplete
and the `/equip/<kind>/<slug>` browse pages but it makes uploads tedious
(retype five fields each time) and degrades coherence: typos and
near-duplicates ("Sky-Watcher 200P" vs "SkyWatcher 200P") create
fragmented browse buckets that should be one. Astrophoto is a community
site for enthusiasts — equipment identity matters because it
characterizes the image.

## Decisions

| #   | Topic                              | Choice                                                                                          |
| --- | ---------------------------------- | ----------------------------------------------------------------------------------------------- |
| 1   | Identity model                     | **Canonical-shared, direct** (D1). Setups reference `equipment_items` rows directly, no per-user instance layer. |
| 2   | Role taxonomy                      | **T2′** — five canonical roles + one free-text. See "Roles" below.                              |
| 3   | Image-characterization principle   | Optics + photon capture + path modifiers are canonical and browseable. Mount is canonical (cataloguable, has community value) but not "characterizing". Guiding is free-text only. |
| 4   | Default-setup auto-apply           | Silent **fill-empty-only** at upload-verify load. Preserves EXIF-derived values.                |
| 5   | Manual setup application           | **Confirm-on-conflict** before overwriting any non-empty equipment field.                       |
| 6   | Per-photo override after apply     | All equipment fields stay editable. `setup_id` survives manual divergence — it records origin, not equality. |
| 7   | Setup edit on already-published photo | Same picker, same overwrite semantics, available from the existing photo edit path.          |
| 8   | Setup visibility (v1)              | Private. Listed only in owner UI. Public profile / browse-by-setup deferred.                    |
| 9   | UI location                        | `/settings/equipment`, following the existing `/settings/*` pattern.                            |
| 10  | Item creation flow                 | Autocomplete on `equipment_items`, fallback "create-if-missing" using lowercased canonical key. No moderation queue in v1. |
| 11  | Specs on `equipment_items`         | Out of scope. Identity = canonical name only. EXIF remains the source of image specs.           |
| 12  | Canonical item rename              | Not exposed in v1. No PATCH on `equipment_items`. Admin tooling for merge/rename is a separate feature. |

## Roles

A setup may contain items playing the following **roles**:

| Role             | Cardinality | Backing `equipment_items.kind` | Image-characterizing? |
| ---------------- | ----------- | ------------------------------ | --------------------- |
| `optical_tube`   | 0 or 1      | `telescope`                    | yes                   |
| `focal_modifier` | 0 or 1      | `focal_modifier` (new)         | yes                   |
| `main_camera`    | 0 or 1      | `camera`                       | yes                   |
| `filter`         | 0 or many   | `filter`                       | yes                   |
| `mount`          | 0 or 1      | `mount`                        | recorded, not characterizing |

In addition, the setup carries a single **free-text `guiding`** field
on the `equipment_setups` row itself. Guiding is **not** a setup_items
role. It is not canonical, not browseable, and not autocompleted under
the new pipeline. The `kind = 'guiding'` rows already in
`equipment_items` from past photo upserts remain in place and inert.

A setup with zero items is allowed (e.g., user creates an empty draft
named "Backyard rig" before adding gear). Filling at least one role is
encouraged in the UI but not enforced at the DB level.

## Data model

### Migration `0014_equipment_setups.sql`

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
    guiding     text,                        -- free-text, may be null
    created_at  timestamptz not null default now(),
    updated_at  timestamptz not null default now(),
    unique (owner_id, name)
);
create index equipment_setups_owner_idx
    on equipment_setups (owner_id, updated_at desc);

-- 3. At most one default per user. Partial unique index — DB-enforced,
--    cheaper than the old project's transactional clear-then-set.
create unique index equipment_setups_owner_default_uidx
    on equipment_setups (owner_id) where is_default;

-- 4. Setup ↔ canonical item junction. Composite PK on
--    (setup_id, role, item_id) allows multi-filter (same role appears
--    multiple times with different items) and prevents the same item
--    being added twice in the same role.
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
    add column focal_modifier text;     -- denormalized canonical name

create index photos_setup_idx
    on photos (setup_id) where setup_id is not null;
create index photos_focal_modifier_lower_idx
    on photos (lower(focal_modifier))
    where published_at is not null and focal_modifier is not null;
```

### FK delete semantics — recap

| Edge                       | Behavior        | Why                                                                       |
| -------------------------- | --------------- | ------------------------------------------------------------------------- |
| `photos.setup_id`          | `set null`      | Denormalized columns are the historical truth; losing the FK is fine.     |
| `setup_items.setup_id`     | `cascade`       | Junction rows are part of the setup.                                      |
| `setup_items.item_id`      | `restrict`      | Canonical items must not be deletable while referenced. v1 never deletes them. |
| `equipment_setups.owner_id`| `cascade`       | Account deletion takes setups with it. Photos' denormalized columns persist. |

### Denormalization rules — apply-setup payload

When a setup is applied to a photo, the photo's columns are updated as
follows:

| Photo column     | Source                                                                                        |
| ---------------- | --------------------------------------------------------------------------------------------- |
| `scope`          | `display_name` of the `optical_tube` item, or null.                                           |
| `focal_modifier` | `display_name` of the `focal_modifier` item, or null.                                         |
| `camera`         | `display_name` of the `main_camera` item, or null.                                            |
| `mount`          | `display_name` of the `mount` item, or null.                                                  |
| `filters`        | Comma-space-joined `display_name` of all `filter` items, **alphabetical order**. Empty → null. |
| `guiding`        | The setup's free-text `guiding` field verbatim, or null.                                      |
| `setup_id`       | The setup's `id`.                                                                             |

**Multi-filter ordering limitation:** without a `position` column on
`setup_items`, the join order is alphabetical. SHO vs HOO ordering
preference is deferred until filter wheel positions are explicitly
modeled in a later iteration.

### Apply-mode semantics

The same six-column write happens in two modes:

- **Fill-empty-only** (default-setup auto-apply at upload-verify load,
  per Decision 4): for each target column, write only if the column is
  currently null or empty. Preserves EXIF-derived `camera` and any
  text the user has already typed.
- **Confirm-on-conflict** (manual setup picker, per Decision 5): the
  client compares incoming values to current ones; if any non-empty
  current value would be overwritten, surface a single confirm
  ("Replace 2 fields?"). On confirm, write everything verbatim.

In **both modes**, `photos.setup_id` is set to the picked setup's id
even if no equipment column was actually overwritten (e.g., EXIF had
already filled `camera` and fill-empty preserved it). The FK records
"this photo originated under this setup", not "every column of this
photo equals the setup". This is the contract that makes per-photo
override (Decision 6) coherent.

### Setup detach

A photo's `setup_id` can be cleared via an explicit **"Detach setup"**
button, available wherever the setup picker appears (upload-verify and
the published-photo edit form). Detaching does **not** clear the
denormalized equipment columns — the user's recorded values stand. The
button is the only way `setup_id` becomes null after being set;
manually editing one field never auto-clears the FK (Decision 6).

## API surface

All routes auth-required unless noted. JSON in/out. CSRF via the
existing session middleware. Errors via `AppError`.

### Setup CRUD

| Method | Path                              | Purpose                                                                 |
| ------ | --------------------------------- | ----------------------------------------------------------------------- |
| GET    | `/api/equipment/setups`           | List the caller's setups, newest-updated first. Each row includes a per-role count of items (no full item rows). |
| GET    | `/api/equipment/setups/:id`       | Single setup with full `setup_items` expansion (role + item canonical/display). 404 if not owner. |
| POST   | `/api/equipment/setups`           | Create. Body: `{ name, description?, location?, is_remote?, is_default?, guiding?, items: [{role, item_id}] }`. Items resolved against `equipment_items`; unknown ids → 422. |
| PATCH  | `/api/equipment/setups/:id`       | Update meta + items (replace-all). Same body shape minus required fields. Only owner. |
| DELETE | `/api/equipment/setups/:id`       | Cascade `setup_items`; sets photos' `setup_id` to null via FK.          |

`is_default = true` on create/patch invokes the partial unique index;
the implementation must clear any other default in the same transaction
before setting the new one (`update equipment_setups set is_default = false where owner_id = $1 and is_default and id <> $2`).

### Item creation / autocomplete

| Method | Path                                  | Purpose                                                                       |
| ------ | ------------------------------------- | ----------------------------------------------------------------------------- |
| GET    | `/api/equipment/autocomplete`         | Existing endpoint. Extended `kind` enum to include `focal_modifier`.          |
| POST   | `/api/equipment/items`                | Resolve-or-create. Body: `{ kind, display_name }`. Lowercased canonical = unique key. Returns existing row or newly inserted. Public to authenticated users; no moderation. |

`POST /api/equipment/items` is the single insertion path used by the
setup builder UI when the user types a name that doesn't autocomplete
to an existing item. It returns the existing row on hit, inserts on
miss. **It does not increment `usage_count`** — that counter remains
photo-save-driven (via the existing upsert at `equipment::upsert`) so
its semantics stay "how often this item appears on a published
photo", not "how often it has been picked in a setup builder".

### Photo apply / detach

| Method | Path                                  | Purpose                                                                       |
| ------ | ------------------------------------- | ----------------------------------------------------------------------------- |
| POST   | `/api/photos/:id/apply-setup`         | Body: `{ setup_id, mode: "fill_empty" \| "overwrite" }`. Server enforces ownership of both photo and setup. Returns the updated denorm columns + setup_id. Replaces direct PATCH-of-five-fields when applying from a setup. |
| POST   | `/api/photos/:id/detach-setup`        | Clears `setup_id` only. Does not touch denormalized columns.                  |

The existing photo metadata PATCH path (used for ad-hoc per-field
edits) is unchanged. Picking a setup goes through `apply-setup`;
typing into a field uses the existing PATCH.

## Frontend pages and flows

### `/settings/equipment`

Three SvelteKit routes following the existing settings pattern:

- `/settings/equipment/+page.svelte` — list of the caller's setups,
  newest first. Each row: name, default badge, remote badge, count of
  items by role, last-updated. Inline actions: "Set as default",
  "Edit", "Delete (with confirm)". CTA "+ New setup" top-right.
- `/settings/equipment/new/+page.svelte` — create form.
- `/settings/equipment/[id]/edit/+page.svelte` — edit form (same
  component as `new`, prefilled).

Form fields:

- **Name** (required, text)
- **Description** (textarea, optional)
- **Location** (text, optional)
- **Remote** (checkbox)
- **Default** (checkbox; tooltip: "Auto-applied to new uploads")
- **Optical tube** (autocomplete on kind=telescope, single, optional)
- **Focal modifier** (autocomplete on kind=focal_modifier, single, optional)
- **Main camera** (autocomplete on kind=camera, single, optional)
- **Mount** (autocomplete on kind=mount, single, optional)
- **Filters** (autocomplete on kind=filter, multi-select chips, optional)
- **Guiding** (free-text, optional, helper text "e.g., ASI120MM Mini + 60mm guide scope")

Submit calls POST or PATCH. The autocomplete inputs accept free text;
on submit, any unresolved label triggers a `POST /api/equipment/items`
to create the canonical entry, then the setup save uses the returned
`item_id`. (UI may surface a small "create new" affordance once the
typed text doesn't match any suggestion.)

### Upload-verify integration

The existing upload-verify form (`/upload/[id]/+page.svelte`) gains a
**setup picker** above the equipment fields:

- Dropdown lists the user's setups; the default is preselected with a
  star marker.
- On page load: if a default exists, the **fill-empty-only** apply runs
  immediately (server-side via the existing `+page.server.ts` load) and
  the form is rendered with the resulting values. The user sees no
  modal; the equipment fields show the setup's values where they were
  empty.
- If the user picks a different setup: the client computes whether any
  current non-empty equipment field would change. If yes → confirm
  dialog "Replace 2 fields?" with cancel / confirm. On confirm, call
  `POST /api/photos/:id/apply-setup` with `mode: "overwrite"`. On
  cancel, revert the dropdown.
- A **"Detach"** button clears `setup_id` (calls
  `POST /api/photos/:id/detach-setup`) without touching the values.

The six equipment inputs (`scope`, `focal_modifier`, `camera`, `mount`,
`filters`, `guiding`) remain editable after applying. Manual edits to a
field do not auto-clear `setup_id` (Decision 6).

### Photo edit (already-published)

The existing photo edit page surfaces the same setup picker with the
same semantics (Decision 7). No new component; the picker is a shared
Svelte component imported by both upload-verify and edit.

## EXIF interaction (recap)

Today, EXIF parsing populates `camera`, `focal_mm`, `aperture_f`, etc.
during the `/api/photos/finalize` step before the user sees the
upload-verify form.

With this feature:

1. EXIF → photo columns at finalize. Unchanged.
2. Upload-verify load: if user has a default setup, fill-empty-only
   apply runs. EXIF-derived `camera` is preserved; any setup column
   that EXIF didn't fill is written. `setup_id` is set.
3. User reviews the form, optionally changes the setup (overwrite mode
   with confirm), edits fields by hand, and submits.

EXIF wins in conflict because of fill-empty-only mode at default
auto-apply. The user can still force the setup's value over EXIF by
manually selecting the same setup from the dropdown (which uses
overwrite mode and confirms).

## Type generation

`equipment_setups`, `setup_items`, the apply-setup body, and the
extended `equipment_items.kind` are exposed via `ts-rs`. After backend
changes, `just types` regenerates `frontend/src/lib/api/types.ts`. The
diff must be committed.

## Testing

Backend (Rust, testcontainers, fresh DB per test):

- Setup CRUD: create with items, read, update with replaced items,
  delete. Ownership enforcement (404 on cross-user access).
- `is_default` exclusivity: setting a second default clears the first
  in the same transaction; the partial unique index never trips on
  legitimate flows.
- `apply-setup` fill-empty: pre-fills `camera`, applies setup whose
  `main_camera` is different. Assert `camera` unchanged, other empty
  columns filled, `setup_id` set.
- `apply-setup` overwrite: same fixture, mode=overwrite. All columns
  written.
- `detach-setup`: setup_id null, columns untouched.
- FK delete semantics: delete a setup, assert photos' `setup_id`
  becomes null and equipment columns are unchanged. Delete an owner,
  assert their setups are gone and their photos remain (with null
  `setup_id`).
- `POST /api/equipment/items` resolve-or-create: identical lowercased
  name returns the existing row without incrementing `usage_count`;
  new name inserts at `usage_count = 0`.
- `kind` constraint: insert with `focal_modifier` succeeds. Insert
  with `guiding` still succeeds (legacy compatibility). Insert with
  unknown kind → DB error surfaced as 422.

Frontend (Playwright, against running backend):

- Create a setup, see it in the list, set as default.
- Upload a photo: default setup auto-applied, equipment fields
  pre-filled, `setup_id` written.
- Change setup mid-form: confirm dialog appears when a field would be
  overwritten.
- Edit a published photo: same picker, same semantics.

## Out of scope (deferred)

The following are explicitly out of scope and addressed in separate
specs as they become priorities:

- Public profile section listing a user's setups.
- Browse-by-setup page (`/equip/setup/<id>` or similar).
- Admin merge / rename tooling for `equipment_items` duplicates.
- Canonical specs on `equipment_items` (`aperture_mm`,
  `focal_length_mm`, `sensor`).
- Image attached to a setup (cover photo).
- Per-item notes inside `setup_items` (e.g., "this scope tonight at
  full focal length, no reducer").
- Sharing or forking another user's setup.
- Additional roles: focuser, filter_wheel, OAG, computer/ASIAir,
  rotator, power. The role check constraint may be extended later
  without table changes.
- Filter wheel positions / explicit ordering on `setup_items`.

## Open questions

None blocking. Items to confirm during implementation review:

- Whether the autocomplete UI should surface a visible "create new"
  affordance, or quietly do resolve-or-create on submit. UX decision,
  not a schema decision.
- Whether to expose `usage_count` to the autocomplete client as a
  sort hint (already used server-side, surfacing it client-side may or
  may not improve perceived quality).

## References

- `backend/migrations/0012_equipment_items.sql` — the existing
  `equipment_items` dictionary this spec builds on.
- `backend/src/equipment/upsert.rs`,
  `backend/src/equipment/autocomplete.rs` — the existing equipment
  module.
- `backend/src/photos/metadata.rs` — where EXIF + manual patches fan
  out into the photo's denormalized equipment columns. The `setup_id`
  / apply-setup logic lives alongside it.
- `docs/superpowers/specs/2026-05-03-photographer-showcase-design.md`
  — the discovery / browse-page surface this feature feeds into.
