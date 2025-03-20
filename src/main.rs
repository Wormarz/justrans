mod models;
mod server;
mod utils;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use log::{error, info};
use rfd::FileDialog;
use slint::SharedString;
use tokio::runtime::Runtime;

use models::{FileInfo as ModelFileInfo, FileList};
use server::FileServer;
use utils::generate_qr_code_for_url;

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
    // Initialize logger
    env_logger::init();
    info!("Starting JusTrans...");

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

                        // Generate QR code
                        let qr_result = generate_qr_code_for_url(&server_info.url);

                        if let Ok(qr_path) = qr_result {
                            // Convert to relative path for Slint
                            let relative_path = qr_path.to_string_lossy().to_string();
                            info!("QR code generated successfully at path: {:?}", qr_path);
                            info!("Setting QR code path in UI: {}", &relative_path);

                            // Check if file exists
                            if std::path::Path::new(&qr_path).exists() {
                                info!("QR code file exists at path: {:?}", qr_path);
                            } else {
                                error!("QR code file DOES NOT EXIST at path: {:?}", qr_path);
                            }

                            slint::invoke_from_event_loop(move || {
                                let ui = ui_handle_clone.unwrap();
                                ui.set_server_url(SharedString::from(server_info.url.clone()));
                                ui.set_server_running(true);
                                ui.set_status_message(SharedString::from("Server running"));
                                ui.set_qr_code_path(SharedString::from(relative_path.clone()));
                                ui.set_is_loading(false);
                                info!(
                                    "UI updated with server_running=true and QR code path: {}",
                                    relative_path
                                );
                            })
                            .unwrap();
                        } else {
                            error!("Failed to generate QR code: {:?}", qr_result.err());
                            slint::invoke_from_event_loop(move || {
                                let ui = ui_handle_clone.unwrap();
                                ui.set_server_url(SharedString::from(server_info.url.clone()));
                                ui.set_server_running(true);
                                ui.set_status_message(SharedString::from(
                                    "Server running (QR code generation failed)",
                                ));
                                ui.set_is_loading(false);
                            })
                            .unwrap();
                        }
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
                            ui.set_qr_code_path(SharedString::from(""));
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

    // Handle add files
    ui.on_add_files({
        let ui_handle = ui.as_weak();
        let app_data = app_data.clone();
        move || {
            let ui = ui_handle.unwrap();

            // Open a file dialog to select files
            if let Some(files) = FileDialog::new()
                .set_title("Select files to share")
                .set_directory(std::env::current_dir().unwrap_or_default())
                .pick_files()
            {
                // First collect all new files
                let mut new_file_infos = Vec::new();
                for file_path in files {
                    if let Ok(file_info) = ModelFileInfo::new(file_path) {
                        new_file_infos.push(file_info);
                    }
                }

                // Update both the file list and server in one transaction
                {
                    // Update local file list
                    let mut file_list = app_data.file_list.lock().unwrap();
                    for file_info in new_file_infos {
                        file_list.add_file(file_info);
                    }

                    // Update server file list immediately to ensure web clients get the update
                    let file_server = app_data.file_server.lock().unwrap();
                    file_server.set_file_list(file_list.clone());

                    // Log the update
                    info!(
                        "File list updated: {} files available",
                        file_list.files.len()
                    );
                }

                // Update UI
                update_ui_file_list(&ui, &app_data);
            }
        }
    });

    // Handle remove file
    ui.on_remove_file({
        let ui_handle = ui.as_weak();
        let app_data = app_data.clone();
        move |index| {
            let ui = ui_handle.unwrap();

            // Remove file from list and update server in one transaction
            {
                let mut file_list = app_data.file_list.lock().unwrap();
                if let Some(removed_file) = file_list.remove_file(index as usize) {
                    // Update server file list immediately to ensure web clients get the update
                    let file_server = app_data.file_server.lock().unwrap();
                    file_server.set_file_list(file_list.clone());

                    // Log the update
                    info!(
                        "File removed: {} ('{}') - {} files remaining",
                        removed_file.id,
                        removed_file.name,
                        file_list.files.len()
                    );
                }
            }

            // Update UI
            update_ui_file_list(&ui, &app_data);
            ui.set_selected_file(-1);
        }
    });

    // Handle open file
    ui.on_open_file({
        let app_data = app_data.clone();
        move |index| {
            let file_list = app_data.file_list.lock().unwrap();
            if let Some(file_info) = file_list.get_file(index as usize) {
                if let Err(err) = open_file(&file_info.path) {
                    error!("Failed to open file: {}", err);
                }
            }
        }
    });

    // Handle copy URL
    ui.on_copy_url({
        let ui_handle = ui.as_weak();
        move || {
            let ui = ui_handle.unwrap();
            let url = ui.get_server_url().to_string();

            // Since we don't have clipboard support, just log the URL
            info!("Server URL: {}", url);
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
        // Create a Slint FileInfo struct
        let slint_file_info = FileInfo {
            name: SharedString::from(&file.name),
            size: SharedString::from(file.formatted_size()),
            path: SharedString::from(file.path.to_string_lossy().to_string()),
        };

        slint_files.push(slint_file_info);
    }

    ui.set_files(slint::ModelRc::new(slint::VecModel::from(slint_files)));
}

fn open_file(path: &PathBuf) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        use std::process::Command;
        Command::new("cmd")
            .args(["/C", "start", "", path.to_string_lossy().as_ref()])
            .spawn()?;
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        Command::new("open").arg(path).spawn()?;
    }

    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        Command::new("xdg-open").arg(path).spawn()?;
    }

    Ok(())
}
