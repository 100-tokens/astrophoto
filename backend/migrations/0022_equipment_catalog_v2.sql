-- 0022 equipment catalog v2
--
-- Phase 1 of the catalog-v2 rollout: structured brand/model/variant on
-- the shared equipment_items header + per-spec completeness columns
-- across telescope/camera/mount/filter/focal_modifier, plus a new
-- guiding_specs sub-table. Frontend stays untouched; new columns are
-- nullable except brand/model on equipment_items, which are NOT NULL
-- after a one-shot whitelist-driven backfill. Reversible via DROP +
-- ALTER … DROP COLUMN of the 5 specs tables.

-- ── 1. Brand/model/variant header columns ───────────────────────────
alter table equipment_items
  add column brand   text,
  add column model   text,
  add column variant text;

-- ── 2. Backfill: first-word-as-brand against a whitelist of known
--      brands; else brand='' and model=display_name (flagged for
--      moderation later). Whitelist covers the top brands per kind;
--      extend later via merge tooling. Match is case-insensitive and
--      tolerates the common hyphen/space variants for Sky-Watcher.
update equipment_items set
  brand = case
    when lower(display_name) like 'sky-watcher %' or lower(display_name) like 'skywatcher %' or lower(display_name) like 'sky watcher %'
      then 'Sky-Watcher'
    when lower(display_name) like 'zwo %' then 'ZWO'
    when lower(display_name) like 'celestron %' then 'Celestron'
    when lower(display_name) like 'takahashi %' then 'Takahashi'
    when lower(display_name) like 'vixen %' then 'Vixen'
    when lower(display_name) like 'ioptron %' then 'iOptron'
    when lower(display_name) like 'astronomik %' then 'Astronomik'
    when lower(display_name) like 'optolong %' then 'Optolong'
    when lower(display_name) like 'antlia %' then 'Antlia'
    -- "Baader Planetarium" must beat plain "Baader" — order matters here.
    when lower(display_name) like 'baader planetarium %' then 'Baader'
    when lower(display_name) like 'baader %' then 'Baader'
    when lower(display_name) like 'player one %' then 'Player One'
    when lower(display_name) like 'touptek %' then 'Touptek'
    when lower(display_name) like 'qhy%' then 'QHY'  -- QHYCCD too (no space after prefix)
    when lower(display_name) like 'william optics %' then 'William Optics'
    when lower(display_name) like 'tele vue %' or lower(display_name) like 'televue %' then 'Tele Vue'
    when lower(display_name) like 'meade %' then 'Meade'
    when lower(display_name) like 'orion %' then 'Orion'
    when lower(display_name) like 'astro-tech %' or lower(display_name) like 'astro tech %' then 'Astro-Tech'
    when lower(display_name) like 'losmandy %' then 'Losmandy'
    when lower(display_name) like 'astro-physics %' or lower(display_name) like 'astro physics %' then 'Astro-Physics'
    when lower(display_name) like 'paramount %' then 'Paramount'
    when lower(display_name) like 'chroma %' then 'Chroma'
    when lower(display_name) like 'astrodon %' then 'Astrodon'
    when lower(display_name) like 'idas %' then 'IDAS'
    when lower(display_name) like 'canon %' then 'Canon'
    when lower(display_name) like 'nikon %' then 'Nikon'
    when lower(display_name) like 'sony %' then 'Sony'
    when lower(display_name) like 'fuji%' then 'Fujifilm'
    when lower(display_name) like 'pentax %' then 'Pentax'
    else ''  -- unknown brand; admin merge later
  end,
  model = case
    -- "Baader Planetarium" first so its longer prefix wins over "Baader".
    when lower(display_name) like 'baader planetarium %' then substring(display_name from 20)
    when lower(display_name) like 'sky-watcher %' then substring(display_name from 13)
    when lower(display_name) like 'skywatcher %'  then substring(display_name from 12)
    when lower(display_name) like 'sky watcher %' then substring(display_name from 13)
    when lower(display_name) like 'zwo %'         then substring(display_name from 5)
    when lower(display_name) like 'celestron %'   then substring(display_name from 11)
    when lower(display_name) like 'takahashi %'   then substring(display_name from 11)
    when lower(display_name) like 'vixen %'       then substring(display_name from 7)
    when lower(display_name) like 'ioptron %'     then substring(display_name from 9)
    when lower(display_name) like 'astronomik %'  then substring(display_name from 12)
    when lower(display_name) like 'optolong %'    then substring(display_name from 10)
    when lower(display_name) like 'antlia %'      then substring(display_name from 8)
    when lower(display_name) like 'baader %'      then substring(display_name from 8)
    when lower(display_name) like 'player one %'  then substring(display_name from 12)
    when lower(display_name) like 'touptek %'     then substring(display_name from 9)
    when lower(display_name) like 'qhy%'          then display_name
    when lower(display_name) like 'william optics %' then substring(display_name from 16)
    when lower(display_name) like 'tele vue %'    then substring(display_name from 10)
    when lower(display_name) like 'televue %'     then substring(display_name from 9)
    when lower(display_name) like 'meade %'       then substring(display_name from 7)
    when lower(display_name) like 'orion %'       then substring(display_name from 7)
    when lower(display_name) like 'astro-tech %'  then substring(display_name from 12)
    when lower(display_name) like 'astro tech %'  then substring(display_name from 12)
    when lower(display_name) like 'losmandy %'    then substring(display_name from 10)
    when lower(display_name) like 'astro-physics %' then substring(display_name from 15)
    when lower(display_name) like 'astro physics %' then substring(display_name from 15)
    when lower(display_name) like 'paramount %'   then substring(display_name from 11)
    when lower(display_name) like 'chroma %'      then substring(display_name from 8)
    when lower(display_name) like 'astrodon %'    then substring(display_name from 10)
    when lower(display_name) like 'idas %'        then substring(display_name from 6)
    when lower(display_name) like 'canon %'       then substring(display_name from 7)
    when lower(display_name) like 'nikon %'       then substring(display_name from 7)
    when lower(display_name) like 'sony %'        then substring(display_name from 6)
    when lower(display_name) like 'fuji%'         then display_name  -- often "Fujifilm X-T5" — keep full
    when lower(display_name) like 'pentax %'      then substring(display_name from 8)
    else display_name
  end;

-- Strip leading/trailing whitespace introduced by the substring trick.
update equipment_items set brand = trim(brand), model = trim(model);

-- ── 3. Detect any unique conflicts that would surface from
--      regenerating canonical_name for known-brand rows. If two rows
--      collapse to the same canonical, the merge tool (separate spec)
--      must run before this migration. Fail loud rather than silently
--      letting the unique index reject one of the regenerated rows.
do $$
declare
  conflict_count int;
begin
  select count(*) into conflict_count from (
    select kind,
           lower(trim(brand) || ' ' || trim(model) || coalesce(' ' || trim(variant), '')) as new_canonical,
           count(*) as n
      from equipment_items
     where brand <> ''
     group by 1, 2
    having count(*) > 1
  ) dups;
  if conflict_count > 0 then
    raise exception
      'equipment catalog v2: % canonical_name conflicts from brand/model regeneration. Run the merge tool first.',
      conflict_count;
  end if;
end $$;

-- ── 4. Regenerate canonical_name for known-brand rows only. Rows where
--      brand='' (unknowns) keep their existing canonical_name so the
--      backfill is a no-op for them and the existing `(kind, canonical)`
--      uniqueness is preserved verbatim.
update equipment_items
   set canonical_name = lower(trim(brand) || ' ' || trim(model)
                              || coalesce(' ' || trim(variant), ''))
 where brand <> '';

-- ── 5. Enforce brand/model NOT NULL going forward. brand='' is the
--      sentinel for "unknown brand"; model is always populated.
alter table equipment_items alter column brand set not null;
alter table equipment_items alter column model set not null;

-- ── 6. Per-spec self_weight + completeness columns ──────────────────
alter table telescope_specs
  add column self_weight_kg     numeric(5,2) check (self_weight_kg > 0 and self_weight_kg <= 200),
  add column optical_length_mm  int          check (optical_length_mm > 0),
  add column backfocus_mm       numeric(4,1) check (backfocus_mm > 0);

alter table camera_specs
  add column self_weight_g            int    check (self_weight_g > 0 and self_weight_g <= 5000),
  add column full_well_capacity_e     int    check (full_well_capacity_e > 0),
  add column read_noise_e             numeric(4,2) check (read_noise_e >= 0),
  add column mount_thread             text,
  add column backfocus_mm             numeric(4,1) check (backfocus_mm > 0);

alter table mount_specs
  add column self_weight_kg          numeric(5,2) check (self_weight_kg > 0 and self_weight_kg <= 100),
  add column periodic_error_arcsec   numeric(4,1) check (periodic_error_arcsec >= 0),
  add column tripod_included         boolean,
  add column control_protocol        text;

alter table filter_specs
  add column mounted_diameter_mm     numeric(5,2) check (mounted_diameter_mm > 0),
  add column thickness_mm            numeric(3,2) check (thickness_mm > 0),
  add column peak_transmission_pct   numeric(4,1) check (peak_transmission_pct >= 0 and peak_transmission_pct <= 100);

alter table focal_modifier_specs
  add column self_weight_g       int          check (self_weight_g > 0 and self_weight_g <= 2000),
  add column backfocus_mm        numeric(4,1) check (backfocus_mm > 0),
  add column image_circle_mm     numeric(4,1) check (image_circle_mm > 0);

-- ── 7. New table for guiding ────────────────────────────────────────
create table guiding_specs (
  item_id           uuid primary key references equipment_items(id) on delete cascade,
  setup_kind        text not null check (setup_kind in ('oag','guidescope','oag_prism','other')),
  guide_focal_mm    int check (guide_focal_mm > 0 and guide_focal_mm <= 1000),
  guide_aperture_mm int check (guide_aperture_mm > 0 and guide_aperture_mm <= 300),
  guide_camera      text
);
