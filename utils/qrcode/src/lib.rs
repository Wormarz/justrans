use anyhow::Result;
use image::{DynamicImage, ImageBuffer, Luma};
use qrcode::QrCode;

pub fn generate_qr_code_for_url(data: &str) -> Result<DynamicImage> {
    // Create QR code with error correction level M (15%)
    let code = QrCode::with_error_correction_level(data, qrcode::EcLevel::M)?;

    // Render the QR code as an image with larger dimensions
    let image = code
        .render::<Luma<u8>>()
        .quiet_zone(true)
        .module_dimensions(10, 10) // Increased size
        .build();

    // Convert to DynamicImage
    let image_buffer = ImageBuffer::from_raw(
        image.width() as u32,
        image.height() as u32,
        image.into_raw(),
    )
    .ok_or_else(|| anyhow::anyhow!("Failed to create image buffer"))?;

    Ok(DynamicImage::ImageLuma8(image_buffer))
}
