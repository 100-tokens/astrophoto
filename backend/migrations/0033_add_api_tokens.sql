-- Personal access tokens for native clients (PixInsight plugin).
-- Only the SHA-256 of the secret is stored; `prefix` is the first
-- characters of the secret for display ("astrophoto_pat_AbCdE…").
create table api_tokens (
    id           uuid primary key default gen_random_uuid(),
    user_id      uuid not null references users(id) on delete cascade,
    name         text not null,
    token_hash   bytea not null unique,
    prefix       text not null,
    scope        text not null default 'publish',
    created_at   timestamptz not null default now(),
    last_used_at timestamptz,
    revoked_at   timestamptz
);

create index api_tokens_user_id_idx on api_tokens (user_id);
