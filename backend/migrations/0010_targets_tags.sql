-- 0010 discovery primitives: targets, tags, and the join tables.
-- photo_targets carries `source` so a future plate-solve job can write
-- rows without schema churn.

create table targets (
    id              uuid primary key default gen_random_uuid(),
    slug            text unique not null,
    canonical_name  text not null,
    aliases         text[] not null default '{}',
    kind            text not null
        check (kind in ('messier','ngc','ic','caldwell','common','other'))
);
create index targets_aliases_gin_idx on targets using gin (aliases);

create table photo_targets (
    photo_id   uuid not null references photos(id) on delete cascade,
    target_id  uuid not null references targets(id) on delete cascade,
    source     text not null check (source in ('manual','plate_solve')),
    confidence numeric,
    is_primary boolean not null default false,
    created_at timestamptz not null default now(),
    primary key (photo_id, target_id)
);
create index photo_targets_target_idx on photo_targets (target_id, photo_id);

create table tags (
    id   uuid primary key default gen_random_uuid(),
    slug text unique not null,
    name text not null
);
create table photo_tags (
    photo_id uuid not null references photos(id) on delete cascade,
    tag_id   uuid not null references tags(id) on delete cascade,
    primary key (photo_id, tag_id)
);
create index photo_tags_tag_idx on photo_tags (tag_id, photo_id);

-- Seed: Messier 1..110 plus a few popular NGC/IC objects.
-- Generate Messier rows from a series.
insert into targets (slug, canonical_name, aliases, kind)
select
    'm' || g,
    'Messier ' || g,
    array['M' || g, 'Messier ' || g],
    'messier'
from generate_series(1, 110) g
on conflict (slug) do nothing;

-- Common-name overrides for the high-traffic ones.
update targets set canonical_name = 'Andromeda Galaxy', aliases = aliases || array['NGC 224']
    where slug = 'm31';
update targets set canonical_name = 'Orion Nebula',     aliases = aliases || array['NGC 1976']
    where slug = 'm42';
update targets set canonical_name = 'Triangulum Galaxy', aliases = aliases || array['NGC 598']
    where slug = 'm33';
update targets set canonical_name = 'Whirlpool Galaxy', aliases = aliases || array['NGC 5194']
    where slug = 'm51';
update targets set canonical_name = 'Pleiades',         aliases = aliases || array['Seven Sisters', 'NGC 1432']
    where slug = 'm45';
update targets set canonical_name = 'Dumbbell Nebula',  aliases = aliases || array['NGC 6853']
    where slug = 'm27';
update targets set canonical_name = 'Hercules Cluster', aliases = aliases || array['NGC 6205']
    where slug = 'm13';

-- A handful of very-popular NGC/IC.
insert into targets (slug, canonical_name, aliases, kind) values
    ('ngc-7000', 'North America Nebula', array['NGC 7000','Caldwell 20'], 'ngc'),
    ('ngc-6960', 'Western Veil Nebula',  array['NGC 6960','Witch''s Broom'], 'ngc'),
    ('ngc-2237', 'Rosette Nebula',       array['NGC 2237','Caldwell 49'], 'ngc'),
    ('ngc-281',  'Pacman Nebula',        array['NGC 281'], 'ngc'),
    ('ngc-3324', 'Cosmic Cliffs',        array['NGC 3324'], 'ngc'),
    ('ic-1805',  'Heart Nebula',         array['IC 1805','Sharpless 2-190'], 'ic'),
    ('ic-1396',  'Elephant''s Trunk',    array['IC 1396'], 'ic'),
    ('ic-434',   'Horsehead Nebula',     array['IC 434','Barnard 33'], 'ic')
on conflict (slug) do nothing;
