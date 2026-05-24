# XISF Processing-History Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Parse the PixInsight processing pipeline (and creator/STF/white-balance) out of an uploaded XISF header locally, store it as a structured `ProcessingReport`, and render it — sanitized — on the public photo page with progressive-disclosure UI.

**Architecture:** A pure parser module (`xisf_processing.rs`) turns the XISF header XML into a `ProcessingReport`. It's invoked from `platesolve_upload::auto_calibrate_xisf` in its own `spawn_blocking`, persisted to a new `photos.processing_json` JSONB column. A public endpoint serves it sanitized; the SvelteKit detail page fetches and renders it. Spec: `docs/superpowers/specs/2026-05-24-xisf-processing-history-design.md`.

**Tech Stack:** Rust (axum, sqlx, roxmltree, base64, ts-rs), SvelteKit (Svelte 5 runes), Postgres, testcontainers.

---

## File Structure

**Backend — create:**
- `backend/src/photos/xisf_processing.rs` — pure parser + types (`ProcessingReport`, `ProcessStep`, `KeyValue`, `ProcessTable`, `WhiteBalance`, `DisplayStretch`), class catalog, long-value truncation.
- `backend/src/photos/xisf_processing_handler.rs` — public `GET /api/photos/:id/processing` + sanitization.
- `backend/src/photos/xisf_processing_backfill.rs` — one-shot backfill over XISF rows with NULL `processing_json`.
- `backend/migrations/0023_photos_processing_json.sql` — `processing_json JSONB`.
- `backend/tests/fixtures/m20_processing_history.xml` — readable inner ProcessingHistory fixture (trimmed real data).

**Backend — modify:**
- `backend/src/photos/mod.rs:36` — declare the two new modules.
- `backend/src/storage/mod.rs` — add `get_range` to the `Storage` trait.
- `backend/src/storage/s3.rs` / `memory.rs` — implement `get_range`.
- `backend/src/photos/platesolve_upload.rs:~316` — parse + persist after byte fetch.
- `backend/src/http/mod.rs:169` — register the public route.
- `backend/src/bin/gen-types.rs` — export `ProcessingReport` (pulls all 6 types transitively).
- `frontend/src/lib/api/types.ts` — barrel re-exports for the new types.
- `backend/src/photos/cleanup.rs` or a `just` recipe — invoke backfill (see Task 14).

**Frontend — create:**
- `frontend/src/lib/components/photos/ProcessingPipeline.svelte` — the UI.
- `frontend/src/lib/components/photos/ProcessingCurveChart.svelte` — small SVG line chart for `(x,y)` tables.

**Frontend — modify:**
- `frontend/src/routes/u/[handle]/p/[shortid]/+page.server.ts` — fetch `/processing` when `has_processing_report`.
- `frontend/src/lib/components/photos/PhotoDetailFull.svelte` — render `<ProcessingPipeline>` below Acquisition Record.

---

## Phase 1 — Pure parser (backend)

### Task 1: Add `roxmltree` and scaffold the module

**Files:**
- Modify: `backend/Cargo.toml`
- Create: `backend/src/photos/xisf_processing.rs`
- Modify: `backend/src/photos/mod.rs:36`

- [ ] **Step 1: Add the dependency.** In `backend/Cargo.toml` under `[dependencies]`, after the `base64 = "0.22"` line, add:

```toml
roxmltree = "0.20"
```

- [ ] **Step 2: Declare the module.** In `backend/src/photos/mod.rs`, after line 36 (`pub mod xisf_meta;`) add:

```rust
pub mod xisf_processing;
```

- [ ] **Step 3: Create the type skeleton** in `backend/src/photos/xisf_processing.rs`:

```rust
//! Local parser for the XISF header's PixInsight processing metadata.
//!
//! The XISF format stores a monolithic XML header at byte offset 16
//! (after the 8-byte `XISF0100` signature + a u32 LE header length +
//! 4 reserved bytes). PixInsight writes the full processing pipeline
//! into a `PixInsight:ProcessingHistory` <Property> as an *embedded*,
//! XML-escaped document. This module extracts that pipeline plus the
//! creator app, the display-stretch (STF), and SPCC white balance.
//!
//! Parsing is pure and infallible-by-design for "no processing info"
//! (returns `Ok(None)`); only a structurally broken header is `Err`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

const SIGNATURE: &[u8] = b"XISF0100";
/// Param values longer than this are stored truncated (spectral / QE
/// curves are thousands of chars and never displayed verbatim).
const MAX_PARAM_VALUE_LEN: usize = 512;

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]  // JSON must be camelCase to match the TS type
#[ts(export, rename_all = "camelCase")]  // export_to defaults to "<TypeName>.ts"
pub struct ProcessingReport {
    pub creator_app: Option<String>,
    pub creator_module: Option<String>,
    pub creator_os: Option<String>,
    pub created_at: Option<String>,
    pub display_stretch: Option<DisplayStretch>,
    pub white_balance: Option<WhiteBalance>,
    pub total_duration_s: Option<f64>,
    pub pipeline: Vec<ProcessStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]  // JSON must be camelCase to match the TS type
#[ts(export, rename_all = "camelCase")]  // export_to defaults to "<TypeName>.ts"
pub struct ProcessStep {
    pub position: u32,
    pub class_name: String,
    pub label: String,
    pub category: String,
    pub summary: Option<String>,
    pub version: Option<String>,
    pub enabled: bool,
    pub started_at: Option<String>,
    pub duration_s: Option<f64>,
    pub params: Vec<KeyValue>,
    pub tables: Vec<ProcessTable>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]  // JSON must be camelCase to match the TS type
#[ts(export, rename_all = "camelCase")]  // export_to defaults to "<TypeName>.ts"
pub struct KeyValue {
    pub key: String,
    pub value: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]  // JSON must be camelCase to match the TS type
#[ts(export, rename_all = "camelCase")]  // export_to defaults to "<TypeName>.ts"
pub struct ProcessTable {
    pub id: String,
    pub kind: String, // "curve" | "histogram" | "channels" | "generic"
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]  // JSON must be camelCase to match the TS type
#[ts(export, rename_all = "camelCase")]  // export_to defaults to "<TypeName>.ts"
pub struct WhiteBalance {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]  // JSON must be camelCase to match the TS type
#[ts(export, rename_all = "camelCase")]  // export_to defaults to "<TypeName>.ts"
pub struct DisplayStretch {
    pub midtones: Vec<f64>,
    pub shadows: Vec<f64>,
    pub highlights: Vec<f64>,
    pub low_range: Vec<f64>,
    pub high_range: Vec<f64>,
}

#[derive(Debug, thiserror::Error)]
pub enum ProcessingParseError {
    #[error("not an XISF file (bad signature)")]
    BadSignature,
    #[error("XISF header truncated or length-prefix invalid")]
    BadHeader,
    #[error("XISF header XML did not parse: {0}")]
    Xml(String),
}
```

- [ ] **Step 4: Build and confirm it compiles.**

Run: `cd backend && cargo build`
Expected: compiles (unused-code warnings on the new types are fine for now).

- [ ] **Step 5: Commit.**

```bash
git add backend/Cargo.toml backend/Cargo.lock backend/src/photos/mod.rs backend/src/photos/xisf_processing.rs
git commit -m "feat(xisf): scaffold processing-report types + roxmltree dep"
```

---

### Task 2: Parse the inner ProcessingHistory (the pipeline)

**Files:**
- Modify: `backend/src/photos/xisf_processing.rs`
- Create: `backend/tests/fixtures/m20_processing_history.xml`

- [ ] **Step 1: Create the readable fixture.** Generate a trimmed inner-document fixture from the real sample (keeps tests CI-stable and small). Run:

```bash
python3 - <<'PY'
import struct, xml.etree.ElementTree as ET, re
f="/Volumes/Pascal4Tb/astrophotos/M20/working/master/LRGB-stretched.xisf"
with open(f,'rb') as fh:
    fh.read(8); hlen=struct.unpack('<I',fh.read(4))[0]; fh.read(4); xml=fh.read(hlen).decode('utf-8')
root=ET.fromstring(xml)
ns=lambda t: re.sub(r'^\{[^}]*\}','',t)
ph=None
for e in root.iter():
    if ns(e.tag)=='Property' and e.get('id')=='PixInsight:ProcessingHistory':
        ph=e.get('value'); break
doc=ET.fromstring(ph)
# keep a representative subset: ChannelCombination, Script, SPCC, BlurX, Histogram, Curves
keep={'ChannelCombination','Script','SpectrophotometricColorCalibration','BlurXTerminator','HistogramTransformation','CurvesTransformation'}
for inst in list(doc):
    if inst.get('class') not in keep: doc.remove(inst)
out="backend/tests/fixtures/m20_processing_history.xml"
import os; os.makedirs(os.path.dirname(out), exist_ok=True)
ET.ElementTree(doc).write(out, encoding='unicode', xml_declaration=True)
print("wrote", out)
PY
```

Expected: writes a ~10-15 KB fixture containing 6 `<instance>` elements. (If the absolute sample path is unavailable, hand-author an equivalent fixture with the same element shapes shown in the spec's "Data shapes observed" section.)

- [ ] **Step 2: Write the failing test** at the bottom of `backend/src/photos/xisf_processing.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const PH: &str = include_str!("../../tests/fixtures/m20_processing_history.xml");

    #[test]
    fn parses_pipeline_in_order_with_timing() {
        let steps = parse_processing_history_xml(PH).expect("parses");
        let classes: Vec<&str> = steps.iter().map(|s| s.class_name.as_str()).collect();
        assert!(classes.contains(&"SpectrophotometricColorCalibration"));
        assert!(classes.contains(&"CurvesTransformation"));
        // positions are 0-based and strictly increasing
        for (i, s) in steps.iter().enumerate() {
            assert_eq!(s.position as usize, i);
        }
        let spcc = steps.iter().find(|s| s.class_name == "SpectrophotometricColorCalibration").unwrap();
        assert!(spcc.enabled);
        assert!(spcc.duration_s.unwrap() > 0.0);
        assert!(spcc.started_at.as_deref().unwrap().starts_with("2025-"));
        // long spectral curve params are truncated, not dumped verbatim
        let cat = spcc.params.iter().find(|p| p.key == "catalogId").unwrap();
        assert_eq!(cat.value, "GaiaDR3SP");
        assert!(spcc.params.iter().any(|p| p.truncated));
    }

    #[test]
    fn parses_curve_table_as_points() {
        let steps = parse_processing_history_xml(PH).unwrap();
        let curves = steps.iter().find(|s| s.class_name == "CurvesTransformation").unwrap();
        let k = curves.tables.iter().find(|t| t.id == "K").unwrap();
        assert_eq!(k.kind, "curve");
        assert_eq!(k.columns, vec!["x".to_string(), "y".to_string()]);
        assert!(k.rows.len() >= 2);
        assert_eq!(k.rows[0], vec!["0.00000".to_string(), "0.00000".to_string()]);
    }
}
```

- [ ] **Step 3: Run to verify failure.**

Run: `cd backend && cargo test xisf_processing -- --nocapture`
Expected: FAIL — `parse_processing_history_xml` not found.

- [ ] **Step 4: Implement `parse_processing_history_xml`** in `backend/src/photos/xisf_processing.rs`:

```rust
use roxmltree::{Document, Node};

/// Parse the inner `<ProcessingHistory>` document into ordered steps.
pub fn parse_processing_history_xml(xml: &str) -> Result<Vec<ProcessStep>, ProcessingParseError> {
    let doc = Document::parse(xml).map_err(|e| ProcessingParseError::Xml(e.to_string()))?;
    let root = doc.root_element();
    let mut steps = Vec::new();
    for (i, inst) in root.children().filter(|n| n.has_tag_name("instance")).enumerate() {
        let class_name = inst.attribute("class").unwrap_or("Unknown").to_string();
        let (label, category, summary) = classify(&class_name);
        let enabled = inst.attribute("enabled").map(|v| v == "true").unwrap_or(true);
        let version = inst.attribute("version").map(str::to_string);

        let mut started_at = None;
        let mut duration_s = None;
        let mut params = Vec::new();
        let mut tables = Vec::new();

        for child in inst.children().filter(Node::is_element) {
            match child.tag_name().name() {
                "time" => {
                    started_at = child.attribute("start").map(str::to_string);
                    duration_s = child.attribute("span").and_then(|s| s.parse().ok());
                }
                "parameter" => {
                    if let Some(key) = child.attribute("id") {
                        let raw = child.attribute("value")
                            .map(str::to_string)
                            .unwrap_or_else(|| child.text().unwrap_or("").trim().to_string());
                        params.push(make_kv(key, raw));
                    }
                }
                "table" => {
                    if let Some(t) = parse_table(child) {
                        tables.push(t);
                    }
                }
                _ => {}
            }
        }

        steps.push(ProcessStep {
            position: i as u32,
            class_name, label, category, summary, version, enabled,
            started_at, duration_s, params, tables,
        });
    }
    Ok(steps)
}

fn make_kv(key: &str, value: String) -> KeyValue {
    if value.chars().count() > MAX_PARAM_VALUE_LEN {
        let n = value.split(',').count();
        KeyValue { key: key.to_string(), value: format!("[{n} values]"), truncated: true }
    } else {
        KeyValue { key: key.to_string(), value, truncated: false }
    }
}

fn parse_table(node: Node) -> Option<ProcessTable> {
    let id = node.attribute("id").unwrap_or("").to_string();
    let mut columns: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<String>> = Vec::new();
    for tr in node.children().filter(|n| n.has_tag_name("tr")) {
        let mut row = Vec::new();
        for td in tr.children().filter(|n| n.has_tag_name("td")) {
            if let Some(col) = td.attribute("id") {
                if !columns.contains(&col.to_string()) && rows.is_empty() {
                    columns.push(col.to_string());
                }
            }
            let v = td.attribute("value")
                .map(str::to_string)
                .unwrap_or_else(|| td.text().unwrap_or("").trim().to_string());
            row.push(v);
        }
        if !row.is_empty() { rows.push(row); }
    }
    let kind = match (id.as_str(), columns.as_slice()) {
        (_, cols) if cols == ["x", "y"] => "curve",
        ("H", _) => "histogram",
        ("channels", _) => "channels",
        _ => "generic",
    }.to_string();
    Some(ProcessTable { id, kind, columns, rows })
}
```

- [ ] **Step 5: Add the class catalog** (same file):

```rust
/// Map a PixInsight process class to (label, category, optional summary).
fn classify(class_name: &str) -> (String, String, Option<String>) {
    let (label, category, summary): (&str, &str, Option<&str>) = match class_name {
        "ChannelCombination" => ("Channel Combination", "Composition", Some("Combine R/G/B channels into a color image")),
        "LRGBCombination" => ("LRGB Combination", "Composition", Some("Blend a luminance layer with RGB color")),
        "ImageIdentifier" => ("Image Identifier", "Bookkeeping", None),
        "Script" => ("Script", "Bookkeeping", None),
        "SpectrophotometricColorCalibration" => ("SpectroPhotometric Color Calibration", "Color calibration", Some("Calibrate color from star spectra (SPCC)")),
        "PhotometricColorCalibration" => ("Photometric Color Calibration", "Color calibration", Some("Calibrate color from catalog photometry")),
        "BackgroundNeutralization" => ("Background Neutralization", "Color calibration", None),
        "BlurXTerminator" => ("BlurXTerminator", "Sharpening (AI)", Some("AI deconvolution & detail recovery")),
        "Deconvolution" => ("Deconvolution", "Sharpening", None),
        "NoiseXTerminator" => ("NoiseXTerminator", "Noise reduction (AI)", Some("AI noise reduction")),
        "MultiscaleLinearTransform" => ("Multiscale Linear Transform", "Noise reduction", None),
        "StarXTerminator" => ("StarXTerminator", "Star handling (AI)", Some("AI star removal")),
        "StarNet2" => ("StarNet", "Star handling (AI)", Some("AI star removal")),
        "DynamicCrop" => ("Dynamic Crop", "Geometry", Some("Crop / rotate the frame")),
        "HistogramTransformation" => ("Histogram Transformation", "Stretch", Some("Non-linear brightness stretch")),
        "CurvesTransformation" => ("Curves Transformation", "Adjustment", Some("Per-channel tone / saturation curves")),
        "ImageIntegration" => ("Image Integration", "Stacking", Some("Stack calibrated sub-exposures")),
        "StarAlignment" => ("Star Alignment", "Registration", Some("Register frames to a reference")),
        "DrizzleIntegration" => ("Drizzle Integration", "Stacking", None),
        "GradientCorrection" | "AutomaticBackgroundExtractor" | "DynamicBackgroundExtraction" =>
            ("Gradient / Background Extraction", "Gradient", Some("Remove sky gradients")),
        _ => return (decamel(class_name), "Other".to_string(), None),
    };
    (label.to_string(), category.to_string(), summary.map(str::to_string))
}

/// "SomeNewProcess" -> "Some New Process" for unknown classes.
fn decamel(s: &str) -> String {
    let mut out = String::new();
    for (i, c) in s.char_indices() {
        if i > 0 && c.is_uppercase() { out.push(' '); }
        out.push(c);
    }
    out
}
```

- [ ] **Step 6: Run tests to verify pass.**

Run: `cd backend && cargo test xisf_processing -- --nocapture`
Expected: PASS (both tests).

- [ ] **Step 7: Commit.**

```bash
git add backend/src/photos/xisf_processing.rs backend/tests/fixtures/m20_processing_history.xml
git commit -m "feat(xisf): parse processing pipeline + tables from history XML"
```

---

### Task 3: Parse the outer header (FITS, props, STF, white balance) + binary wrapper

**Files:**
- Modify: `backend/src/photos/xisf_processing.rs`

- [ ] **Step 1: Write the failing tests** (append to the `tests` module):

```rust
fn wrap_header(inner_history: &str) -> String {
    // Build a minimal XISF header XML embedding an escaped ProcessingHistory.
    let escaped = inner_history
        .replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;");
    format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<xisf version="1.0" xmlns="http://www.pixinsight.com/xisf">
<Image geometry="100:100:1" sampleFormat="Float32" colorSpace="Gray">
<FITSKeyword name="FILTER" value="'L'" comment="filter"/>
<DisplayFunction m="0.44:0.41:0.41:0.5" s="0:0.01:0:0" h="1:1:1:1" l="0:0:0:0" r="1:1:1:1"/>
<Property id="XISF:CreatorApplication" type="String" value="PixInsight 1.9.2"/>
<Property id="XISF:CreatorOS" type="String" value="macOS"/>
<Property id="XISF:CreationTime" type="String" value="2025-07-20T14:26:17Z"/>
<Property id="PixInsight:ProcessingHistory" type="String" value="{escaped}"/>
</Image>
</xisf>"#)
}

#[test]
fn parses_full_header_report() {
    let header = wrap_header(PH);
    let report = parse_header_xml(&header).unwrap().expect("has report");
    assert_eq!(report.creator_app.as_deref(), Some("PixInsight 1.9.2"));
    assert_eq!(report.creator_os.as_deref(), Some("macOS"));
    assert_eq!(report.created_at.as_deref(), Some("2025-07-20T14:26:17Z"));
    assert!(!report.pipeline.is_empty());
    assert!(report.total_duration_s.unwrap() > 0.0);
    let stf = report.display_stretch.unwrap();
    assert_eq!(stf.midtones.len(), 4);
    assert!((stf.midtones[0] - 0.44).abs() < 1e-6);
}

#[test]
fn no_processing_history_yields_none() {
    let header = r#"<?xml version="1.0"?><xisf xmlns="http://www.pixinsight.com/xisf"><Image><FITSKeyword name="FILTER" value="'L'"/></Image></xisf>"#;
    assert!(parse_header_xml(header).unwrap().is_none());
}

#[test]
fn parse_xisf_reads_binary_envelope() {
    let header = wrap_header(PH);
    let mut buf = Vec::new();
    buf.extend_from_slice(SIGNATURE);
    buf.extend_from_slice(&(header.len() as u32).to_le_bytes());
    buf.extend_from_slice(&[0u8; 4]); // reserved
    buf.extend_from_slice(header.as_bytes());
    let report = parse_xisf(&buf).unwrap().expect("report");
    assert_eq!(report.creator_app.as_deref(), Some("PixInsight 1.9.2"));
}

#[test]
fn parse_xisf_rejects_non_xisf() {
    assert!(matches!(parse_xisf(b"NOTXISF.."), Err(ProcessingParseError::BadSignature)));
}
```

- [ ] **Step 2: Run to verify failure.**

Run: `cd backend && cargo test xisf_processing -- --nocapture`
Expected: FAIL — `parse_header_xml` / `parse_xisf` not found.

- [ ] **Step 3: Implement `parse_header_xml` and `parse_xisf`** (same file):

```rust
use base64::{engine::general_purpose::STANDARD, Engine};

/// Read the XISF binary envelope and parse its header XML.
pub fn parse_xisf(bytes: &[u8]) -> Result<Option<ProcessingReport>, ProcessingParseError> {
    if bytes.len() < 16 || &bytes[0..8] != SIGNATURE {
        return Err(ProcessingParseError::BadSignature);
    }
    let hlen = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    let end = 16usize.checked_add(hlen).ok_or(ProcessingParseError::BadHeader)?;
    if end > bytes.len() {
        return Err(ProcessingParseError::BadHeader);
    }
    let xml = std::str::from_utf8(&bytes[16..end]).map_err(|_| ProcessingParseError::BadHeader)?;
    parse_header_xml(xml)
}

/// Parse the XISF header XML into a report. `Ok(None)` when the file
/// carries no PixInsight processing history.
pub fn parse_header_xml(xml: &str) -> Result<Option<ProcessingReport>, ProcessingParseError> {
    let doc = Document::parse(xml).map_err(|e| ProcessingParseError::Xml(e.to_string()))?;

    let mut creator_app = None;
    let mut creator_module = None;
    let mut creator_os = None;
    let mut created_at = None;
    let mut white_balance = None;
    let mut history: Option<Vec<ProcessStep>> = None;

    for p in doc.descendants().filter(|n| n.has_tag_name("Property")) {
        let id = p.attribute("id").unwrap_or("");
        let val = p.attribute("value").map(str::to_string)
            .unwrap_or_else(|| p.text().unwrap_or("").trim().to_string());
        match id {
            "XISF:CreatorApplication" => creator_app = Some(val),
            "XISF:CreatorModule" => creator_module = Some(val),
            "XISF:CreatorOS" => creator_os = Some(val),
            "XISF:CreationTime" => created_at = Some(val),
            "PCL:SPCC:WhiteBalanceFactors" => white_balance = decode_white_balance(&val),
            "PixInsight:ProcessingHistory" => {
                history = Some(parse_processing_history_xml(&val)?);
            }
            _ => {}
        }
    }

    let display_stretch = doc.descendants()
        .find(|n| n.has_tag_name("DisplayFunction"))
        .and_then(parse_display_function);

    let Some(pipeline) = history else {
        return Ok(None);
    };

    let total_duration_s = {
        let sum: f64 = pipeline.iter().filter_map(|s| s.duration_s).sum();
        (sum > 0.0).then_some(sum)
    };

    Ok(Some(ProcessingReport {
        creator_app, creator_module, creator_os, created_at,
        display_stretch, white_balance, total_duration_s, pipeline,
    }))
}

fn parse_display_function(node: Node) -> Option<DisplayStretch> {
    let split = |attr: &str| -> Vec<f64> {
        node.attribute(attr).map(|s| s.split(':').filter_map(|x| x.parse().ok()).collect())
            .unwrap_or_default()
    };
    Some(DisplayStretch {
        midtones: split("m"), shadows: split("s"), highlights: split("h"),
        low_range: split("l"), high_range: split("r"),
    })
}

/// SPCC white-balance factors are a base64 little-endian f64 vector
/// `[R, G, B, ...]`. Decode the first three.
fn decode_white_balance(b64: &str) -> Option<WhiteBalance> {
    let raw = STANDARD.decode(b64.trim()).ok()?;
    let f = |i: usize| -> Option<f64> {
        raw.get(i * 8..i * 8 + 8).map(|s| f64::from_le_bytes(s.try_into().unwrap()))
    };
    Some(WhiteBalance { red: f(0)?, green: f(1)?, blue: f(2)? })
}
```

- [ ] **Step 4: Run all parser tests.**

Run: `cd backend && cargo test xisf_processing -- --nocapture`
Expected: PASS (all 6 tests).

- [ ] **Step 5: Clippy + commit.**

Run: `cd backend && cargo clippy --all-targets -- -D warnings`
Expected: clean.

```bash
git add backend/src/photos/xisf_processing.rs
git commit -m "feat(xisf): parse header envelope, creator, STF, white balance"
```

---

## Phase 2 — Storage, migration, persistence

### Task 4: Add `Storage::get_range`

**Files:**
- Modify: `backend/src/storage/mod.rs`, `backend/src/storage/s3.rs`, `backend/src/storage/memory.rs`

- [ ] **Step 1: Add the trait method.** In `backend/src/storage/mod.rs`, in the `Storage` trait after the existing `async fn get`:

```rust
/// Fetch an inclusive byte range `[start, end]`. Returns `None` if the
/// object doesn't exist. Used to read just an XISF header cheaply.
async fn get_range(&self, key: &str, start: u64, end: u64) -> Result<Option<Bytes>, AppError>;
```

- [ ] **Step 2: Implement for S3** in `backend/src/storage/s3.rs` (mirror the existing `get`, adding `.range`):

```rust
async fn get_range(&self, key: &str, start: u64, end: u64) -> Result<Option<Bytes>, AppError> {
    let resp = self.client.get_object()
        .bucket(&self.bucket).key(key)
        .range(format!("bytes={start}-{end}"))
        .send().await;
    match resp {
        Ok(out) => {
            let data = out.body.collect().await
                .map_err(|e| AppError::internal(format!("s3 range read: {e}")))?;
            Ok(Some(data.into_bytes()))
        }
        Err(e) if is_no_such_key(&e) => Ok(None),
        Err(e) => Err(AppError::internal(format!("s3 get_range: {e}"))),
    }
}
```

(Reuse whatever `is_no_such_key`/not-found mapping the existing `get` uses; match its exact error handling.)

- [ ] **Step 3: Implement for memory** in `backend/src/storage/memory.rs`:

```rust
async fn get_range(&self, key: &str, start: u64, end: u64) -> Result<Option<Bytes>, AppError> {
    let map = self.inner.lock().expect("memory storage lock");
    Ok(map.get(key).map(|b| {
        let lo = (start as usize).min(b.len());
        let hi = ((end as usize).saturating_add(1)).min(b.len());
        b.slice(lo..hi)
    }))
}
```

(Match the field name the memory backend actually uses for its map.)

- [ ] **Step 4: Write a memory-backend test** in `backend/src/storage/memory.rs` tests module:

```rust
#[tokio::test]
async fn get_range_slices_inclusive() {
    let s = MemoryStorage::new();
    s.put("k", Bytes::from_static(b"0123456789"), "application/octet-stream").await.unwrap();
    assert_eq!(s.get_range("k", 2, 5).await.unwrap().unwrap().as_ref(), b"2345");
    assert!(s.get_range("missing", 0, 4).await.unwrap().is_none());
}
```

(Adjust `put` signature to match the real one.)

- [ ] **Step 5: Run + commit.**

Run: `cd backend && cargo test storage -- --nocapture && cargo clippy --all-targets -- -D warnings`
Expected: PASS, clean.

```bash
git add backend/src/storage/
git commit -m "feat(storage): add get_range for cheap XISF header reads"
```

---

### Task 5: Migration for `processing_json`

**Files:**
- Create: `backend/migrations/0023_photos_processing_json.sql`

- [ ] **Step 1: Generate the migration file.**

Run: `just db-migrate add_photos_processing_json`
Then replace the generated file's body with:

```sql
-- Structured PixInsight processing report parsed from the XISF header.
-- NULL = not an XISF / no processing history / not yet backfilled.
ALTER TABLE photos ADD COLUMN processing_json JSONB;
```

(If `just db-migrate` produces a different filename number, keep its number; references below to `0023` are nominal.)

- [ ] **Step 2: Apply + verify.**

Run: `just db-reset` (dev DB) — confirms the migration applies cleanly.
Expected: no error; `\d photos` shows `processing_json | jsonb`.

- [ ] **Step 3: Commit.**

```bash
git add backend/migrations/
git commit -m "feat(db): add photos.processing_json column"
```

---

### Task 6: Persist the report on XISF upload

**Files:**
- Modify: `backend/src/photos/platesolve_upload.rs` (in `auto_calibrate_xisf`, after `state.storage.get(&storage_key)` ~line 316)

- [ ] **Step 1: Add a persistence helper** at the bottom of `platesolve_upload.rs`:

```rust
/// Parse the XISF header and persist the processing report. Best-effort:
/// any failure is logged and swallowed so it never blocks calibration.
async fn persist_processing_report(pool: &PgPool, photo_id: Uuid, bytes: bytes::Bytes) {
    let parsed = tokio::task::spawn_blocking(move || {
        crate::photos::xisf_processing::parse_xisf(&bytes)
    })
    .await;
    let report = match parsed {
        Ok(Ok(Some(r))) => r,
        Ok(Ok(None)) => return, // valid XISF, no processing history
        Ok(Err(e)) => { tracing::warn!(%photo_id, error=%e, "xisf processing parse failed"); return; }
        Err(e) => { tracing::warn!(%photo_id, error=%e, "xisf processing parse panicked"); return; }
    };
    let json = match serde_json::to_value(&report) {
        Ok(v) => v,
        Err(e) => { tracing::warn!(%photo_id, error=%e, "serialize processing report"); return; }
    };
    if let Err(e) = sqlx::query("UPDATE photos SET processing_json = $1 WHERE id = $2")
        .bind(json).bind(photo_id)
        .execute(pool).await
    {
        tracing::warn!(%photo_id, error=%e, "persist processing_json failed");
    }
}
```

- [ ] **Step 2: Call it after the bytes are fetched.** In `auto_calibrate_xisf`, immediately after the `let xisf_bytes = match state.storage.get(&storage_key).await { ... }` block succeeds (before/independent of `run_solve`), add:

```rust
persist_processing_report(&state.pool, photo_id, xisf_bytes.clone()).await;
```

(`xisf_bytes` is `bytes::Bytes`, cheap to clone — it's an `Arc` inside. The clone leaves the original for `run_solve`.)

- [ ] **Step 3: Build + clippy.**

Run: `cd backend && cargo build && cargo clippy --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 4: `cargo sqlx prepare`** (a new runtime query was added; keep offline mode green).

Run: `cd backend && DATABASE_URL=$APP_DATABASE_URL cargo sqlx prepare` (use the dev DB URL from `.env`).
Expected: `.sqlx/` updated (the new query uses runtime `sqlx::query`, so this may be a no-op — run it anyway and commit any diff).

- [ ] **Step 5: Commit.**

```bash
git add backend/src/photos/platesolve_upload.rs backend/.sqlx
git commit -m "feat(xisf): parse + persist processing report on upload"
```

---

## Phase 3 — API + types

### Task 7: Public `/processing` endpoint + sanitization

**Files:**
- Create: `backend/src/photos/xisf_processing_handler.rs`
- Modify: `backend/src/photos/mod.rs`, `backend/src/http/mod.rs:169`

- [ ] **Step 1: Write the sanitization unit tests** in a new `xisf_processing_handler.rs` (tests at bottom):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::photos::xisf_processing::{KeyValue, ProcessStep, ProcessingReport};

    fn kv(k: &str, v: &str) -> KeyValue { KeyValue { key: k.into(), value: v.into(), truncated: false } }

    #[test]
    fn strips_paths_keeps_identifiers() {
        assert!(looks_like_path("$PXI_SRCDIR/scripts/AdP/ImageSolver.js"));
        assert!(looks_like_path("/Users/me/data/light.xisf"));
        assert!(looks_like_path(r"C:\Users\me\light.xisf"));
        assert!(looks_like_path("fe992f408d2f2de770c7ce87451c548b")); // bare md5
        assert!(!looks_like_path("GaiaDR3SP"));
        assert!(!looks_like_path("BlurXTerminator.4.mlpackage"));
        assert!(!looks_like_path("ImageSolver 6.3.1"));
        assert!(!looks_like_path("0.05"));
    }

    #[test]
    fn sanitize_drops_path_params_only() {
        let mut report = ProcessingReport {
            creator_app: None, creator_module: None, creator_os: None, created_at: None,
            display_stretch: None, white_balance: None, total_duration_s: None,
            pipeline: vec![ProcessStep {
                position: 0, class_name: "Script".into(), label: "Script".into(),
                category: "Bookkeeping".into(), summary: None, version: None, enabled: true,
                started_at: None, duration_s: None,
                params: vec![kv("filePath", "$PXI_SRCDIR/x.js"), kv("information", "ImageSolver 6.3.1")],
                tables: vec![],
            }],
        };
        sanitize(&mut report);
        let keys: Vec<&str> = report.pipeline[0].params.iter().map(|p| p.key.as_str()).collect();
        assert_eq!(keys, vec!["information"]);
    }
}
```

- [ ] **Step 2: Run to verify failure.**

Run: `cd backend && cargo test xisf_processing_handler -- --nocapture`
Expected: FAIL — module/functions not found.

- [ ] **Step 3: Implement the handler + sanitization:**

```rust
//! `GET /api/photos/:id/processing` — public, sanitized view of the
//! parsed XISF processing report. No auth: the photo page is public.

use axum::{Json, extract::{Path, State}};
use serde_json::Value;
use uuid::Uuid;

use crate::error::AppError;
use crate::http::AppState;
use crate::photos::xisf_processing::ProcessingReport;

pub async fn handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Option<ProcessingReport>>, AppError> {
    let row: Option<(Option<Value>,)> =
        sqlx::query_as::<_, (Option<Value>,)>("SELECT processing_json FROM photos WHERE id = $1")
            .bind(id)
            .fetch_optional(&state.pool)
            .await
            .map_err(AppError::from)?;

    let Some((Some(json),)) = row else {
        // photo missing OR no report → null body, 200
        return Ok(Json(None));
    };
    let mut report: ProcessingReport = serde_json::from_value(json)
        .map_err(|e| AppError::internal(format!("decode processing_json: {e}")))?;
    sanitize(&mut report);
    Ok(Json(Some(report)))
}

/// Drop params whose value looks like a filesystem path or a bare hash.
fn sanitize(report: &mut ProcessingReport) {
    for step in &mut report.pipeline {
        step.params.retain(|p| !looks_like_path(&p.value));
    }
}

fn looks_like_path(v: &str) -> bool {
    let v = v.trim();
    if v.starts_with('$') || v.starts_with('/') || v.starts_with("~/") { return true; }
    if v.len() >= 3 && v.as_bytes()[1] == b':' && (v.contains('\\')) { return true; } // C:\...
    if v.contains('/') && v.contains('.') { return true; } // foo/bar.ext
    // bare 32/40-char hex hash (md5/sha1)
    if (v.len() == 32 || v.len() == 40) && v.bytes().all(|b| b.is_ascii_hexdigit()) { return true; }
    false
}
```

- [ ] **Step 4: Declare module + register route.** In `backend/src/photos/mod.rs` add `pub mod xisf_processing_handler;`. In `backend/src/http/mod.rs`, after the `/xisf-meta` route (line ~169) add:

```rust
.route(
    "/api/photos/:id/processing",
    get(crate::photos::xisf_processing_handler::handler),
)
```

- [ ] **Step 5: Run tests + clippy + build.**

Run: `cd backend && cargo test xisf_processing_handler -- --nocapture && cargo clippy --all-targets -- -D warnings`
Expected: PASS, clean.

- [ ] **Step 6: Commit.**

```bash
git add backend/src/photos/xisf_processing_handler.rs backend/src/photos/mod.rs backend/src/http/mod.rs
git commit -m "feat(xisf): public sanitized /processing endpoint"
```

---

### Task 8: Export the processing types to TypeScript

> **Design note:** we do *not* add a `has_processing_report` column/field.
> `PhotoDetail` already carries `mime`, and XISF photos have
> `mime == "application/x-xisf"` (see `upload_finalize.rs:69`). The frontend
> gates the `/processing` fetch on that — no schema/query/api_types change.
> `export_all_to` writes a type **and all its transitive dependencies**, so
> exporting `ProcessingReport` alone emits all six `.ts` files.

**Files:**
- Modify: `backend/src/bin/gen-types.rs`, `frontend/src/lib/api/types.ts`

- [ ] **Step 1: Register the export.** In `backend/src/bin/gen-types.rs`, add to the `use astrophoto::photos::...` imports:

```rust
use astrophoto::photos::xisf_processing::ProcessingReport;
```

and after the `XisfDisplayMeta::export_all_to(out_dir)?;` line (≈ line 96) add:

```rust
ProcessingReport::export_all_to(out_dir)?;
```

- [ ] **Step 2: Add barrel re-exports.** In the hand-maintained `frontend/src/lib/api/types.ts`, add (near the other photo types):

```ts
export type { ProcessingReport } from './ProcessingReport';
export type { ProcessStep } from './ProcessStep';
export type { ProcessTable } from './ProcessTable';
export type { KeyValue } from './KeyValue';
export type { WhiteBalance } from './WhiteBalance';
export type { DisplayStretch } from './DisplayStretch';
```

- [ ] **Step 3: Regenerate + build.**

Run: `just types && cd backend && cargo build`
Expected: `frontend/src/lib/api/{ProcessingReport,ProcessStep,ProcessTable,KeyValue,WhiteBalance,DisplayStretch}.ts` created; build clean. (No `cargo sqlx prepare` — no SQL changed in this task.)

- [ ] **Step 4: Commit.**

```bash
git add backend/src/bin/gen-types.rs frontend/src/lib/api/
git commit -m "feat(api): export processing-report types to TypeScript"
```

---

## Phase 4 — Frontend

### Task 9: Fetch the report in the page load

**Files:**
- Modify: `frontend/src/routes/u/[handle]/p/[shortid]/+page.server.ts`

- [ ] **Step 1: Fetch `/processing` for XISF photos.** After the existing `PhotoDetail` fetch resolves to `photo`, add (gate on `mime`, which `PhotoDetail` already carries — no extra fetch for non-XISF photos):

```ts
let processing: ProcessingReport | null = null;
if (photo.mime === 'application/x-xisf') {
    const r = await fetch(`${API_BASE}/api/photos/${photo.id}/processing`);
    if (r.ok) processing = await r.json(); // endpoint returns null when no report
}
return { ...existing, processing };
```

(Use the same `API_BASE`/fetch helper and `ProcessingReport` import path the file already uses for `PhotoDetail`. Type import: `import type { ProcessingReport } from '$lib/api/types';`.)

- [ ] **Step 2: Type-check.**

Run: `cd frontend && pnpm check`
Expected: no new errors (will error until the component consumes `processing` — that's Task 11; if `processing` is unused, reference it in the return only).

- [ ] **Step 3: Commit.**

```bash
git add frontend/src/routes/u/\[handle\]/p/\[shortid\]/+page.server.ts
git commit -m "feat(web): load processing report on photo detail page"
```

---

### Task 10: Curve chart component

**Files:**
- Create: `frontend/src/lib/components/photos/ProcessingCurveChart.svelte`

- [ ] **Step 1: Implement a small SVG line chart** (Svelte 5 runes):

```svelte
<script lang="ts">
  import type { ProcessTable } from '$lib/api/types';
  let { table }: { table: ProcessTable } = $props();

  const W = 160, H = 120, pad = 8;
  const points = $derived(
    table.rows
      .map((r) => ({ x: parseFloat(r[0]), y: parseFloat(r[1]) }))
      .filter((p) => Number.isFinite(p.x) && Number.isFinite(p.y))
  );
  const path = $derived(
    points
      .map((p, i) => {
        const px = pad + p.x * (W - 2 * pad);
        const py = H - pad - p.y * (H - 2 * pad);
        return `${i === 0 ? 'M' : 'L'}${px.toFixed(1)},${py.toFixed(1)}`;
      })
      .join(' ')
  );
</script>

<svg viewBox="0 0 {W} {H}" class="curve" role="img" aria-label="Processing curve">
  <rect x={pad} y={pad} width={W - 2 * pad} height={H - 2 * pad} class="frame" />
  <line x1={pad} y1={H - pad} x2={W - pad} y2={pad} class="diagonal" />
  <path d={path} class="curve-line" />
</svg>

<style>
  .curve { width: 160px; height: 120px; }
  .frame { fill: none; stroke: var(--border, #2a2a35); stroke-width: 1; }
  .diagonal { stroke: var(--border-subtle, #1d1d27); stroke-width: 1; stroke-dasharray: 3 3; }
  .curve-line { fill: none; stroke: var(--accent, #7aa2f7); stroke-width: 2; stroke-linejoin: round; }
</style>
```

(Match the project's CSS variable names — inspect a sibling component for the real `--accent`/`--border` tokens.)

- [ ] **Step 2: Type-check + commit.**

Run: `cd frontend && pnpm check`
Expected: clean.

```bash
git add frontend/src/lib/components/photos/ProcessingCurveChart.svelte
git commit -m "feat(web): processing curve chart component"
```

---

### Task 11: ProcessingPipeline component + page integration

**Files:**
- Create: `frontend/src/lib/components/photos/ProcessingPipeline.svelte`
- Modify: `frontend/src/lib/components/photos/PhotoDetailFull.svelte`

- [ ] **Step 1: Implement the pipeline component** (Svelte 5 runes; progressive disclosure via `<details>`):

```svelte
<script lang="ts">
  import type { ProcessingReport } from '$lib/api/types';
  import ProcessingCurveChart from './ProcessingCurveChart.svelte';
  let { report }: { report: ProcessingReport } = $props();

  function fmtDuration(s: number | null): string {
    if (s == null) return '';
    if (s < 60) return `${s.toFixed(1)}s`;
    const m = Math.floor(s / 60);
    return `${m}m ${Math.round(s % 60)}s`;
  }
  const headline = $derived(
    [report.creatorApp, `${report.pipeline.length} steps`,
     report.totalDurationS != null ? `total ${fmtDuration(report.totalDurationS)}` : null]
      .filter(Boolean).join(' · ')
  );
</script>

<section class="processing">
  <h2>Processing</h2>
  <p class="headline">{headline}</p>

  <ol class="timeline">
    {#each report.pipeline as step (step.position)}
      <li class:disabled={!step.enabled}>
        <details>
          <summary>
            <span class="cat">{step.category}</span>
            <span class="label">{step.label}</span>
            {#if step.durationS != null}<span class="dur">{fmtDuration(step.durationS)}</span>{/if}
            {#if !step.enabled}<span class="badge">disabled</span>{/if}
          </summary>

          {#if step.summary}<p class="desc">{step.summary}</p>{/if}

          {#if step.params.length}
            <dl class="params">
              {#each step.params as p (p.key)}
                <dt>{p.key}</dt>
                <dd class:truncated={p.truncated}>{p.value}</dd>
              {/each}
            </dl>
          {/if}

          {#each step.tables as t (t.id)}
            {#if t.kind === 'curve'}
              <ProcessingCurveChart table={t} />
            {:else}
              <table class="datatable">
                <thead><tr>{#each t.columns as c}<th>{c}</th>{/each}</tr></thead>
                <tbody>
                  {#each t.rows as row}<tr>{#each row as cell}<td>{cell}</td>{/each}</tr>{/each}
                </tbody>
              </table>
            {/if}
          {/each}
        </details>
      </li>
    {/each}
  </ol>
</section>

<style>
  .processing { margin-top: 2rem; }
  .headline { color: var(--text-muted, #9aa); font-size: 0.9rem; margin: 0 0 1rem; }
  .timeline { list-style: none; padding: 0; margin: 0; border-left: 2px solid var(--border, #2a2a35); }
  .timeline li { padding: 0.25rem 0 0.25rem 1rem; }
  .timeline li.disabled { opacity: 0.5; }
  summary { cursor: pointer; display: flex; gap: 0.6rem; align-items: baseline; }
  .cat { font-size: 0.7rem; text-transform: uppercase; color: var(--text-muted, #9aa); min-width: 9rem; }
  .label { font-weight: 600; }
  .dur { color: var(--text-muted, #9aa); font-size: 0.8rem; }
  .badge { font-size: 0.7rem; background: var(--border, #2a2a35); padding: 0 0.4rem; border-radius: 4px; }
  .desc { color: var(--text-muted, #9aa); margin: 0.3rem 0; }
  .params { display: grid; grid-template-columns: max-content 1fr; gap: 0.15rem 1rem; margin: 0.5rem 0; font-size: 0.85rem; }
  .params dt { color: var(--text-muted, #9aa); }
  .params dd { margin: 0; word-break: break-word; }
  .params dd.truncated { font-style: italic; opacity: 0.7; }
  .datatable { font-size: 0.8rem; border-collapse: collapse; margin: 0.4rem 0; }
  .datatable th, .datatable td { padding: 0.1rem 0.5rem; text-align: right; }
</style>
```

- [ ] **Step 2: Render it on the detail page.** In `PhotoDetailFull.svelte`: accept `processing` from props/data, import the component, and render below the Acquisition Record block:

```svelte
<script lang="ts">
  import ProcessingPipeline from './ProcessingPipeline.svelte';
  // add `processing` to the existing $props() destructure
</script>

{#if processing}
  <ProcessingPipeline report={processing} />
{/if}
```

(Wire `processing` through from `+page.server.ts` data → `+page.svelte` → `PhotoDetailFull` props, following how `photo` is already threaded.)

- [ ] **Step 3: Type-check.**

Run: `cd frontend && pnpm check`
Expected: clean.

- [ ] **Step 4: Commit.**

```bash
git add frontend/src/lib/components/photos/ProcessingPipeline.svelte frontend/src/lib/components/photos/PhotoDetailFull.svelte frontend/src/routes/u/\[handle\]/p/\[shortid\]/
git commit -m "feat(web): render processing pipeline on photo detail page"
```

---

## Phase 5 — Backfill & verification

### Task 12: Backfill existing XISF photos

**Files:**
- Create: `backend/src/photos/xisf_processing_backfill.rs`
- Modify: `backend/src/photos/mod.rs`; add a `just` recipe or a `--backfill-processing` flag (mirror how other one-shots are invoked).

- [ ] **Step 1: Implement the backfill** (header-only range read):

```rust
//! One-shot: parse + store processing_json for XISF photos that
//! predate the feature. Reads only the header via Storage::get_range —
//! a two-step exact read (16-byte length prefix, then the header), so
//! memory is bounded to the real header size, not a fixed guess.

use crate::http::AppState;
use crate::storage::Storage;

/// Hard ceiling on header size; a larger length-prefix means a corrupt
/// or hostile file — skip with a warning rather than allocate.
const MAX_HEADER_BYTES: u64 = 64 * 1024 * 1024;

/// Fetch exactly the XISF envelope + header (bytes `0..16+header_len`).
async fn fetch_header(storage: &dyn Storage, key: &str) -> anyhow::Result<Option<bytes::Bytes>> {
    let Some(prefix) = storage.get_range(key, 0, 15).await? else {
        return Ok(None); // side-channel upload: XISF not stored
    };
    if prefix.len() < 16 || &prefix[0..8] != b"XISF0100" {
        return Ok(None);
    }
    let hlen = u32::from_le_bytes([prefix[8], prefix[9], prefix[10], prefix[11]]) as u64;
    if hlen == 0 || hlen > MAX_HEADER_BYTES {
        anyhow::bail!("implausible XISF header length {hlen}");
    }
    Ok(storage.get_range(key, 0, 16 + hlen - 1).await?)
}

pub async fn run(state: &AppState) -> anyhow::Result<usize> {
    let rows: Vec<(uuid::Uuid, String)> = sqlx::query_as(
        "SELECT id, storage_key FROM photos \
         WHERE mime = 'application/x-xisf' AND processing_json IS NULL",
    )
    .fetch_all(&state.pool)
    .await?;

    let mut done = 0;
    for (id, key) in rows {
        let bytes = match fetch_header(state.storage.as_ref(), &key).await {
            Ok(Some(b)) => b,
            Ok(None) => continue,
            Err(e) => { tracing::warn!(%id, error=%e, "backfill header read failed"); continue; }
        };
        match crate::photos::xisf_processing::parse_xisf(&bytes) {
            Ok(Some(report)) => {
                let json = serde_json::to_value(&report)?;
                sqlx::query("UPDATE photos SET processing_json = $1 WHERE id = $2")
                    .bind(json).bind(id).execute(&state.pool).await?;
                done += 1;
            }
            Ok(None) => {}
            Err(e) => tracing::warn!(%id, error=%e, "backfill parse failed"),
        }
    }
    Ok(done)
}
```

(`state.storage` is an `Arc<dyn Storage>`; pass `state.storage.as_ref()`. Confirm the field name/type against `AppState`.)

- [ ] **Step 2: Wire an invocation** mirroring the existing one-shot pattern (e.g. a flag handled in `main.rs`, or a `just backfill-processing` recipe). Declare `pub mod xisf_processing_backfill;` in `mod.rs`.

- [ ] **Step 3: Build + clippy + commit.**

Run: `cd backend && cargo build && cargo clippy --all-targets -- -D warnings`

```bash
git add backend/src/photos/xisf_processing_backfill.rs backend/src/photos/mod.rs backend/src/main.rs justfile
git commit -m "feat(xisf): backfill processing_json for existing XISF photos"
```

---

### Task 13: API integration test (testcontainers)

**Files:**
- Create or extend: `backend/tests/test_photos.rs` (match the existing integration-test file naming)

- [ ] **Step 1: Write the test** — seed a photo with `processing_json`, hit the endpoint, assert sanitized shape; seed a photo without it, assert `null`:

```rust
#[tokio::test]
async fn processing_endpoint_returns_sanitized_report_or_null() {
    let app = TestApp::spawn().await; // match the existing harness constructor
    let owner = app.signup_user("astro").await;
    let photo = app.insert_ready_photo(&owner).await; // existing helper

    // no report yet → null
    let resp = app.get(&format!("/api/photos/{}/processing", photo.id)).await;
    assert_eq!(resp.status(), 200);
    assert!(resp.json::<Option<serde_json::Value>>().await.is_none());

    // seed a report with a path param that must be stripped
    let report = serde_json::json!({
        "creatorApp": "PixInsight 1.9.2", "creatorModule": null, "creatorOs": null,
        "createdAt": null, "displayStretch": null, "whiteBalance": null,
        "totalDurationS": 1.0,
        "pipeline": [{
            "position": 0, "className": "Script", "label": "Script", "category": "Bookkeeping",
            "summary": null, "version": null, "enabled": true, "startedAt": null, "durationS": null,
            "params": [{"key":"filePath","value":"$PXI_SRCDIR/x.js","truncated":false},
                       {"key":"information","value":"ImageSolver 6.3.1","truncated":false}],
            "tables": []
        }]
    });
    app.set_processing_json(photo.id, report).await; // small helper: UPDATE photos SET processing_json
    let body = app.get(&format!("/api/photos/{}/processing", photo.id)).await
        .json::<serde_json::Value>().await;
    let params = &body["pipeline"][0]["params"];
    assert_eq!(params.as_array().unwrap().len(), 1);
    assert_eq!(params[0]["key"], "information");
}
```

(Adapt to the real test harness API — constructor name, request helpers, and add the tiny `set_processing_json` helper inline if none exists.)

- [ ] **Step 2: Run (Docker required).**

Run: `cd backend && cargo test processing_endpoint -- --nocapture`
Expected: PASS.

- [ ] **Step 3: Commit.**

```bash
git add backend/tests/
git commit -m "test(xisf): integration test for /processing endpoint"
```

---

### Task 14: End-to-end verification with the real M20 file

- [ ] **Step 1: Start the stack.**

Run: `just dev`
Expected: postgres + minio + backend + frontend up. Confirm `.env` has `APP_PLATESOLVE_BASE_URL` set (XISF upload is gated on it).

- [ ] **Step 2: Upload the sample** via the `/upload` flow (browser, driven by chrome-devtools-mcp per project preference): upload `/Volumes/Pascal4Tb/astrophotos/M20/working/master/LRGB-stretched.xisf`. Wait for `status` to leave `awaiting-calibration`.

- [ ] **Step 3: Verify DB.**

Run: `psql "$APP_DATABASE_URL" -c "select jsonb_array_length(processing_json->'pipeline') from photos order by created_at desc limit 1;"`
Expected: a number ≈ 12.

- [ ] **Step 4: Verify the public page** (chrome-devtools-mcp): open the photo permalink, confirm the **Processing** section shows the headline (PixInsight 1.9.2 · N steps · total …), the ordered pipeline, that expanding SPCC shows params (and the spectral curves show as `[N values]`, not a number dump), and that CurvesTransformation renders a curve chart. Take a screenshot.

- [ ] **Step 5: Confirm sanitization** — the `Script` step must NOT show `$PXI_SRCDIR/...` or the md5sum, but SHOULD show `ImageSolver 6.3.1`.

- [ ] **Step 6: `just check`.**

Run: `just check`
Expected: zero output / all gates pass.

---

## Self-review notes (author)

- Spec coverage: parser (T2–3), storage (T4), column (T5), persistence (T6), public+sanitized API (T7), `has_processing_report` (T8), UI incl. charts & progressive disclosure (T9–11), backfill (T12), tests (T2,3,4,7,13), e2e (T14). All spec sections mapped.
- Sanitization tested both directions (T7 keeps `GaiaDR3SP`/`ImageSolver 6.3.1`/`BlurXTerminator.4.mlpackage`, drops `$PXI_SRCDIR`, abs paths, `C:\`, bare md5).
- Type names consistent across backend (`ProcessingReport`/`ProcessStep`/…) and frontend imports (`$lib/api/types`).
- Open adaptation points flagged inline where the plan must match existing idioms: S3 not-found mapping (T4), `get.rs` query style (T8), `gen-types.rs` export idiom (T8), test-harness API (T13), one-shot invocation pattern (T12).
```
