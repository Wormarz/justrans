use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub mime_type: String,
}

impl FileInfo {
    pub fn new(path: PathBuf) -> anyhow::Result<Self> {
        let metadata = std::fs::metadata(&path)?;
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let mime_type = mime_guess::from_path(&path)
            .first_or_octet_stream()
            .to_string();

        Ok(Self {
            id: Uuid::new_v4().to_string(),
            name,
            path,
            size: metadata.len(),
            mime_type,
        })
    }

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

    pub fn remove_file(&mut self, index: usize) -> Option<FileInfo> {
        if index < self.files.len() {
            Some(self.files.remove(index))
        } else {
            None
        }
    }

    pub fn get_file(&self, index: usize) -> Option<&FileInfo> {
        self.files.get(index)
    }

    pub fn get_file_by_id(&self, id: &str) -> Option<&FileInfo> {
        self.files.iter().find(|f| f.id == id)
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

impl Default for FileList {
    fn default() -> Self {
        Self::new()
    }
}
