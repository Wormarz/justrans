use anyhow::Result;
use image::Luma;
use qrcode::QrCode;
use std::path::PathBuf;

pub fn generate_qr_code(data: &str, output_path: &PathBuf) -> Result<()> {
    // Create QR code
    let code = QrCode::new(data)?;

    // Render the QR code as an image
    let image = code
        .render::<Luma<u8>>()
        .quiet_zone(true)
        .module_dimensions(6, 6)
        .build();

    // Save the image
    image.save(output_path)?;

    Ok(())
}

pub fn generate_qr_code_for_url(url: &str) -> Result<PathBuf> {
    // Create assets directory if it doesn't exist
    let assets_dir = PathBuf::from("assets/qrcode");
    std::fs::create_dir_all(&assets_dir)?;

    // Create output path
    let output_path = assets_dir.join("qrcode.png");

    // Generate QR code
    generate_qr_code(url, &output_path)?;

    Ok(output_path)
}
