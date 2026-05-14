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

-- 8. Backfill photo_filters from photos.filters comma-joined string.
--    Best-effort: tokens that don't match any canonical_name are
--    silently dropped. The string cache stays as-is for those photos.
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
