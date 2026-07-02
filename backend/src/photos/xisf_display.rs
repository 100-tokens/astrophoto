//! Display-only view over the persisted plate-solve response.
//!
//! `photos.platesolve_embed_json` (jsonb, written by
//! [`crate::photos::platesolve::save_result`]) is the source of truth
//! for everything the upstream service told us about the XISF —
//! including instrumentation that doesn't have a dedicated `photos`
//! column. Instead of growing the schema for every nice-to-have field
//! (filter / telescope / observation span / site coordinates / FITS
//! HISTORY), this module produces a typed view the verify form can
//! render directly.
//!
//! Why not extract these into the regular [`crate::photos::xisf_meta`]
//! pipeline that writes columns? Two reasons:
//!
//! 1. **Zero migration cost.** Reading the existing jsonb means we
//!    don't churn the schema for what is essentially a UI
//!    presentation concern.
//! 2. **Re-renderable.** When the upstream service starts echoing
//!    additional keys (e.g. `Process:Calibration:*` once PixInsight
//!    masters are routinely uploaded), surfacing them is a frontend
//!    change — the data has been sitting in `platesolve_embed_json`
//!    the whole time.

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use ts_rs::TS;

/// Processing-history fields we can pull out of a plate-solve
/// response. Every field is optional — XISFs from different
/// processing tools carry different subsets.
///
/// See also [`crate::photos::xisf_processing::ObservationSummary`] — the
/// public photo-page view parsed from the XISF header; deliberately
/// separate, not a duplicate.
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "XisfDisplayMeta.ts", rename_all = "camelCase")]
pub struct XisfDisplayMeta {
    /// Filter band (Hα, L, R, G, B, OIII, …). Read from FITS
    /// `FILTER` first, falls back to PCL `Instrument:Filter:Name`.
    pub filter: Option<String>,
    /// Telescope name (`TELESCOP` / `Instrument:Telescope:Name`).
    pub telescope: Option<String>,
    /// First sub-exposure start. RFC 3339. Useful as a "session"
    /// marker; the difference with [`Self::observation_end`] is a
    /// strong signal that the XISF is a multi-night stack.
    pub observation_start: Option<String>,
    /// Last sub-exposure end. RFC 3339.
    pub observation_end: Option<String>,
    /// Observatory geographic coordinates, if the file carries them.
    pub latitude_deg: Option<f64>,
    pub longitude_deg: Option<f64>,
    pub elevation_m: Option<f64>,
    /// Sub-frame count when the integration tool wrote an explicit
    /// `NCOMBINE` FITS keyword or `Process:Integration:ImageCount`
    /// PCL property. Many real-world XISFs omit this; in that case
    /// the UI shows nothing here and relies on the time span instead.
    pub subframes: Option<i32>,
    /// Filter binning (`XBINNING` / `Instrument:Camera:XBinning`).
    /// Mostly 1 in practice but useful for diagnostics.
    pub binning_x: Option<i32>,
    pub binning_y: Option<i32>,
    /// Free-text FITS `HISTORY` lines collected from the file. The
    /// XISF format keeps one `HISTORY` per logical processing step
    /// (calibration, registration, integration) — each rendered as
    /// a separate entry here. Empty lines and whitespace-only entries
    /// are filtered out.
    pub history: Vec<String>,
    /// Total integration time across all subframes, in seconds.
    /// Decoded from `PCL:TotalExposureTime` (`F64Vector` whose body is
    /// base64-encoded little-endian f64). When the vector has multiple
    /// channels (RGB integrations etc.), this is the sum across
    /// channels — same behaviour as PixInsight's display.
    pub total_exposure_s: Option<f64>,
}

impl XisfDisplayMeta {
    /// Returns true when every field is empty — caller can render a
    /// hint like "no XISF processing metadata available" instead of
    /// the section.
    pub fn is_empty(&self) -> bool {
        self.filter.is_none()
            && self.telescope.is_none()
            && self.observation_start.is_none()
            && self.observation_end.is_none()
            && self.latitude_deg.is_none()
            && self.longitude_deg.is_none()
            && self.elevation_m.is_none()
            && self.subframes.is_none()
            && self.binning_x.is_none()
            && self.binning_y.is_none()
            && self.history.is_empty()
            && self.total_exposure_s.is_none()
    }
}

/// Build a display view from the raw `platesolve_embed_json` value.
/// Accepts `null` / missing / wrong-shape gracefully — every field
/// stays `None` rather than failing.
pub fn extract_from_embed(embed: Option<&Value>) -> XisfDisplayMeta {
    let Some(embed) = embed else {
        return empty();
    };
    let fits = embed.get("fits").and_then(Value::as_array);
    let pcl = embed.get("pcl_properties").and_then(Value::as_array);
    XisfDisplayMeta {
        filter: find_fits(fits, "FILTER")
            .map(strip_quotes)
            .or_else(|| find_pcl(pcl, "Instrument:Filter:Name").map(strip_quotes))
            .filter(|s| !s.is_empty()),
        telescope: find_fits(fits, "TELESCOP")
            .map(strip_quotes)
            .or_else(|| find_pcl(pcl, "Instrument:Telescope:Name").map(strip_quotes))
            .filter(|s| !s.is_empty()),
        observation_start: parse_datetime(
            find_pcl(pcl, "Observation:Time:Start").or_else(|| find_fits(fits, "DATE-OBS")),
        )
        .map(|t| t.to_rfc3339()),
        observation_end: parse_datetime(
            find_pcl(pcl, "Observation:Time:End").or_else(|| find_fits(fits, "DATE-END")),
        )
        .map(|t| t.to_rfc3339()),
        // PCL coords are degrees; FITS `OBSGEO-*` are too (PixInsight
        // mirrors them — the `OBSGEO-X/Y/Z` ECEF triple is a separate
        // convention, but the `B/L/H` lat-lon-height set we look for
        // here is the human one).
        latitude_deg: parse_f64(
            find_pcl(pcl, "Observation:Location:Latitude")
                .or_else(|| find_fits(fits, "OBSGEO-B"))
                .or_else(|| find_fits(fits, "LAT-OBS")),
        ),
        longitude_deg: parse_f64(
            find_pcl(pcl, "Observation:Location:Longitude")
                .or_else(|| find_fits(fits, "OBSGEO-L"))
                .or_else(|| find_fits(fits, "LONG-OBS")),
        ),
        elevation_m: parse_f64(
            find_pcl(pcl, "Observation:Location:Elevation")
                .or_else(|| find_fits(fits, "OBSGEO-H"))
                .or_else(|| find_fits(fits, "ALT-OBS")),
        ),
        subframes: parse_i32(find_fits(fits, "NCOMBINE"))
            .or_else(|| parse_i32(find_pcl(pcl, "Process:Integration:ImageCount"))),
        binning_x: parse_i32(find_fits(fits, "XBINNING"))
            .or_else(|| parse_i32(find_pcl(pcl, "Instrument:Camera:XBinning"))),
        binning_y: parse_i32(find_fits(fits, "YBINNING"))
            .or_else(|| parse_i32(find_pcl(pcl, "Instrument:Camera:YBinning"))),
        history: collect_history(fits),
        // `PCL:TotalExposureTime` is an F64Vector. xisf-rs-core's
        // inline-base64 capture (PR on the service repo, late May
        // 2026) gives us the raw base64 string here; we decode it
        // ourselves to keep the platesolve-server response neutral.
        // A multi-channel master returns one entry per channel —
        // sum them so the displayed value matches PixInsight.
        total_exposure_s: decode_total_exposure(find_pcl(pcl, "PCL:TotalExposureTime")),
    }
}

fn empty() -> XisfDisplayMeta {
    XisfDisplayMeta {
        filter: None,
        telescope: None,
        observation_start: None,
        observation_end: None,
        latitude_deg: None,
        longitude_deg: None,
        elevation_m: None,
        subframes: None,
        binning_x: None,
        binning_y: None,
        history: Vec::new(),
        total_exposure_s: None,
    }
}

// ─────────────────────────────────────────────────────── helpers

fn find_fits<'a>(arr: Option<&'a Vec<Value>>, name: &str) -> Option<&'a str> {
    arr?.iter()
        .find(|item| {
            item.get("name")
                .and_then(Value::as_str)
                .is_some_and(|n| n.eq_ignore_ascii_case(name))
        })
        .and_then(|item| item.get("value").and_then(Value::as_str))
}

fn find_pcl<'a>(arr: Option<&'a Vec<Value>>, id: &str) -> Option<&'a str> {
    arr?.iter()
        .find(|item| {
            item.get("id")
                .and_then(Value::as_str)
                .is_some_and(|n| n.eq_ignore_ascii_case(id))
        })
        .and_then(|item| item.get("value").and_then(Value::as_str))
}

fn strip_quotes(s: &str) -> String {
    let v = s.trim();
    if v.starts_with('\'') && v.ends_with('\'') && v.len() >= 2 {
        v[1..v.len() - 1].trim().to_string()
    } else {
        v.to_string()
    }
}

fn parse_f64(s: Option<&str>) -> Option<f64> {
    s.and_then(|v| v.trim().trim_matches('\'').parse::<f64>().ok())
}

fn parse_i32(s: Option<&str>) -> Option<i32> {
    parse_f64(s).and_then(|v| {
        if (i32::MIN as f64..=i32::MAX as f64).contains(&v) {
            Some(v.round() as i32)
        } else {
            None
        }
    })
}

fn parse_datetime(s: Option<&str>) -> Option<DateTime<Utc>> {
    let s = s?.trim().trim_matches('\'');
    if let Ok(t) = DateTime::parse_from_rfc3339(s) {
        return Some(t.with_timezone(&Utc));
    }
    if let Ok(t) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
        return Some(DateTime::from_naive_utc_and_offset(t, Utc));
    }
    if let Ok(t) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Some(DateTime::from_naive_utc_and_offset(t, Utc));
    }
    None
}

/// Decode a base64 little-endian f64 vector (`PCL:TotalExposureTime`
/// shape) and return the sum across entries. Returns `None` on any
/// decode failure or when the string is empty / not 8-byte-aligned
/// (a pre-passthrough server response will have `value: ""`, which
/// short-circuits cleanly). Shared with [`crate::photos::xisf_meta`],
/// which decodes the same property straight from the XISF header.
pub(crate) fn decode_total_exposure(s: Option<&str>) -> Option<f64> {
    use base64::Engine;
    let raw = s?.trim();
    if raw.is_empty() {
        return None;
    }
    let bytes = base64::engine::general_purpose::STANDARD.decode(raw).ok()?;
    if bytes.is_empty() || bytes.len() % 8 != 0 {
        return None;
    }
    let mut sum = 0.0_f64;
    for chunk in bytes.chunks_exact(8) {
        // Per XISF 1.0 §10, multi-byte numeric properties are stored
        // little-endian regardless of host. PixInsight is the
        // canonical writer and follows the spec.
        let arr: [u8; 8] = chunk.try_into().ok()?;
        let v = f64::from_le_bytes(arr);
        if !v.is_finite() || v < 0.0 {
            // Defensive: don't surface NaN/Inf or negative values
            // (which would indicate a corrupted payload rather than
            // a real exposure time).
            return None;
        }
        sum += v;
    }
    Some(sum)
}

/// Collect non-empty FITS `HISTORY` lines. PixInsight writes one
/// per processing step, so a typical master ships with multiple
/// entries laid out like calibration → debayer → registration →
/// integration. We strip the FITS string quotes and skip blanks.
fn collect_history(fits: Option<&Vec<Value>>) -> Vec<String> {
    let Some(arr) = fits else { return Vec::new() };
    arr.iter()
        .filter(|item| {
            item.get("name")
                .and_then(Value::as_str)
                .is_some_and(|n| n.eq_ignore_ascii_case("HISTORY"))
        })
        .filter_map(|item| item.get("value").and_then(Value::as_str))
        .map(strip_quotes)
        .filter(|s| !s.is_empty())
        .collect()
}

// ─────────────────────────────────────────────────────── tests

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fits_kw(name: &str, value: &str) -> Value {
        json!({ "name": name, "value": value, "comment": "" })
    }

    fn pcl_prop(id: &str, type_name: &str, value: &str) -> Value {
        json!({ "id": id, "type": type_name, "value": value })
    }

    #[test]
    fn handles_null_embed_gracefully() {
        let m = extract_from_embed(None);
        assert!(m.is_empty());
        assert!(m.history.is_empty());
    }

    #[test]
    fn handles_missing_arrays() {
        // Object exists but with neither `fits` nor `pcl_properties`.
        let embed = json!({ "wcs": {} });
        let m = extract_from_embed(Some(&embed));
        assert!(m.is_empty());
    }

    #[test]
    fn extracts_filter_and_telescope_from_fits() {
        let embed = json!({
            "fits": [
                fits_kw("FILTER", "'Hα'"),
                fits_kw("TELESCOP", "'Tak FSQ-106EDX4'"),
            ],
            "pcl_properties": [],
        });
        let m = extract_from_embed(Some(&embed));
        assert_eq!(m.filter.as_deref(), Some("Hα"));
        assert_eq!(m.telescope.as_deref(), Some("Tak FSQ-106EDX4"));
    }

    #[test]
    fn falls_back_to_pcl_when_fits_absent() {
        let embed = json!({
            "fits": [],
            "pcl_properties": [
                pcl_prop("Instrument:Filter:Name", "String", "L"),
                pcl_prop("Instrument:Telescope:Name", "String", "Askar 65PHQ"),
            ],
        });
        let m = extract_from_embed(Some(&embed));
        assert_eq!(m.filter.as_deref(), Some("L"));
        assert_eq!(m.telescope.as_deref(), Some("Askar 65PHQ"));
    }

    #[test]
    fn skips_empty_filter_strings() {
        // Real-world: the PCL property exists but the value is "",
        // e.g. because the integration step didn't preserve it. We
        // shouldn't render an empty filter name in the UI.
        let embed = json!({
            "fits": [],
            "pcl_properties": [pcl_prop("Instrument:Filter:Name", "String", "")],
        });
        let m = extract_from_embed(Some(&embed));
        assert!(m.filter.is_none());
    }

    #[test]
    fn extracts_observation_time_span() {
        let embed = json!({
            "fits": [],
            "pcl_properties": [
                pcl_prop("Observation:Time:Start", "TimePoint", "2024-07-08T00:42:25.883Z"),
                pcl_prop("Observation:Time:End",   "TimePoint", "2024-07-13T03:22:42.706Z"),
            ],
        });
        let m = extract_from_embed(Some(&embed));
        assert!(
            m.observation_start
                .as_deref()
                .expect("start")
                .starts_with("2024-07-08")
        );
        assert!(
            m.observation_end
                .as_deref()
                .expect("end")
                .starts_with("2024-07-13")
        );
    }

    #[test]
    fn extracts_geolocation_from_pcl() {
        let embed = json!({
            "fits": [],
            "pcl_properties": [
                pcl_prop("Observation:Location:Latitude",  "Float64", "52.5163"),
                pcl_prop("Observation:Location:Longitude", "Float64", "13.4047"),
                pcl_prop("Observation:Location:Elevation", "Float64", "34.0"),
            ],
        });
        let m = extract_from_embed(Some(&embed));
        assert_eq!(m.latitude_deg, Some(52.5163));
        assert_eq!(m.longitude_deg, Some(13.4047));
        assert_eq!(m.elevation_m, Some(34.0));
    }

    #[test]
    fn extracts_geolocation_from_fits_aliases() {
        // OBSGEO-B / L / H is the PixInsight FITS alias for lat/lon/
        // elevation (degrees / degrees / metres).
        let embed = json!({
            "fits": [
                fits_kw("OBSGEO-B", "52.5163"),
                fits_kw("OBSGEO-L", "13.4047"),
                fits_kw("OBSGEO-H", "34.0"),
            ],
            "pcl_properties": [],
        });
        let m = extract_from_embed(Some(&embed));
        assert_eq!(m.latitude_deg, Some(52.5163));
        assert_eq!(m.longitude_deg, Some(13.4047));
        assert_eq!(m.elevation_m, Some(34.0));
    }

    #[test]
    fn ncombine_extracted_when_present() {
        let embed = json!({
            "fits": [fits_kw("NCOMBINE", "15")],
            "pcl_properties": [],
        });
        let m = extract_from_embed(Some(&embed));
        assert_eq!(m.subframes, Some(15));
    }

    #[test]
    fn subframes_falls_back_to_pcl_integration_count() {
        let embed = json!({
            "fits": [],
            "pcl_properties": [
                pcl_prop("Process:Integration:ImageCount", "Int32", "120"),
            ],
        });
        let m = extract_from_embed(Some(&embed));
        assert_eq!(m.subframes, Some(120));
    }

    #[test]
    fn collects_history_lines_and_skips_empties() {
        let embed = json!({
            "fits": [
                fits_kw("HISTORY", "Calibration: master_bias + master_dark + master_flat"),
                fits_kw("HISTORY", ""),                     // skip
                fits_kw("HISTORY", "   "),                  // skip whitespace
                fits_kw("HISTORY", "ImageRegistration: drizzle 2x"),
                fits_kw("HISTORY", "'ImageIntegration: average + Winsorized sigma clipping'"),
                fits_kw("COMMENT", "irrelevant"),           // not HISTORY
            ],
            "pcl_properties": [],
        });
        let m = extract_from_embed(Some(&embed));
        assert_eq!(m.history.len(), 3);
        assert_eq!(
            m.history[0],
            "Calibration: master_bias + master_dark + master_flat"
        );
        assert_eq!(m.history[1], "ImageRegistration: drizzle 2x");
        assert_eq!(
            m.history[2],
            "ImageIntegration: average + Winsorized sigma clipping"
        );
    }

    #[test]
    fn binning_extracted_from_both_sources() {
        let embed = json!({
            "fits": [fits_kw("XBINNING", "2"), fits_kw("YBINNING", "2")],
            "pcl_properties": [],
        });
        let m = extract_from_embed(Some(&embed));
        assert_eq!(m.binning_x, Some(2));
        assert_eq!(m.binning_y, Some(2));
    }

    #[test]
    fn fits_string_quotes_stripped_on_text_fields() {
        let embed = json!({
            "fits": [
                fits_kw("FILTER", "'L'"),
                fits_kw("TELESCOP", "'Askar 65PHQ'"),
            ],
            "pcl_properties": [],
        });
        let m = extract_from_embed(Some(&embed));
        assert_eq!(m.filter.as_deref(), Some("L"));
        assert_eq!(m.telescope.as_deref(), Some("Askar 65PHQ"));
    }

    #[test]
    fn decodes_total_exposure_from_pcl_base64_f64() {
        // The real PixInsight body for NGC6822-L120, captured from
        // staging: 8 base64 bytes encode one f64 LE. Decoded value
        // (~12664.9 s) corresponds to the master's total integration
        // time. The exact number isn't important for this test — only
        // that the decoder produces a finite positive value matching
        // a direct from_le_bytes round-trip.
        let embed = json!({
            "fits": [],
            "pcl_properties": [
                pcl_prop("PCL:TotalExposureTime", "F64Vector", "X7yZOEZ7yUA="),
            ],
        });
        let m = extract_from_embed(Some(&embed));
        let total = m.total_exposure_s.expect("decoded");
        // Reproduce the decode independently to assert the exact value.
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode("X7yZOEZ7yUA=")
            .unwrap();
        let expected = f64::from_le_bytes(bytes.try_into().unwrap());
        assert!(
            (total - expected).abs() < 1e-9,
            "total {total} vs {expected}"
        );
        assert!(total > 0.0);
    }

    #[test]
    fn sums_multi_channel_total_exposure() {
        // Multi-channel RGB integration has one entry per channel —
        // PixInsight's display sums them and we match that.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&100.0_f64.to_le_bytes());
        bytes.extend_from_slice(&200.0_f64.to_le_bytes());
        bytes.extend_from_slice(&400.0_f64.to_le_bytes());
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let embed = json!({
            "fits": [],
            "pcl_properties": [pcl_prop("PCL:TotalExposureTime", "F64Vector", &b64)],
        });
        let m = extract_from_embed(Some(&embed));
        assert_eq!(m.total_exposure_s, Some(700.0));
    }

    #[test]
    fn empty_total_exposure_string_yields_none() {
        // Pre-xisf-rs-fix the service returned `value: ""` for vector
        // properties. The decoder must short-circuit rather than
        // surface a 0.0 that would render as "0 s".
        let embed = json!({
            "fits": [],
            "pcl_properties": [pcl_prop("PCL:TotalExposureTime", "F64Vector", "")],
        });
        let m = extract_from_embed(Some(&embed));
        assert!(m.total_exposure_s.is_none());
    }

    #[test]
    fn malformed_total_exposure_rejected_quietly() {
        // Garbage in the base64 or non-8-byte payload → None, not a
        // panic. Important because the property is untrusted user
        // input through several network hops.
        for bad in [
            "not-base64!!!",
            "AAAA",             // 3 bytes after decode — not 8-byte aligned
            "////////////////", // 12 bytes — also not aligned
        ] {
            let embed = json!({
                "fits": [],
                "pcl_properties": [pcl_prop("PCL:TotalExposureTime", "F64Vector", bad)],
            });
            let m = extract_from_embed(Some(&embed));
            assert!(m.total_exposure_s.is_none(), "should reject `{bad}`");
        }
    }

    #[test]
    fn case_insensitive_lookup() {
        // FITS spec is case-insensitive on names; PCL IDs are case-
        // sensitive in principle but real exporters disagree —
        // accepting case-insensitive is more forgiving.
        let embed = json!({
            "fits": [fits_kw("filter", "Hα")],
            "pcl_properties": [pcl_prop("instrument:telescope:name", "String", "RC10")],
        });
        let m = extract_from_embed(Some(&embed));
        assert_eq!(m.filter.as_deref(), Some("Hα"));
        assert_eq!(m.telescope.as_deref(), Some("RC10"));
    }
}
