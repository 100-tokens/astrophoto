# `/t` "Optimal now" sort + size & multi-type filters

Date: 2026-05-30
Status: approved (brainstorming)
Builds on: the opposition feature (PR #79) — `targets.opposition_doy`,
`crate::discovery::opposition`.

## Goal

Help a photographer find objects that are **optimal to image right now** and
that **suit their gear**, by reworking the `/t` catalog's opposition sort and
adding two filters. Replaces the calendar-order "Opposition (Jan → Dec)" sort,
which ordered by absolute opposition date rather than by what is well-placed
tonight.

## Changes

### 1. Sort: replace `opposition` (Jan→Dec) with `optimal` (near opposition now)

- URL/sort value renames `opposition` → `optimal`; UI label "Optimal now".
  Safe to rename: the previous value shipped only to staging.
- Reuses `targets.opposition_doy` — **no schema change**.
- The handler computes today's day-of-year (`chrono::Utc::now().ordinal()`,
  clamped to `1..=365`) and orders by the **circular distance** to each
  object's opposition, ascending:

  ```
  least(abs(opposition_doy - $today), 365 - abs(opposition_doy - $today))
  ```

  Range `0..=182`. Distance 0 = at opposition today (transits at midnight,
  up all night). Symmetric: 2 weeks before opposition ties 2 weeks after —
  both well-placed tonight. NULL `opposition_doy` (unknown RA) → `coalesce`
  to the existing `9999` sentinel → sorts last.
- Keyset cursor changes from day-of-year to the computed distance:
  `OptimalCursor { dist: i32, id: Uuid }`, keyset
  `(coalesce(<dist>, 9999), id) > ($cur_dist, $cur_id)`.
- `today` is per-request; within a page run it is stable, so pagination is
  consistent. Across a day boundary the distance shifts by ~1 — acceptable
  for a "best now" ordering (documented in code).

### 2. Size filter (focal-length-hinted buckets)

A new single-select "Size" control (buckets by the object's **major axis**):

| Label      | major axis | focal-length hint |
|------------|-----------:|-------------------|
| All        | —          | —                 |
| Very large | > 60′      | < 400 mm          |
| Large      | 30–60′     | 400–800 mm        |
| Medium     | 10–30′     | 700–1500 mm       |
| Small      | 2–10′      | 1500 mm+          |

- Frontend maps the chosen bucket to `size_min` / `size_max` query params (a
  shareable `?size=<bucket>` lives in the page URL; the load fn translates it).
- Backend applies, to **every** sort branch:
  `($min::real is null or t.major_axis_arcmin >= $min)` and
  `($max::real is null or t.major_axis_arcmin < $max)`.
- Objects with NULL `major_axis_arcmin` are excluded when a bucket is active
  (cannot be confirmed to match). The hints are label text only — no sensor /
  FOV math, no tie to the user's saved equipment.

### 3. Type filter → multi-select

- The single `<select>` becomes a row of toggle chips (Galaxy, Nebula, Open
  cluster, Globular cluster, Planetary nebula, HII region, Supernova remnant).
- Selected types are sent comma-joined (`object_type=G,Neb,OCl`); the backend
  splits and matches `t.object_type = any($n::text[])`. Empty = all (today's
  behaviour).

## Unchanged

- `opposition_doy` column, boot backfill, seed-writer cache contract.
- The opposition date shown on cards / detail ("◐ Opp · early Oct"). When
  sorted by "Optimal now", the top cards naturally read the current month, so
  the existing display self-documents the sort.
- `popular` (default) and `name` sorts — both gain the new size / multi-type
  filters but keep their ordering and cursors.

## Out of scope (easy future adds, not requested)

- Tie the size filter to the user's saved telescope/setup focal length.
- A "days until / since opposition" badge on cards.
- A precise FOV/plate-scale filter from sensor + focal length.

## Testing

- **Pure unit test** (no DB): circular-distance helper — distance 0 at
  opposition, symmetric, wraps the year (e.g. doy 5 vs doy 360 → 10, not 355),
  max 182.
- **Live psql** (rolled back): the `optimal` ORDER BY + keyset, the size-range
  WHERE, and the multi-type `= any(...)` clause on synthetic rows.
- `just check` (clippy/fmt/svelte-check/eslint) on changed files; no DTO
  changes → no `just types`. `cargo sqlx prepare` after the query edits.
- Staging smoke test (browser) as for PR #79.
