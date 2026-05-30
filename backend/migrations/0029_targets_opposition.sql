-- 0029 add opposition / midnight-culmination date to targets.
--
-- `opposition_doy` is the day-of-year (1..365, non-leap reference calendar) on
-- which a fixed object reaches opposition — sitting opposite the Sun, so it
-- transits the meridian at local midnight: its best-observation date. It is a
-- denormalised cache of a pure function of `right_ascension`
-- (see crate::discovery::opposition). Nullable because not every target has a
-- known RA (manual seeds, M40/M45 cluster, etc.).
--
-- Cache contract (mirrors photos.filters / photo_filters): every writer of
-- right_ascension — the seed-targets and seed-pgc binaries — recomputes this in
-- the same statement, and crate::discovery::opposition::backfill_missing fills
-- it on boot for rows written before this column existed. No index: the catalog
-- is a few thousand rows and the sort already keyset-paginates over a full scan,
-- matching the existing `popular` sort.

alter table targets
  add column opposition_doy smallint;
