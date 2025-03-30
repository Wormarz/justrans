use anyhow::Result;
use image::Luma;
use log::{error, info};
use qrcode::QrCode;
use std::path::PathBuf;

pub fn generate_qr_code(data: &str, output_path: &PathBuf) -> Result<()> {
    // Create QR code with error correction level M (15%)
    let code = QrCode::with_error_correction_level(data, qrcode::EcLevel::M)?;

    // Render the QR code as an image with larger dimensions
    let image = code
        .render::<Luma<u8>>()
        .quiet_zone(true)
        .module_dimensions(10, 10) // Increased size
        .build();

    // Ensure the directory exists
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

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

    // Verify the file was created
    if !output_path.exists() {
        error!("QR code file was not created at: {:?}", output_path);
        return Err(anyhow::anyhow!("QR code file was not created"));
    }

    info!("QR code file verified at: {:?}", output_path);
    Ok(output_path)
}
