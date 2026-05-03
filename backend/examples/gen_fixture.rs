/// One-shot scaffold: generates `tests/fixtures/sample.jpg` (800x600 gradient).
/// Run once with: cargo run --example gen_fixture
/// May be deleted after the fixture is committed.
use image::{DynamicImage, ImageFormat, RgbImage};
use std::io::Cursor;

#[allow(clippy::unwrap_used)]
fn main() {
    let img = DynamicImage::ImageRgb8(RgbImage::from_fn(800, 600, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, 64])
    }));
    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, ImageFormat::Jpeg).unwrap();
    let bytes = buf.into_inner();
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/sample.jpg");
    std::fs::write(path, &bytes).unwrap();
    println!("wrote {} bytes to {}", bytes.len(), path);
}
