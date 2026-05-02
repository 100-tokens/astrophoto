-- Phase 8b: drafts, replace tracking, pipeline error capture,
-- and a deferred-S3-delete table used by the replace endpoint to
-- avoid the data-loss window where a corrupt new master would leave
-- the user with no recoverable original.

-- 1. Drafts: published_at NULL = draft, NOT NULL = published.
--    Pipeline state (status) and publish state stay separate concerns.
alter table photos
  add column published_at timestamptz;

create index photos_published_at_idx on photos (published_at desc)
  where published_at is not null;

create index photos_drafts_owner_idx on photos (owner_id, created_at desc)
  where published_at is null;

create index photos_owner_published_idx on photos (owner_id, published_at desc)
  where published_at is not null;

-- Backfill: every existing 'ready' photo is considered published at its
-- creation time. 'processing' / 'failed' rows stay draft (NULL).
update photos set published_at = created_at where status = 'ready';

-- 2. Replace tracking.
alter table photos
  add column replaced_at timestamptz,
  add column original_uploaded_at timestamptz;

update photos set original_uploaded_at = created_at;
alter table photos alter column original_uploaded_at set not null;

-- 3. Track upload progress for the draft card chrome.
alter table photos
  add column last_step text
    check (last_step in ('upload', 'verify', 'caption'));

update photos set last_step = 'caption'
  where status = 'ready' and published_at is not null;
update photos set last_step = 'upload'
  where status in ('processing', 'failed');

-- 4. Pipeline error capture: written when the pipeline marks a row
--    'failed' so the verify-step UI can surface the reason and the
--    user can choose Discard vs Retry.
alter table photos
  add column pipeline_error text;

-- 5. Deferred S3 deletion table — populated by the replace endpoint,
--    drained by the pipeline on successful 'ready' transition or by
--    the hourly purge worker for rows older than 7 days.
create table photo_pending_deletes (
  id          bigserial primary key,
  photo_id    uuid not null references photos(id) on delete cascade,
  storage_key text not null,
  queued_at   timestamptz not null default now()
);

create index photo_pending_deletes_photo_idx
  on photo_pending_deletes (photo_id);

create index photo_pending_deletes_queued_idx
  on photo_pending_deletes (queued_at);
