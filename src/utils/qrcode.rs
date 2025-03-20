use anyhow::Result;
use image::Luma;
use log::{error, info};
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
    info!("Saving QR code to: {:?}", output_path);
    match image.save(output_path) {
        Ok(_) => {
            info!("QR code saved successfully");
            Ok(())
        }
        Err(e) => {
            error!("Failed to save QR code: {:?}", e);
            Err(anyhow::anyhow!("Failed to save QR code: {:?}", e))
        }
    }
}

pub fn generate_qr_code_for_url(url: &str) -> Result<PathBuf> {
    info!("Generating QR code for URL: {}", url);

    // Create assets directory if it doesn't exist
    let assets_dir = PathBuf::from("assets/qrcode");
    if let Err(e) = std::fs::create_dir_all(&assets_dir) {
        error!("Failed to create directory: {:?} - {:?}", assets_dir, e);
        return Err(anyhow::anyhow!("Failed to create directory: {:?}", e));
    }
    info!("Assets directory created/verified: {:?}", assets_dir);

    // Create output path - this must match the hardcoded path in the Slint UI
    let output_path = assets_dir.join("qrcode.png");
    info!("Output path for QR code: {:?}", output_path);

    // Generate QR code
    generate_qr_code(url, &output_path)?;

    // Return the full path for reference, though Slint UI uses a hardcoded path
    Ok(output_path)
}
