-- 0001 init: users, oauth identities, sessions, photos, thumbnails.

create extension if not exists pgcrypto;
create extension if not exists pg_trgm;
create extension if not exists citext;

create table users (
    id            uuid primary key default gen_random_uuid(),
    email         citext unique not null,
    password_hash text,
    display_name  text not null,
    created_at    timestamptz not null default now()
);

create table oauth_identities (
    user_id    uuid not null references users(id) on delete cascade,
    provider   text not null,
    subject    text not null,
    created_at timestamptz not null default now(),
    primary key (provider, subject)
);

create table sessions (
    id         bytea primary key,
    user_id    uuid not null references users(id) on delete cascade,
    expires_at timestamptz not null,
    created_at timestamptz not null default now(),
    user_agent text,
    ip         inet
);
create index sessions_user_id_idx   on sessions (user_id);
create index sessions_expires_at_idx on sessions (expires_at);

create table photos (
    id            uuid primary key default gen_random_uuid(),
    owner_id      uuid not null references users(id) on delete cascade,
    storage_key   text not null,
    original_name text not null,
    bytes         bigint not null,
    mime          text not null,
    width         int,
    height        int,
    -- EXIF (denormalized for query)
    taken_at      timestamptz,
    camera        text,
    lens          text,
    iso           int,
    exposure_s    double precision,
    focal_mm      double precision,
    -- Astro
    ra_deg        double precision,
    dec_deg       double precision,
    target        text,
    -- Raw + metadata
    exif_json     jsonb,
    caption       text,
    status        text not null default 'ready',
    created_at    timestamptz not null default now()
);
create index photos_owner_created_idx on photos (owner_id, created_at desc);
create index photos_caption_trgm_idx
    on photos using gin (caption gin_trgm_ops);
create index photos_target_idx on photos (target);

create table thumbnails (
    photo_id    uuid not null references photos(id) on delete cascade,
    size        int not null,
    storage_key text not null,
    bytes       bigint not null,
    primary key (photo_id, size)
);
