//! XISF header → photo column mapping.
//!
//! The plate-solve service returns the photo's FITS keywords + PCL
//! properties as part of every successful `/v1/solve` response (see
//! [`crate::photos::platesolve::PlatesolveResult`]). For the
//! primary-XISF-upload flow this is the "EXIF analog": the
//! instrumentation captured at the telescope (camera model, exposure
//! time, focal length, gain, sensor temperature, etc.) lives in the
//! XISF header, never in the JPEG-style EXIF blob that the standard
//! upload pipeline parses with `kamadak-exif`.
//!
//! ## Source priority
//!
//! For each field we check the FITS keyword first (more universally
//! present — most stacking tools write FITS into XISF for back-compat
//! with older astro software) and fall back to the PCL property if
//! no FITS keyword is present. Caller-supplied values always win:
//! [`apply`] uses `COALESCE(column, $param)` so existing data on the
//! photo row is never overwritten.
//!
//! ## Service-side prerequisite
//!
//! As of this commit `xisf-rs-platesolve-server`'s `/v1/solve`
//! response only contains WCS-relevant FITS keywords + the
//! `Observation:Center:*` + `AstrometricSolution:*` PCL properties
//! it produces itself. The Instrument:* / EXPTIME / FOCALLEN / etc.
//! from the input XISF are read by the solver (for hint derivation)
//! but not echoed back. A follow-up service change to passthrough
//! those keys is what makes this extractor populate real values; the
//! consumer code lands first so the wiring is in place when that
//! lands.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::photos::platesolve::{FitsKeyword, PclProperty, PlatesolveResult};

/// Subset of `photos` columns we can populate from a typical XISF
/// header. Every field is `Option`: missing keys stay `None` and the
/// caller's [`apply`] preserves whatever was already on the row.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct XisfMetadata {
    pub camera: Option<String>,
    pub exposure_s: Option<f64>,
    pub focal_mm: Option<f64>,
    pub aperture_f: Option<f32>,
    pub gain: Option<i16>,
    pub sensor_temp_c: Option<f32>,
    pub sessions: Option<i16>,
    pub taken_at: Option<DateTime<Utc>>,
    pub target: Option<String>,
    /// Total integration time in seconds, decoded from the
    /// `PCL:TotalExposureTime` property PixInsight's ImageIntegration
    /// writes (F64Vector, summed across channels). Deliberately NOT
    /// derived from `exposure_s × sessions` here — stats queries do
    /// that fallback in SQL so we never persist derived data.
    pub integration_s: Option<f64>,
}

impl XisfMetadata {
    /// Returns true if no field was populated — caller can skip the
    /// SQL UPDATE entirely.
    pub fn is_empty(&self) -> bool {
        self.camera.is_none()
            && self.exposure_s.is_none()
            && self.focal_mm.is_none()
            && self.aperture_f.is_none()
            && self.gain.is_none()
            && self.sensor_temp_c.is_none()
            && self.sessions.is_none()
            && self.taken_at.is_none()
            && self.target.is_none()
            && self.integration_s.is_none()
    }
}

/// Walk the solve response's FITS + PCL arrays and build a
/// [`XisfMetadata`]. FITS wins when both sources define the same
/// field; both being absent leaves the field `None`.
pub fn extract(result: &PlatesolveResult) -> XisfMetadata {
    extract_parts(&result.fits, &result.pcl_properties)
}

/// Parse the XISF header XML directly and build a [`XisfMetadata`].
///
/// This is the local counterpart of [`extract`]: it reads the same
/// FITS keywords + PCL properties straight from the header bytes we
/// already hold at calibration time, so it works even when the
/// plate-solve fails or the solver never echoes instrument keywords
/// back (the gap documented in the module docs above). `None` only
/// when the XML itself does not parse.
pub fn extract_from_header_xml(xml: &str) -> Option<XisfMetadata> {
    let doc = roxmltree::Document::parse(xml).ok()?;
    let mut fits: Vec<FitsKeyword> = Vec::new();
    let mut pcl: Vec<PclProperty> = Vec::new();
    for n in doc.descendants().filter(roxmltree::Node::is_element) {
        match n.tag_name().name() {
            "FITSKeyword" => {
                if let (Some(name), Some(value)) = (n.attribute("name"), n.attribute("value")) {
                    fits.push(FitsKeyword {
                        name: name.to_string(),
                        value: value.to_string(),
                        comment: String::new(),
                    });
                }
            }
            "Property" => {
                if let Some(id) = n.attribute("id") {
                    // Scalar properties carry `value=`; vector properties
                    // (e.g. PCL:TotalExposureTime) carry base64 body text.
                    let value = n
                        .attribute("value")
                        .map(str::to_string)
                        .unwrap_or_else(|| n.text().unwrap_or("").trim().to_string());
                    pcl.push(PclProperty {
                        id: id.to_string(),
                        type_name: n.attribute("type").unwrap_or("").to_string(),
                        value,
                    });
                }
            }
            _ => {}
        }
    }
    Some(extract_parts(&fits, &pcl))
}

fn extract_parts(fits: &[FitsKeyword], pcl: &[PclProperty]) -> XisfMetadata {
    XisfMetadata {
        camera: find_fits(fits, "INSTRUME")
            .map(strip_fits_quotes)
            .or_else(|| find_pcl(pcl, "Instrument:Camera:Name").map(strip_fits_quotes)),
        // FITS EXPTIME / EXPOSURE are both seconds. Some camera
        // control software writes EXPOSURE only.
        exposure_s: parse_f64(find_fits(fits, "EXPTIME"))
            .or_else(|| parse_f64(find_fits(fits, "EXPOSURE")))
            .or_else(|| parse_f64(find_pcl(pcl, "Instrument:ExposureTime"))),
        // FITS FOCALLEN is millimetres by convention; PCL's
        // FocalLength is metres (FITS convention is what most users
        // expect on the form, so we convert PCL on the way out).
        focal_mm: parse_f64(find_fits(fits, "FOCALLEN")).or_else(|| {
            parse_f64(find_pcl(pcl, "Instrument:Telescope:FocalLength")).map(metres_to_mm)
        }),
        // FITS rarely has FNUMBER (it's a photographic concept, not
        // astrographic). Skip f-ratio derivation for v1 — users can
        // enter it manually if they care.
        aperture_f: parse_f32(find_fits(fits, "FNUMBER")),
        gain: parse_i16(find_fits(fits, "GAIN"))
            .or_else(|| parse_i16(find_pcl(pcl, "Instrument:Camera:Gain"))),
        sensor_temp_c: parse_f32(find_fits(fits, "CCD-TEMP"))
            .or_else(|| parse_f32(find_fits(fits, "CCDTEMP")))
            .or_else(|| parse_f32(find_pcl(pcl, "Instrument:Sensor:Temperature"))),
        // NCOMBINE is what PixInsight's integration writes for the
        // number of stacked subexposures.
        sessions: parse_i16(find_fits(fits, "NCOMBINE"))
            .or_else(|| parse_i16(find_pcl(pcl, "Process:Integration:ImageCount"))),
        taken_at: parse_datetime(find_fits(fits, "DATE-OBS"))
            .or_else(|| parse_datetime(find_pcl(pcl, "Observation:Time:Start"))),
        target: find_fits(fits, "OBJECT")
            .map(strip_fits_quotes)
            .or_else(|| find_pcl(pcl, "Observation:Object:Name").map(strip_fits_quotes)),
        // PixInsight writes the total as an F64Vector (base64 LE);
        // tolerate a plain scalar first for non-PixInsight writers.
        integration_s: parse_f64(find_pcl(pcl, "PCL:TotalExposureTime")).or_else(|| {
            crate::photos::xisf_display::decode_total_exposure(find_pcl(
                pcl,
                "PCL:TotalExposureTime",
            ))
        }),
    }
}

/// Strip the FITS single-quote string convention (`'NGC 6822'` →
/// `NGC 6822`) and surrounding whitespace. PCL string values aren't
/// quoted but harmlessly pass through.
fn strip_fits_quotes(s: &str) -> String {
    let v = s.trim();
    let stripped = if v.starts_with('\'') && v.ends_with('\'') && v.len() >= 2 {
        v[1..v.len() - 1].trim()
    } else {
        v
    };
    stripped.to_string()
}

/// Persist whatever non-`None` fields the extractor produced. Uses
/// `COALESCE(column, $param)` so a value the user (or an earlier
/// pipeline run) already set is never overwritten. Runtime sqlx —
/// the column set is wide enough that promoting to compile-time
/// after `cargo sqlx prepare` is a separate small chore.
///
/// No-op fast-path when [`XisfMetadata::is_empty`].
pub async fn apply(pool: &PgPool, photo_id: Uuid, meta: &XisfMetadata) -> Result<(), AppError> {
    if meta.is_empty() {
        return Ok(());
    }
    sqlx::query(
        r#"
        update photos set
            camera        = coalesce(camera, $1),
            exposure_s    = coalesce(exposure_s, $2),
            focal_mm      = coalesce(focal_mm, $3),
            aperture_f    = coalesce(aperture_f, $4),
            gain          = coalesce(gain, $5),
            sensor_temp_c = coalesce(sensor_temp_c, $6),
            sessions      = coalesce(sessions, $7),
            taken_at      = coalesce(taken_at, $8),
            target        = coalesce(target, $9),
            integration_s = coalesce(integration_s, $10)
        where id = $11
        "#,
    )
    .bind(&meta.camera)
    .bind(meta.exposure_s)
    .bind(meta.focal_mm)
    .bind(meta.aperture_f)
    .bind(meta.gain)
    .bind(meta.sensor_temp_c)
    .bind(meta.sessions)
    .bind(meta.taken_at)
    .bind(&meta.target)
    .bind(meta.integration_s)
    .bind(photo_id)
    .execute(pool)
    .await
    .map_err(AppError::from)?;
    Ok(())
}

// ─────────────────────────────────────────────────────── helpers

fn find_fits<'a>(keys: &'a [FitsKeyword], name: &str) -> Option<&'a str> {
    keys.iter()
        .find(|k| k.name.eq_ignore_ascii_case(name))
        .map(|k| k.value.as_str())
}

fn find_pcl<'a>(props: &'a [PclProperty], id: &str) -> Option<&'a str> {
    props
        .iter()
        .find(|p| p.id.eq_ignore_ascii_case(id))
        .map(|p| p.value.as_str())
}

fn parse_f64(s: Option<&str>) -> Option<f64> {
    s.and_then(|v| v.trim().trim_matches('\'').parse::<f64>().ok())
}

fn parse_f32(s: Option<&str>) -> Option<f32> {
    s.and_then(|v| v.trim().trim_matches('\'').parse::<f32>().ok())
}

fn parse_i16(s: Option<&str>) -> Option<i16> {
    parse_f64(s).and_then(|v| {
        if (i16::MIN as f64..=i16::MAX as f64).contains(&v) {
            Some(v.round() as i16)
        } else {
            None
        }
    })
}

fn parse_datetime(s: Option<&str>) -> Option<DateTime<Utc>> {
    let s = s?.trim().trim_matches('\'');
    // RFC 3339 / ISO 8601 with a timezone — PCL `Observation:Time:Start`.
    if let Ok(t) = DateTime::parse_from_rfc3339(s) {
        return Some(t.with_timezone(&Utc));
    }
    // FITS DATE-OBS: naive `YYYY-MM-DDTHH:MM:SS[.fff]`, UTC by spec.
    if let Ok(t) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f") {
        return Some(DateTime::from_naive_utc_and_offset(t, Utc));
    }
    if let Ok(t) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
        return Some(DateTime::from_naive_utc_and_offset(t, Utc));
    }
    None
}

fn metres_to_mm(m: f64) -> f64 {
    m * 1000.0
}

// ─────────────────────────────────────────────────────── tests

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::photos::platesolve::{HintSource, Wcs};

    fn empty_result() -> PlatesolveResult {
        PlatesolveResult {
            wcs: Wcs {
                ra_deg: 0.0,
                dec_deg: 0.0,
                pixel_scale_arcsec: 0.0,
                rotation_deg: 0.0,
                flip_x: false,
                crpix_x: 0.0,
                crpix_y: 0.0,
                cd: [[0.0, 0.0], [0.0, 0.0]],
            },
            rms_arcsec: 0.0,
            matched_count: 0,
            detected_count: 0,
            iterations: 0,
            obs_epoch_jyear: 0.0,
            hint_source: HintSource {
                ra: String::new(),
                dec: String::new(),
                scale: String::new(),
                rotation: None,
                epoch: None,
            },
            fits: vec![],
            pcl_properties: vec![],
            has_distortion: false,
            elapsed_ms: 0,
            render: None,
        }
    }

    fn fits(name: &str, value: &str) -> FitsKeyword {
        FitsKeyword {
            name: name.to_string(),
            value: value.to_string(),
            comment: String::new(),
        }
    }

    fn pcl(id: &str, type_name: &str, value: &str) -> PclProperty {
        PclProperty {
            id: id.to_string(),
            type_name: type_name.to_string(),
            value: value.to_string(),
        }
    }

    #[test]
    fn extract_from_fits_keywords() {
        let mut r = empty_result();
        r.fits = vec![
            fits("INSTRUME", "ZWO ASI2600MM Pro"),
            fits("EXPTIME", "300.0"),
            fits("FOCALLEN", "1000.0"),
            fits("GAIN", "100"),
            fits("CCD-TEMP", "-10.5"),
            fits("NCOMBINE", "24"),
            fits("DATE-OBS", "2024-05-15T02:07:46.123"),
            fits("OBJECT", "M51"),
        ];
        let m = extract(&r);
        assert_eq!(m.camera.as_deref(), Some("ZWO ASI2600MM Pro"));
        assert_eq!(m.exposure_s, Some(300.0));
        assert_eq!(m.focal_mm, Some(1000.0));
        assert_eq!(m.gain, Some(100));
        assert_eq!(m.sensor_temp_c, Some(-10.5));
        assert_eq!(m.sessions, Some(24));
        assert!(m.taken_at.is_some());
        assert_eq!(m.target.as_deref(), Some("M51"));
        assert_eq!(m.aperture_f, None);
    }

    #[test]
    fn extract_falls_back_to_pcl_when_fits_missing() {
        let mut r = empty_result();
        r.pcl_properties = vec![
            pcl("Instrument:Camera:Name", "String", "QHY 268M"),
            pcl("Instrument:ExposureTime", "Float64", "180.0"),
            // PCL focal length is in metres; extractor converts to mm.
            pcl("Instrument:Telescope:FocalLength", "Float64", "1.5"),
            pcl("Instrument:Camera:Gain", "Int32", "26"),
            pcl("Observation:Object:Name", "String", "NGC 7000"),
        ];
        let m = extract(&r);
        assert_eq!(m.camera.as_deref(), Some("QHY 268M"));
        assert_eq!(m.exposure_s, Some(180.0));
        assert_eq!(m.focal_mm, Some(1500.0));
        assert_eq!(m.gain, Some(26));
        assert_eq!(m.target.as_deref(), Some("NGC 7000"));
    }

    #[test]
    fn fits_wins_over_pcl_when_both_present() {
        let mut r = empty_result();
        r.fits = vec![fits("EXPTIME", "300.0")];
        r.pcl_properties = vec![pcl("Instrument:ExposureTime", "Float64", "180.0")];
        let m = extract(&r);
        assert_eq!(
            m.exposure_s,
            Some(300.0),
            "FITS should win when both present"
        );
    }

    #[test]
    fn empty_response_yields_empty_metadata() {
        let r = empty_result();
        let m = extract(&r);
        assert!(m.is_empty());
    }

    #[test]
    fn ccdtemp_alias_parsed() {
        // Some camera control software writes CCDTEMP without the
        // hyphen — both should be accepted.
        let mut r = empty_result();
        r.fits = vec![fits("CCDTEMP", "-15.0")];
        let m = extract(&r);
        assert_eq!(m.sensor_temp_c, Some(-15.0));
    }

    #[test]
    fn datetime_parser_accepts_fits_and_pcl_shapes() {
        // FITS DATE-OBS: naive ISO, no timezone (UTC by spec).
        assert!(parse_datetime(Some("2024-05-15T02:07:46.123")).is_some());
        assert!(parse_datetime(Some("2024-05-15T02:07:46")).is_some());
        // PCL Observation:Time:Start: RFC 3339 with timezone.
        assert!(parse_datetime(Some("2024-05-15T02:07:46.123Z")).is_some());
        assert!(parse_datetime(Some("2024-05-15T04:07:46+02:00")).is_some());
        // Garbage rejected.
        assert!(parse_datetime(Some("not a date")).is_none());
        assert!(parse_datetime(None).is_none());
    }

    #[test]
    fn name_lookup_is_case_insensitive() {
        // Some pipelines write keys in mixed case; the FITS standard
        // is case-insensitive on keyword names.
        let mut r = empty_result();
        r.fits = vec![fits("instrume", "Mixed Case Cam")];
        let m = extract(&r);
        assert_eq!(m.camera.as_deref(), Some("Mixed Case Cam"));
    }

    #[test]
    fn quoted_fits_values_are_stripped() {
        // Some emitters wrap numeric values in single quotes (FITS
        // string literal). The parser must tolerate.
        let mut r = empty_result();
        r.fits = vec![fits("EXPTIME", "'120.5'")];
        let m = extract(&r);
        assert_eq!(m.exposure_s, Some(120.5));
    }

    #[test]
    fn quoted_fits_string_values_are_unquoted() {
        // FITS strings are wrapped in single quotes per the spec; the
        // service hands them through verbatim. Strip them so the UI
        // doesn't render `'NGC 6822'` to the user.
        let mut r = empty_result();
        r.fits = vec![
            fits("INSTRUME", "'ZWO ASI533MM Pro'"),
            fits("OBJECT", "'NGC 6822'"),
        ];
        let m = extract(&r);
        assert_eq!(m.camera.as_deref(), Some("ZWO ASI533MM Pro"));
        assert_eq!(m.target.as_deref(), Some("NGC 6822"));
    }

    /// Base64 LE f64 vector, PixInsight's `PCL:TotalExposureTime` shape.
    fn b64_f64s(vals: &[f64]) -> String {
        use base64::Engine;
        let mut bytes = Vec::with_capacity(vals.len() * 8);
        for v in vals {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        base64::engine::general_purpose::STANDARD.encode(bytes)
    }

    #[test]
    fn header_xml_extracts_fits_and_pcl_locally() {
        let total = b64_f64s(&[401_400.0]);
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<xisf version="1.0" xmlns="http://www.pixinsight.com/xisf">
<Image geometry="10:10:1" sampleFormat="Float32" colorSpace="Gray">
<FITSKeyword name="INSTRUME" value="'ZWO ASI533MM PRO'" comment="camera"/>
<FITSKeyword name="EXPTIME" value="300.0" comment="s"/>
<FITSKeyword name="NCOMBINE" value="1338" comment=""/>
<FITSKeyword name="OBJECT" value="'NGC 5982'" comment=""/>
<Property id="PCL:TotalExposureTime" type="F64Vector" length="1">{total}</Property>
</Image>
</xisf>"#
        );
        let m = extract_from_header_xml(&xml).expect("parses");
        assert_eq!(m.camera.as_deref(), Some("ZWO ASI533MM PRO"));
        assert_eq!(m.exposure_s, Some(300.0));
        assert_eq!(m.sessions, Some(1338));
        assert_eq!(m.target.as_deref(), Some("NGC 5982"));
        assert_eq!(m.integration_s, Some(401_400.0));
    }

    #[test]
    fn header_xml_total_exposure_sums_channels() {
        let total = b64_f64s(&[100.0, 250.5, 50.0]);
        let xml = format!(
            r#"<xisf xmlns="http://www.pixinsight.com/xisf"><Image>
<Property id="PCL:TotalExposureTime" type="F64Vector" length="3">{total}</Property>
</Image></xisf>"#
        );
        let m = extract_from_header_xml(&xml).expect("parses");
        assert_eq!(m.integration_s, Some(400.5));
    }

    #[test]
    fn header_xml_scalar_total_exposure_accepted() {
        // Non-PixInsight writers may emit a plain scalar.
        let xml = r#"<xisf><Image>
<Property id="PCL:TotalExposureTime" type="Float64" value="7200"/>
</Image></xisf>"#;
        let m = extract_from_header_xml(xml).expect("parses");
        assert_eq!(m.integration_s, Some(7200.0));
    }

    #[test]
    fn header_xml_without_metadata_is_empty() {
        let xml = r#"<xisf><Image geometry="10:10:1"/></xisf>"#;
        let m = extract_from_header_xml(xml).expect("parses");
        assert!(m.is_empty());
    }

    #[test]
    fn header_xml_garbage_is_none() {
        assert!(extract_from_header_xml("not xml at all <<<").is_none());
    }

    #[test]
    fn solver_echo_extracts_integration_when_present() {
        // If the solve service ever echoes the property back, the
        // solver-side extractor picks it up identically.
        let mut r = empty_result();
        r.pcl_properties = vec![pcl(
            "PCL:TotalExposureTime",
            "F64Vector",
            &b64_f64s(&[1800.0]),
        )];
        let m = extract(&r);
        assert_eq!(m.integration_s, Some(1800.0));
    }

    #[test]
    fn unquoted_string_values_pass_through() {
        // PCL string values aren't quoted; the strip must be a no-op.
        let mut r = empty_result();
        r.pcl_properties = vec![
            pcl("Instrument:Camera:Name", "String", "QHY 268M"),
            pcl("Observation:Object:Name", "String", "M31"),
        ];
        let m = extract(&r);
        assert_eq!(m.camera.as_deref(), Some("QHY 268M"));
        assert_eq!(m.target.as_deref(), Some("M31"));
    }
}
