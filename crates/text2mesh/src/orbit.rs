//! Mock T2I: write gray-studio PNGs that pass G3/G4 (S6). Not a real generator.

use image::{ImageEncoder, Rgba, RgbaImage};

use crate::contract::ViewContract;
use crate::error::{error_type, Error};

pub fn mock_view_png(contract: &ViewContract, cam_id: &str) -> Result<Vec<u8>, Error> {
    let mut img = RgbaImage::from_pixel(64, 64, Rgba([0xB4, 0xB4, 0xB4, 255]));
    // Stable subject block; one corner pixel encodes camera so files differ.
    for y in 14..50 {
        for x in 14..50 {
            img.put_pixel(x, y, Rgba([180, 80, 40, 255]));
        }
    }
    let tag = cam_id.bytes().fold(0u8, |a, b| a.wrapping_add(b));
    img.put_pixel(14, 14, Rgba([180, 80, 40u8.wrapping_add(tag % 8), 255]));
    let _ = contract;
    encode_png(&img)
}

pub fn encode_png(img: &RgbaImage) -> Result<Vec<u8>, Error> {
    let mut buf = Vec::new();
    image::codecs::png::PngEncoder::new(&mut buf)
        .write_image(
            img.as_raw(),
            img.width(),
            img.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| Error::new(error_type::INTERNAL, e.to_string()))?;
    Ok(buf)
}

pub fn decode_png(bytes: &[u8]) -> Result<RgbaImage, Error> {
    let dynimg = image::load_from_memory(bytes)
        .map_err(|e| Error::new(error_type::SPEC_REJECTED, e.to_string()))?;
    Ok(dynimg.to_rgba8())
}
