//! EXIF parsing. Runs in spawn_blocking; `kamadak-exif` is sync.

use std::io::Cursor;

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use serde::Serialize;
use serde_json::Value as Json;

use crate::AppError;

#[derive(Default, Debug, Serialize)]
pub struct ExifData {
    pub camera: Option<String>,
    pub lens: Option<String>,
    pub iso: Option<i32>,
    pub exposure_s: Option<f64>,
    pub focal_mm: Option<f64>,
    pub taken_at: Option<DateTime<Utc>>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    /// Raw payload as JSON for full preservation
    pub raw: Json,
}

/// Parse EXIF synchronously. Caller wraps in `spawn_blocking`.
pub fn parse_blocking(bytes: &[u8]) -> Result<ExifData, AppError> {
    let exif_reader = exif::Reader::new()
        .read_from_container(&mut Cursor::new(bytes))
        .ok();
    let mut data = ExifData::default();
    let mut raw = serde_json::Map::new();

    if let Some(ref reader) = exif_reader {
        for f in reader.fields() {
            let key = format!("{}", f.tag);
            let val = f.display_value().with_unit(reader).to_string();
            raw.insert(key, Json::String(val));
        }
        data.camera = read_string(reader, exif::Tag::Model);
        data.lens = read_string(reader, exif::Tag::LensModel);
        data.iso = read_int(reader, exif::Tag::PhotographicSensitivity);
        data.exposure_s = read_rational(reader, exif::Tag::ExposureTime);
        data.focal_mm = read_rational(reader, exif::Tag::FocalLength);
        data.taken_at = read_datetime(reader, exif::Tag::DateTimeOriginal);
        data.width = read_int(reader, exif::Tag::PixelXDimension);
        data.height = read_int(reader, exif::Tag::PixelYDimension);
    }
    data.raw = Json::Object(raw);
    Ok(data)
}

fn read_string(r: &exif::Exif, tag: exif::Tag) -> Option<String> {
    let f = r.get_field(tag, exif::In::PRIMARY)?;
    Some(f.display_value().to_string().trim_matches('"').to_string())
}

fn read_int(r: &exif::Exif, tag: exif::Tag) -> Option<i32> {
    let f = r.get_field(tag, exif::In::PRIMARY)?;
    f.value.get_uint(0).map(|n| n as i32)
}

fn read_rational(r: &exif::Exif, tag: exif::Tag) -> Option<f64> {
    let f = r.get_field(tag, exif::In::PRIMARY)?;
    if let exif::Value::Rational(ref v) = f.value {
        v.first().map(|r| r.to_f64())
    } else {
        None
    }
}

fn read_datetime(r: &exif::Exif, tag: exif::Tag) -> Option<DateTime<Utc>> {
    let f = r.get_field(tag, exif::In::PRIMARY)?;
    if let exif::Value::Ascii(ref bytes) = f.value {
        let s = bytes.first().and_then(|v| std::str::from_utf8(v).ok())?;
        let dt = NaiveDateTime::parse_from_str(s, "%Y:%m:%d %H:%M:%S").ok()?;
        Some(Utc.from_utc_datetime(&dt))
    } else {
        None
    }
}
