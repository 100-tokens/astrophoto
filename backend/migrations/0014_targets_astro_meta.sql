-- 0014 add astronomical metadata to targets, populated by seed-targets binary
-- against pinned OpenNGC CSVs. All columns nullable: OpenNGC does not
-- cover every existing row (M40, M45 cluster, custom seeds).

alter table targets
  add column right_ascension   double precision,
  add column declination       double precision,
  add column magnitude_v       real,
  add column object_type       text,
  add column constellation     char(3),
  add column major_axis_arcmin real,
  add column minor_axis_arcmin real,
  add column updated_at        timestamptz not null default now();

create index targets_object_type_idx
    on targets (object_type)  where object_type is not null;
create index targets_constellation_idx
    on targets (constellation) where constellation is not null;
