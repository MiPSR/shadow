use image::ImageFormat;
use std::io::Cursor;

pub fn image_to_webp(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, ImageFormat::WebP).map_err(|e| e.to_string())?;
    Ok(out.into_inner())
}

pub fn verify_webp(bytes: &[u8]) -> bool {
    image::load_from_memory_with_format(bytes, ImageFormat::WebP).is_ok()
}
