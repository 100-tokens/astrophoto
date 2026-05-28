# Celestial Identification + Overlay — D5 / D6 / Overlay UI

**Date:** 2026-05-28
**Status:** Draft — pending written-spec review
**Author:** Pascal (with Claude)
**Predecessor:** [2026-05-06-celestial-objects-design.md](2026-05-06-celestial-objects-design.md)

## Goal

For every plate-solved photo, automatically identify the celestial
objects whose coordinates fall inside the field of view, persist that
identification, and render it on the public photo page as a color-coded
overlay on the image plus a structured side panel — with click-through
to per-object detail (mini Aladin Lite embed) and to the existing
`/t/<slug>` pages.

This is the explicit continuation of the predecessor spec, which
deferred:

- **D5** — plate-solve → `photo_targets` writes with `source='plate_solve'`,
- **D6** — cone search in RA/Dec,
- *(implicit)* the overlay UI itself.

This spec also adds **PGC (HyperLEDA)** galaxies to the catalog so that
dense galaxy fields (Virgo Cluster, Coma Cluster, etc.) get useful
identifications — OpenNGC alone is too sparse for that.

## Why now

The infrastructure is mostly in place:

- `targets` carries `right_ascension`, `declination`, `magnitude_v`,
  `object_type`, `constellation`, `major_axis_arcmin`,
  `minor_axis_arcmin` (migration 0014, ~14k OpenNGC rows seeded).
- `photo_targets` was designed with `source ∈ {'manual','plate_solve'}`
  in its check constraint from day one (migration 0010) — the
  `'plate_solve'` value has never been written.
- The plate-solve worker persists `ra_deg`, `dec_deg`,
  `pixel_scale_arcsec`, `rotation_deg`; `photos` has `width`/`height`
  — i.e. the WCS is fully available.
- `AladinSkyMap.svelte` is in production on `/t/<slug>` (D3 of the
  predecessor) and can be reused as-is for the detail subpanel.

The user-visible payoff: a photo that previously displayed only its
photographer-typed primary target now reveals every catalogued object
in its frame, with the same color-coding conventions astronomers
recognize from Aladin/SDSS/Stellarium.

---

## Decisions

| #  | Topic                            | Choice |
| -- | -------------------------------- | ------ |
| 1  | Scope                            | End-to-end in one spec: D5 + D6 + overlay UI. Rollout is phased (data layer first, then UI, then backfill), but the design is unified so back/front contracts line up. |
| 2  | Catalog coverage                 | OpenNGC (already seeded) **+ PGC galaxies** (~700–800 k after filter). Stars and solar-system bodies are explicitly out of scope. |
| 3  | Schema strategy                  | **Option U** — extend `targets` with `kind='pgc'` and a new `position_angle_deg` column. No separate `pgc_objects` table. |
| 4  | Cone search implementation       | Bounding-box SQL on a B-tree index `(declination, right_ascension)` + exact haversine filter in Rust. No `cube`/`earthdistance`/PostGIS for the MVP. |
| 5  | NGC ↔ PGC dedup                  | At seed time. Each PGC row with an `NGC ddd`/`IC ddd` ref in `objname` is **skipped** if the corresponding slug already exists in `targets`. UGC dedup may follow in a phase 2. |
| 6  | Identification trigger           | **Auto, in the same DB transaction** as `platesolve::save_result`. Tx atomic — if identification fails, the solve is rolled back. Plus a manual `POST /celestial-objects/recompute` for owners. |
| 7  | Write-time filter                | **F1** — only persist rows with on-screen size ≥ 0.5 px **or** `kind ∈ {messier, ngc, ic, caldwell}`. Drops the ~80% of PGC noise; keeps `photo_targets` lean (typically 5–50 rows per photo). |
| 8  | Confidence formula               | Weighted: `0.40 × center_score + 0.30 × size_score + 0.20 × named_bonus + 0.10 × mag_quality`, all ∈ [0,1]. See §5. |
| 9  | API shape                        | `GET /api/photos/:id/celestial-objects` returns raw `(RA, Dec, …)` per object; the client does the projection. `POST /api/photos/:id/celestial-objects/recompute` is owner-only. |
| 10 | WCS projection                   | Pure TypeScript (`$lib/utils/wcs.ts`), ~30 lines of standard gnomonic-projection math. No external library. |
| 11 | Surface                          | **Public photo page only** (`/u/<handle>/p/<short_id>`). The verify form does not gain an overlay in the MVP. |
| 12 | UI pattern                       | Overlay (SVG) on top of the image **plus** a structured side panel in the existing `aside.info` of `PhotoDetailFull.svelte`. Click on a marker or list row → in-place detail subpanel with mini Aladin embed. |
| 13 | Design approach                  | **Approach C** — color-coded layers by `object_type`, layer toggles, hover→tooltip, click→detail. Layer pills include a separate `PGC` toggle (off by default). |
| 14 | Color palette                    | 7 type colors + null fallback, declared as CSS variables (`--celestial-galaxy`, `--celestial-nebula`, etc.). Galaxies = red, nebulae = green, open clusters = blue, globulars = violet, planetary nebulae = yellow, stars = light gray, other = dim gray. |
| 15 | Marker rendering                 | SVG (`<circle>` or `<ellipse>` when `position_angle_deg` is present), `pointer-events:none` on the wrapper, `pointer-events:auto` on each marker. Cap at 200 markers; if more, drop by ascending size with a "+N hidden" badge. |
| 16 | Aladin in the detail subpanel    | Reuse `AladinSkyMap.svelte` as-is. **Lazy-loaded** at first detail open (saves ~500 KB JS on initial page render). Centered on the object's RA/Dec, FOV ≈ 2 × `major_axis_arcmin`. |
| 17 | Backfill                         | Rust binary `backfill_celestial_objects.rs`, dry-run by default, `--apply` to write. Idempotent. Optional `--reidentify` flag to re-run on photos that already have plate-solve rows (useful after a seed refresh). |
| 18 | License attribution              | OpenNGC attribution stays as-is. Add HyperLEDA citation (Makarov et al. 2014, A&A 570, A13) in the `/t/<slug>` footer for `kind='pgc'` rows. |

---

## Architecture overview

Three independent flows:

**① Seed PGC (one-shot, idempotent)**

```
backend/data/pgc/*.csv  →  seed_pgc.rs  →  targets (kind='pgc')
                                              ↑
                                       NGC↔PGC dedup (skip
                                       rows already in targets)
```

**② Auto-identification at plate-solve (D5 + D6)**

```
platesolve worker → save_result()
                  → celestial::identify(photo_id, &mut tx)
                  → INSERT photo_targets … source='plate_solve'
                  tx.commit()              confidence ∈ [0,1]
```

**③ Display on the public photo page**

```
/u/<h>/p/<sid>/+page.server.ts
   ↓ load
   GET /api/photos/:id/celestial-objects → [{ra, dec, …, confidence}]
   ↓
PhotoDetailFull.svelte
   ├── .stage-frame  → <CelestialOverlay … />   (SVG abs. positioned)
   └── aside.info    → <CelestialPanel … />     (layers + list + detail)
                                  ↑
                          on click, lazy-load <AladinSkyMap …/>
```

**Components touched**

- **New:**
  - migration `0026_targets_pgc.sql`
  - `backend/data/pgc/` (CSV + README)
  - `backend/src/bin/seed_pgc.rs`
  - `backend/src/celestial/identify.rs` (cone-search + confidence)
  - `backend/src/celestial/handler.rs` (GET + POST endpoints)
  - `backend/src/bin/backfill_celestial_objects.rs`
  - `frontend/src/lib/utils/wcs.ts`
  - `frontend/src/lib/components/celestial/CelestialOverlay.svelte`
  - `frontend/src/lib/components/celestial/CelestialPanel.svelte`
  - `frontend/src/lib/components/celestial/CelestialObjectDetail.svelte`
- **Modified:**
  - `backend/src/photos/platesolve.rs` (call `celestial::identify` in `save_result`'s tx)
  - `backend/src/http/mod.rs` (mount the two new routes)
  - `frontend/src/lib/components/photos/PhotoDetailFull.svelte` (insert the overlay in `.stage-frame`, the panel in `aside.info`)
  - `frontend/src/routes/u/[handle]/p/[shortid]/+page.server.ts` (fetch celestial objects in `load`)
  - `justfile` (`seed-pgc`, `backfill-celestial-objects` recipes)
- **Reused:**
  - `frontend/src/lib/components/discovery/AladinSkyMap.svelte` (mini embed)

---

## 1. PGC catalog import

### 1.1 Source

- HyperLEDA / PGC (Makarov et al. 2014, A&A 570, A13).
- Extracted as CSV from `http://leda.univ-lyon1.fr/`, filtered:
  - `de2000 IS NOT NULL`
  - `logd25 > 0` (has a measured major-axis diameter)
  - `bt < 19` (mag B brighter than 19 — cuts the long tail)
- After filter: ~700–800 k rows (subject to extraction date).
- Committed under `backend/data/pgc/`:
  - `pgc.csv` — the filtered extract (≈ 50 MB; gzipped if needed)
  - `README.md` — extraction date, URL, license citation, filter SQL used

### 1.2 Schema migration

File: `backend/migrations/0026_targets_pgc.sql`.

```sql
-- 0030 add PGC support: extend kind enum, add position angle for ellipses,
-- add a spatial index for cone search.

alter table targets
  drop constraint targets_kind_check;
alter table targets
  add constraint targets_kind_check
    check (kind in ('messier','ngc','ic','caldwell','common','other','pgc'));

alter table targets
  add column position_angle_deg real;

create index targets_radec_idx
  on targets (declination, right_ascension)
  where right_ascension is not null and declination is not null;
```

Notes:

- The kind extension is backwards-compatible — every existing row keeps a valid `kind`.
- `position_angle_deg` stays nullable; OpenNGC does not carry PA, only PGC does.
- The partial index (`where … not null`) keeps it small; the cone search always filters on those columns being present.
- No `cube`/`earthdistance` extension — the bounding-box approach scales to ~750k rows comfortably (see §3 perf note).

### 1.3 Seed binary

`backend/src/bin/seed_pgc.rs`, invoked via `just seed-pgc`. Algorithm per CSV row:

```
1. Parse: pgc, objname, ra2000, de2000, bt, logd25, logr25, pa.
2. Convert dimensions: major_axis_arcmin = 10^logd25 × 0.1
                       minor_axis_arcmin = major × 10^(-logr25)
3. Slug: 'pgc-{pgc}' (PGC ID, no zero-padding).
4. NGC/IC dedup: parse objname for 'NGC ddd' / 'IC ddd' tokens.
   If found and corresponding slug already exists in targets → skip.
   Counter: "pgc_deduped_with_ngc: N".
5. UPSERT by slug (INSERT ... ON CONFLICT DO UPDATE):
     - INSERT: canonical_name = objname or 'PGC {pgc}', kind = 'pgc'.
     - UPDATE only astro fields (ra, dec, magnitude_v, object_type='G',
       major/minor_axis, position_angle, updated_at). Never overwrite
       canonical_name or aliases (mirrors the OpenNGC seed contract).
6. Batch in 500-row transactions; full pass < 2 min on a dev DB.
```

Idempotent: re-running converges. Counters logged at end (`inserted`, `updated`, `deduped_with_ngc`, `skipped_invalid`).

### 1.4 Tests

- Unit: parser fixtures (5–6 representative rows: with NGC ref, without, missing logd25, RA wrap, deep south, missing pa).
- Unit: dedup — given a `targets` row with `slug='ngc-224'`, a PGC row with `objname='NGC0224'` is skipped.
- Integration (testcontainers): run migrations → seed a 50-row fixture → assert counts (`inserted`, `deduped_with_ngc`); re-run → `inserted=0, updated=N`.

---

## 2. Cone search

### 2.1 Field-of-view bounds

Given the photo's `width`, `height`, `pixel_scale_arcsec`:

```
fov_x_deg = width  × pixel_scale_arcsec / 3600
fov_y_deg = height × pixel_scale_arcsec / 3600
search_radius_deg = 0.5 × sqrt(fov_x² + fov_y²)
```

The half-diagonal guarantees that every pixel inside the frame is within the search cone.

### 2.2 Bounding-box query

Implemented in Rust (`sqlx::query!`):

```sql
-- $1 = ra_deg, $2 = dec_deg, $3 = radius_deg
select id, slug, canonical_name, kind, object_type, magnitude_v,
       right_ascension, declination,
       major_axis_arcmin, minor_axis_arcmin, position_angle_deg
  from targets
 where declination between ($2 - $3) and ($2 + $3)
   and right_ascension is not null
   and declination is not null
   -- bbox-ra: gestion du wrap 0/360 dans le code Rust qui appelle
   --   (deux requêtes UNION ALL si le wrap est franchi)
   and ra_in_window(right_ascension, $1, $3 / cos(radians($2)));
```

`ra_in_window(target_ra, center_ra, half_width)` is a SQL function (or inline expression) handling the 0/360 wrap. A helper in Rust composes the query, with at most a 2-clause `UNION ALL` when the window crosses 0.

### 2.3 Exact haversine filter

After the bbox query returns its rows, Rust filters with the spherical distance formula:

```rust
fn arc_distance_deg(a1: f64, d1: f64, a2: f64, d2: f64) -> f64 {
    let (a1, d1, a2, d2) = (a1.to_radians(), d1.to_radians(),
                             a2.to_radians(), d2.to_radians());
    let cos_c = d1.sin() * d2.sin()
              + d1.cos() * d2.cos() * (a1 - a2).cos();
    cos_c.acos().to_degrees()
}
```

Rows with `arc_distance > search_radius_deg` are dropped.

### 2.4 Performance

- Expected SQL latency on production-sized `targets` (~750 k rows):
  **< 5 ms** per cone search at typical FOVs (< 1°).
- Verified with `EXPLAIN ANALYZE` on staging before the rollout step that runs the backfill.
- Fallback if degradation is observed: add `cube`/`earthdistance` in a separate migration; the surrounding Rust code does not change.

---

## 3. Identification service

### 3.1 Module layout

`backend/src/celestial/`

```
mod.rs            -- pub use identify, handler
identify.rs       -- the cone-search + confidence + write logic
handler.rs        -- GET + POST HTTP handlers
confidence.rs     -- the formula (kept separate for unit testing)
queries.rs        -- sqlx::query! invocations (cone search)
```

### 3.2 The `identify` entry point

```rust
pub async fn identify(
    photo_id: Uuid,
    tx: &mut Transaction<'_, Postgres>,
) -> Result<IdentifyOutcome, AppError>;
```

It is called from `crate::photos::platesolve::save_result` **inside the same transaction**. Flow:

1. Read the photo's solve telemetry (`ra_deg`, `dec_deg`, `pixel_scale_arcsec`) and `width`/`height`.
2. Compute the search radius (§2.1).
3. Run the cone search (§2.2), apply the haversine filter (§2.3).
4. For each surviving target, compute confidence (§4) and the on-screen size in pixels.
5. Apply the **write-time filter (F1):** keep rows with `on_screen_size_px ≥ 0.5` OR `kind ∈ {messier, ngc, ic, caldwell}`.
6. `DELETE FROM photo_targets WHERE photo_id = $1 AND source = 'plate_solve'`.
7. Bulk `INSERT` the kept rows with `source = 'plate_solve'` and `confidence`.

Returns `{ found: usize, kept: usize, dropped: usize }` for logging / API response.

Atomicity: identification failure (DB error, bad solve data) propagates up — the surrounding `save_result` transaction rolls back. The plate-solve telemetry stays "uncommitted" rather than be persisted alongside no identification. This matches Decision 6.

### 3.3 Backfill binary

`backend/src/bin/backfill_celestial_objects.rs`, invoked via `just backfill-celestial-objects [--apply] [--reidentify]`.

- Selects:
  ```sql
  select id from photos
   where ra_deg is not null
     and (
       not exists (select 1 from photo_targets pt
                    where pt.photo_id = photos.id
                      and pt.source = 'plate_solve')
       or $1::bool  -- --reidentify forces it
     )
  ```
- For each, runs `identify` in its own transaction.
- Dry-run by default; `--apply` writes; `--reidentify` re-runs even when rows already exist.
- Logs counts and timing.
- Idempotent — a second `--apply` with no `--reidentify` is a no-op (the existence guard skips).

---

## 4. Confidence formula

In `confidence.rs`, kept pure for unit testing:

```rust
pub fn confidence(
    arc_distance_deg: f64,
    half_diagonal_deg: f64,
    on_screen_size_px: f64,
    kind: &str,
    magnitude_v: Option<f32>,
) -> f32 {
    let center_score = (1.0 - (arc_distance_deg / half_diagonal_deg)).max(0.0);
    let size_score   = (on_screen_size_px / 20.0).min(1.0);
    let named_bonus  = match kind {
        "messier" | "ngc" | "ic" | "caldwell" => 1.0,
        _ => 0.5,
    };
    let mag_quality = match magnitude_v {
        None       => 0.5,
        Some(m) if m <= 12.0 => 1.0,
        Some(m)              => ((1.0 - (m - 12.0) / 6.0) as f64).max(0.0),
    };
    (0.40 * center_score
   + 0.30 * size_score
   + 0.20 * named_bonus
   + 0.10 * mag_quality) as f32
}
```

Range guaranteed in [0, 1]. Used by the UI to:

- sort the panel list (high confidence first),
- modulate marker opacity (`opacity = 0.3 + 0.6 × confidence`),
- support a future user-facing minimum threshold (the MVP shows everything stored).

---

## 5. API

### 5.1 `GET /api/photos/:id/celestial-objects`

**Auth:** public if the photo is published; owner-only if it is still a draft.

**Response (200):**

```json
{
  "objects": [
    {
      "slug": "m20",
      "canonical_name": "Trifid Nebula",
      "kind": "messier",
      "object_type": "Cl+N",
      "magnitude_v": 6.3,
      "right_ascension": 270.6225,
      "declination": -22.9711,
      "major_axis_arcmin": 28.0,
      "minor_axis_arcmin": 28.0,
      "position_angle_deg": null,
      "confidence": 0.94
    },
    { "...": "..." }
  ]
}
```

Sorted by `confidence` desc, then `magnitude_v` asc (brighter first when confidence ties).

The endpoint does **not** project to pixel coordinates — the client does that with the solve parameters it already has (avoids any cache invalidation tied to projection, and lets the client handle zoom/pan without re-fetching). See §6.

**Caching:** `Cache-Control: public, max-age=60, stale-while-revalidate=300` for published photos; `no-store` for drafts. `ETag` = a hash of `(photos.platesolve_solved_at, max(photo_targets.created_at) for source='plate_solve')`. Both values are already on disk; the digest is cheap.

### 5.2 `POST /api/photos/:id/celestial-objects/recompute`

**Auth:** owner-only.

Re-runs `identify` in a new transaction (independent of any solve). Returns the same counts as the service: `{ found, kept, dropped }`. Used by the "↻ recompute" button in the panel.

### 5.3 Mounting

In `backend/src/http/mod.rs`, alongside the existing photo routes:

```rust
.route(
    "/api/photos/:id/celestial-objects",
    get(crate::celestial::handler::list),
)
.route(
    "/api/photos/:id/celestial-objects/recompute",
    post(crate::celestial::handler::recompute),
)
```

---

## 6. Frontend — projection helper

`frontend/src/lib/utils/wcs.ts` — pure function, ~30 lines:

```typescript
export interface Solve {
  raDeg: number;          // image center
  decDeg: number;
  pixelScaleArcsec: number;
  rotationDeg: number;    // sky → image rotation, plate-solve convention
  width: number;          // image px
  height: number;
}

export function projectRaDecToPixel(
  raDeg: number,
  decDeg: number,
  s: Solve,
): { x: number; y: number; inFrame: boolean } | null {
  // Gnomonic (tangent-plane) projection, all angles in radians internally.
  const a  = (raDeg  * Math.PI) / 180;
  const d  = (decDeg * Math.PI) / 180;
  const a0 = (s.raDeg  * Math.PI) / 180;
  const d0 = (s.decDeg * Math.PI) / 180;

  const cos_c = Math.sin(d0) * Math.sin(d)
              + Math.cos(d0) * Math.cos(d) * Math.cos(a - a0);
  if (cos_c <= 0) return null;            // antipodal hemisphere

  const xi  =  Math.cos(d) * Math.sin(a - a0) / cos_c;
  const eta = (Math.cos(d0) * Math.sin(d)
             - Math.sin(d0) * Math.cos(d) * Math.cos(a - a0)) / cos_c;

  // radians → arcsec → pixels (image Y axis points down).
  const RAD_TO_ARCSEC = (180 / Math.PI) * 3600;
  const dxPx =  xi  * RAD_TO_ARCSEC / s.pixelScaleArcsec;
  const dyPx = -eta * RAD_TO_ARCSEC / s.pixelScaleArcsec;

  // Apply rotation (plate-solve "rotation_deg" is the angle of sky-up
  // relative to image-up, positive counter-clockwise).
  const r = (s.rotationDeg * Math.PI) / 180;
  const xRot = dxPx * Math.cos(r) - dyPx * Math.sin(r);
  const yRot = dxPx * Math.sin(r) + dyPx * Math.cos(r);

  const x = s.width  / 2 + xRot;
  const y = s.height / 2 + yRot;
  const inFrame = x >= 0 && x < s.width && y >= 0 && y < s.height;
  return { x, y, inFrame };
}
```

**Tests** (`wcs.test.ts`):

- Center of frame → `(width/2, height/2)`.
- Object at (RA + fov_x/2 / cos(dec), dec) → x ≈ width.
- Object on the antipode → `null`.
- RA wrap: object at RA = 1° with center at RA = 359° resolves correctly.
- High dec: object at dec = 85° within 0.5° of pole projects without NaN.
- Rotation 0 / 90 / 180 / 270 → correct sign flips.

---

## 7. Frontend — components

### 7.1 `CelestialOverlay.svelte`

**Location:** `frontend/src/lib/components/celestial/CelestialOverlay.svelte`

**Props:**

```ts
{
  objects: CelestialObject[];     // from the API
  solve: Solve;                   // for projection
  layers: Set<string>;            // active object_type keys
  showPgc: boolean;
  labelsAlwaysOn: boolean;
  selectedSlug: string | null;
}
```

**Events:** `select` (slug), `hover` (slug | null).

**Render:** absolutely positioned `<svg>` inside the parent (`.stage-frame`). The container is `pointer-events: none` so it never blocks the existing image zoom; each `<circle>`/`<ellipse>` re-enables `pointer-events: auto` for hover/click.

**Marker geometry:**

- Use `<ellipse>` when `position_angle_deg` is non-null (PGC galaxies primarily).
- Otherwise `<circle>` scaled to `major_axis_arcmin / pixel_scale_arcsec × 60`.
- Minimum radius 6 px (small objects stay clickable), capped at the diagonal length so very wide nebulae do not paint the whole frame.

**Color:** `var(--celestial-{type})` where `{type}` is one of `galaxy | nebula | open-cluster | globular | planetary-nebula | star | other`.

**Opacity:** `0.3 + 0.6 × confidence`.

**Cap:** at most 200 markers in the DOM, ranked by **descending** `on_screen_size_px × confidence` so the biggest, most confident objects always win the budget; the rest are hidden and the panel shows a `+N hidden` badge with a button to "show all (slow)".

### 7.2 `CelestialPanel.svelte`

**Location:** `frontend/src/lib/components/celestial/CelestialPanel.svelte`

**Props:**

```ts
{
  objects: CelestialObject[];
  fov: { widthDeg: number; heightDeg: number };
  selectedSlug: string | null;
}
```

**Two-way state with the overlay:**

- `layers: Set<string>` — initial `{'G','Neb','OCl','GCl','PN','HII','SNR','Cl+N','*Ass'}` excluding `PGC`-style.
- `showPgc: boolean` — `false` by default.
- `labelsAlwaysOn: boolean` — `false` by default.
- `selectedSlug: string | null`.

**Two visual states:**

1. **List view (default):** layer pills row + a vertical list of `(color dot, slug ★ if primary, canonical_name, object_type)` rows. Each row hover-highlights the corresponding marker; click selects it (and switches to detail).
2. **Detail view:** replaces the list with `<CelestialObjectDetail>` (§7.3). A `← back` link returns to the list.

**`↻ recompute` link:** at the bottom of the panel. Owner-only (hidden otherwise). POSTs `/celestial-objects/recompute` and reloads the list on success.

### 7.3 `CelestialObjectDetail.svelte`

**Props:** `{ object: CelestialObject; solve: Solve }`.

**Layout:**

```
M20 — Trifid Nebula            ← back
Cl+N · Sagittarius · mag 6.3 · 28′ × 28′
RA 18h02m40s · Dec −22°57′59″
[ mini Aladin embed, lazy-loaded ]
→ /t/m20 (full page)
```

**Mini Aladin:** dynamic `import('./AladinSkyMap.svelte')` on mount. Centered on `(object.right_ascension, object.declination)`, FOV `2 × major_axis_arcmin / 60` degrees (sensible default; users can pan/zoom inside the embed).

### 7.4 Integration into `PhotoDetailFull.svelte`

Currently:

```svelte
<main>
  <article class="detail">
    <div class="stage">
      <div class="stage-frame">
        <img … />
      </div>
    </div>
    <aside class="info"> … existing sections … </aside>
  </article>
</main>
```

Changes:

- Inside `.stage-frame`, after `<img>`:
  ```svelte
  {#if celestialObjects?.length}
    <CelestialOverlay
      objects={celestialObjects}
      solve={solveFromPhoto}
      bind:layers
      bind:showPgc
      bind:labelsAlwaysOn
      bind:selectedSlug
    />
  {/if}
  ```
- Inside `aside.info`, as a new section between the EQUIPMENT block and the existing ACQUISITION block:
  ```svelte
  {#if celestialObjects?.length}
    <CelestialPanel
      objects={celestialObjects}
      fov={...}
      bind:selectedSlug
      bind:layers
      bind:showPgc
      bind:labelsAlwaysOn
    />
  {/if}
  ```
- `celestialObjects` and `solveFromPhoto` come from the route's `+page.server.ts` load.

### 7.5 Data loading

`frontend/src/routes/u/[handle]/p/[shortid]/+page.server.ts` already loads the photo. Add a parallel fetch:

```ts
const [photo, celestialObjects] = await Promise.all([
  fetchPhoto(...),
  photo.raDeg != null
    ? fetchCelestialObjects(photo.id, fetch)
    : Promise.resolve([]),
]);
```

SSR-rendered → no JS-hydration flash. Failed fetch logs and returns `[]` (silent degrade — the overlay simply does not render).

### 7.6 Color palette CSS

Declared at app theme level (`frontend/src/lib/styles/theme.css` or wherever the rest of `--*` lives):

```css
:root {
  --celestial-galaxy:          #f08070;
  --celestial-nebula:          #7cd9a3;
  --celestial-open-cluster:    #9ec5ff;
  --celestial-globular:        #c990e8;
  --celestial-planetary-nebula:#f0c040;
  --celestial-star:            #d4d4d4;
  --celestial-other:           #888888;
}
```

Lookup table in `$lib/utils/celestial-colors.ts` (small constant map from OpenNGC `object_type` codes to CSS-var names, with a sane fallback for null/unknown). Aligned with the existing `$lib/data/celestial.ts` patterns (label dictionaries).

---

## 8. Tests

### 8.1 Backend

- Unit (`seed_pgc`): parser fixtures, dedup logic, slug derivation.
- Unit (`confidence.rs`): center/edge × small/large × named/PGC × bright/faint matrix.
- Unit (`identify`): given a fixture photo + 6 fixture targets, asserts which ones survive the write filter.
- Integration (testcontainers): migration → seed minimal `targets` (M31 + 5 PGC around) → solve a fake photo on M31 → assert `photo_targets` contains M31 + the PGCs that pass the filter.
- Integration: `GET /celestial-objects` on a non-existent photo → 404; on a draft owned by someone else → 403; on a draft owned by us → 200; on a published photo → 200 (no auth).
- Integration: `POST /recompute` → 200 with counts; non-owner → 403.
- Integration: `backfill_celestial_objects` dry-run vs `--apply` vs second `--apply` (idempotent) vs `--reidentify` (re-runs).

### 8.2 Frontend

- Vitest (`wcs.test.ts`): 6–8 projection cases listed in §6.
- Vitest (`celestial-colors.test.ts`): every known OpenNGC type code → expected CSS var; unknown → fallback.
- Component (`CelestialPanel.test.ts`): toggling a layer pill removes/restores rows; clicking a row enters detail; click `← back` returns to list.

### 8.3 E2E (chrome-devtools MCP, per memory)

- Navigate to a solved, published photo: overlay renders, panel populated, counts match.
- Click a marker on the image → panel switches to detail; Aladin lazy-loads.
- Toggle "PGC" pill → markers appear/disappear.
- Click "↻ recompute" as the owner → POST 200 → list refreshes.
- Click "/t/<slug>" from detail → navigates to the existing target page.

---

## 9. Rollout

Each step is independent and revertable. The spec is end-to-end (Decision 1), but the implementation plan derived from it may legitimately split into two or three PRs (data layer / backend service / frontend) so each merges in a small, reviewable chunk.

1. **Migration** (`0026_targets_pgc.sql`) — additive: new `position_angle_deg` column, extended `kind` check, partial index. Rollback = drop column + restore check (additive nature makes this trivial).
2. **Backend deploy** with `seed_pgc` and `backfill_celestial_objects` binaries, the new `celestial` module, and the routes mounted but unexercised in prod traffic.
3. **`just seed-pgc` on staging.** Smoke checks:
   - `count(*) where kind = 'pgc' and right_ascension is not null` ≈ 700–800 k.
   - `count(*) where slug = 'ngc-224' and kind = 'ngc'` = 1 (no PGC duplicate).
   - A spot check on M31's row (`canonical_name = 'Andromeda Galaxy'`) survived (it must, because OpenNGC handled that slug first).
4. **Frontend deploy** with the new components. From this point, newly plate-solved photos render their overlay automatically (the platesolve worker now calls `identify`).
5. **`just backfill-celestial-objects --apply` on staging.** Spot-check M20 (the one we just verified end-to-end), M42, M31 if a public photo exists. Manual QA via chrome-devtools MCP.
6. **Run `just seed-pgc` then `backfill-celestial-objects --apply` on prod.** Announce in the changelog. Acceptance notes in `docs/operations/p?-acceptance.md`.

Migrations run on boot (per existing infra). Koyeb deploy verification follows the documented pattern (`active_deployment_id` flips before HEALTHY; grep a served chunk to confirm the bundle).

---

## 10. Risks and mitigations

- **Cone search performance at ~750 k rows.** The B-tree index on `(declination, right_ascension)` should keep typical queries under 5 ms. *Mitigation:* `EXPLAIN ANALYZE` on staging during step 3 of the rollout; switch to `cube`/`earthdistance` in a separate migration if degradation is observed (no Rust changes needed).

- **PGC CSV size.** The pre-filtered extract is ~50 MB. *Mitigation:* gzip the file in-repo if needed; document the regeneration recipe in `backend/data/pgc/README.md`. If still too large for the repo, switch to a release asset with a checksum.

- **Imperfect NGC↔PGC dedup.** Some PGC rows reference NGC subcomponents (e.g. `NGC 224A`) that we don't carry as `ngc-224a`. *Mitigation:* log warnings at seed time; add a `PGC_NGC_CROSSWALK_OVERRIDES` constant in `seed_pgc.rs` (mirroring the `KEEP_MANUAL_META` pattern in `seed_targets.rs`) and grow it case by case.

- **WCS projection edge cases.** RA wrap around 0/360, very high declination (FOV deforms near poles), large FOV (gnomonic loses accuracy above ~5°). *Mitigation:* explicit unit tests for each; document the > 5° FOV limit (in practice no on-site photo approaches this).

- **High marker count in galaxy fields.** A Virgo Cluster shot with PGC ON could surface > 500 candidates. SVG degrades past ~200 nodes. *Mitigation:* cap at 200 markers by ascending size; expose a "show all (slow)" affordance; consider Canvas fallback in a phase 2.

- **Color palette accessibility.** Galaxy red vs. globular violet are close in deuteranopia. *Mitigation:* labels remain the source of truth; add a secondary encoding (marker shape — circle vs. ellipse vs. cross) in a phase 2 if user research demands it.

- **Aladin embed weight.** ~500 KB JS + DSS2 tiles. *Mitigation:* lazy-load at first detail open; the user never pays the cost on the list-only path.

- **Re-trigger after a PGC reseed.** A reseed may introduce new objects that did not exist when a photo was solved. *Mitigation:* `--reidentify` flag on the backfill binary; the owner-facing "↻ recompute" button covers per-photo re-runs.

- **Plate-solve telemetry without a valid FOV.** A solve missing `pixel_scale_arcsec` or with `width`/`height` zero would break radius computation. *Mitigation:* `identify` short-circuits to an empty result if any of these are null/invalid, logs once, does not fail the transaction.

---

## 11. Out of scope (explicitly deferred)

- **Star catalogs** (Yale Bright Star, Hipparcos) — would clutter the overlay; OpenNGC + PGC covers the deep-sky use case.
- **Solar-system bodies** (planets, asteroids, comets at `DATE-OBS`). Requires ephemerides; useful only for a small share of photos.
- **`/t/pgc-12345` enriched pages.** PGC rows do get a slug and thus a `/t/<slug>` page, but the page currently shows minimal data (no common name, no description). A polish pass on the PGC-specific page presentation is its own follow-up.
- **PGC-aware `/t` browse.** This spec adds one minimal change to the existing `/t` index: exclude `kind = 'pgc'` from the default browse so 750 k faint galaxies do not drown out the curated NGC/IC/M list. A dedicated "browse PGC" surface — filter pills, paginated catalog — is deferred.
- **WCS via FITS WCS headers** (CD matrix, distortion polynomials). Our solve gives center + scale + rotation, which is sufficient for the gnomonic-projection assumption used here. Distortion correction would matter only for very large FOVs.
- **Marker shape as a colorblind-safety encoding.** Postponed pending user feedback.
- **Verify-page overlay** (aid during upload tagging). MVP keeps the feature in consultation only.

---

## References

- Predecessor: [`docs/superpowers/specs/2026-05-06-celestial-objects-design.md`](2026-05-06-celestial-objects-design.md)
- OpenNGC — https://github.com/mattiaverga/OpenNGC (CC-BY-SA 4.0)
- HyperLEDA / PGC — http://leda.univ-lyon1.fr/ (Makarov et al. 2014, A&A 570, A13)
- Existing schema: `backend/migrations/0010_targets_tags.sql`, `0014_targets_astro_meta.sql`
- Existing seed binary: `backend/src/bin/seed_targets.rs`
- Existing AladinSkyMap component: `frontend/src/lib/components/discovery/AladinSkyMap.svelte`
- Existing photo detail layout: `frontend/src/lib/components/photos/PhotoDetailFull.svelte`
- Plate-solve persistence: `backend/src/photos/platesolve.rs::save_result`
- Plate-solve status endpoint: `backend/src/photos/platesolve_status.rs`
