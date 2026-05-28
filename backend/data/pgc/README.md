# PGC catalog (HyperLEDA)

Source: <http://leda.univ-lyon1.fr/>

## Extraction recipe

Visit <http://leda.univ-lyon1.fr/leda/fullsql.html> and run:

```sql
select pgc, objname, ra2000, de2000, bt, logd25, logr25, pa
  from pgc
 where de2000 is not null
   and logd25 > 0
   and bt < 19;
```

Download the result as CSV and save it as `backend/data/pgc/pgc.csv` (or
`pgc.csv.gz` if larger than 50 MB — the seed binary auto-detects the
`.gz` extension via `flate2`).

Expected size: ~700–800 k rows, ~50 MB raw / ~15 MB gzipped.

## Citation (required)

> Makarov, D., Prugniel, P., Terekhova, N., Courtois, H., & Vauglin, I. 2014,
> *HyperLEDA III. The catalogue of extragalactic distances*,
> A&A, 570, A13.
> <https://doi.org/10.1051/0004-6361/201423496>

The app displays an attribution footer on `/t/<slug>` pages for `kind='pgc'`
rows (deferred follow-up, see spec §11).

## Refresh

Re-run the SQL above and replace `pgc.csv`. Then `just seed-pgc` is
idempotent and converges — manual `canonical_name` overrides are
preserved.

## Status

`pgc.csv` is **not** committed to this repo (file size, license terms).
Extract it locally before running `just seed-pgc`. On staging/prod, it
must be present in the container's `backend/data/pgc/` path at deploy
time — see the Phase 4 rollout steps in the implementation plan.
