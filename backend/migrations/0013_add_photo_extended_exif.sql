-- 0013 extended EXIF / acquisition columns on photos.
--
-- The design handoff (docs/design/handoff/screens-2.jsx, Verify-Data
-- screen, lines 167-180) lists Sessions, Aperture, Gain, and Sensor
-- temp alongside the columns introduced in 0001 (camera/iso/exposure
-- /focal). All four are user-entered on the Verify Data form and
-- optional. Aperture is also EXIF-recoverable from the standard
-- FNumber tag — the upload pipeline can pre-fill it; the others are
-- astro-camera-specific and don't have universal EXIF tags so they
-- remain manual entry.
--
-- All four are nullable. No backfill needed: existing rows keep NULL.

alter table photos
    add column sessions      smallint
        check (sessions is null or sessions > 0),
    add column aperture_f    real
        check (aperture_f is null or aperture_f > 0),
    add column gain          smallint
        check (gain is null or gain >= 0),
    add column sensor_temp_c real;
