-- 0021 plate-solve telemetry columns on `photos`.
--
-- The plate-solve service (xisf-rs-platesolve-server, deployed at
-- platesolve.astrophoto.pics) returns RA/Dec + ancillary metadata for
-- a calibrated XISF master. RA/Dec already exist on the table from
-- migration 0001 (originally surfaced manually via the verify form),
-- so we don't touch those — a successful solve simply overwrites them
-- and additionally stamps the columns below for diagnostic transparency
-- ("how good was the solve?", "when was it run?").
--
-- All columns are nullable: every existing photo predates plate-solve
-- support, and even after rollout most photos won't have an XISF master
-- to solve against. `platesolve_solved_at` doubles as the
-- "do we have a solve?" boolean — IS NOT NULL means yes, IS NULL with
-- a non-null `platesolve_error` means we tried and failed.

alter table photos
  add column platesolve_pixel_scale_arcsec real,
  add column platesolve_rotation_deg       real,
  add column platesolve_rms_arcsec         real,
  add column platesolve_matched_count      int,
  add column platesolve_detected_count     int,
  add column platesolve_solved_at          timestamptz,
  add column platesolve_error              text,
  -- Verbatim FITS keyword + PCL property payload from the solver,
  -- so consumers (PixInsight, AstroPixel Processor, an embed-back-into-
  -- XISF step) can re-materialize the full astrometric solution
  -- without re-running the solve.
  add column platesolve_embed_json         jsonb;

-- Sparse — only rows that have been solved get an index hit, which
-- is exactly what we want for "show me only photos with WCS data"
-- gallery filters.
create index photos_platesolve_solved_at_idx
    on photos (platesolve_solved_at desc)
    where platesolve_solved_at is not null;

-- "Where can we point a follow-up scope?" — bounded box queries by
-- RA/Dec are the natural follow-up. Existing `ra_deg`/`dec_deg`
-- columns get an index now that they're going to be populated
-- programmatically and queried geographically.
create index photos_radec_idx
    on photos (ra_deg, dec_deg)
    where ra_deg is not null and dec_deg is not null;
