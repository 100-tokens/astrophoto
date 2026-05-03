-- 0011 appreciations_count: denormalised counter on photos.
-- Backfills from the existing appreciations table (Phase 7).
-- Application code (POST /appreciate, DELETE /appreciate) maintains
-- this counter transactionally.

alter table photos
    add column appreciations_count integer not null default 0;

update photos p
    set appreciations_count = (
        select count(*) from appreciations a where a.photo_id = p.id
    );

-- Index: most-appreciated sort across all published photos.
create index photos_published_popular_idx
    on photos (appreciations_count desc, published_at desc, id desc)
    where published_at is not null;
