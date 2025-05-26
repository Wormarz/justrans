use std::io::Write;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

use axum::extract::Multipart;
use axum::response::AppendHeaders;
use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use local_ip_address::local_ip;
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use crate::config::ConfigData;
use crate::models::{FileInfo, FileList};

#[derive(Clone)]
pub struct AppState {
    pub file_list: Arc<Mutex<FileList>>,
    pub temp_dir: PathBuf,
    pub config: ConfigData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub url: String,
    pub ip: String,
    pub port: u16,
    pub running: bool,
}

pub struct FileServer {
    state: AppState,
    server_info: Arc<Mutex<ServerInfo>>,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl FileServer {
    pub fn new(config: &crate::config::ConfigData) -> anyhow::Result<Self> {
        // Create temp directory for uploaded files
        let storage_dir = PathBuf::from(&config.storage.storage_dir);
        std::fs::create_dir_all(&storage_dir)?;

        // Get local IP address
        let ip = match local_ip() {
            Ok(ip) => ip.to_string(),
            Err(_) => "127.0.0.1".to_string(),
        };

        // Get port from settings
        let port = config.server.port;

        let server_info = ServerInfo {
            url: format!("http://{}:{}", ip, port),
            ip,
            port,
            running: false,
        };

        Ok(Self {
            state: AppState {
                file_list: Arc::new(Mutex::new(FileList::new())),
                temp_dir: storage_dir,
                config: config.clone(),
            },
            server_info: Arc::new(Mutex::new(server_info)),
            shutdown_tx: None,
        })
    }

    pub fn get_server_info(&self) -> ServerInfo {
        let info = self.server_info.lock().unwrap();
        ServerInfo {
            url: info.url.clone(),
            ip: info.ip.clone(),
            port: info.port,
            running: info.running,
        }
    }

    pub async fn start(&mut self) -> anyhow::Result<()> {
        if self.shutdown_tx.is_some() {
            return Ok(());
        }

        let app_state = self.state.clone();
        let server_info = self.server_info.clone();

        // Update server info
        {
            let mut info = server_info.lock().unwrap();
            info.running = true;
        }

        // Create static file service
        let static_files_service = ServeDir::new("assets/web");

        // Create CORS layer
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        // Build router
        let app = Router::new()
            .route("/", get(serve_index))
            .route("/api/files", get(get_files))
            .route("/api/files/:id", get(download_file))
            .route("/api/config", get(get_config))
            .route(
                "/api/upload",
                post(upload_file).layer(axum::extract::DefaultBodyLimit::max(
                    (self.state.config.server.upload_chunk_size_mb + 1) as usize * 1024 * 1024,
                )),
            )
            .nest_service("/static", static_files_service)
            .layer(TraceLayer::new_for_http())
            .layer(cors)
            .with_state(app_state);

        // Get server address with binding based on settings
        let addr = {
            let info = server_info.lock().unwrap();
            let bind_address = "0.0.0.0";
            SocketAddr::new(bind_address.parse()?, info.port)
        };

        log::info!("Starting server on {}", addr);

        // Create shutdown channel
        let (tx, rx) = oneshot::channel::<()>();
        self.shutdown_tx = Some(tx);

        // Start server
        tokio::spawn(async move {
            let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
            let server = axum::serve(listener, app);

            let server = server.with_graceful_shutdown(async {
                rx.await.ok();
            });

            if let Err(err) = server.await {
                log::error!("Server error: {}", err);
                let mut info = server_info.lock().unwrap();
                info.running = false;
            }
        });

        Ok(())
    }

    pub async fn stop(&mut self) -> anyhow::Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());

            // Update server info
            let mut info = self.server_info.lock().unwrap();
            info.running = false;
        }

        Ok(())
    }
}

#[axum::debug_handler]
async fn serve_index() -> Html<&'static str> {
    Html(include_str!("../../assets/web/index.html"))
}

#[axum::debug_handler]
async fn get_files(State(state): State<AppState>) -> Json<FileList> {
    let file_list = state.file_list.lock().unwrap().clone();
    Json(file_list)
}

#[derive(Serialize)]
struct ConfigResponse {
    upload_chunk_size_mb: u64,
}

#[axum::debug_handler]
async fn get_config(State(state): State<AppState>) -> Json<ConfigResponse> {
    Json(ConfigResponse {
        upload_chunk_size_mb: state.config.server.upload_chunk_size_mb,
    })
}

#[axum::debug_handler]
async fn download_file(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Response, StatusCode> {
    // Get file info from the list
    let file_info = {
        let file_list = state.file_list.lock().unwrap();
        match file_list.get_file_by_id(&id) {
            Some(info) => info.clone(),
            None => return Err(StatusCode::NOT_FOUND),
        }
    };

    let path = file_info.path.clone();

    // Open the file
    let mut file = match File::open(&path).await {
        Ok(file) => file,
        Err(_) => return Err(StatusCode::NOT_FOUND),
    };

    // Read the file content
    let mut contents = Vec::new();
    if file.read_to_end(&mut contents).await.is_err() {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Create response with appropriate headers
    let headers = AppendHeaders([
        (header::CONTENT_TYPE, file_info.mime_type),
        (
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", file_info.name),
        ),
    ]);

    Ok((headers, contents).into_response())
}

#[axum::debug_handler]
async fn upload_file(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Json<FileInfo>, StatusCode> {
    log::info!("Starting file upload processing");

    // First collect metadata from the multipart form
    let mut file_name = None;
    let mut segment_index = None;
    let mut total_segments = None;
    let mut file_id = None;
    let mut file_data: Option<Vec<u8>> = None;

    // Log all received form fields for debugging
    log::info!("Processing multipart form data");

    // Process each field in the multipart form
    let mut field_count = 0;
    while let Ok(Some(mut field)) = multipart.next_field().await {
        field_count += 1;
        let field_name = field.name().unwrap_or("unnamed").to_string();
        log::info!("Processing field #{}: name='{}'", field_count, field_name);

        match field_name.as_str() {
            "file" => {
                let original_filename = field.file_name().unwrap_or("unknown").to_string();
                log::info!("Found file field with filename: {}", original_filename);
                file_name = Some(original_filename);

                // Read data in smaller chunks for better memory management
                let mut buffer = Vec::new();
                let mut bytes_read = 0;

                // Process chunks of the file
                log::info!("Reading file data chunks");
                while let Ok(Some(chunk)) = field.chunk().await {
                    bytes_read += chunk.len();
                    log::info!(
                        "Read chunk: {} bytes (total: {} bytes)",
                        chunk.len(),
                        bytes_read
                    );
                    buffer.extend_from_slice(&chunk);
                }

                if bytes_read > 0 {
                    log::info!("Successfully read file data: {} bytes", bytes_read);
                    file_data = Some(buffer);
                } else {
                    log::error!("No data read from file field");
                }
            }
            "segment_index" => {
                if let Ok(data) = field.text().await {
                    log::info!("Found segment_index: {}", data);
                    match data.parse::<usize>() {
                        Ok(idx) => segment_index = Some(idx),
                        Err(e) => log::error!("Failed to parse segment_index '{}': {}", data, e),
                    }
                } else {
                    log::error!("Could not read segment_index field as text");
                }
            }
            "total_segments" => {
                if let Ok(data) = field.text().await {
                    log::info!("Found total_segments: {}", data);
                    match data.parse::<usize>() {
                        Ok(total) => total_segments = Some(total),
                        Err(e) => log::error!("Failed to parse total_segments '{}': {}", data, e),
                    }
                } else {
                    log::error!("Could not read total_segments field as text");
                }
            }
            "file_id" => {
                if let Ok(data) = field.text().await {
                    log::info!("Found file_id: {}", data);
                    file_id = Some(data);
                } else {
                    log::error!("Could not read file_id field as text");
                }
            }
            _ => log::warn!("Unexpected field name: {}", field_name),
        }
    }

    // Log results of field processing
    log::info!("Processed {} fields in multipart form", field_count);
    log::info!("file_name: {:?}", file_name);
    log::info!("segment_index: {:?}", segment_index);
    log::info!("total_segments: {:?}", total_segments);
    log::info!("file_id: {:?}", file_id);
    log::info!(
        "file_data: {} bytes",
        file_data.as_ref().map_or(0, |d| d.len())
    );

    // Validate required fields
    let (file_name, segment_index, total_segments, file_id, file_data) =
        match (file_name, segment_index, total_segments, file_id, file_data) {
            (Some(name), Some(idx), Some(total), Some(id), Some(data)) => {
                (name, idx, total, id, data)
            }
            _ => {
                log::error!("Missing required fields in multipart upload");
                return Err(StatusCode::BAD_REQUEST);
            }
        };

    // Create the temporary directory for segments
    log::info!(
        "Creating temp directory for file segments: {:?}",
        state.temp_dir.join(&file_id)
    );
    let temp_dir = state.temp_dir.join(&file_id);
    std::fs::create_dir_all(&temp_dir).map_err(|e| {
        log::error!(
            "Failed to create temp directory: {:?}, error: {}",
            temp_dir,
            e
        );
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Save the segment
    let segment_path = temp_dir.join(format!("segment_{}", segment_index));
    log::info!("Saving segment to: {:?}", segment_path);
    std::fs::write(&segment_path, &file_data).map_err(|e| {
        log::error!(
            "Failed to write segment file: {:?}, error: {}",
            segment_path,
            e
        );
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    log::info!(
        "Received segment {} of {} for file '{}' (ID: {}), size: {} bytes",
        segment_index + 1,
        total_segments,
        file_name,
        file_id,
        file_data.len()
    );

    // If this is the last segment, combine all segments
    if segment_index == total_segments - 1 {
        log::info!(
            "Processing final segment for file '{}', combining chunks",
            file_name
        );

        // Check if all previous segments exist
        let mut missing_segments = Vec::new();
        for i in 0..total_segments {
            let segment_path = temp_dir.join(format!("segment_{}", i));
            if !segment_path.exists() {
                missing_segments.push(i);
            }
        }

        if !missing_segments.is_empty() {
            log::error!("Missing segments: {:?}", missing_segments);
            return Err(StatusCode::BAD_REQUEST);
        }

        // Combine all segments into the final file
        let final_path = state.temp_dir.join(format!("{}_file", file_id));
        log::info!("Creating final file: {:?}", final_path);
        let mut final_file = std::fs::File::create(&final_path).map_err(|e| {
            log::error!(
                "Failed to create final file: {:?}, error: {}",
                final_path,
                e
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        let mut total_size: u64 = 0;

        // Combine all segments
        for i in 0..total_segments {
            let segment_path = temp_dir.join(format!("segment_{}", i));
            log::info!("Reading segment {}: {:?}", i, segment_path);

            let segment_data = std::fs::read(&segment_path).map_err(|e| {
                log::error!(
                    "Failed to read segment file: {:?}, error: {}",
                    segment_path,
                    e
                );
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            total_size += segment_data.len() as u64;
            log::info!("Read segment {} ({} bytes)", i, segment_data.len());

            log::info!("Writing segment {} to final file", i);
            final_file.write_all(&segment_data).map_err(|e| {
                log::error!(
                    "Failed to write to final file: {:?}, error: {}",
                    final_path,
                    e
                );
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        }

        // Flush and close file
        final_file.flush().map_err(|e| {
            log::error!("Failed to flush final file: {:?}, error: {}", final_path, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        drop(final_file);

        // Clean up temporary directory
        log::info!("Cleaning up temporary directory: {:?}", temp_dir);
        if let Err(e) = std::fs::remove_dir_all(&temp_dir) {
            log::warn!(
                "Failed to clean up temp directory: {:?}, error: {}",
                temp_dir,
                e
            );
            // Continue despite cleanup failure
        }

        log::info!(
            "File '{}' (ID: {}) successfully combined from {} segments, total size: {}",
            file_name,
            file_id,
            total_segments,
            total_size
        );

        // Create file info
        let file_info = FileInfo {
            id: file_id,
            name: file_name,
            path: final_path,
            size: total_size,
            mime_type: "application/octet-stream".to_string(),
        };

        // Add file to the list
        {
            let mut file_list = state.file_list.lock().unwrap();
            file_list.add_file(file_info.clone());
            log::info!(
                "Web upload: Added file '{}' to server file list. Total files: {}",
                file_info.name,
                file_list.files.len()
            );
        }

        log::info!(
            "Successfully completed upload process for file: {}",
            file_info.name
        );
        Ok(Json(file_info))
    } else {
        // Return a response indicating segment was received
        log::info!(
            "Successfully saved segment {} of {}",
            segment_index + 1,
            total_segments
        );
        Ok(Json(FileInfo {
            id: file_id,
            name: format!("segment_{} of {}", segment_index + 1, total_segments),
            path: segment_path,
            size: file_data.len() as u64,
            mime_type: "application/octet-stream".to_string(),
        }))
    }
}
