use serde::{Deserialize, Serialize};
use settings::Settings as _;
use settings_derive::Settings;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Settings)]
struct Config {
    pub port: u16,
    pub bind_address: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            port: 8080,
            bind_address: "0.0.0.0".to_string(),
        }
    }
}

#[test]
fn test_settings_derive() {
    {
        let config = Config::instance().unwrap();
        let config = config.lock().unwrap();
        assert_eq!(config.port, 8080);
        assert_eq!(config.bind_address, "0.0.0.0");
    }

    {
        let config = Config::instance().unwrap();
        let mut config = config.lock().unwrap();
        config.port = 8081;
        assert_eq!(config.port, 8081);
    }

    {
        let config = Config::instance().unwrap();
        let config = config.lock().unwrap();
        assert_eq!(config.port, 8081);
    }

    {
        let config = Config::instance().unwrap();
        let config = config.lock().unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("test_config.yaml");

        config.save(&config_path).unwrap();

        // check file exists and content is correct
        assert!(config_path.exists());
        let content = std::fs::read_to_string(&config_path).unwrap();
        println!("{}", content);
        assert_eq!(content, "port: 8081\nbind_address: 0.0.0.0\n");

        // test load
        let loaded_config = Config::load(&config_path).unwrap();
        assert_eq!(loaded_config.port, 8081);
        assert_eq!(loaded_config.bind_address, "0.0.0.0");

        // test load with non-existent file
        let non_existent_path = temp_dir.path().join("non_existent.yaml");
        let default_config = Config::load(&non_existent_path).unwrap();
        assert_eq!(default_config.port, 8080);
        assert_eq!(default_config.bind_address, "0.0.0.0");
    }
}
