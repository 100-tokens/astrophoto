-- 0014 equipment_setups: per-user reusable gear bundles, applied at
-- upload-verify to fill the photo's equipment columns. See
-- docs/superpowers/specs/2026-05-04-equipment-setups-design.md.

-- 1. Extend equipment_items.kind to include focal_modifier. The
--    existing 'guiding' value is intentionally retained: legacy rows
--    persist inert. Do not "clean up" the constraint.
alter table equipment_items
    drop constraint equipment_items_kind_check,
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
    guiding     text,
    created_at  timestamptz not null default now(),
    updated_at  timestamptz not null default now(),
    unique (owner_id, name)
);
create index equipment_setups_owner_idx
    on equipment_setups (owner_id, updated_at desc);

-- 3. At most one default per user — DB-enforced via partial unique idx.
create unique index equipment_setups_owner_default_uidx
    on equipment_setups (owner_id) where is_default;

-- 4. Setup ↔ canonical item junction. Composite PK on
--    (setup_id, role, item_id) allows multi-filter and prevents the
--    same item being added twice in the same role.
create table setup_items (
    setup_id  uuid not null references equipment_setups(id) on delete cascade,
    role      text not null
        check (role in ('optical_tube','focal_modifier','main_camera',
                        'mount','filter')),
    item_id   uuid not null references equipment_items(id) on delete restrict,
    primary key (setup_id, role, item_id)
);
create index setup_items_item_idx on setup_items (item_id);

-- Singleton-role enforcement: at most one item per role for all roles
-- except 'filter' (which is intentionally multi-cardinality for LRGB/SHO
-- wheels). The composite PK already prevents the same item appearing
-- twice in the same role; this index also forbids two different items
-- in a singleton role.
create unique index setup_items_singleton_role_uidx
    on setup_items (setup_id, role)
    where role in ('optical_tube','focal_modifier','main_camera','mount');

-- 5. Photo points back to the setup it originated from. on delete set
--    null because the photo's denormalized columns preserve historical
--    truth even after the setup is deleted.
alter table photos
    add column setup_id uuid references equipment_setups(id) on delete set null,
    add column focal_modifier text;

create index photos_setup_idx
    on photos (setup_id) where setup_id is not null;
create index photos_focal_modifier_lower_idx
    on photos (lower(focal_modifier))
    where published_at is not null and focal_modifier is not null;
