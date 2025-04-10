use anyhow::{Context, Result};
use log::{debug, info};
use serde::{de::DeserializeOwned, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Manages application settings, loads from and saves to a YAML configuration file
/// Generic over the type of configuration data stored
#[derive(Debug)]
pub struct SettingsManager<T>
where
    T: DeserializeOwned + Serialize + Default + Clone + 'static,
{
    settings_path: PathBuf,
    config: Arc<Mutex<Option<T>>>,
    _phantom: PhantomData<T>,
}

// Implementation for the generic SettingsManager
impl<T> SettingsManager<T>
where
    T: DeserializeOwned + Serialize + Default + Clone + 'static,
{
    /// Create a new settings manager with the given settings file path
    pub fn new<P: AsRef<Path>>(settings_path: P) -> Self {
        SettingsManager {
            settings_path: settings_path.as_ref().to_path_buf(),
            config: Arc::new(Mutex::new(None)),
            _phantom: PhantomData,
        }
    }

    /// Load settings from the configuration file
    pub fn load(&self) -> Result<T> {
        self.load_from_path(&self.settings_path)
    }

    /// Load settings from a specific path
    pub fn load_from_path<P: AsRef<Path>>(&self, path: P) -> Result<T> {
        let path = path.as_ref();
        debug!("Loading settings from: {:?}", path);

        // If the file doesn't exist, create it with default settings
        if !path.exists() {
            info!(
                "Settings file not found, creating with defaults: {:?}",
                path
            );

            // Create parent directories if they don't exist
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .context(format!("Failed to create settings directory: {:?}", parent))?;
            }

            // Create a default configuration
            let default_config = T::default();

            // Save the default configuration
            self.save_to_path(&default_config, path)?;

            let mut config = self.config.lock().unwrap();
            *config = Some(default_config.clone());

            return Ok(default_config);
        }

        // Read and parse the YAML file
        let config_content = fs::read_to_string(path)
            .context(format!("Failed to read settings file: {:?}", path))?;

        let config: T = serde_yaml::from_str(&config_content)
            .context(format!("Failed to parse settings file: {:?}", path))?;

        // Update the cached configuration
        let mut cached_config = self.config.lock().unwrap();
        *cached_config = Some(config.clone());

        info!("Settings loaded successfully from: {:?}", path);
        Ok(config)
    }

    /// Save settings to the configuration file
    pub fn save(&self, config: &T) -> Result<()> {
        self.save_to_path(config, &self.settings_path)
    }

    /// Save settings to a specific path
    pub fn save_to_path<P: AsRef<Path>>(&self, config: &T, path: P) -> Result<()> {
        let path = path.as_ref();
        debug!("Saving settings to: {:?}", path);

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .context(format!("Failed to create settings directory: {:?}", parent))?;
        }

        // Serialize the configuration to YAML
        let yaml = serde_yaml::to_string(config).context("Failed to serialize settings to YAML")?;

        // Write the YAML to the file
        let mut file =
            File::create(path).context(format!("Failed to create settings file: {:?}", path))?;

        file.write_all(yaml.as_bytes())
            .context(format!("Failed to write settings to file: {:?}", path))?;

        // Update the cached configuration
        let mut cached_config = self.config.lock().unwrap();
        *cached_config = Some(config.clone());

        info!("Settings saved successfully to: {:?}", path);
        Ok(())
    }

    /// Get a copy of the current configuration
    pub fn get_config(&self) -> Result<T> {
        let cached_config = self.config.lock().unwrap();
        match &*cached_config {
            Some(config) => Ok(config.clone()),
            None => {
                // If the configuration isn't loaded yet, load it
                drop(cached_config);
                self.load()
            }
        }
    }

    /// Update configuration using a closure
    pub fn update<F>(&self, update_fn: F) -> Result<()>
    where
        F: FnOnce(&mut T),
    {
        let mut config = self.get_config()?;
        update_fn(&mut config);
        self.save(&config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use tempfile::tempdir;

    #[test]
    fn test_load_and_save_settings() {
        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
        struct TestConfig {
            value: i32,
            name: String,
        }

        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test_settings.yaml");

        // Create a settings manager
        let settings = SettingsManager::<TestConfig>::new(&config_path);

        // Load settings (should create default since file doesn't exist)
        let config = settings.load().unwrap();
        assert_eq!(config.value, 0);
        assert_eq!(config.name, "");

        // Modify and save settings
        let mut modified_config = config.clone();
        modified_config.value = 42;
        modified_config.name = "test".to_string();
        settings.save(&modified_config).unwrap();

        // Load again and verify changes were saved
        let reloaded_config = settings.load().unwrap();
        assert_eq!(reloaded_config.value, 42);
        assert_eq!(reloaded_config.name, "test");
    }

    #[test]
    fn test_update_function() {
        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
        struct TestConfig {
            value: i32,
            name: String,
        }

        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test_update.yaml");

        // Create a settings manager
        let settings = SettingsManager::<TestConfig>::new(&config_path);

        // Load default settings
        let _ = settings.load().unwrap();

        // Update settings using the new update method
        settings
            .update(|config| {
                config.value = 42;
                config.name = "test".to_string();
            })
            .unwrap();

        // Verify changes
        let config = settings.get_config().unwrap();
        assert_eq!(config.value, 42);
        assert_eq!(config.name, "test");
    }
}
