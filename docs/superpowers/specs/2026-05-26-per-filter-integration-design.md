# Per-filter integration breakdown (L/R/G/B subs × exposure)

- **Date:** 2026-05-26
- **Status:** Draft — awaiting user review
- **Scope:** Verify-form capture (manual entry + optional client-side XISF
  header auto-fill) + public photo-page display. XISF headers parsed
  **in-browser**; master files are never uploaded.

## Motivation

A deep-sky image's real provenance is its **integration breakdown** — how
much time per filter. "M20, LRGB" means little; "L: 120×120s, R/G/B:
40×120s each — 6h40 total" is the fact other imagers actually want. Today
the verify form has a flat `ACQUISITION & FRAMING` grid with a single
`EXPOSURE` and `SESSIONS`, which cannot express a per-filter LRGB/SHO stack.

We can't recover this from the uploaded master either. The final combined
master is post-processing and drops the acquisition stats.

## Evidence (the live M20 case, photo `55bbb681`)

The uploaded `LRGB-stretched.xisf` (a final stretched master) parses to:

```
filter:          "L"        ← a single FILTER keyword
subframes:       null       ← no NCOMBINE / Process:Integration:ImageCount
totalExposureS:  null       ← no PCL:TotalExposureTime
observationStart→End: 2025-06-28 → 2025-07-05   (6-day span)
binningX: 1,  history: 0 lines
```

So the combined master gives us essentially nothing — not even an aggregate
integration time. Per-filter data lives only in the **per-filter
integration masters** (the `ImageIntegration` / WBPP outputs, e.g.
`masterLight_…_FILTER-L.xisf`), each of which carries `FILTER` + `NCOMBINE`
+ `PCL:TotalExposureTime`. Those are the auto-fill source; the combined
master is not.

## Goals / non-goals

**Goals**
- Let the user record, per filter: **sub count** and **sub-exposure (s)**;
  derive per-filter total and the grand total.
- Optional convenience: **drop the per-filter integration masters** to
  auto-fill those rows. Parse the XISF **header only, client-side** — the
  master (hundreds of MB) never leaves the browser.
- Show the breakdown on the public photo page.

**Non-goals**
- Re-deriving per-filter time from a single combined master (not reliably
  possible — luminance is blended, `TotalExposureTime` is per *color
  channel* not per *filter*; see the eval thread).
- Uploading/storing master files. We read the header and discard.
- Parsing individual calibrated/registered subs (volume; the aggregate
  lives in the integration master).
- FITS (`.fits`) header auto-fill in v1 — XISF only (manual entry still
  works for any source). FITS parsing is a later enhancement.

## Data model — **Option A: independent per-filter list**

Per-filter integration is *acquisition detail*, deliberately decoupled from
the structured filter chips (which exist for tagging/search). This matters:
M20's chips are R/G/B (from the setup) but the integration also has **L**,
which is not a chip. An independent list represents L naturally.

Store as a **JSONB column** on `photos` — display-only, never queried
independently, so a normalized junction would be pure overhead (YAGNI).

```sql
-- migration: add_photo_filter_integrations
alter table photos
  add column filter_integrations jsonb not null default '[]'::jsonb;
```

```rust
// backend/src/photos/... (ts-rs exported)
#[derive(Serialize, Deserialize, TS)]
pub struct FilterIntegration {
    pub filter: String,        // "L" | "R" | "G" | "B" | "Ha" | "OIII" | "SII" | free text
    pub sub_count: i32,        // ≥ 0
    pub sub_exposure_s: f64,   // ≥ 0 ; per-sub seconds
}
// stored/read as sqlx::types::Json<Vec<FilterIntegration>>
```

Per-filter total = `sub_count * sub_exposure_s`; grand total =
Σ rows (derived, never stored). The existing `sessions` (nights) and the
observation span stay as-is — they answer a different question.

## API

Extend the photo metadata `PUT` (`backend/src/photos/metadata.rs`). Mirror
the `double_option` pattern so absent = unchanged, `[]`/`null` = clear:

```rust
#[serde(default, with = "::serde_with::rust::double_option")]
pub filter_integrations: Option<Option<Vec<FilterIntegration>>>,
```

Validate server-side: `sub_count ≥ 0`, `sub_exposure_s ≥ 0`, drop rows with
both zero/empty, cap at 12 rows. `GET /api/photos/:id` returns the field;
run `just types` to regenerate the TS type.

## Client-side XISF header parser (auto-fill)

New util `frontend/src/lib/upload/xisfHeader.ts`:

```ts
// Reads only the header; the File body is never uploaded.
export async function parseXisfHeader(file: File): Promise<{
  filter: string | null; frames: number | null; totalExposureS: number | null;
} | null>
```

Algorithm (mirrors `backend/src/photos/xisf_display.rs`):
1. `await file.slice(0, 262_144).arrayBuffer()` — first 256 KB.
2. Bytes 0..8 must equal `XISF0100`; header length = u32 LE at byte 8.
3. If the header exceeds 256 KB (rare — M20's was 104 KB), re-slice to
   `16 + headerLen`.
4. `DOMParser` the XML; extract:
   - `filter` ← `<FITSKeyword name="FILTER">` value (or
     `Property id="Instrument:Filter:Name">`), unquoted; filename
     `FILTER-<x>` as last-ditch fallback.
   - `frames` ← `NCOMBINE` FITS or `Process:Integration:ImageCount` PCL.
   - `totalExposureS` ← `Property id="PCL:TotalExposureTime"` base64 → LE
     f64 vector → **sum** (matches PixInsight / `decode_total_exposure`).
5. Returns `null` on bad signature / parse failure — caller ignores the file.

Derive the row: `sub_exposure_s = totalExposureS / frames` when both present
(this is how we get per-sub without a reliable `EXPTIME`); else leave
sub-exposure blank for the user to fill.

## UI (verify form)

New component `frontend/src/lib/components/verify-form/FilterIntegration.svelte`,
placed under `ACQUISITION & FRAMING`:

- **PER-FILTER INTEGRATION** header + a row list. Each row: filter select
  (L/R/G/B/Ha/OIII/SII/other-freetext) · `sub_count` · `sub_exposure_s`
  (suffix `s`) · computed per-filter total (read-only) · remove.
- **+ Add filter** button. Pre-seed rows from the existing filter chips +
  an L row, as a convenience (user edits/removes).
- A dropzone: *"Drop per-filter masters (L/R/G/B) — we read the header
  locally; the file is not uploaded."* Each dropped file → `parseXisfHeader`
  → upsert the matching row (by filter). Accept multiple at once.
- Aggregate line: *"Total integration: 6 h 40 min · 240 subs · 4 filters"*
  (derived).
- Binds through to the page's autosave/`PUT` as `filter_integrations`.

## Display (public photo page)

A compact **Acquisition** block on `/u/[handle]/p/[shortid]`: one line per
filter (`L  120 × 120 s  4 h 00 m`) + the grand total. Hidden when the list
is empty. This is the payoff — the shareable "how it was integrated" record.

## Error handling

- Parser returns `null` on any malformed/non-XISF input → the dropped file
  is silently skipped (toast: "couldn't read N file(s)").
- Backend validation rejects negative numbers with `AppError::Validation`.
- A dropped master with an unknown/missing FILTER → row added with an empty
  filter for the user to label (frames/total still filled).

## Testing

- **Rust:** `PUT filter_integrations → GET` round-trip; JSONB serde;
  validation (negatives rejected, >12 capped, empties dropped).
- **TS unit (vitest):** `parseXisfHeader` — valid header, multi-channel
  `TotalExposureTime` sum, `NCOMBINE` vs PCL fallback, bad signature → null,
  oversized-header re-slice. Reuse `m20_processing_history` fixture shapes.
- **chrome-devtools-mcp (interactive, not authored Playwright):** manual entry persists
  + renders on the photo page; drop a real per-filter master → row
  auto-fills and the master is never seen in the network panel.

## Phasing

1. **Data model + API + manual entry UI** (capture works end-to-end).
2. **Client-side master header auto-fill** (the dropzone + `parseXisfHeader`).
3. **Public photo-page display.**

Each phase is independently shippable and testable.

## Open decisions for review

- **Storage = JSONB column** (recommended, least code) vs a normalized
  `photo_filter_integrations` table (queryable but more plumbing). Confirm.
- **Filter vocabulary**: fixed list + free-text "other", or pure free-text?
- **Display placement**: standalone Acquisition block vs folded into the
  existing equipment/specs area on the photo page.
