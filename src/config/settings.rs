//! Configuration settings and validation.

use crate::{Error, Result};
use std::path::PathBuf;

/// Main configuration for Nellie server.
#[derive(Debug, Clone)]
pub struct Config {
    /// Directory for `SQLite` database and other data.
    pub data_dir: PathBuf,

    /// Host address to bind to.
    pub host: String,

    /// Port to listen on.
    pub port: u16,

    /// Log level (trace, debug, info, warn, error).
    pub log_level: String,

    /// Directories to watch for code changes.
    pub watch_dirs: Vec<PathBuf>,

    /// Maximum number of embedding worker threads.
    pub embedding_threads: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("./data"),
            host: "127.0.0.1".to_string(),
            port: 8080,
            log_level: "info".to_string(),
            watch_dirs: Vec::new(),
            embedding_threads: std::thread::available_parallelism()
                .map(|n| n.get().min(4))
                .unwrap_or(4),
        }
    }
}

impl Config {
    /// Create a new configuration with defaults.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Load configuration from environment variables and defaults.
    ///
    /// Note: This is a simplified loader. Full loading is done via clap in main.rs.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration is invalid.
    pub fn load() -> Result<Self> {
        let config = Self::default();
        config.validate()?;
        Ok(config)
    }

    /// Validate configuration values.
    ///
    /// # Errors
    ///
    /// Returns an error if any configuration value is invalid.
    pub fn validate(&self) -> Result<()> {
        // Validate port
        if self.port == 0 {
            return Err(Error::config("port cannot be 0"));
        }

        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.log_level.to_lowercase().as_str()) {
            return Err(Error::config(format!(
                "invalid log level '{}', must be one of: {}",
                self.log_level,
                valid_levels.join(", ")
            )));
        }

        // Validate embedding threads
        if self.embedding_threads == 0 {
            return Err(Error::config("embedding_threads cannot be 0"));
        }

        if self.embedding_threads > 32 {
            return Err(Error::config(
                "embedding_threads cannot exceed 32 (hardware limit)",
            ));
        }

        // Validate host is not empty
        if self.host.is_empty() {
            return Err(Error::config("host cannot be empty"));
        }

        Ok(())
    }

    /// Get the path to the `SQLite` database file.
    #[must_use]
    pub fn database_path(&self) -> PathBuf {
        self.data_dir.join("nellie.db")
    }

    /// Get the server address as a string.
    #[must_use]
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.port, 8080);
        assert_eq!(config.host, "127.0.0.1");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_new() {
        let config = Config::new();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_port() {
        let config = Config {
            port: 0,
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("port"));
    }

    #[test]
    fn test_validate_invalid_log_level() {
        let config = Config {
            log_level: "invalid".to_string(),
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("log level"));
    }

    #[test]
    fn test_validate_invalid_embedding_threads_zero() {
        let config = Config {
            embedding_threads: 0,
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("embedding_threads"));
    }

    #[test]
    fn test_validate_invalid_embedding_threads_too_high() {
        let config = Config {
            embedding_threads: 100,
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("32"));
    }

    #[test]
    fn test_validate_empty_host() {
        let config = Config {
            host: String::new(),
            ..Default::default()
        };
        let err = config.validate().unwrap_err();
        assert!(err.to_string().contains("host"));
    }

    #[test]
    fn test_database_path() {
        let config = Config {
            data_dir: PathBuf::from("/var/lib/nellie"),
            ..Default::default()
        };
        assert_eq!(
            config.database_path(),
            PathBuf::from("/var/lib/nellie/nellie.db")
        );
    }

    #[test]
    fn test_server_addr() {
        let config = Config {
            host: "0.0.0.0".to_string(),
            port: 9090,
            ..Default::default()
        };
        assert_eq!(config.server_addr(), "0.0.0.0:9090");
    }

    #[test]
    fn test_all_log_levels_valid() {
        for level in ["trace", "debug", "info", "warn", "error"] {
            let config = Config {
                log_level: level.to_string(),
                ..Default::default()
            };
            assert!(config.validate().is_ok(), "Level '{level}' should be valid");
        }
    }

    #[test]
    fn test_log_level_case_insensitive() {
        for level in ["TRACE", "Debug", "INFO", "Warn", "ERROR"] {
            let config = Config {
                log_level: level.to_string(),
                ..Default::default()
            };
            assert!(
                config.validate().is_ok(),
                "Level '{level}' should be valid (case insensitive)"
            );
        }
    }
}
