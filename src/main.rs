mod config;
mod models;
mod server;

use std::sync::{Arc, Mutex};

use anyhow::Result;
use log::{error, info};
use qrcode::generate_qr_code_for_url;
use settings::Settings;
use slint::{ComponentHandle, SharedString};
use tokio::runtime::Runtime;

use config::ConfigData;
use server::FileServer;

// Add this const to get version from Cargo.toml
const VERSION: &str = env!("CARGO_PKG_VERSION");

slint::include_modules!();

#[derive(Clone)]
struct AppData {
    file_server: Arc<Mutex<FileServer>>,
    runtime: Arc<Runtime>,
}

impl AppData {
    fn new() -> Result<Self> {
        let runtime = Arc::new(Runtime::new()?);

        // Create file server (will get config from singleton instance)
        let file_server = Arc::new(Mutex::new(FileServer::new()?));

        Ok(Self {
            file_server,
            runtime,
        })
    }
}

fn main() -> Result<()> {
    // Initialize logger with timestamped log file
    let log_path = logger::timestamped_log_path()?;
    logger::init(&log_path, log::Level::Info)?;

    info!(
        "Starting JusTrans v{} with log file at {:?}",
        VERSION, log_path
    );

    // Create app data (includes loading settings)
    let app_data = Arc::new(AppData::new()?);

    // Log some settings info
    info!(
        "Loaded settings - Server port: {}, Theme: {}",
        {
            let instance = ConfigData::instance().unwrap();
            let config = instance.lock().unwrap();
            config.server.port
        },
        {
            let instance = ConfigData::instance().unwrap();
            let config = instance.lock().unwrap();
            config.display.theme.clone()
        }
    );

    // Create UI
    let ui = AppWindow::new()?;

    // Set initial UI state
    {
        let server_info = app_data.file_server.lock().unwrap().get_server_info();
        ui.set_server_url(SharedString::from(server_info.url));
        ui.set_server_running(server_info.running);
        ui.set_status_message(SharedString::from("Server not running"));

        // Set config values from singleton instance
        let instance = ConfigData::instance()?;
        let config = instance.lock().unwrap();
        ui.set_config_server_port(config.server.port as i32);
        ui.set_config_upload_chunk_size_mb(config.server.upload_chunk_size_mb as i32);
        ui.set_config_theme(SharedString::from(config.display.theme.clone()));
        ui.set_config_storage_dir(SharedString::from(config.storage.storage_dir.clone()));
    }

    // Set up version information
    ui.set_version(SharedString::from(VERSION));

    // Handle start server
    ui.on_start_server({
        let ui_handle = ui.as_weak();
        let app_data = app_data.clone();
        move || {
            let ui = ui_handle.unwrap();
            let app_data_clone = app_data.clone();

            ui.set_is_loading(true);

            // Clone ui_handle for use in async block
            let ui_handle_clone = ui_handle.clone();

            // Start the server in a separate thread to avoid MutexGuard across await points
            std::thread::spawn(move || {
                // Start the server
                let server_result = {
                    let mut file_server = app_data_clone.file_server.lock().unwrap();
                    app_data_clone
                        .runtime
                        .block_on(async { file_server.start().await })
                };

                match server_result {
                    Ok(_) => {
                        // Get server info after starting
                        let server_info = {
                            let file_server = app_data_clone.file_server.lock().unwrap();
                            file_server.get_server_info()
                        };

                        // Update UI only after QR code is generated
                        slint::invoke_from_event_loop(move || {
                            let ui = ui_handle_clone.unwrap();
                            ui.set_server_url(SharedString::from(server_info.url.clone()));
                            ui.set_server_running(true);
                            ui.set_status_message(SharedString::from(
                                "Server running - QR code ready",
                            ));
                            ui.set_is_loading(false);
                            info!("UI updated with server_running=true and QR code ready");
                        })
                        .unwrap();
                    }
                    Err(err) => {
                        let error_msg = format!("Failed to start server: {}", err);
                        error!("{}", error_msg);

                        slint::invoke_from_event_loop(move || {
                            let ui = ui_handle_clone.unwrap();
                            ui.set_server_running(false);
                            ui.set_status_message(SharedString::from(error_msg));
                            ui.set_is_loading(false);
                        })
                        .unwrap();
                    }
                }
            });
        }
    });

    // Handle stop server
    ui.on_stop_server({
        let ui_handle = ui.as_weak();
        let app_data = app_data.clone();
        move || {
            let ui = ui_handle.unwrap();
            let app_data_clone = app_data.clone();

            ui.set_is_loading(true);

            // Clone ui_handle for use in async block
            let ui_handle_clone = ui_handle.clone();

            // Stop the server in a separate thread to avoid MutexGuard across await points
            std::thread::spawn(move || {
                // Stop the server
                let stop_result = {
                    let mut file_server = app_data_clone.file_server.lock().unwrap();
                    app_data_clone
                        .runtime
                        .block_on(async { file_server.stop().await })
                };

                match stop_result {
                    Ok(_) => {
                        slint::invoke_from_event_loop(move || {
                            let ui = ui_handle_clone.unwrap();
                            ui.set_server_running(false);
                            ui.set_status_message(SharedString::from("Server stopped"));
                            // No need to set QR code path
                            ui.set_is_loading(false);
                        })
                        .unwrap();
                    }
                    Err(err) => {
                        let error_msg = format!("Failed to stop server: {}", err);
                        error!("{}", error_msg);

                        slint::invoke_from_event_loop(move || {
                            let ui = ui_handle_clone.unwrap();
                            ui.set_status_message(SharedString::from(error_msg));
                            ui.set_is_loading(false);
                        })
                        .unwrap();
                    }
                }
            });
        }
    });

    ui.on_render_qr({
        let file_server = app_data.file_server.clone();
        move || match generate_qr_code_for_url(&file_server.lock().unwrap().get_server_info().url) {
            Ok(qr_image) => {
                info!("QR code generated successfully");
                let rgba = qr_image.to_rgba8();
                slint::Image::from_rgba8(slint::SharedPixelBuffer::clone_from_slice(
                    &rgba,
                    rgba.width(),
                    rgba.height(),
                ))
            }
            Err(_) => slint::Image::default(),
        }
    });

    // Handle URL click
    ui.on_open_url({
        let file_server = app_data.file_server.clone();
        move || {
            let server_url = &file_server.lock().unwrap().get_server_info().url;

            info!("Opening server URL in browser: {}", server_url);
            if let Err(e) = open::that(server_url) {
                error!("Failed to open URL: {:?}", e);
            }
        }
    });

    // Handle save config
    ui.on_save_config({
        let ui_handle = ui.as_weak();
        let app_data_clone = app_data.clone();
        move |port, chunk_size, theme, storage_dir| {
            let ui = ui_handle.unwrap();

            info!(
                "Saving config: port={}, chunk_size={}, theme={}, storage_dir={}",
                port, chunk_size, theme, storage_dir
            );

            match ConfigData::instance() {
                Ok(instance) => {
                    // Get current port for comparison
                    let current_port = {
                        let config = instance.lock().unwrap();
                        config.server.port
                    };

                    // Update config
                    {
                        let mut config = instance.lock().unwrap();
                        config.server.port = port as u16;
                        config.server.upload_chunk_size_mb = chunk_size as u64;
                        config.display.theme = theme.to_string();
                        config.storage.storage_dir = storage_dir.to_string();

                        // Save the updated config
                        let default_path = std::path::PathBuf::from("config/settings.yaml");
                        if let Err(e) = config.save(&default_path) {
                            error!("Failed to save config: {}", e);
                            ui.set_status_message(SharedString::from(format!(
                                "Failed to save config: {}",
                                e
                            )));
                            return;
                        }
                    }

                    info!("Config saved successfully");

                    // Check if server is running and port changed
                    let server_running = {
                        let file_server = app_data_clone.file_server.lock().unwrap();
                        file_server.get_server_info().running
                    };

                    if server_running && current_port != port as u16 {
                        ui.set_status_message(SharedString::from(
                            "Configuration saved - restart server to apply port changes",
                        ));
                    } else {
                        ui.set_status_message(SharedString::from(
                            "Configuration saved successfully",
                        ));
                    }
                }
                Err(e) => {
                    error!("Failed to access config instance: {}", e);
                    ui.set_status_message(SharedString::from(format!(
                        "Failed to access config: {}",
                        e
                    )));
                }
            }
        }
    });

    // Run the UI
    ui.run()?;

    Ok(())
}
