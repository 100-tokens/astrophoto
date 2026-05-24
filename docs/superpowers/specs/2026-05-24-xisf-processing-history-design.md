# XISF processing-history extraction & display

- **Date:** 2026-05-24
- **Status:** Approved (validated via `advisor` per autonomous-loop workflow)
- **Scope:** XISF/FITS only. Public display, sanitized. Progressive-disclosure UI.

## Motivation

Astrophotographers upload XISF master images produced by PixInsight. These
files embed the *complete processing pipeline* — every process applied, in
order, with its parameters and timing. Surfacing this turns a flat image page
into a transparent "how it was made" record, which is exactly what the
community values when sharing work.

Today none of it is shown. The existing `xisf_display.rs` reads only FITS
`HISTORY` cards, and the external plate-solve service does **not** echo the
input file's properties back (`docs/platesolve-integration.md:171-180`), so the
rich data never reaches us. We will parse the XISF header **locally** instead.

## Evidence (from the M20 LRGB sample)

`/Volumes/Pascal4Tb/astrophotos/M20/working/master/LRGB-stretched.xisf`
(85 MB) has a 106 720-byte XML header at offset 16 containing:

- **47 FITS keywords** (INSTRUME, TELESCOP, FOCALLEN, OBJECT, RA/DEC,
  DATE-OBS/END, FILTER, XPIXSZ, EGAIN, XBINNING, OBSGEO-\*…), duplicated across
  channels. **No `HISTORY` cards** — which is why current code shows nothing.
- **27 XISF `<Property>` elements**, including the prize:
  - `PixInsight:ProcessingHistory` — an 87 KB **embedded XML document**
    (`<ProcessingHistory version="1.0">`) listing **12 ordered processes**:
    ChannelCombination → ImageIdentifier → Script (ImageSolver) → SPCC →
    BlurXTerminator → NoiseXTerminator ×3 → DynamicCrop →
    HistogramTransformation → LRGBCombination → CurvesTransformation. Each
    `<instance>` carries `class`, `version`, `enabled`, a `<time start span>`
    (ISO timestamp + duration in seconds), `<parameter>` entries, and
    `<table>` data.
  - `Instrument:*`, `Observation:*` (equipment, target, site, time span).
  - `PCL:SPCC:WhiteBalanceFactors` / `BackgroundReference` — base64 F64 vectors.
  - `XISF:CreatorApplication` = "PixInsight 1.9.2", `CreatorModule`,
    `CreatorOS` = "macOS", `CreationTime`.
- One `<DisplayFunction>` (the STF / autostretch baked into the display image).

### Data shapes observed

```xml
<instance class="SpectrophotometricColorCalibration" version="1" enabled="true">
  <time start="2025-07-05T12:12:27.789Z" span="11.772377" />
  <parameter id="catalogId" value="GaiaDR3SP" />          <!-- value attr -->
  <parameter id="id">RGB</parameter>                       <!-- or text -->
  ... (54 parameters; long spectral/QE curves among them)
</instance>

<table id="K" rows="4">                                    <!-- CurvesTransformation: (x,y) points -->
  <tr><td id="x" value="0.00000"/><td id="y" value="0.00000"/></tr>
  <tr><td id="x" value="0.26027"/><td id="y" value="0.21854"/></tr> ...
</table>

<table id="H" rows="5">                                    <!-- HistogramTransformation: per-channel -->
  <tr><td id="c0" value="0.0267"/><td id="m" value="0.00975"/><td id="c1" value="1.0"/>
      <td id="r0" value="-0.05"/><td id="r1" value="1.0"/></tr> ...
</table>
```

Some param values are **filesystem paths** (`$PXI_SRCDIR/scripts/AdP/ImageSolver.js`)
and md5sums — these are sanitized out for the public view; the useful sibling
`information = "ImageSolver 6.3.1"` is kept.

## Goals / non-goals

**Goals**
- Parse the XISF header locally and extract the full processing pipeline plus
  creator app, display stretch (STF), and color-calibration white balance.
- Store a structured `ProcessingReport` and serve it (sanitized) on the public
  photo permalink page with progressive-disclosure UI.
- Be robust: a non-PixInsight or header-less XISF, or a parse failure, must
  never break upload or the photo page — it just yields no report.

**Non-goals (explicit, to keep the diff minimal)**
- *Not* backfilling acquisition columns (camera/exposure/…) from the local
  parse. They're currently empty for XISF (server passthrough never shipped) —
  a tempting near-free win, but out of scope here. Noted as follow-up.
- *Not* replacing the verify-form's `XisfDisplayMeta` path. Additive only.
- *Not* decoding XISF pixel data (no `xisf-rs-core` dependency). Header XML only.
- *Not* normalizing into relational tables — the data is document-shaped.

## Architecture

Parse in `platesolve_upload::auto_calibrate_xisf`, right after it fetches the
XISF bytes from S3 (`storage.get`, line ~316) — the bytes are already in memory,
zero extra round-trip. That function is fully `async` (the XISF *decode* happens
on the remote solver; there is **no** local `spawn_blocking` today), so the
~100 KB XML parse gets its **own** `tokio::task::spawn_blocking`. It runs
**independent of the solve outcome** — even a failed solve still yields a
processing report. Store the structured report in a new JSONB column, and serve
it sanitized from a new public endpoint that the photo page's server `load`
fetches.

```
upload XISF ─► platesolve_upload (bytes in hand)
                 ├─ existing: solve + render + xisf_meta
                 └─ NEW: xisf_processing::parse_xisf(&bytes)
                          └─► UPDATE photos.processing_json = <ProcessingReport>
                                                 │
public page load ─► GET /api/photos/:id/processing
                          └─► read processing_json, sanitize, return ProcessingReport
                                                 │
                                   ProcessingPipeline.svelte (detail page)
```

### Components

1. **`backend/src/photos/xisf_processing.rs`** (new, pure + thin I/O wrapper)
   - `parse_xisf(bytes: &[u8]) -> Result<Option<ProcessingReport>, ProcessingParseError>`
     — validate signature `XISF0100`, read header length (u32 LE @ offset 8),
     slice header XML (@ offset 16), call `parse_header_xml`.
   - `parse_header_xml(xml: &str) -> Result<Option<ProcessingReport>, _>` —
     **pure**, unit-testable. Walks `<Property>`/`<FITSKeyword>`/`<DisplayFunction>`;
     for `PixInsight:ProcessingHistory` it unescapes the attribute value and
     parses the nested document; maps each `<instance>` to a `ProcessStep`.
   - Returns `Ok(None)` when there's no PixInsight processing history (valid
     XISF, just nothing to show) so callers don't treat it as an error.
   - Dependency: **`roxmltree`** (read-only zero-copy DOM; lightweight, no pixel
     decoder). `base64` (already present) decodes SPCC F64 vectors.
   - **Class catalog**: a static map `class -> (label, category, summary)` for
     ~25 common PixInsight processes (the 12 here + ImageIntegration,
     StarAlignment, Deconvolution, GraXpert/DBE/ABE, StarXTerminator,
     PhotometricColorCalibration, MultiscaleLinearTransform, …). Unknown
     classes fall back to a de-camelCased label + category "Other".
   - **Long-value handling**: param values > 512 chars (spectral/QE curves) are
     stored truncated with `truncated: true` so the JSON stays small and the UI
     can show "[spectral curve, N values]".

2. **Storage** — `Storage::get_range(key, start, end_inclusive) -> Option<Bytes>`
   added to the trait: S3 backend issues a ranged GET; memory backend slices.
   Used by the backfill (header-only fetch ≈128 KB instead of the whole 85 MB).

3. **Migration** — `0023_photos_processing_json.sql`:
   `ALTER TABLE photos ADD COLUMN processing_json JSONB;` (nullable). The
   **full** parsed report is stored (unsanitized — it's just structured data,
   the column is not public); sanitization happens at the API boundary so the
   policy can evolve without re-parsing.

4. **API** — `GET /api/photos/:id/processing` (public, no auth) in a new
   `xisf_processing_handler.rs`: reads `processing_json`, applies
   `sanitize(report)` (drop params whose value looks like a filesystem path:
   contains `/`, `\`, starts with `$`, or matches a home/abs-path pattern),
   returns `Option<ProcessingReport>` (204/null when absent). Registered in
   `main.rs` next to the existing `xisf-meta` route.

5. **Frontend** — `ProcessingPipeline.svelte` (Svelte 5 runes), rendered by
   `PhotoDetailFull.svelte` below the Acquisition Record. The
   `+page.server.ts` `load` fetches `/processing` alongside the photo (keeps the
   main `PhotoDetail` payload lean). Layout:
   - **Header summary**: "Processed in PixInsight 1.9.2 · 12 steps · total 2m 41s".
   - **Vertical timeline**: each step row = category icon + friendly label +
     duration + disabled badge (if `!enabled`). Collapsed by default; expanding
     shows all params (key/value) and renders tables — `(x,y)` point tables as a
     small SVG line chart, the histogram table as shadows/midtones/highlights
     values. Truncated long values render as a non-expandable chip.

### Types (Rust, `#[derive(TS)]`, `rename_all = "camelCase"`)

```rust
struct ProcessingReport {
    creator_app: Option<String>,      // "PixInsight 1.9.2"
    creator_module: Option<String>,
    creator_os: Option<String>,
    created_at: Option<String>,       // XISF:CreationTime, ISO
    display_stretch: Option<DisplayStretch>,
    white_balance: Option<WhiteBalance>,   // decoded SPCC RGB factors
    total_duration_s: Option<f64>,    // Σ step spans
    pipeline: Vec<ProcessStep>,
}
// <DisplayFunction m="..:..:..:.." s=".." h=".." l=".." r=".."> — each attr is
// a ':'-split 4-vector over channels [R, G, B, RGB/L]. NOT a <table>.
struct DisplayStretch {
    midtones: Vec<f64>, shadows: Vec<f64>, highlights: Vec<f64>,
    low_range: Vec<f64>, high_range: Vec<f64>,
}
struct ProcessStep {
    position: u32, class_name: String, label: String, category: String,
    summary: Option<String>, version: Option<String>, enabled: bool,
    started_at: Option<String>, duration_s: Option<f64>,
    params: Vec<KeyValue>, tables: Vec<ProcessTable>,
}
struct KeyValue { key: String, value: String, truncated: bool }
struct ProcessTable { id: String, kind: String /* curve|histogram|channels|generic */,
                      columns: Vec<String>, rows: Vec<Vec<String>> }
struct WhiteBalance { red: f64, green: f64, blue: f64 }
```

The frontend gates the `/processing` fetch on `photo.mime ===
"application/x-xisf"` — `PhotoDetail` already carries `mime`, so non-XISF photos
skip the request with no new column/field. XISF photos that happen to have no
parseable history just get one request that returns `null`.

## Error handling

- Parse is **best-effort and non-fatal**: invoked in the XISF path inside a
  `match`; on `Err` we `tracing::warn!` and leave `processing_json` NULL. Upload,
  solve, and the photo page proceed unaffected.
- CPU-bound XML parsing of the ~100 KB string runs in its own
  `tokio::task::spawn_blocking` (the XISF path is otherwise fully async — the
  decode happens on the remote solver, so there's no existing blocking region to
  reuse).
- Endpoint returns `null` (200) when `processing_json` is NULL, so the frontend
  simply renders nothing.

## Testing

- **Unit (pure)**: `parse_header_xml` against a committed fixture — the real M20
  header trimmed to a handful of representative instances (ChannelCombination,
  SPCC, BlurXTerminator, HistogramTransformation, CurvesTransformation) plus the
  XISF:* creator props. Assert ordering, timing, enabled, param extraction,
  table parsing, long-value truncation, and the no-history → `Ok(None)` case.
- **Sanitization**: unit-test that `$PXI_SRCDIR/...` paths and md5sum-only values
  are dropped while `information`/catalog values survive.
- **API**: a `testcontainers` test seeding `processing_json` and asserting the
  endpoint returns the sanitized shape (and `null` when absent).
- **Frontend**: verified in-browser via chrome-devtools-mcp (per project E2E
  preference) by uploading the M20 sample through the dev flow and confirming the
  12-step pipeline, expansion, and curve charts render on the public page.

## Rollout / backfill

- New uploads parse automatically.
- One-off backfill (`just`-invokable bin or `photos::cleanup` one-shot) iterates
  XISF photos with NULL `processing_json` and still-present S3 objects,
  `get_range`-fetches the header, parses, and updates. Side-channel-only uploads
  (no stored XISF) are skipped — nothing to read.

## Out of scope / future

- Acquisition-column backfill from the local parser (currently broken path).
- Consolidating `XisfDisplayMeta`/verify-form onto this parser.
- Per-process deep-links / PixInsight-doc references; sky-position-style filters
  on "processed with X".
```
