-- 0009 photo featured pinning + category + per-photo equipment fields.
-- These columns sit alongside the existing `camera`/`lens` columns from
-- 0001. `scope`/`mount`/`filters`/`guiding` are user-entered (no EXIF
-- source). `category` is a small fixed taxonomy.

alter table photos
    add column featured_at       timestamptz,
    add column featured_position smallint
        check (featured_position is null or featured_position between 1 and 6),
    add column category          text
        check (category is null or category in
               ('dso','planetary','lunar','solar','wide_field','nightscape','other')),
    add column scope             text,
    add column mount             text,
    add column filters           text,
    add column guiding           text;

-- One photo per slot per owner.
create unique index photos_featured_per_user_uidx
    on photos (owner_id, featured_position)
    where featured_at is not null;

-- featured_position must be set when featured_at is set, and vice versa.
alter table photos
    add constraint photos_featured_pair_chk
        check ((featured_at is null) = (featured_position is null));
