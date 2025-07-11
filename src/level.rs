//! # Logging System

use crate::error::Error;
use std::{fmt, str::FromStr};

/// Represents a log message with an associated severity level.
///
/// This enum is used internally to encapsulate different types of log entries
/// and enables easy filtering, formatting, and dispatching of logs.
#[derive(Debug)]
pub enum Log {
    /// A debug-level log message, used for development and low-level diagnostics.
    Debug(String),

    /// An info-level log message, typically used for general runtime events.
    Info(String),

    /// A warning-level log message, indicating a potential issue.
    Warn(String),

    /// An error-level log message, indicating that an operation has failed.
    Error(String),

    /// A special variant to indicate that the logger is shutting down.
    ///
    /// Used internally to gracefully terminate logging threads.
    Shutdown,
}

/// Represents the severity level for filtering log messages.
///
/// This enum is used to configure which types of log messages should be emitted.
/// Log messages below the specified `Filter` level are ignored.
///
/// The levels follow increasing severity: `Debug < Info < Warn < Error`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Filter {
    /// Show all log messages, including debug information.
    Debug = 0,

    /// Show informational, warning, and error messages.
    Info = 1,

    /// Show only warnings and errors.
    Warn = 2,

    /// Show only error messages.
    Error = 3,
}

impl fmt::Display for Log {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Log::Debug(msg) => write!(f, "{msg}"),
            Log::Info(msg) => write!(f, "{msg}"),
            Log::Warn(msg) => write!(f, "{msg}"),
            Log::Error(msg) => write!(f, "{msg}"),
            Log::Shutdown => write!(f, "[SHUTDOWN]"),
        }
    }
}

impl FromStr for Filter {
    type Err = Error;

    /// Parses a string into a `Filter` level.
    ///
    /// Accepts `"debug"`, `"info"`, `"warn"`, and `"error"` (case-insensitive).
    /// Returns an `Error::InvalidLevel` if the input is unrecognized.
    fn from_str(s: &str) -> Result<Self, Error> {
        match s.to_lowercase().as_str() {
            "debug" => Ok(Filter::Debug),
            "info" => Ok(Filter::Info),
            "warn" => Ok(Filter::Warn),
            "error" => Ok(Filter::Error),
            _ => Err(Error::InvalidLevel(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(test)]
    mod test_filters {
        use super::*;

        #[test]
        fn test_debug_filter_values() {
            assert_eq!(Filter::Debug as u8, 0);
            assert!(Filter::Debug < Filter::Info);
            assert!(Filter::Debug < Filter::Warn);
            assert!(Filter::Debug < Filter::Error);
        }

        #[test]
        fn test_info_filter_values() {
            assert_eq!(Filter::Info as u8, 1);
            assert!(Filter::Info > Filter::Debug);
            assert!(Filter::Info < Filter::Warn);
            assert!(Filter::Info < Filter::Error);
        }

        #[test]
        fn test_warn_filter_values() {
            assert_eq!(Filter::Warn as u8, 2);
            assert!(Filter::Warn > Filter::Debug);
            assert!(Filter::Warn > Filter::Info);
            assert!(Filter::Warn < Filter::Error);
        }

        #[test]
        fn test_error_filter_values() {
            assert_eq!(Filter::Error as u8, 3);
            assert!(Filter::Error > Filter::Debug);
            assert!(Filter::Error > Filter::Info);
            assert!(Filter::Error > Filter::Warn);
        }
    }

    mod test_log {
        use super::*;

        #[test]
        fn test_log_debug() {
            let log = Log::Debug("This is a debug message".to_string());
            assert_eq!(log.to_string(), "This is a debug message");
        }

        #[test]
        fn test_log_info() {
            let log = Log::Info("This is an info message".to_string());
            assert_eq!(log.to_string(), "This is an info message");
        }

        #[test]
        fn test_log_warn() {
            let log = Log::Warn("This is a warning message".to_string());
            assert_eq!(log.to_string(), "This is a warning message");
        }

        #[test]
        fn test_log_error() {
            let log = Log::Error("This is an error message".to_string());
            assert_eq!(log.to_string(), "This is an error message");
        }

        #[test]
        fn test_log_shutdown() {
            let log = Log::Shutdown;
            assert_eq!(log.to_string(), "[SHUTDOWN]");
        }
    }
}
