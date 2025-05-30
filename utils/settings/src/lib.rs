use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::fs;
use std::io::Write;
use std::sync::Mutex;
use std::{path::PathBuf, sync::Arc};

/// Trait for managing application settings
pub trait Settings: Serialize {
    type Config: DeserializeOwned + Serialize + Default + Clone + 'static;

    /// Get the singleton instance of the settings manager
    fn instance() -> Result<Arc<Mutex<Self>>>
    where
        Self: Sized;

    /// Save the current configuration
    fn save(&self, path: &PathBuf) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .context(format!("Failed to create settings directory: {:?}", parent))?;
        }

        let yaml = serde_yaml::to_string(self).context("Failed to serialize settings to YAML")?;

        let mut file = fs::File::create(path)
            .context(format!("Failed to create settings file: {:?}", path))?;

        file.write_all(yaml.as_bytes())
            .context(format!("Failed to write settings to file: {:?}", path))?;

        Ok(())
    }

    /// Load the configuration
    fn load(path: &PathBuf) -> Result<Self::Config> {
        // if !path.exists() {
        //     let default_config = Self::Config::default();
        //     return Ok(default_config);
        // }

        // let config_content = fs::read_to_string(path)
        //     .context(format!("Failed to read settings file: {:?}", path))?;

        // let config: Self::Config = serde_yaml::from_str(&config_content)
        //     .context(format!("Failed to parse settings file: {:?}", path))?;
        let config = Self::Config::default();
        Ok(config)
    }
}

#[cfg(feature = "settings_derive")]
pub use settings_derive::Settings;
