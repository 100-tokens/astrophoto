# Plate-solve integration

Astrophoto talks to a dedicated plate-solve service
(`xisf-rs-platesolve-server` at `platesolve.astrophoto.pics`) to extract
WCS (world-coordinate-system) data from XISF master images.

This doc captures the integration so far and the still-open design
question that determines the user-facing flow.

## What ships today (this commit)

- `backend/migrations/0021_photos_platesolve.sql` — adds
  `platesolve_*` telemetry columns to the `photos` table:
  - `platesolve_pixel_scale_arcsec` / `platesolve_rotation_deg`
  - `platesolve_rms_arcsec` / `platesolve_matched_count` /
    `platesolve_detected_count`
  - `platesolve_solved_at` (timestamptz, doubles as "have we solved?")
  - `platesolve_error` (text, last failure reason)
  - `platesolve_embed_json` (jsonb — full FITS + PCL payload so consumers
    can re-materialize the solution without re-running)
  - + a sparse index on `platesolve_solved_at` for gallery filters
  - + an index on `(ra_deg, dec_deg)` for sky-position queries
- `backend/src/photos/platesolve.rs` — the typed HTTP client:
  - `PlatesolveClient::from_config` builds the client iff
    `APP_PLATESOLVE_BASE_URL` is set (feature is opt-in).
  - `solve_xisf(bytes, filename, options)` POSTs to `/v1/solve` and
    returns a parsed `PlatesolveResult` — or one of 10 typed
    `PlatesolveError` variants matching the documented service
    responses.
  - `save_result(pool, photo_id, result)` writes the success onto
    the row (RA/Dec on the existing columns, telemetry on the new
    ones).
  - `save_error(pool, photo_id, error)` records a failed attempt
    so the UI can surface "we tried, here's why it didn't work".
- `backend/src/config.rs` — three new env-driven fields:
  - `APP_PLATESOLVE_BASE_URL`
  - `APP_PLATESOLVE_API_KEY`
  - `APP_PLATESOLVE_TIMEOUT_SECS` (default 90)

The client + types are **not yet wired into any handler**. See the
open question below.

## Open question — XISF support

Astrophoto's image pipeline only accepts JPEG / PNG / TIFF
(`photos::magic` and the `image` crate features in
`backend/Cargo.toml`). XISF would not decode with the existing
pipeline, so we can't just "accept XISF uploads through the standard
path and add a solve step at the end."

Three viable wiring strategies, listed from cheapest to richest:

### A. Side-channel "calibrated master" upload (smallest, recommended for v1)

Add a new endpoint `POST /api/photos/:id/platesolve` that accepts an
XISF body, forwards it to the plate-solve service, and writes the
result back onto the existing photo row. The XISF itself is *not*
stored — we only persist the WCS result + `platesolve_embed_json`.

- Pros: no XISF in the storage pipeline, no decoder dependency in
  astrophoto, no schema churn beyond migration 0021.
- Cons: user has to upload the JPEG (for display) AND the XISF (for
  solve) separately. UX is "two-step upload" or a single multipart
  with two fields.
- UI: a "calibrate from XISF" button on the verify form, opens a
  file picker scoped to `.xisf`.

### B. Accept XISF as the primary upload, derive JPEG inside astrophoto

Teach `photos::magic` to recognize XISF (`XISF0100` signature) and
plug `xisf-rs-core` into the pipeline to derive a display JPEG +
thumbnails from the XISF original. Plate-solve runs in the same
background task as the standard EXIF + thumbnail extraction.

- Pros: single upload, no UX duplication. XISF master persists in S3
  so re-solving with newer software is possible.
- Cons: doubles the storage footprint (an XISF master is 50–500 MB
  uncompressed). Adds `xisf-rs-core` as an astrophoto dependency
  (currently only the platesolve service depends on it). Pipeline
  refactor across EXIF + thumbs + display + blurhash to handle two
  decode paths.

### C. Background queue + retries (orthogonal to A/B)

Whichever shape (A or B), the plate-solve call should not block the
upload-finalize HTTP response. Spawn it via `tokio::spawn` after
`finalize` completes (same pattern as the existing thumbnail
generation), and persist a `pending`/`retrying` state in
`platesolve_error` so the UI can show "solving…" rather than
"failed."

## Recommended next step

Ship **A** as v1 — it's the smallest commit that delivers user value
(WCS data persisted, UI can show "this image is at RA/Dec X"), and
the design naturally extends to B later if we decide XISF-as-primary
makes sense.

## Operational notes

- Service contract: see `xisf-rs/docs/platesolve-service-spec.md`.
- Service threat model: see `xisf-rs/docs/platesolve-threat-model.md`.
- Service deploy runbook: see
  `xisf-rs/xisf-rs-platesolve-server/deploy/README.md`.
- Error semantics: callers should branch on `PlatesolveError`
  variant. `NoHintAvailable` → ask user for hint. `RateLimited` /
  `ServiceUnavailable` → schedule retry after the embedded
  `retry_after_secs`. Everything else → record + give up (operator
  reviews via tracing).

## After migration 0021 lands

The `sqlx::query!` compile-time-checked macro is bypassed in
`platesolve.rs` (uses runtime `sqlx::query()` instead) so the module
compiles before `cargo sqlx prepare` is rerun. Once migration 0021
is applied:

```bash
DATABASE_URL=postgres://… cargo sqlx prepare -- --bin astrophoto
```

then optionally promote the runtime queries to the compile-time form
for stricter type checking.
