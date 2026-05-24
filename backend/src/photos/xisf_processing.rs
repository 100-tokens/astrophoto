//! Local parser for the XISF header's PixInsight processing metadata.
//!
//! The XISF format stores a monolithic XML header at byte offset 16
//! (after the 8-byte `XISF0100` signature + a u32 LE header length +
//! 4 reserved bytes). PixInsight writes the full processing pipeline
//! into a `PixInsight:ProcessingHistory` <Property> as an *embedded*,
//! XML-escaped document (carried in the element's TEXT, not a `value`
//! attribute, for a doc this large). This module extracts that pipeline
//! plus the creator app, the display-stretch (STF), and SPCC white
//! balance.
//!
//! The outer header carries a default XML namespace
//! (`xmlns="http://www.pixinsight.com/xisf"`); the inner ProcessingHistory
//! document does not. We therefore match elements by *local name*
//! (`tag_name().name()`) so both parse uniformly regardless of namespace.
//!
//! Parsing is pure and returns `Ok(None)` for a valid XISF that carries
//! no processing history; only a structurally broken header is `Err`.

use base64::{Engine, engine::general_purpose::STANDARD};
use roxmltree::{Document, Node};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

const SIGNATURE: &[u8] = b"XISF0100";
/// Param values longer than this are stored truncated (spectral / QE
/// curves are thousands of chars and never displayed verbatim).
const MAX_PARAM_VALUE_LEN: usize = 512;

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct KeyValue {
    pub key: String,
    pub value: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct ProcessTable {
    pub id: String,
    pub kind: String, // "curve" | "histogram" | "channels" | "generic"
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
pub struct WhiteBalance {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
#[ts(export, rename_all = "camelCase")]
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

// ──────────────────────────────────────────────── implementation

/// Read the XISF binary envelope and parse its header XML.
pub fn parse_xisf(bytes: &[u8]) -> Result<Option<ProcessingReport>, ProcessingParseError> {
    if bytes.len() < 16 || &bytes[0..8] != SIGNATURE {
        return Err(ProcessingParseError::BadSignature);
    }
    let hlen = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    let end = 16usize
        .checked_add(hlen)
        .ok_or(ProcessingParseError::BadHeader)?;
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

    for p in doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "Property")
    {
        let id = p.attribute("id").unwrap_or("");
        let val = p
            .attribute("value")
            .map(str::to_string)
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

    let display_stretch = doc
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "DisplayFunction")
        .and_then(parse_display_function);

    let Some(pipeline) = history else {
        return Ok(None);
    };

    let total_duration_s = {
        let sum: f64 = pipeline.iter().filter_map(|s| s.duration_s).sum();
        (sum > 0.0).then_some(sum)
    };

    Ok(Some(ProcessingReport {
        creator_app,
        creator_module,
        creator_os,
        created_at,
        display_stretch,
        white_balance,
        total_duration_s,
        pipeline,
    }))
}

/// Parse the inner `<ProcessingHistory>` document into ordered steps.
pub fn parse_processing_history_xml(xml: &str) -> Result<Vec<ProcessStep>, ProcessingParseError> {
    let doc = Document::parse(xml).map_err(|e| ProcessingParseError::Xml(e.to_string()))?;
    let root = doc.root_element();
    let mut steps = Vec::new();
    for (i, inst) in root
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "instance")
        .enumerate()
    {
        let class_name = inst.attribute("class").unwrap_or("Unknown").to_string();
        let (label, category, summary) = classify(&class_name);
        let enabled = inst
            .attribute("enabled")
            .map(|v| v == "true")
            .unwrap_or(true);
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
                        let raw = child
                            .attribute("value")
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
            class_name,
            label,
            category,
            summary,
            version,
            enabled,
            started_at,
            duration_s,
            params,
            tables,
        });
    }
    Ok(steps)
}

fn make_kv(key: &str, value: String) -> KeyValue {
    if value.chars().count() > MAX_PARAM_VALUE_LEN {
        let n = value.split(',').count();
        KeyValue {
            key: key.to_string(),
            value: format!("[{n} values]"),
            truncated: true,
        }
    } else {
        KeyValue {
            key: key.to_string(),
            value,
            truncated: false,
        }
    }
}

fn parse_table(node: Node) -> Option<ProcessTable> {
    let id = node.attribute("id").unwrap_or("").to_string();
    let mut columns: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<String>> = Vec::new();
    for tr in node
        .children()
        .filter(|n| n.is_element() && n.tag_name().name() == "tr")
    {
        let mut row = Vec::new();
        for td in tr
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() == "td")
        {
            if rows.is_empty()
                && let Some(col) = td.attribute("id")
            {
                columns.push(col.to_string());
            }
            let v = td
                .attribute("value")
                .map(str::to_string)
                .unwrap_or_else(|| td.text().unwrap_or("").trim().to_string());
            row.push(v);
        }
        if !row.is_empty() {
            rows.push(row);
        }
    }
    let kind = if columns.iter().map(String::as_str).eq(["x", "y"]) {
        "curve"
    } else if id == "H" {
        "histogram"
    } else if id == "channels" {
        "channels"
    } else {
        "generic"
    }
    .to_string();
    Some(ProcessTable {
        id,
        kind,
        columns,
        rows,
    })
}

fn parse_display_function(node: Node) -> Option<DisplayStretch> {
    let split = |attr: &str| -> Vec<f64> {
        node.attribute(attr)
            .map(|s| s.split(':').filter_map(|x| x.parse().ok()).collect())
            .unwrap_or_default()
    };
    Some(DisplayStretch {
        midtones: split("m"),
        shadows: split("s"),
        highlights: split("h"),
        low_range: split("l"),
        high_range: split("r"),
    })
}

/// SPCC white-balance factors are a base64 little-endian f64 vector
/// `[R, G, B, ...]`. Decode the first three.
fn decode_white_balance(b64: &str) -> Option<WhiteBalance> {
    let raw = STANDARD.decode(b64.trim()).ok()?;
    let f = |i: usize| -> Option<f64> {
        let arr: [u8; 8] = raw.get(i * 8..i * 8 + 8)?.try_into().ok()?;
        Some(f64::from_le_bytes(arr))
    };
    Some(WhiteBalance {
        red: f(0)?,
        green: f(1)?,
        blue: f(2)?,
    })
}

/// Map a PixInsight process class to (label, category, optional summary).
fn classify(class_name: &str) -> (String, String, Option<String>) {
    let (label, category, summary): (&str, &str, Option<&str>) = match class_name {
        "ChannelCombination" => (
            "Channel Combination",
            "Composition",
            Some("Combine R/G/B channels into a color image"),
        ),
        "LRGBCombination" => (
            "LRGB Combination",
            "Composition",
            Some("Blend a luminance layer with RGB color"),
        ),
        "ImageIdentifier" => ("Image Identifier", "Bookkeeping", None),
        "Script" => ("Script", "Bookkeeping", None),
        "SpectrophotometricColorCalibration" => (
            "SpectroPhotometric Color Calibration",
            "Color calibration",
            Some("Calibrate color from star spectra (SPCC)"),
        ),
        "PhotometricColorCalibration" => (
            "Photometric Color Calibration",
            "Color calibration",
            Some("Calibrate color from catalog photometry"),
        ),
        "BackgroundNeutralization" => ("Background Neutralization", "Color calibration", None),
        "BlurXTerminator" => (
            "BlurXTerminator",
            "Sharpening (AI)",
            Some("AI deconvolution & detail recovery"),
        ),
        "Deconvolution" => ("Deconvolution", "Sharpening", None),
        "NoiseXTerminator" => (
            "NoiseXTerminator",
            "Noise reduction (AI)",
            Some("AI noise reduction"),
        ),
        "MultiscaleLinearTransform" => ("Multiscale Linear Transform", "Noise reduction", None),
        "StarXTerminator" => (
            "StarXTerminator",
            "Star handling (AI)",
            Some("AI star removal"),
        ),
        "StarNet2" => ("StarNet", "Star handling (AI)", Some("AI star removal")),
        "DynamicCrop" => ("Dynamic Crop", "Geometry", Some("Crop / rotate the frame")),
        "HistogramTransformation" => (
            "Histogram Transformation",
            "Stretch",
            Some("Non-linear brightness stretch"),
        ),
        "CurvesTransformation" => (
            "Curves Transformation",
            "Adjustment",
            Some("Per-channel tone / saturation curves"),
        ),
        "ImageIntegration" => (
            "Image Integration",
            "Stacking",
            Some("Stack calibrated sub-exposures"),
        ),
        "StarAlignment" => (
            "Star Alignment",
            "Registration",
            Some("Register frames to a reference"),
        ),
        "DrizzleIntegration" => ("Drizzle Integration", "Stacking", None),
        "GradientCorrection" | "AutomaticBackgroundExtractor" | "DynamicBackgroundExtraction" => (
            "Gradient / Background Extraction",
            "Gradient",
            Some("Remove sky gradients"),
        ),
        _ => return (decamel(class_name), "Other".to_string(), None),
    };
    (
        label.to_string(),
        category.to_string(),
        summary.map(str::to_string),
    )
}

/// "SomeNewProcess" -> "Some New Process" for unknown classes.
fn decamel(s: &str) -> String {
    let mut out = String::new();
    for (i, c) in s.char_indices() {
        if i > 0 && c.is_uppercase() {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

// ─────────────────────────────────────────────────────────── tests

#[cfg(test)]
mod tests {
    use super::*;

    const PH: &str = include_str!("../../tests/fixtures/m20_processing_history.xml");

    /// Build a minimal namespaced XISF header that embeds the inner
    /// ProcessingHistory as escaped element TEXT — matching how real
    /// PixInsight files store it (not a `value` attribute).
    fn wrap_header(inner_history: &str) -> String {
        let escaped = inner_history
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<xisf version="1.0" xmlns="http://www.pixinsight.com/xisf">
<Image geometry="100:100:1" sampleFormat="Float32" colorSpace="Gray">
<FITSKeyword name="FILTER" value="'L'" comment="filter"/>
<DisplayFunction m="0.44:0.41:0.41:0.5" s="0:0.01:0:0" h="1:1:1:1" l="0:0:0:0" r="1:1:1:1"/>
<Property id="XISF:CreatorApplication" type="String" value="PixInsight 1.9.2"/>
<Property id="XISF:CreatorOS" type="String" value="macOS"/>
<Property id="XISF:CreationTime" type="String" value="2025-07-20T14:26:17Z"/>
<Property id="PixInsight:ProcessingHistory" type="String">{escaped}</Property>
</Image>
</xisf>"#
        )
    }

    #[test]
    fn parses_pipeline_in_order_with_timing() {
        let steps = parse_processing_history_xml(PH).expect("parses");
        let classes: Vec<&str> = steps.iter().map(|s| s.class_name.as_str()).collect();
        assert!(classes.contains(&"SpectrophotometricColorCalibration"));
        assert!(classes.contains(&"CurvesTransformation"));
        for (i, s) in steps.iter().enumerate() {
            assert_eq!(s.position as usize, i);
        }
        let spcc = steps
            .iter()
            .find(|s| s.class_name == "SpectrophotometricColorCalibration")
            .unwrap();
        assert!(spcc.enabled);
        assert_eq!(spcc.label, "SpectroPhotometric Color Calibration");
        assert_eq!(spcc.category, "Color calibration");
        assert!(spcc.duration_s.unwrap() > 0.0);
        assert!(spcc.started_at.as_deref().unwrap().starts_with("2025-"));
        let cat = spcc.params.iter().find(|p| p.key == "catalogId").unwrap();
        assert_eq!(cat.value, "GaiaDR3SP");
        // long spectral-curve params are summarized, not dumped verbatim
        assert!(spcc.params.iter().any(|p| p.truncated));
    }

    #[test]
    fn parses_curve_table_as_points() {
        let steps = parse_processing_history_xml(PH).unwrap();
        let curves = steps
            .iter()
            .find(|s| s.class_name == "CurvesTransformation")
            .unwrap();
        let k = curves.tables.iter().find(|t| t.id == "K").unwrap();
        assert_eq!(k.kind, "curve");
        assert_eq!(k.columns, vec!["x".to_string(), "y".to_string()]);
        assert!(k.rows.len() >= 2);
        assert_eq!(
            k.rows[0],
            vec!["0.00000".to_string(), "0.00000".to_string()]
        );
    }

    #[test]
    fn unknown_class_falls_back_to_decamel() {
        let xml = r#"<?xml version="1.0"?><ProcessingHistory version="1.0">
            <instance class="SomeNovelProcess" version="1" enabled="true"/>
            </ProcessingHistory>"#;
        let steps = parse_processing_history_xml(xml).unwrap();
        assert_eq!(steps[0].label, "Some Novel Process");
        assert_eq!(steps[0].category, "Other");
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
        assert!((stf.shadows[1] - 0.01).abs() < 1e-6);
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
        assert!(matches!(
            parse_xisf(b"NOTXISF.."),
            Err(ProcessingParseError::BadSignature)
        ));
    }

    /// Manual end-to-end check against a real PixInsight master. Ignored
    /// by default (CI has no such file); run locally with:
    /// `XISF_SAMPLE=/path/to.xisf cargo test --lib parses_real_xisf_sample -- --ignored --nocapture`
    #[test]
    #[ignore = "reads a local .xisf file named by the XISF_SAMPLE env var"]
    fn parses_real_xisf_sample() {
        let path = std::env::var("XISF_SAMPLE").expect("set XISF_SAMPLE to a .xisf path");
        let bytes = std::fs::read(&path).expect("read sample file");
        let report = parse_xisf(&bytes).expect("parse ok").expect("has report");
        eprintln!(
            "creator={:?} os={:?} steps={} total={:?}s wb={:?}",
            report.creator_app,
            report.creator_os,
            report.pipeline.len(),
            report.total_duration_s,
            report.white_balance,
        );
        for s in &report.pipeline {
            eprintln!(
                "  [{:>2}] {:<32} {:<22} {:>8.2?}s  params={:<2} tables={}",
                s.position,
                s.label,
                s.category,
                s.duration_s,
                s.params.len(),
                s.tables.len()
            );
        }
        if let Ok(out) = std::env::var("XISF_JSON_OUT") {
            std::fs::write(&out, serde_json::to_string_pretty(&report).unwrap()).unwrap();
            eprintln!("wrote report JSON to {out}");
        }
        assert!(report.pipeline.len() >= 10, "expected the full pipeline");
        assert!(report.display_stretch.is_some(), "STF present");
    }
}
