//! # Error Handling

use std::fmt;

/// Represents all possible errors that can occur in the LogKeeper system.
pub enum Error {
    /// Represents an underlying I/O error.
    Io(std::io::Error),

    /// A generic catch-all for unexpected or unknown errors.
    /// Typically used when the source of the error isn't a standard I/O or logging issue.
    Unknown(String),

    /// Error triggered when a log entry exceeds the configured size limit.
    ///
    /// # Fields
    /// - `size`: The actual size of the log entry.
    /// - `limit`: The configured maximum allowed size.
    LogKeeperTooLarge { size: usize, limit: usize },

    /// Error triggered when the logger is initialized or configured with an invalid log level string.
    ///
    /// This usually comes from a misconfiguration or a typo in environment/config values.
    InvalidLevel(String),

    /// The logger operation timed out before completion.
    ///
    /// This may indicate that the logging system is overloaded or blocked.
    Timeout,

    /// Failed to print a log message to the output (e.g., terminal, file).
    ///
    /// This might happen if the underlying writer is broken or locked.
    PrintError,

    /// Indicates that the logger is shutting down, potentially with a reason.
    ///
    /// Used to gracefully notify consumers of termination.
    Shutdown(String),
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(err) => f.debug_tuple("Io").field(err).finish(),
            Error::Unknown(s) => f.debug_tuple("Unknown Error").field(s).finish(),
            Error::LogKeeperTooLarge { size, limit } => f
                .debug_struct("LogKeeperTooLarge")
                .field("size", size)
                .field("limit", limit)
                .finish(),
            Error::InvalidLevel(level) => f.debug_tuple("InvalidLevel").field(level).finish(),
            Error::Timeout => f.debug_tuple("Timeout").finish(),
            Error::PrintError => f.debug_tuple("PrintError").finish(),
            Error::Shutdown(msg) => f.debug_tuple("Shutdown").field(msg).finish(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(err) => write!(f, "IO error: {err}"),
            Error::Unknown(s) => write!(f, "Connection closed: {s}"),
            Error::LogKeeperTooLarge { size, limit } => {
                write!(
                    f,
                    "LogKeeper too large: {size} bytes exceeds limit of {limit} bytes"
                )
            }
            Error::InvalidLevel(level) => write!(f, "Invalid log level: {level}"),
            Error::Timeout => write!(f, "LogKeeper timeout"),
            Error::PrintError => write!(f, "LogKeeper print error"),
            Error::Shutdown(msg) => write!(f, "LogKeeper shutdown: {msg}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_io_error_display() {
        let io_err = io::Error::other("fail");
        let err = Error::Io(io_err);
        let s = format!("{err}");
        assert!(s.contains("IO error: fail"));
    }

    #[test]
    fn test_unknown_display() {
        let err = Error::Unknown("oops".to_string());
        assert_eq!(format!("{err}"), "Connection closed: oops");
    }

    #[test]
    fn test_arc_logger_too_large_display() {
        let err = Error::LogKeeperTooLarge {
            size: 2048,
            limit: 1024,
        };
        assert_eq!(
            format!("{err}"),
            "LogKeeper too large: 2048 bytes exceeds limit of 1024 bytes"
        );
    }

    #[test]
    fn test_invalid_level_display() {
        let err = Error::InvalidLevel("SILLY".to_string());
        assert_eq!(format!("{err}"), "Invalid log level: SILLY");
    }

    #[test]
    fn test_timeout_display() {
        let err = Error::Timeout;
        assert_eq!(format!("{err}"), "LogKeeper timeout");
    }

    #[test]
    fn test_print_error_display() {
        let err = Error::PrintError;
        assert_eq!(format!("{err}"), "LogKeeper print error");
    }

    #[test]
    fn test_shutdown_display() {
        let err = Error::Shutdown("done".to_string());
        assert_eq!(format!("{err}"), "LogKeeper shutdown: done");
    }
}
