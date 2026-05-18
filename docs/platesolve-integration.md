# Plate-solve integration

Astrophoto talks to a dedicated plate-solve service
(`xisf-rs-platesolve-server` at `platesolve.astrophoto.pics`) to extract
WCS (world-coordinate-system) data from XISF master images.

This doc captures what is wired today and the open questions that
remain.

## What ships today

### Schema — migration `0021_photos_platesolve.sql`

Telemetry columns on the `photos` table:

- `platesolve_pixel_scale_arcsec` / `platesolve_rotation_deg`
- `platesolve_rms_arcsec` / `platesolve_matched_count` /
  `platesolve_detected_count`
- `platesolve_solved_at` (`timestamptz`, doubles as "have we solved?")
- `platesolve_error` (`text`, last failure reason or in-progress
  sentinel — see below)
- `platesolve_embed_json` (`jsonb`, full FITS + PCL payload so
  consumers can re-materialize the solution without re-running)
- Sparse index on `platesolve_solved_at` for gallery filters
- Index on `(ra_deg, dec_deg)` for sky-position queries

### Typed HTTP client — `backend/src/photos/platesolve.rs`

- `PlatesolveClient::from_config` builds the client iff
  `APP_PLATESOLVE_BASE_URL` is set (opt-in). Errors at boot if the URL
  is set without an API key.
- `solve_xisf(bytes, filename, options)` POSTs to `/v1/solve` and
  returns a parsed `PlatesolveResult` or one of 10 typed
  `PlatesolveError` variants matching documented service responses.
- `save_result(pool, photo_id, result)` writes RA/Dec onto the
  existing columns plus telemetry on the new `platesolve_*` columns,
  clearing any prior `platesolve_error`.
- `save_error(pool, photo_id, error)` records a failed attempt for
  UI surfacing.
- `try_claim(pool, photo_id, user_id)` atomically marks a photo as
  "solving in progress" iff it's owned by `user_id` and not already
  mid-solve. Returns `ClaimOutcome::{Claimed, NotFound, AlreadySolving}`.
- `mark_aborted_if_solving(pool, photo_id)` — best-effort drop-guard
  cleanup that swaps the in-progress sentinel for
  `ABORTED_SENTINEL` when the spawned task panics. Fires-and-forgets
  via `tokio::spawn` because `Drop` is sync.

### Side-channel HTTP endpoint — `POST /api/photos/:id/platesolve`

Implemented in `backend/src/photos/platesolve_upload.rs`. This is the
v1 "Strategy A" wiring (see § History below): the XISF is uploaded as
a multipart body, forwarded to the upstream service, and discarded —
the XISF itself is **not** stored in S3.

**Request:** `multipart/form-data` with:

- `file` (required) — XISF bytes (≤ 128 MB).
- `options` (optional) — JSON matching `SolveOptions` (RA/Dec/scale
  hints, etc.). Overrides any XISF-header-derived hints on the
  service side.

**Responses:**

| Status | When |
|---|---|
| 202 Accepted | Solve has been queued. Poll the photo row for `platesolve_solved_at` (success) or `platesolve_error` (failure / sentinel). |
| 400 Bad Request | Magic-byte sniff says the body isn't XISF (`XISF0100` signature missing). |
| 404 Not Found | Photo doesn't exist, not owned by caller, OR the endpoint is not mounted (server has no `APP_PLATESOLVE_BASE_URL`). |
| 409 Conflict | A solve is already in flight for this photo (`platesolve_error = 'solving'`). |
| 413 Payload Too Large | XISF body > 128 MB. |
| 422 Validation | Multipart shape malformed (missing `file` part, bad `options` JSON). |

**Concurrency:** bounded by a `tokio::sync::Semaphore(1)` on
`AppState` so we don't OOM on the Koyeb Nano/Micro tier. The
sentinel is set before the permit is acquired, so concurrent solves
on **different** photos queue cleanly and concurrent solves on the
**same** photo are rejected with 409 by the atomic claim.

**Retry policy:** the spawned background task retries on transient
upstream failures (`RateLimited`, `ServiceUnavailable`, `Transport`)
up to 3 attempts. `RateLimited` / `ServiceUnavailable` honour the
server-supplied `retry_after_secs` (capped at 60 s); `Transport`
uses exponential backoff (1 s, 2 s, 4 s). Terminal errors
(400/401/413/415/422/internal) are not retried.

### In-progress sentinel

`platesolve_error` doubles as the "is a solve running?" flag:

| `platesolve_error` value | `platesolve_solved_at` | meaning |
|---|---|---|
| `null` | `null` | never attempted |
| `null` | non-null | solved successfully (current values are authoritative) |
| `"solving"` (`SOLVING_SENTINEL`) | `null` | a background task is in flight |
| `"stuck: solver aborted, retry to clear"` (`ABORTED_SENTINEL`) | `null` | the background task panicked / runtime shut down mid-solve; a new POST clears it |
| any other string | `null` or set | last attempt failed with this human-readable reason |

The UI checks for `SOLVING_SENTINEL` exactly to render "solving…"
state. Future work (not in v1): a periodic sweep in
`photos::cleanup` to age out `SOLVING_SENTINEL` rows older than ~10
minutes — covers the runtime-shutdown case the drop guard can't
reach.

### Bundled JPEG render → `display_key`

The side-channel handler always passes `options.render = true` to
`/v1/solve`. The service bundles a display-ready JPEG (≤ 4096 px long
edge, q=85) into the response as `render: { mime, width, height,
bytes_b64 }`. On a successful solve the background task base64-
decodes the bytes, stores them at `display/<photo_id>.jpg` in S3,
and points `photos.display_key` at the new key — closing the loop so
a calibrated XISF appears in the gallery without a separate JPEG
upload.

Render failure is non-fatal: when the service omits the `render`
field (e.g. decoder edge case) the WCS still ships, `save_result`
still runs, and the operator sees a `tracing::warn!` rather than a
user-visible failure. The photo's existing `display_key` is left
untouched in that path.

### Config

`backend/src/config.rs` gains three env-driven fields:

- `APP_PLATESOLVE_BASE_URL` — e.g. `https://platesolve.astrophoto.pics`
  in prod; unset disables the feature (the route is not mounted).
- `APP_PLATESOLVE_API_KEY` — bearer token; required if the base URL
  is set (boot fails otherwise).
- `APP_PLATESOLVE_TIMEOUT_SECS` — per-request timeout, default 90.

### Primary XISF upload — auto-calibrate on finalize

`POST /api/uploads/init` now accepts `application/x-xisf` (gated on
`APP_PLATESOLVE_BASE_URL` being configured — otherwise the photo
would sit in `awaiting-calibration` forever, so the init handler
returns 400 `unsupported-format` early).

When `upload_finalize` sees an XISF original, it skips the JPEG
decode pipeline (no XISF decoder in astrophoto), marks the photo
`status='awaiting-calibration'`, and fires
[`platesolve_upload::auto_calibrate_xisf`] in a background task.

The auto-trigger:

1. Fetches the XISF bytes from S3 via `storage.get(storage_key)`.
2. Claims the sentinel via `platesolve::try_claim(pool, photo_id, owner_id)`.
3. Calls `run_solve` (same code path as the side-channel POST):
   - The bundled JPEG render lands as `display/<photo_id>.jpg`.
   - WCS lands on the existing `ra_deg`/`dec_deg`/`platesolve_*` columns.
4. On success: transitions `status='ready'` (pipeline_error cleared).
5. On terminal failure: transitions `status='failed'` with the
   reason in `pipeline_error` (matches the JPEG-pipeline failure
   shape so the UI surfaces it the same way).

Concurrency between auto-trigger and side-channel POST is bounded
by the same `platesolve_permits` semaphore + sentinel claim: a
concurrent side-channel POST for the same photo gets 409; the
auto-trigger detects an already-claimed sentinel and skips
gracefully (logged for operator visibility).

### XISF header → photo columns (EXIF analog)

After `run_solve` returns successfully, [`platesolve_upload::auto_calibrate_xisf`]
calls [`xisf_meta::extract`] to map common FITS keywords + PCL
properties to the existing photo columns (`camera`, `exposure_s`,
`focal_mm`, `gain`, `sensor_temp_c`, `sessions`, `taken_at`,
`target`), then [`xisf_meta::apply`] UPDATEs the row with
`COALESCE(column, $param)` so any value the user already set is
preserved.

**Service-side prerequisite, not yet shipped.** As of this commit
the platesolve service's `/v1/solve` response only echoes the
WCS-relevant FITS keywords + `AstrometricSolution:*` /
`Observation:Center:*` PCL properties it produces itself. The
input XISF's `Instrument:*` and `EXPTIME` / `FOCALLEN` / etc. are
read by the solver (for hint derivation, see `hint_source`) but
not echoed back. A follow-up service change to passthrough those
keys is what makes this extractor populate real values; the
consumer code (and its 8 unit tests against synthetic FITS+PCL
arrays) lands first so the wiring is in place when that lands.

## Open question — XISF as primary upload (Strategy B)

Strategy A above keeps XISF out of the storage pipeline. Strategy B
would teach `photos::magic` and `photos::pipeline` to ingest XISF as
the primary upload (derive JPEG + thumbnails from the XISF master),
giving a single-upload UX at the cost of:

- Doubling storage footprint (XISF masters are 50–500 MB).
- Adding `xisf-rs-core` as an astrophoto dependency.
- Refactoring EXIF + thumbs + display + blurhash to handle two
  decode paths.

B remains a future option; v1 ships A and we revisit once we see
actual usage and storage costs.

## Operational notes

- Service contract: see `xisf-rs/docs/platesolve-service-spec.md`.
- Service threat model: see `xisf-rs/docs/platesolve-threat-model.md`.
- Service deploy runbook: see
  `xisf-rs/xisf-rs-platesolve-server/deploy/README.md`.
- Error semantics: callers should branch on `PlatesolveError`
  variant. `NoHintAvailable` → ask user for hint. `RateLimited` /
  `ServiceUnavailable` → schedule retry (already handled in the
  background task). Everything else → record + give up (operator
  reviews via tracing).

## sqlx prepare follow-up

The persistence helpers (`save_result`, `save_error`, `try_claim`,
`mark_aborted_if_solving`) use runtime `sqlx::query()` (not
`sqlx::query!`) so the module compiles before `.sqlx/` is regenerated
against migration 0021. Once a dev DB has the migration applied:

```bash
DATABASE_URL=postgres://… cargo sqlx prepare -- --bin astrophoto
```

then optionally promote the runtime queries to the compile-time form
for stricter type checking.

## History

The original integration shipped only the client + telemetry columns
(PR #24, branch `feat/platesolve-client`). At the time, three wiring
strategies were under consideration:

- **A.** Side-channel "calibrated master" upload endpoint (chosen for
  v1; this doc).
- **B.** XISF as primary upload — see § Open question above.
- **C.** Background queue + retries — folded into A (the spawned
  task implements bounded retries).
