//! # Configuration Module
//!
//! This module defines the global [`Config`] struct using the [`config_generator!`] macro.
//!
//! It provides a thread-safe, lazily-initialized configuration object that can be populated from a
//! `HashMap<String, String>` (e.g., parsed from CLI, environment variables, or a config file).
//!
//! The config values are accessible globally through generated static getter methods like
//! `Config::log_level()` or `Config::flush_interval()`.
//!
//! ## Purpose
//!
//! The main goal of this configuration system is to provide:
//! - Centralized, static configuration
//! - Type-safe access to values
//! - Optional overrides from runtime maps
//! - One-time initialization using `OnceLock`
//!
//! ## Usage
//!
//! ```rust
//! use macro_keeper::config_generator;
//! use std::collections::HashMap;
//! use log_keeper::Filter;
//!
//! config_generator!(Config, CONFIG, [
//!    (log_level, Filter, Filter::Info),
//!    (environment, String, "development".to_string()),
//!    (worker_id, String, format!("logger-{}", std::process::id())),
//!    (log_file_buffer_capacity, usize, 100),
//!    (stdout_buffer_capacity, usize, 100),
//!    (flush_interval, u64, 1000),
//!    (log_to_file, bool, true),
//!    (log_to_stdout, bool, true),
//!    (log_file_path, String, "tmp/app.log".to_string()),
//!    ]);
//!
//! let mut map = HashMap::new();
//! map.insert("environment".to_string(), "production".to_string());
//! Config::from_hashmap(Some(map));
//!
//! assert_eq!(*Config::environment(), "production".to_string());
//! assert_eq!(*Config::log_file_buffer_capacity(), 100);
//! ```
//!
//! ## Fields
//! - `log_level`: Logging verbosity level (`Filter` enum)
//! - `environment`: The environment name (e.g., "development", "production")
//! - `worker_id`: A unique identifier for the logger instance (includes process ID)
//! - `log_file_buffer_capacity`: Max lines to buffer before flushing to disk
//! - `stdout_buffer_capacity`: Max lines to buffer before flushing to stdout
//! - `flush_interval`: Interval in milliseconds to auto-flush buffers
//! - `log_to_file`: Whether to enable file logging
//! - `log_to_stdout`: Whether to enable stdout logging
//! - `log_file_path`: Path to the log file (if `log_to_file` is true)
//!
//! ## Safety
//!
//! Calling `from_hashmap()` multiple times has no effect after the first call.
//! Use it during early initialization or application boot.
use crate::Filter;
use macro_keeper::config_generator;

config_generator!(
    Config,
    CONFIG,
    [
        (log_level, Filter, Filter::Info),
        (environment, String, "development".to_string()),
        (worker_id, String, format!("logger-{}", std::process::id())),
        (log_file_buffer_capacity, usize, 100),
        (stdout_buffer_capacity, usize, 100),
        (flush_interval, u64, 1000),
        (log_to_file, bool, true),
        (log_to_stdout, bool, true),
        (log_file_path, String, "tmp/app.log".to_string()),
    ]
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::from_hashmap(None);
        assert_eq!(config.log_level, Filter::Info);
        assert_eq!(config.environment, "development");
        assert!(config.worker_id.starts_with("logger-"));
        assert_eq!(config.log_file_buffer_capacity, 100);
        assert_eq!(config.stdout_buffer_capacity, 100);
        assert_eq!(config.flush_interval, 1000);
        assert!(config.log_to_file);
        assert!(config.log_to_stdout);
        assert_eq!(config.log_file_path, "tmp/app.log");
    }

    #[test]
    fn test_getter_functions() {
        let _config = Config::from_hashmap(None);
        assert_eq!(Config::log_level(), &Filter::Info);
        assert_eq!(Config::environment(), "development");
        assert!(Config::worker_id().starts_with("logger-"));
        assert_eq!(Config::log_file_buffer_capacity(), &100);
        assert_eq!(Config::stdout_buffer_capacity(), &100);
        assert_eq!(Config::flush_interval(), &1000);
        assert!(Config::log_to_file());
        assert!(Config::log_to_stdout());
        assert_eq!(Config::log_file_path(), "tmp/app.log");
    }
}
