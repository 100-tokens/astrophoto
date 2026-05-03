-- 0005 handles: required @handle on users + redirect history.
-- Existing users get an auto-generated placeholder ('u-' + 6 chars of
-- their UUID) so the NOT NULL constraint is satisfied; a banner on
-- next login will prompt them to pick a real handle.

alter table users
    add column handle citext;

update users
    set handle = 'u-' || left(replace(id::text, '-', ''), 6)
    where handle is null;

alter table users
    alter column handle set not null,
    add constraint users_handle_format_chk
        check (handle ~ '^[a-z0-9_-]{3,30}$' or handle ~ '^u-[a-f0-9]{6}$');

create unique index users_handle_uidx on users (handle);

-- Old-handle redirects, written when a user renames their handle.
-- The old handle becomes reservable again 90 days after `released_at`.
create table handle_redirects (
    old_handle  citext primary key,
    user_id     uuid not null references users(id) on delete cascade,
    released_at timestamptz not null
);
create index handle_redirects_user_idx on handle_redirects (user_id);
