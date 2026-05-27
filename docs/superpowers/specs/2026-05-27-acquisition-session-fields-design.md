# Acquisition session fields — design (B)

**Date:** 2026-05-27
**Status:** Implemented (`feat/acquisition-session-fields`)
**Author:** verify-form review (Pascal)

> **Implementation note.** FROM SOLVE provenance is scoped to the verify
> form only (it reads `data.platesolveStatus.state` from the existing load),
> so no `platesolve_pixel_scale_arcsec` was added to `PhotoDetail` and no
> `sqlx prepare` was needed. The adaptive-hide is **lossless**: when
> per-filter rows exist, the global gain/exposure/temp inputs are hidden but
> their values are preserved via hidden form carriers (no silent wipe).
> Published-page FROM SOLVE labelling (step 6 only covers per-filter
> gain/temp display) is out of scope for this pass.

## Context

The verify form has two adjacent sections that overlap conceptually:

- **ACQUISITION & FRAMING** — lens, focal, aperture, ISO, exposure, gain,
  sensor temp, sessions, RA, Dec.
- **EQUIPMENT** — setup picker, camera, telescope, focal modifier, mount,
  guiding, structured filters.
- **PER-FILTER INTEGRATION** — rows of `{ filter, filter_item_id,
  sub_count, sub_exposure_s }` with a grand total.

Two corrections came out of review. **(A) is already shipped** (see
`2026-05-27` deploy / PR "apply-setup derives FOCAL+APERTURE" and the
plate-solve focal override): FOCAL/APERTURE are derived — plate-solve
measured first, setup-theoretical fallback. **(B), this spec**, addresses
the remaining overlap:

> "gain / exposure / sensor temp are session-specific — they belong in the
> per-filter integration, not in the global ACQUISITION grid."

This is right for the multi-filter (mono + filter wheel) workflow: R might
be 60×60 s at gain 100 / −10 °C, Hα 30×600 s at gain 200. A single global
gain/exposure/temp can't represent that and reads as a false summary.

## The OSC / simple-upload tension

The global fields can't simply be removed: a one-shot-colour (OSC) camera,
a DSLR, or a plain processed-JPEG upload has **one** session with one
gain/exposure/temp and no per-filter breakdown. Forcing such uploads to
create a per-filter row to record gain is friction.

**Chosen model (confirmed): adaptive.**
- **0 per-filter rows** → gain / exposure / sensor temp live as **global**
  ACQUISITION fields (today's behaviour). Covers OSC, DSLR, single-channel,
  and quick uploads.
- **≥1 per-filter row** → those three fields move **into the rows**; the
  global trio is hidden (or collapses to a read-only derived summary). The
  per-filter values are authoritative.

No duplicate inputs visible at once → kills the confusion the review
flagged, without hurting the simple case.

## Data model

`FilterIntegration` (jsonb on `photos.filter_integrations`) gains two
optional fields:

```rust
pub struct FilterIntegration {
    pub filter: String,
    pub sub_count: i32,
    pub sub_exposure_s: f64,      // already present
    pub filter_item_id: Option<String>,  // already present (catalog link)
    pub gain: Option<i32>,        // NEW — per-session, nullable
    pub sensor_temp_c: Option<f64>, // NEW — per-session, nullable
}
```

`sub_exposure_s` already covers per-filter exposure. ISO is DSLR-only and
mutually exclusive with gain in practice — keep ISO **global** (a DSLR
shoots one ISO; mono-CMOS rigs use gain). `sessions` (night count) stays
global. RA/Dec stay global (plate-solve, one per frame).

Migration: **none** — additive jsonb fields, same as the
`filter_item_id` addition.

`photos.gain` / `photos.exposure_s` / `photos.sensor_temp_c` columns stay
(they back the global path and the published-photo display). When per-filter
rows exist, the published page shows the per-filter breakdown and may show
a derived aggregate (e.g. total integration, dominant gain) rather than the
single global columns.

## UI

In `FilterIntegration.svelte`:
- When rows exist, each row grows two compact inputs: **gain** and
  **temp (°C)** (alongside the existing subs / sub-exposure). Master-drop
  auto-fill: read `GAIN`/`EGAIN` and `CCD-TEMP`/`SET-TEMP` from the XISF
  header (the parser already reads the header; extend `parseXisfHeader` to
  return `gain` + `sensorTempC`).
- In `AcquisitionGrid.svelte`: gain / exposure / sensor-temp fields are
  shown only when `filterIntegrations.length === 0`. When rows exist, hide
  them (or render a one-line read-only "per-filter — see breakdown" note).

State lives in `+page.svelte` already; the adaptive switch is a `$derived`
on `filterIntegrations.length`.

## XISF header extraction (parser)

`parseXisfHeader` currently returns `{ filter, frames, totalExposureS,
subExposureS }`. Add:
- `gain`: FITS `GAIN` (ZWO) or `EGAIN` (e⁻/ADU — NOT the same; prefer
  `GAIN`, the unitless camera gain setting). Integer.
- `sensorTempC`: FITS `CCD-TEMP` (actual) preferred over `SET-TEMP`
  (target). Float.
Both nullable; absent in many masters.

## Provenance / FROM SETUP labelling (carry-over from A)

A shipped FOCAL/APERTURE as FROM SETUP whenever a setup is applied. With
the plate-solve override (A), a solved focal is actually *measured*, not
theoretical. The label taxonomy should grow a third source:

- **FROM SOLVE** — focal/aperture/RA/Dec when a plate-solve exists.
- **FROM SETUP** — focal/aperture from the setup's optical train when
  unsolved.
- **FROM EXIF** — straight from the file header.

`computeProvenance` needs `photo.platesolve_pixel_scale_arcsec` (expose it
on `PhotoDetail` if not already) to branch focal/aperture to FROM SOLVE
when solved. This is a small follow-up to A and naturally belongs with B's
provenance pass. Gain/temp from a dropped master read FROM EXIF (header).

## Sequencing

1. Extend `parseXisfHeader` (+ tests) for gain / sensor temp.
2. `FilterIntegration` type + jsonb fields; metadata PUT validation (clamp,
   nullable). Regenerate ts-rs + `.sqlx` if any query! changes (the jsonb
   write is serde, so likely none).
3. `FilterIntegration.svelte`: per-row gain/temp inputs + master-drop
   auto-fill.
4. `AcquisitionGrid.svelte`: hide gain/exposure/temp when rows exist.
5. Provenance: add FROM SOLVE; expose solve scale on PhotoDetail.
6. Published photo page: show per-filter gain/temp in the breakdown.

## Out of scope
- Per-sub-frame metadata (we record per-filter aggregates, not per-sub).
- Dithering / guiding RMS / SQM per session — future "session log".
- Multi-night session splitting (one row per filter, not per night).
