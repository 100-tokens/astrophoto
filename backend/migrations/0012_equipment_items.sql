-- 0012 equipment_items: the lookup dictionary populated by upsert on
-- photo save. Powers the autocomplete in upload-verify (P1) and the
-- equipment browse pages /equip/<kind>/<slug> (P3).

create table equipment_items (
    id             uuid primary key default gen_random_uuid(),
    kind           text not null
        check (kind in ('telescope','camera','mount','filter','guiding')),
    canonical_name text not null,           -- lowercased, slug-friendly
    display_name   text not null,           -- as first seen
    usage_count    integer not null default 0,
    unique (kind, canonical_name)
);
create index equipment_items_kind_count_idx
    on equipment_items (kind, usage_count desc);

-- Discovery indexes on the per-photo equipment fields.
create index photos_camera_lower_idx
    on photos (lower(camera))  where published_at is not null;
create index photos_scope_lower_idx
    on photos (lower(scope))   where published_at is not null;
create index photos_mount_lower_idx
    on photos (lower(mount))   where published_at is not null;
create index photos_filters_lower_idx
    on photos (lower(filters)) where published_at is not null;
create index photos_guiding_lower_idx
    on photos (lower(guiding)) where published_at is not null;

-- Category browse + cursor.
create index photos_category_published_idx
    on photos (category, published_at desc, id desc)
    where published_at is not null;

-- "Newest" cursor (also used by /explore).
-- Note: a simpler photos_published_at_idx exists from 0004; this one
-- adds id as the tie-breaker required for stable cursor pagination.
create index photos_published_newest_idx
    on photos (published_at desc, id desc) where published_at is not null;
