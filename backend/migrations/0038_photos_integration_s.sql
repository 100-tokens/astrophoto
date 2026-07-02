-- Total integration time in seconds for the photo (sum over all stacked
-- subframes), decoded from the XISF header's PCL:TotalExposureTime at
-- calibration time, with EXPTIME × NCOMBINE as the parse-side fallback.
--
-- Distinct from `exposure_s` (single-sub exposure) × `sessions` (sub
-- count): many real-world master lights carry only the PCL total, and
-- the pair can't represent multi-filter stacks with unequal exposures.
ALTER TABLE photos
    ADD COLUMN integration_s DOUBLE PRECISION;
