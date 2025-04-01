use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub mime_type: String,
}

impl FileInfo {
    pub fn formatted_size(&self) -> String {
        let size = self.size as f64;

        if size < 1024.0 {
            format!("{:.0} B", size)
        } else if size < 1024.0 * 1024.0 {
            format!("{:.1} KB", size / 1024.0)
        } else if size < 1024.0 * 1024.0 * 1024.0 {
            format!("{:.1} MB", size / (1024.0 * 1024.0))
        } else {
            format!("{:.1} GB", size / (1024.0 * 1024.0 * 1024.0))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileList {
    pub files: Vec<FileInfo>,
}

impl FileList {
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    pub fn add_file(&mut self, file: FileInfo) {
        self.files.push(file);
    }

    pub fn get_file_by_id(&self, id: &str) -> Option<&FileInfo> {
        self.files.iter().find(|f| f.id == id)
    }
}

impl Default for FileList {
    fn default() -> Self {
        Self::new()
    }
}
