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

/// Decode an image, resize to `max_size` (longest side), encode JPEG.
/// Returns the encoded bytes plus actual width/height.
pub fn generate_blocking(input: &[u8], max_size: u32) -> Result<Thumb, AppError> {
    let img =
        image::load_from_memory(input).map_err(|e| AppError::Validation(format!("decode: {e}")))?;
    let resized = if img.width().max(img.height()) <= max_size {
        img
    } else {
        img.resize(max_size, max_size, FilterType::Lanczos3)
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
        let t = generate_blocking(&jpeg, 400).unwrap();
        assert_eq!(t.size, 400);
        assert!(t.width <= 400 && t.height <= 400);
        assert!(!t.bytes.is_empty());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn smaller_input_unchanged_dims() {
        let small_jpeg = {
            use image::{DynamicImage, RgbImage};
            let img =
                DynamicImage::ImageRgb8(RgbImage::from_fn(200, 150, |_, _| image::Rgb([0, 0, 0])));
            let mut buf = Cursor::new(Vec::new());
            img.write_to(&mut buf, ImageFormat::Jpeg).unwrap();
            buf.into_inner()
        };
        let t = generate_blocking(&small_jpeg, 400).unwrap();
        assert_eq!(t.width, 200);
        assert_eq!(t.height, 150);
    }
}
