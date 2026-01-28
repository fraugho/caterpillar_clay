use image::{DynamicImage, ImageFormat, ImageReader};
use std::io::Cursor;

// Target 800x800 (2x retina for ~400px grid display)
const MAX_WIDTH: u32 = 800;
const MAX_HEIGHT: u32 = 800;
const JPEG_QUALITY: u8 = 80;

pub struct ProcessedImage {
    pub data: Vec<u8>,
    pub content_type: String,
    pub extension: String,
}

/// Process and resize an image if it exceeds max dimensions
pub fn process_image(data: &[u8], original_extension: &str) -> Result<ProcessedImage, String> {
    // Try to decode the image
    let img = ImageReader::new(Cursor::new(data))
        .with_guessed_format()
        .map_err(|e| format!("Failed to read image format: {}", e))?
        .decode()
        .map_err(|e| format!("Failed to decode image: {}", e))?;

    let (width, height) = (img.width(), img.height());

    // Check if resizing is needed
    let needs_resize = width > MAX_WIDTH || height > MAX_HEIGHT;

    let processed = if needs_resize {
        // Calculate new dimensions maintaining aspect ratio
        let ratio = (MAX_WIDTH as f64 / width as f64).min(MAX_HEIGHT as f64 / height as f64);
        let new_width = (width as f64 * ratio) as u32;
        let new_height = (height as f64 * ratio) as u32;

        tracing::info!(
            "Resizing image from {}x{} to {}x{}",
            width,
            height,
            new_width,
            new_height
        );

        img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3)
    } else {
        img
    };

    // Encode the image - convert to JPEG for smaller file sizes (except PNG with transparency)
    let (output_data, content_type, extension) = encode_image(&processed, original_extension)?;

    Ok(ProcessedImage {
        data: output_data,
        content_type,
        extension,
    })
}

fn encode_image(
    img: &DynamicImage,
    original_extension: &str,
) -> Result<(Vec<u8>, String, String), String> {
    let mut buffer = Vec::new();
    let ext_lower = original_extension.to_lowercase();

    match ext_lower.as_str() {
        "webp" | "png" => {
            // Convert to WebP (lossless encoder - image crate doesn't support lossy quality)
            // Still better compression than PNG
            img.write_to(&mut Cursor::new(&mut buffer), ImageFormat::WebP)
                .map_err(|e| format!("Failed to encode WebP: {}", e))?;
            Ok((buffer, "image/webp".to_string(), "webp".to_string()))
        }
        "gif" => {
            // Keep GIF as-is (may be animated)
            img.write_to(&mut Cursor::new(&mut buffer), ImageFormat::Gif)
                .map_err(|e| format!("Failed to encode GIF: {}", e))?;
            Ok((buffer, "image/gif".to_string(), "gif".to_string()))
        }
        _ => {
            // JPEG for jpg/jpeg/unknown - with quality setting
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buffer, JPEG_QUALITY);
            encoder
                .encode_image(img)
                .map_err(|e| format!("Failed to encode JPEG: {}", e))?;
            Ok((buffer, "image/jpeg".to_string(), "jpg".to_string()))
        }
    }
}
