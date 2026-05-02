-- Phase 7: appreciations, comments, follows.

create table appreciations (
    user_id    uuid not null references users(id) on delete cascade,
    photo_id   uuid not null references photos(id) on delete cascade,
    created_at timestamptz not null default now(),
    primary key (user_id, photo_id)
);
create index appreciations_photo_id_idx on appreciations (photo_id);

create table comments (
    id          uuid primary key default gen_random_uuid(),
    photo_id    uuid not null references photos(id) on delete cascade,
    author_id   uuid not null references users(id) on delete cascade,
    body        text not null check (length(body) between 1 and 2000),
    created_at  timestamptz not null default now()
);
create index comments_photo_created_idx on comments (photo_id, created_at);

create table follows (
    follower_id uuid not null references users(id) on delete cascade,
    followed_id uuid not null references users(id) on delete cascade,
    created_at  timestamptz not null default now(),
    primary key (follower_id, followed_id),
    check (follower_id <> followed_id)
);
create index follows_followed_idx on follows (followed_id);
