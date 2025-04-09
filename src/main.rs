mod models;
mod server;

use std::sync::{Arc, Mutex};

use anyhow::Result;
use log::{error, info};
use models::FileList;
use qrcode::generate_qr_code_for_url;
use slint::{ComponentHandle, SharedString};
use tokio::runtime::Runtime;

use server::FileServer;

// Add this const to get version from Cargo.toml
const VERSION: &str = env!("CARGO_PKG_VERSION");

slint::include_modules!();

#[derive(Clone)]
struct AppData {
    file_server: Arc<Mutex<FileServer>>,
    file_list: Arc<Mutex<FileList>>,
    runtime: Arc<Runtime>,
}

impl AppData {
    fn new() -> Result<Self> {
        let runtime = Arc::new(Runtime::new()?);
        let file_server = Arc::new(Mutex::new(FileServer::new()?));
        let file_list = Arc::new(Mutex::new(FileList::new()));

        Ok(Self {
            file_server,
            file_list,
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

    // Create app data
    let app_data = AppData::new()?;

    // Create UI
    let ui = AppWindow::new()?;

    // Set initial UI state
    {
        let server_info = app_data.file_server.lock().unwrap().get_server_info();
        ui.set_server_url(SharedString::from(server_info.url));
        ui.set_server_running(server_info.running);
        ui.set_status_message(SharedString::from("Server not running"));
    }

    // Set up version information
    ui.set_version(SharedString::from(VERSION));

    // Set up periodic refresh timer (every 5 seconds)
    let ui_handle_for_timer = ui.as_weak();
    let app_data_for_timer = app_data.clone();
    std::thread::spawn(move || {
        while let Some(ui) = ui_handle_for_timer.upgrade() {
            // Sleep for 5 seconds
            std::thread::sleep(std::time::Duration::from_secs(5));

            // Check if UI still exists and server is running
            if !ui.get_server_running() {
                continue;
            }

            // Do the refresh on the main thread
            if let Err(_) = slint::invoke_from_event_loop({
                let ui_handle = ui_handle_for_timer.clone();
                let app_data = app_data_for_timer.clone();
                move || {
                    if let Some(ui) = ui_handle.upgrade() {
                        // Refresh files if server is running
                        info!("Auto-refresh: Checking for new files...");

                        // Get latest files from server to ensure UI is in sync with web uploads
                        let server_files = {
                            let file_server = app_data.file_server.lock().unwrap();
                            file_server.get_file_list()
                        };

                        // Update local file list with server's list
                        let update_needed = {
                            let mut local_file_list = app_data.file_list.lock().unwrap();

                            // Check for changes by comparing number of files
                            // and also check for differences in file IDs
                            let local_count = local_file_list.files.len();
                            let server_count = server_files.files.len();

                            // First, a simple count check
                            let count_changed = local_count != server_count;

                            // Then a more detailed check comparing file IDs
                            // This helps when files are removed from one side but total count remains the same
                            let ids_differ = if !count_changed && local_count > 0 {
                                // Create sets of file IDs for easy comparison
                                let local_ids: std::collections::HashSet<&String> =
                                    local_file_list.files.iter().map(|f| &f.id).collect();
                                let server_ids: std::collections::HashSet<&String> =
                                    server_files.files.iter().map(|f| &f.id).collect();

                                // Check if the sets are different
                                local_ids != server_ids
                            } else {
                                false
                            };

                            if count_changed || ids_differ {
                                info!(
                                    "Auto-refresh: Found file changes: local={}, server={}",
                                    local_count, server_count
                                );
                                *local_file_list = server_files;
                                true
                            } else {
                                false
                            }
                        };

                        // Update UI if needed
                        if update_needed {
                            update_ui_file_list(&ui, &app_data);
                        }
                    }
                }
            }) {
                // If we can't invoke on the UI thread, the app is probably shutting down
                break;
            }
        }
    });

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

                        // Generate QR code first
                        let qr_result = generate_qr_code_for_url(&server_info.url);
                        match qr_result {
                            Ok(_) => {
                                info!("QR code generated successfully");
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
                            Err(e) => {
                                error!("Failed to generate QR code: {:?}", e);
                                // Still start the server but show error in UI
                                slint::invoke_from_event_loop(move || {
                                    let ui = ui_handle_clone.unwrap();
                                    ui.set_server_url(SharedString::from(server_info.url.clone()));
                                    ui.set_server_running(true);
                                    ui.set_status_message(SharedString::from(
                                        "Server running - QR code generation failed",
                                    ));
                                    ui.set_is_loading(false);
                                    info!("UI updated with server_running=true but QR code failed");
                                })
                                .unwrap();
                            }
                        };
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

    // Handle URL click
    ui.on_open_url({
        let app_data = app_data.clone();
        move || {
            let server_url = {
                let file_server = app_data.file_server.lock().unwrap();
                file_server.get_server_info().url
            };

            info!("Opening server URL in browser: {}", server_url);
            if let Err(e) = open::that(&server_url) {
                error!("Failed to open URL: {:?}", e);
            }
        }
    });

    // Run the UI
    ui.run()?;

    Ok(())
}

fn update_ui_file_list(ui: &AppWindow, app_data: &AppData) {
    let file_list = app_data.file_list.lock().unwrap();
    let mut slint_files = Vec::new();

    for file in &file_list.files {
        // Create a Slint FileInfo struct - the `id` field is required in the Slint struct
        let slint_file_info = FileInfo {
            name: SharedString::from(&file.name),
            size: SharedString::from(file.formatted_size()),
            path: SharedString::from(file.path.to_string_lossy().to_string()),
            id: SharedString::from(&file.id),
        };

        slint_files.push(slint_file_info);
    }

    ui.set_files(slint::ModelRc::new(slint::VecModel::from(slint_files)));
}
