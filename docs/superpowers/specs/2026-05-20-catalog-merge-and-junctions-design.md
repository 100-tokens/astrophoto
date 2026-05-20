# Catalog merge tooling + equipment junctions — design

**Date:** 2026-05-20
**Status:** Spec — deferred follow-up from catalog coherence audit
**Author:** verify-page smoke session

## Context

The equipment catalog audit (May 2026) surfaced eight coherence issues. Five
were shipped in `fix/catalog-coherence` (sync VALID_KINDS, normalize
canonical_name, recompute usage_count, propagate submitted_by, bump on
apply-setup) and one in `fix/filter-specs-required` (force filter_type +
bandwidth_nm at create time). Two are deferred:

- **#3 — Merge / dedup tooling for catalog drift.** No mechanism today to
  collapse `"Sky-Watcher Esprit 100 ED"` + `"SkyWatcher Esprit 100ED"` into
  one canonical row, even though the `equipment_items.status = 'merged'`
  enum is already provisioned in the schema.
- **#7 — Junction tables for camera / telescope / mount / focal_modifier.**
  Today only `photo_filters` is a typed FK junction. The other four kinds
  live as freetext columns on `photos`, denormalized. A rename in the
  catalog (or a merge per #3) cannot propagate to existing photos because
  there's no FK to update.

Both are real, neither is urgent. The freetext columns work; catalog drift
on staging is bounded (12 items, no observed duplicates yet). But once
real users start uploading, the noise will compound. This spec captures
the design so we can act when the data justifies it.

---

## #3 — Catalog merge tooling

### Goals

- Pick two `equipment_items` of the same `kind`, declare one the canonical,
  collapse the other into it.
- Preserve every reference (setup_items, photo_filters) — they must point
  at the survivor.
- Sum `usage_count` (or just recompute via the helper added in
  `fix/catalog-coherence`).
- Keep a paper trail — set `status='merged'` and add a
  `merged_into_id uuid references equipment_items(id)` column on the
  losing row, so the row stays for FK history but is hidden from
  autocomplete.

### Non-goals

- Bulk auto-merge by similarity. Out of scope. Phase 1 is one-at-a-time
  manual merge by an admin or the item's `submitted_by`.
- Reverse merge / split. A merge is one-way.

### Schema change

```sql
alter table equipment_items
  add column merged_into_id uuid references equipment_items(id);

-- Hide merged rows from autocomplete + listing
create index equipment_items_visible_idx
  on equipment_items (kind, canonical_name)
  where merged_into_id is null;
```

### API

```
POST /api/equipment/items/:loser_id/merge
Body: { into: <winner_id> }
Auth: caller must be loser_id.submitted_by OR have a future admin role.
Both items must share the same kind.

Behavior in one transaction:
  - update photo_filters set item_id = winner where item_id = loser
  - update setup_items set item_id = winner where item_id = loser
  - update photos
       set camera = winner.display_name
     where camera = loser.display_name and kind=camera  -- and similar
       (only for freetext kinds — once #7 ships this clause goes away)
  - call recompute_usage(winner)
  - update equipment_items
       set merged_into_id = $winner, status = 'merged'
     where id = $loser
Returns 200 { winner_id }.
```

### UI

A new admin/owner-only page `/settings/equipment/merge`. Two
autocomplete pickers ("merge this …", "into this …"). Confirm button
shows a preview of how many photos / setups will be re-pointed and which
display_name survives.

Out of phase-1 scope. Build the endpoint first; expose UI later.

### Why not now

Until #7 ships, merging only resolves filters cleanly. Camera / scope /
mount merges need to walk every photos row by `display_name` string match,
which is O(photos × items) per merge and racy against concurrent edits. Do
the junction tables first.

---

## #7 — Equipment junctions for camera / scope / mount / focal_modifier

### Goals

- Replace the freetext `photos.camera` / `.scope` / `.mount` /
  `.focal_modifier` columns with FK references to `equipment_items.id`.
- Make rename / merge in the catalog automatically propagate to every
  photo that referenced the renamed item.
- Keep `.guiding` as freetext for now — it's the only kind that catalogues
  values like "unguided" / "ZWO ASI120MM via OAG", neither of which feels
  catalog-shaped. Junction is overkill there.

### Schema migration

One new junction table mirrors `setup_items`:

```sql
create table photo_equipment (
    photo_id   uuid    not null references photos(id)           on delete cascade,
    role       text    not null check (role in (
                                 'main_camera','optical_tube',
                                 'mount','focal_modifier')),
    item_id    uuid    not null references equipment_items(id)  on delete restrict,
    primary key (photo_id, role)
);
create index photo_equipment_item_idx on photo_equipment (item_id);
```

Backfill, in the same migration:

```sql
-- For each non-null freetext field, resolve canonical_name to item_id
-- and insert. Drift-handling: if no catalog row exists for the freetext
-- value (rare — every freetext write goes through upsert.rs), the
-- backfill creates one in the same transaction, with submitted_by
-- inferred from the photo's owner.
insert into equipment_items (kind, canonical_name, display_name,
                             usage_count, submitted_by, approved_at)
select 'camera', equipment::normalize_canonical(p.camera), p.camera, 0, p.owner_id, now()
  from photos p
 where p.camera is not null and p.camera <> ''
on conflict (kind, canonical_name) do nothing;

insert into photo_equipment (photo_id, role, item_id)
select p.id, 'main_camera', ei.id
  from photos p
  join equipment_items ei
    on ei.kind = 'camera'
   and ei.canonical_name = equipment::normalize_canonical(p.camera)
 where p.camera is not null and p.camera <> '';
-- … similar for scope/mount/focal_modifier
```

Then DROP the freetext columns on a follow-up migration after one release
window (so rollback stays trivial inside the deploy window). Optional:
keep them as denormalized cache like `photos.filters` already does, and
have a `filters_cache::rebuild`-style helper write the display_name into
the column from the junction. Decision deferred to implementation time.

### Code changes

- `backend/src/photos/queries.rs::PhotoRow` — replace the four
  `Option<String>` fields with `Option<Uuid>` item IDs and join
  `equipment_items` in selects to return `display_name`.
- `backend/src/photos/metadata.rs` PUT — accept `camera_item_id`,
  `scope_item_id`, `mount_item_id`, `focal_modifier_item_id` instead of
  freetext. The verify form already calls `POST /api/equipment/items` to
  resolve-or-create the catalog row first, so this is mostly a payload
  rename.
- `backend/src/photos/apply_setup.rs` — write to `photo_equipment` instead
  of the four columns. The CASE expression collapses to a single upsert
  per role.
- Frontend `verify-form/AcquisitionGrid.svelte` (and the legacy verify
  page if not yet retired) — the `<EquipmentAutocomplete>` already
  exposes the canonical item id (it just doesn't get used). Pass it
  through in the FormData instead of the freetext.
- DTO `PhotoDetail` in `backend/src/photos/get.rs` — return the joined
  `display_name` AND the `item_id` so the verify form can re-render the
  chip + know what to send back unchanged.

### Why not now

Hard reasons:

1. Two-step migration (insert junction rows, then drop columns) needs a
   release-window cushion. We don't have that operational maturity yet.
2. Apply-setup is the most-tested flow and has subtle conflict-detection
   logic; rewriting it to junction-write changes the transaction shape.
   Better to land it on a release cycle that's otherwise quiet.
3. Without merge tooling (#3 finalized), a junction write that uses a
   user-typed canonical that matches an existing slightly-different row
   would create a new orphan junction row. Need #3 first.

Soft reasons:

- The frontend equipment autocomplete UI already passes through the user's
  typed string. To send `item_id` instead, the form contract changes —
  small but needs care.

### Sequencing

1. Ship `fix/catalog-coherence` (this round).
2. Ship `fix/filter-specs-required` (this round).
3. Watch the catalog for two months. If duplicate rate < 5%, no urgency.
   If > 10%, build merge tooling (#3) first, validate the migrate-references
   side of it on real data.
4. Then design and ship #7 in a quiet sprint.

---

## Out of this spec

- Camera / scope / mount spec sub-tables (`telescope_specs`, etc.) are
  already typed and FK'd correctly. No change.
- The legacy `photos.filters` cache string is already handled by
  `crate::photos::filters_cache::rebuild`. The same pattern applies to
  the freetext columns once junctions ship.
- Item deletion. Today FKs are `on delete restrict` for setups and
  filter junctions — same intent for the new `photo_equipment` table.
  Deleting a catalog item with non-zero usage is forbidden; the
  merge-tool flow is the only sanctioned way to retire a row.
