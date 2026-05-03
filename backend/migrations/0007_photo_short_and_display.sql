-- 0007 photo permalink + display master + blurhash + content hash.
-- short_id is filled by the application on insert (8-char base62);
-- existing rows get a backfill from a deterministic UUID hash.

alter table photos
    add column short_id        text,
    add column display_key     text,
    add column original_hash   text,
    add column blurhash        text;

-- Backfill short_id for existing rows. Deterministic mapping from
-- the photo UUID's first 6 bytes -> base62, padded to 8.
update photos
    set short_id = upper(left(replace(id::text, '-', ''), 8));

alter table photos
    alter column short_id set not null;

create unique index photos_short_id_uidx on photos (short_id);

-- original_hash is per-owner unique to dedup re-uploads.
create unique index photos_owner_hash_uidx
    on photos (owner_id, original_hash)
    where original_hash is not null;
