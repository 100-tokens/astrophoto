//! JPEG thumbnail generation. Runs in spawn_blocking; `image` is sync.

use std::io::Cursor;

use bytes::Bytes;
use image::{ImageFormat, imageops::FilterType};

use crate::AppError;

#[derive(Debug, Clone)]
pub struct Thumb {
    pub size: u32,
    pub bytes: Bytes,
    pub width: u32,
    pub height: u32,
}

/// Resize an already-decoded image to `max_size` (longest side), encode
/// JPEG. Returns the encoded bytes plus actual width/height. Takes a
/// `DynamicImage` (not raw bytes) so the pipeline decodes the original
/// exactly once and derives every artifact from it.
pub fn generate_blocking(img: &image::DynamicImage, max_size: u32) -> Result<Thumb, AppError> {
    let resized_owned;
    let resized: &image::DynamicImage = if img.width().max(img.height()) <= max_size {
        img
    } else {
        resized_owned = img.resize(max_size, max_size, FilterType::Lanczos3);
        &resized_owned
    };
    let (w, h) = (resized.width(), resized.height());
    let mut out = Cursor::new(Vec::with_capacity(64 * 1024));
    resized
        .to_rgb8()
        .write_to(&mut out, ImageFormat::Jpeg)
        .map_err(|e| AppError::Internal(format!("encode: {e}")))?;
    Ok(Thumb {
        size: max_size,
        bytes: Bytes::from(out.into_inner()),
        width: w,
        height: h,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generate a synthetic 800×600 RGB image, encode as JPEG, resize.
    #[allow(clippy::unwrap_used)]
    fn make_test_jpeg() -> Vec<u8> {
        use image::{DynamicImage, RgbImage};
        let img = DynamicImage::ImageRgb8(RgbImage::from_fn(800, 600, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
        }));
        let mut buf = Cursor::new(Vec::new());
        img.write_to(&mut buf, ImageFormat::Jpeg).unwrap();
        buf.into_inner()
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn resize_to_400() {
        let jpeg = make_test_jpeg();
        let img = image::load_from_memory(&jpeg).unwrap();
        let t = generate_blocking(&img, 400).unwrap();
        assert_eq!(t.size, 400);
        assert!(t.width <= 400 && t.height <= 400);
        assert!(!t.bytes.is_empty());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn smaller_input_unchanged_dims() {
        let small = {
            use image::{DynamicImage, RgbImage};
            DynamicImage::ImageRgb8(RgbImage::from_fn(200, 150, |_, _| image::Rgb([0, 0, 0])))
        };
        let t = generate_blocking(&small, 400).unwrap();
        assert_eq!(t.width, 200);
        assert_eq!(t.height, 150);
    }
}
