use serde::{Deserialize, Serialize};
use settings_manager::SettingsManager;
use std::path::Path;

/// Application configuration data
/// This struct will be serialized/deserialized to/from YAML
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

    /// Whether to bind to all network interfaces (0.0.0.0) or just localhost (127.0.0.1)
    #[serde(default = "default_bind_all")]
    pub bind_all_interfaces: bool,

    /// Maximum upload file size in megabytes
    #[serde(default = "default_max_upload_size_mb")]
    pub max_upload_size_mb: u64,
}

/// Display configuration options
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DisplayConfig {
    /// Whether to show tooltips
    #[serde(default = "default_true")]
    pub show_tooltips: bool,

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

    /// Whether to use relative paths
    #[serde(default = "default_true")]
    pub use_relative_paths: bool,
}

// Default function implementations
fn default_port() -> u16 {
    8080
}

fn default_bind_all() -> bool {
    true
}

fn default_max_upload_size_mb() -> u64 {
    100
}

fn default_true() -> bool {
    true
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
            bind_all_interfaces: default_bind_all(),
            max_upload_size_mb: default_max_upload_size_mb(),
        }
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        DisplayConfig {
            show_tooltips: default_true(),
            theme: default_theme(),
        }
    }
}

impl Default for StorageConfig {
    fn default() -> Self {
        StorageConfig {
            storage_dir: default_storage_dir(),
            use_relative_paths: default_true(),
        }
    }
}

impl ConfigData {
    /// Get a reference to the server configuration
    pub fn server(&self) -> &ServerConfig {
        &self.server
    }

    /// Get a reference to the display configuration
    pub fn display(&self) -> &DisplayConfig {
        &self.display
    }

    /// Get a reference to the storage configuration
    pub fn storage(&self) -> &StorageConfig {
        &self.storage
    }
}

/// Helper for managing application configuration
pub struct ConfigManager {
    settings: SettingsManager<ConfigData>,
}

impl ConfigManager {
    /// Create a new configuration manager with the given settings file path
    pub fn new<P: AsRef<Path>>(settings_path: P) -> Self {
        ConfigManager {
            settings: SettingsManager::new(settings_path),
        }
    }

    /// Load the application configuration
    pub fn load(&self) -> anyhow::Result<ConfigData> {
        self.settings.load()
    }

    /// Save the application configuration
    pub fn save(&self, config: &ConfigData) -> anyhow::Result<()> {
        self.settings.save(config)
    }

    /// Get the current configuration
    pub fn get_config(&self) -> anyhow::Result<ConfigData> {
        self.settings.get_config()
    }

    /// Update the configuration using a closure
    pub fn update<F>(&self, update_fn: F) -> anyhow::Result<()>
    where
        F: FnOnce(&mut ConfigData),
    {
        self.settings.update(update_fn)
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
        assert_eq!(config.bind_all_interfaces, true);
        assert_eq!(config.max_upload_size_mb, 100);
    }

    #[test]
    fn test_load_and_save_config() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test_server_config.yaml");

        let config_manager = ConfigManager::new(&config_path);

        // Load default config (should create file)
        let config = config_manager.load().unwrap();
        assert_eq!(config.server.port, 8080);

        // Modify and save config
        let mut modified_config = config.clone();
        modified_config.server.port = 9000;
        config_manager.save(&modified_config).unwrap();

        // Load again and verify changes
        let reloaded_config = config_manager.load().unwrap();
        assert_eq!(reloaded_config.server.port, 9000);
    }

    #[test]
    fn test_update_config() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test_update_config.yaml");

        let config_manager = ConfigManager::new(&config_path);

        // Load default config
        let _ = config_manager.load().unwrap();

        // Update config
        config_manager
            .update(|config| {
                config.server.port = 9090;
                config.server.max_upload_size_mb = 200;
            })
            .unwrap();

        // Verify changes
        let config = config_manager.get_config().unwrap();
        assert_eq!(config.server.port, 9090);
        assert_eq!(config.server.max_upload_size_mb, 200);
    }
}
