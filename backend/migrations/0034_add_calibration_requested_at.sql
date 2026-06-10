-- 0034 track when an XISF photo entered `awaiting-calibration`.
--
-- `photos.created_at` / `replaced_at` are the wrong clock for the
-- calibration-timeout sweep: a user can re-run finalize on an old
-- failed row days after `created_at`, and the sweep would instantly
-- mark the fresh attempt as stale. A dedicated timestamp, written by
-- `mark_awaiting_calibration`, gives the sweep a precise start time.

alter table photos
    add column calibration_requested_at timestamptz;
