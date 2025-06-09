use serde::{Deserialize, Serialize};
use settings::Settings;

/// Application configuration data
/// This struct will be serialized/deserialized to/from YAML
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Settings)]
pub struct ConfigData {
    /// Server configuration
    #[serde(default)]
    pub server: ServerConfig,

    /// Display configuration
    #[serde(default)]
    pub display: DisplayConfig,

    /// File storage configuration
    #[serde(default)]
    pub storage: StorageConfig,
}

/// Server configuration options
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerConfig {
    /// Port for the server to listen on
    #[serde(default = "default_port")]
    pub port: u16,

    /// Upload chunk size in megabytes
    #[serde(default = "default_upload_chunk_size_mb")]
    pub upload_chunk_size_mb: u64,
}

/// Display configuration options
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DisplayConfig {
    /// Default theme (light or dark)
    #[serde(default = "default_theme")]
    pub theme: String,
}

/// File storage configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StorageConfig {
    /// Directory to store uploaded files
    #[serde(default = "default_storage_dir")]
    pub storage_dir: String,
}

// Default function implementations
fn default_port() -> u16 {
    8080
}

fn default_upload_chunk_size_mb() -> u64 {
    5
}

fn default_theme() -> String {
    "light".to_string()
}

fn default_storage_dir() -> String {
    "uploads".to_string()
}

// Default implementations
impl Default for ConfigData {
    fn default() -> Self {
        ConfigData {
            server: ServerConfig::default(),
            display: DisplayConfig::default(),
            storage: StorageConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            port: default_port(),
            upload_chunk_size_mb: default_upload_chunk_size_mb(),
        }
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        DisplayConfig {
            theme: default_theme(),
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        StorageConfig {
            storage_dir: default_storage_dir(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();
        assert_eq!(config.port, 8080);
        assert_eq!(config.upload_chunk_size_mb, 5);
    }

    #[test]
    fn test_load_and_save_config() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test_server_config.yaml");

        // Load default config using Settings trait
        let config = ConfigData::load(&config_path).unwrap();
        assert_eq!(config.server.port, 8080);

        // Modify and save config using Settings trait
        let mut modified_config = config.clone();
        modified_config.server.port = 9000;
        modified_config.save(&config_path).unwrap();

        // Load again and verify changes
        let reloaded_config = ConfigData::load(&config_path).unwrap();
        assert_eq!(reloaded_config.server.port, 9000);
    }

    #[test]
    fn test_settings_instance() {
        // Test that we can get the singleton instance using Settings trait
        let instance_result = ConfigData::instance();
        assert!(instance_result.is_ok());
    }
}
