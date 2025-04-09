use anyhow::Result;
use chrono::Local;
use log::{Level, LevelFilter, Metadata, Record};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Custom logger that writes to both file and console
pub struct FileLogger {
    level: Level,
    file: Arc<Mutex<File>>,
}

impl FileLogger {
    /// Create a new logger that writes to the specified file path
    pub fn new(file_path: &Path, level: Level) -> Result<Self> {
        // Create directory if it doesn't exist
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Open log file with append mode
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)?;

        Ok(FileLogger {
            level,
            file: Arc::new(Mutex::new(file)),
        })
    }
}

impl log::Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let now = Local::now();
            let message = format!(
                "[{} {} {}:{}] {}\n",
                now.format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.args()
            );

            // Write to file
            if let Ok(mut file) = self.file.lock() {
                if let Err(e) = file.write_all(message.as_bytes()) {
                    eprintln!("Failed to write to log file: {}", e);
                }
            }

            // Also print to console
            match record.level() {
                Level::Error => eprintln!("{}", message),
                _ => println!("{}", message),
            }
        }
    }

    fn flush(&self) {
        if let Ok(mut file) = self.file.lock() {
            let _ = file.flush();
        }
    }
}

/// Initialize the logger to write to both file and console
pub fn init(log_file_path: &Path, level: Level) -> Result<()> {
    let logger = FileLogger::new(log_file_path, level)?;

    // Convert Level to LevelFilter manually
    let level_filter = match level {
        Level::Error => LevelFilter::Error,
        Level::Warn => LevelFilter::Warn,
        Level::Info => LevelFilter::Info,
        Level::Debug => LevelFilter::Debug,
        Level::Trace => LevelFilter::Trace,
    };

    if let Err(e) =
        log::set_boxed_logger(Box::new(logger)).map(|()| log::set_max_level(level_filter))
    {
        return Err(anyhow::anyhow!("Failed to set logger: {}", e));
    }

    log::info!(
        "Logger initialized at level {} with output to {}",
        level,
        log_file_path.display()
    );

    Ok(())
}

/// A convenience function to initialize the logger with default settings
/// Logs to "./logs/justrans.log" at INFO level
pub fn init_default() -> Result<()> {
    init(Path::new("logs/justrans.log"), Level::Info)
}

/// Helper function to create timestamped log file path
pub fn timestamped_log_path() -> Result<std::path::PathBuf> {
    let now = Local::now();
    let log_file_name = format!("justrans_{}.log", now.format("%Y%m%d_%H%M%S"));
    let log_path = Path::new("logs").join(log_file_name);

    // Ensure logs directory exists
    std::fs::create_dir_all("logs")?;

    Ok(log_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use log::{debug, error, info, warn};
    use std::io::Read;

    #[test]
    fn test_logger_creates_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let log_path = temp_dir.path().join("test.log");

        // Initialize logger
        let result = init(&log_path, Level::Debug);
        assert!(result.is_ok());

        // Log some messages
        debug!("This is a debug message");
        info!("This is an info message");
        warn!("This is a warning message");
        error!("This is an error message");

        // Check that file exists and contains our logs
        assert!(log_path.exists());

        let mut file = File::open(&log_path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

        assert!(contents.contains("debug message"));
        assert!(contents.contains("info message"));
        assert!(contents.contains("warning message"));
        assert!(contents.contains("error message"));
    }
}
