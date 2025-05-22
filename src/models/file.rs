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
