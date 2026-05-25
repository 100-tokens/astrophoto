//! Local XISF → display-JPEG renderer.
//!
//! Why this exists: the plate-solve service's bundled render mishandles
//! planar multi-channel RGB (it stacks the channel planes as one tall
//! grayscale image instead of interleaving them into colour). XISF stores
//! pixel data **planar** (channel-major): all of channel 0, then all of
//! channel 1, etc. We decode the planes ourselves and combine them.
//!
//! "Stretched" masters carry display-ready data normalised to `[0, 1]`
//! (the `<Image bounds="0:1">`), so a straight linear map to 8-bit gives
//! the correct colour image — no screen-transfer maths required.
//!
//! Supported: uncompressed planar Float32/Float64/UInt8/UInt16, Gray or
//! RGB. Anything else (compressed, exotic formats) returns `Ok(None)` so
//! the caller can fall back to whatever render it already has.

use std::io::Cursor;

use image::{ImageFormat, RgbImage, imageops::FilterType};
use roxmltree::Document;

use crate::photos::xisf_processing::ProcessingParseError;

const SIGNATURE: &[u8] = b"XISF0100";

struct ImageMeta {
    width: u32,
    height: u32,
    channels: usize,
    sample_format: String,
    /// Absolute byte offset of the pixel data within the file.
    offset: usize,
    length: usize,
    compressed: bool,
}

/// Render the XISF's main image to a JPEG (long edge ≤ `max_edge`).
/// `Ok(None)` when the format isn't one we decode (caller keeps its
/// existing display image).
pub fn render_display_jpeg(
    bytes: &[u8],
    max_edge: u32,
) -> Result<Option<Vec<u8>>, ProcessingParseError> {
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

    let Some(meta) = parse_image_meta(xml)? else {
        return Ok(None);
    };
    if meta.compressed || !(meta.channels == 1 || meta.channels == 3) {
        return Ok(None);
    }
    let sample_bytes = match meta.sample_format.as_str() {
        "UInt8" => 1,
        "UInt16" => 2,
        "Float32" | "UInt32" => 4,
        "Float64" => 8,
        _ => return Ok(None),
    };
    let plane = meta.width as usize * meta.height as usize;
    let needed = plane * meta.channels * sample_bytes;
    let data = bytes
        .get(meta.offset..meta.offset + meta.length)
        .filter(|d| d.len() >= needed)
        .ok_or(ProcessingParseError::BadHeader)?;

    let read = |i: usize| -> f32 {
        let b = &data[i * sample_bytes..(i + 1) * sample_bytes];
        match meta.sample_format.as_str() {
            "UInt8" => b[0] as f32 / 255.0,
            "UInt16" => u16::from_le_bytes([b[0], b[1]]) as f32 / 65535.0,
            "UInt32" => u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f32 / 4294967295.0,
            "Float32" => f32::from_le_bytes([b[0], b[1], b[2], b[3]]),
            "Float64" => f64::from_le_bytes(b.try_into().unwrap_or([0; 8])) as f32,
            _ => 0.0,
        }
    };
    let to_u8 = |v: f32| -> u8 { (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8 };

    let mut img = RgbImage::new(meta.width, meta.height);
    for (i, px) in img.pixels_mut().enumerate() {
        if meta.channels == 3 {
            // planar: R plane, then G plane, then B plane
            px.0 = [
                to_u8(read(i)),
                to_u8(read(plane + i)),
                to_u8(read(2 * plane + i)),
            ];
        } else {
            let g = to_u8(read(i));
            px.0 = [g, g, g];
        }
    }

    let img = if meta.width.max(meta.height) > max_edge {
        let (nw, nh) = if meta.width >= meta.height {
            (max_edge, (meta.height * max_edge) / meta.width)
        } else {
            ((meta.width * max_edge) / meta.height, max_edge)
        };
        image::imageops::resize(&img, nw.max(1), nh.max(1), FilterType::Lanczos3)
    } else {
        img
    };

    let mut out = Vec::new();
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut Cursor::new(&mut out), ImageFormat::Jpeg)
        .map_err(|e| ProcessingParseError::Xml(format!("jpeg encode: {e}")))?;
    Ok(Some(out))
}

fn parse_image_meta(xml: &str) -> Result<Option<ImageMeta>, ProcessingParseError> {
    let doc = Document::parse(xml).map_err(|e| ProcessingParseError::Xml(e.to_string()))?;
    let Some(node) = doc
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "Image")
    else {
        return Ok(None);
    };
    // geometry = "W:H:Channels"
    let geom: Vec<u32> = node
        .attribute("geometry")
        .unwrap_or("")
        .split(':')
        .filter_map(|s| s.parse().ok())
        .collect();
    if geom.len() < 3 {
        return Ok(None);
    }
    // location = "attachment:offset:length"
    let loc = node.attribute("location").unwrap_or("");
    let parts: Vec<&str> = loc.split(':').collect();
    if parts.first() != Some(&"attachment") || parts.len() < 3 {
        return Ok(None);
    }
    let (Ok(offset), Ok(length)) = (parts[1].parse::<usize>(), parts[2].parse::<usize>()) else {
        return Ok(None);
    };
    Ok(Some(ImageMeta {
        width: geom[0],
        height: geom[1],
        channels: geom[2] as usize,
        sample_format: node.attribute("sampleFormat").unwrap_or("").to_string(),
        offset,
        length,
        compressed: node.attribute("compression").is_some(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Manual check against a real XISF (ignored; needs the sample file).
    /// `XISF_SAMPLE=/path.xisf XISF_RENDER_OUT=/tmp/out.jpg \
    ///   cargo test --lib render_real_xisf_sample -- --ignored --nocapture`
    #[test]
    #[ignore = "reads a local .xisf file named by the XISF_SAMPLE env var"]
    fn render_real_xisf_sample() {
        let path = std::env::var("XISF_SAMPLE").expect("set XISF_SAMPLE");
        let bytes = std::fs::read(&path).expect("read sample");
        let jpeg = render_display_jpeg(&bytes, 1200)
            .expect("render ok")
            .expect("rendered something");
        assert!(jpeg.len() > 1000, "non-trivial jpeg");
        // decode it back to confirm it's a valid, roughly-square colour image
        let decoded = image::load_from_memory(&jpeg).expect("valid jpeg");
        eprintln!("rendered {}x{}", decoded.width(), decoded.height());
        assert!(decoded.width() <= 1200 && decoded.height() <= 1200);
        // M20 is ~square; the BUG produced a ~1:3 tall image. Guard the ratio.
        let ratio = decoded.width() as f32 / decoded.height() as f32;
        assert!(ratio > 0.6 && ratio < 1.6, "aspect {ratio} — not stacked");
        if let Ok(out) = std::env::var("XISF_RENDER_OUT") {
            std::fs::write(out, &jpeg).unwrap();
        }
    }
}
