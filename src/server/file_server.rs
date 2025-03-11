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

use crate::models::{FileInfo, FileList};

const DEFAULT_PORT: u16 = 8080;

#[derive(Clone)]
pub struct AppState {
    pub file_list: Arc<Mutex<FileList>>,
    pub temp_dir: PathBuf,
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
    pub fn new() -> anyhow::Result<Self> {
        // Create temp directory for uploaded files
        let temp_dir = std::env::temp_dir().join("justrans");
        std::fs::create_dir_all(&temp_dir)?;

        // Get local IP address
        let ip = match local_ip() {
            Ok(ip) => ip.to_string(),
            Err(_) => "127.0.0.1".to_string(),
        };

        let server_info = ServerInfo {
            url: format!("http://{}:{}", ip, DEFAULT_PORT),
            ip,
            port: DEFAULT_PORT,
            running: false,
        };

        Ok(Self {
            state: AppState {
                file_list: Arc::new(Mutex::new(FileList::new())),
                temp_dir,
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

    pub fn set_file_list(&self, file_list: FileList) {
        let mut list = self.state.file_list.lock().unwrap();
        *list = file_list;
    }

    pub fn get_file_list(&self) -> FileList {
        self.state.file_list.lock().unwrap().clone()
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
            .route("/api/upload", post(upload_file))
            .nest_service("/static", static_files_service)
            .layer(TraceLayer::new_for_http())
            .layer(cors)
            .with_state(app_state);

        // Get server address
        let addr = {
            let info = server_info.lock().unwrap();
            SocketAddr::new(info.ip.parse()?, info.port)
        };

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
    while let Ok(Some(field)) = multipart.next_field().await {
        let file_name = match field.file_name() {
            Some(name) => name.to_string(),
            None => continue,
        };

        let content_type = field
            .content_type()
            .map(|ct| ct.to_string())
            .unwrap_or_else(|| "application/octet-stream".to_string());

        // Read the file data
        let data = match field.bytes().await {
            Ok(data) => data,
            Err(_) => return Err(StatusCode::BAD_REQUEST),
        };

        // Create a unique file path
        let file_id = uuid::Uuid::new_v4().to_string();
        let file_path = state.temp_dir.join(&file_id);

        // Write the file to disk
        if std::fs::write(&file_path, &data).is_err() {
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }

        // Create file info
        let file_info = FileInfo {
            id: file_id,
            name: file_name,
            path: file_path,
            size: data.len() as u64,
            mime_type: content_type,
        };

        // Add file to the list
        {
            let mut file_list = state.file_list.lock().unwrap();
            file_list.add_file(file_info.clone());
        }

        return Ok(Json(file_info));
    }

    Err(StatusCode::BAD_REQUEST)
}
