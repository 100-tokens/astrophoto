-- 0008 user profile fields used by the hero page (P2 of showcase).
-- Schema only — no UI yet.

alter table users
    add column tagline             text,
    add column bio_html            text,
    add column cover_photo_id      uuid references photos(id) on delete set null,
    add column equipment_telescope text,
    add column equipment_camera    text,
    add column equipment_mount     text,
    add column equipment_filters   text,
    add column equipment_guiding   text,
    add column location_text       text,
    add column bortle_class        smallint
        check (bortle_class is null or bortle_class between 1 and 9),
    add column sqm                 numeric(4,2),
    add column social_links        jsonb not null default '[]'::jsonb;
