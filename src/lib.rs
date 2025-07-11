//! # LogKeeper - High-Performance Asynchronous Logging Library
//!
//! LogKeeper is a buffered, asynchronous logging library designed to minimize I/O contention
//! and provide high-performance logging capabilities for Rust applications.
//!
//! ## Key Features
//!
//! - **Asynchronous Processing**: Log messages are processed in a separate worker thread
//! - **Buffered I/O**: Messages are batched before writing to reduce system calls
//! - **Dual Output Support**: Simultaneous logging to both file and stdout
//! - **Configurable Filtering**: Support for different log levels (Debug, Info, Warn, Error)
//! - **Thread-Safe**: Designed for use in multi-threaded applications
//! - **Graceful Shutdown**: Proper cleanup and flushing on termination
//!
//! ## Architecture
//!
//! The library uses a producer-consumer pattern:
//! - **Producer**: The main [`LogKeeper`] instance that accepts log messages
//! - **Consumer**: A [`Worker`] thread that processes messages and writes to outputs
//! - **Channel**: An unbounded MPSC channel for communication between producer and consumer
//! - **Buffers**: [`LogBuffer`] instances that batch writes to reduce I/O operations
//!
//! ## Configuration Reference
//!
//! The `LogKeeper::new(Some(config))` method accepts a `HashMap<String, String>`.
//! Below is the full list of accepted keys:
//!
//! - `"log_level"`: `"debug"`, `"info"`, `"warn"`, or `"error"`
//! - `"log_file_path"`: file path (e.g., `"tmp/app.log"`)
//! - `"flush_interval"`: milliseconds (e.g., `"1000"`)
//! - `"log_file_buffer_capacity"`: number of messages (e.g., `"100"`)
//! - `"stdout_buffer_capacity"`: number of messages (e.g., `"100"`)
//! - `"log_to_file"`: `"true"` or `"false"`
//! - `"log_to_stdout"`: `"true"` or `"false"`
//! - `"worker_id"`: optional identifier (default is `logger-<pid>`)
//!
//! ## Usage Example
//!
//! ```rust
//! use std::collections::HashMap;
//! use log_keeper::LogKeeper;
//!
//! // Create a logger with default configuration
//! let logger = LogKeeper::new(None);
//!
//! // Log messages at different levels
//! logger.info("Application started");
//! logger.warn("This is a warning");
//! logger.error("An error occurred");
//! logger.debug("Debug information");
//!
//! // Shutdown gracefully
//! logger.shutdown();
//! ```
//!
//! ## Custom Configuration
//!
//! ```rust
//! use std::collections::HashMap;
//! use log_keeper::LogKeeper;
//!
//! let mut config: HashMap<String, String> = HashMap::new();
//! config.insert("log_level".to_string(), "debug".to_string());
//! config.insert("log_file_path".to_string(), "tmp/app_new_tmp.log".to_string());
//! config.insert("flush_interval".to_string(), "500".to_string());
//!
//! let logger = LogKeeper::new(Some(config));
//! // Use logger...
//! logger.shutdown();
//! ```
mod buffer;
mod config;
mod error;
mod level;
mod processor;
mod worker;

use std::collections::HashMap;

use crate::config::Config;
pub use crate::error::Error;
pub use crate::level::{Filter, Log};
use crate::processor::JobLogger;
use crate::worker::Worker;

/// A trait defining the logging interface that all logger implementations must provide.
///
/// This trait ensures consistent logging behavior across different logger implementations
/// and provides a default debug implementation that outputs to stdout.
///
/// # Thread Safety
///
/// Implementors must be `Send + Sync + 'static` to ensure they can be safely used
/// across thread boundaries and have no lifetime constraints.
pub trait Logger: Send + Sync + 'static {
    /// Logs an informational message.
    ///
    /// Info messages are used for general application events and status updates.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to log (anything that can be converted to a string reference)
    fn info<S: AsRef<str>>(&self, message: S);

    /// Logs a warning message.
    ///
    /// Warning messages indicate potentially problematic situations that don't
    /// prevent the application from continuing.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to log (anything that can be converted to a string reference)
    fn warn<S: AsRef<str>>(&self, message: S);

    /// Logs an error message.
    ///
    /// Error messages indicate serious problems that might affect application functionality.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to log (anything that can be converted to a string reference)
    fn error<S: AsRef<str>>(&self, message: S);

    /// Logs a debug message.
    ///
    /// Debug messages are used for detailed diagnostic information during development.
    /// This method has a default implementation that prints directly to stdout,
    /// but can be overridden by implementors for custom behavior.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to log (anything that can be converted to a string reference)
    ///
    /// # Default Behavior
    ///
    /// The default implementation prints the message to stdout with a `[DEBUG]` prefix.
    /// This bypasses the normal logging pipeline and is intended for development use.
    fn debug<S: AsRef<str>>(&self, message: S) {
        println!("[DEBUG] {}", message.as_ref());
    }
}

/// The main logging facade that provides a high-level interface for application logging.
///
/// `LogKeeper` orchestrates the entire logging system by managing:
/// - A [`JobLogger`] that handles log message filtering and channel sending
/// - A [`Worker`] thread that processes messages asynchronously
/// - An unbounded channel for communication between logger and worker
///
/// ## Lifecycle
///
/// 1. **Creation**: Initialize configuration, create channel, start worker thread
/// 2. **Logging**: Accept log messages and send them through the channel
/// 3. **Shutdown**: Send shutdown signal, wait for worker to complete, cleanup resources
///
/// ## Thread Safety
///
/// `LogKeeper` is designed to be used from multiple threads safely. The internal
/// channel handling ensures that log messages from different threads are properly
/// serialized and processed.
#[derive(Debug)]
pub struct LogKeeper {
    /// The primary logger instance that handles message filtering and channel communication
    pub logger: JobLogger,
    /// The worker thread that processes log messages (None after shutdown)
    pub worker: Option<Worker>,
    /// Channel sender for direct communication (used for shutdown signaling)
    pub sender: futures::channel::mpsc::UnboundedSender<Log>,
}

impl LogKeeper {
    /// Creates a new `LogKeeper` instance with the specified configuration.
    ///
    /// This method initializes the entire logging system:
    /// - Sets up global configuration from the provided HashMap
    /// - Creates an unbounded channel for message passing
    /// - Starts a worker thread for asynchronous log processing
    /// - Initializes the logger with appropriate filtering
    ///
    /// # Arguments
    ///
    /// * `config` - Optional configuration overrides as key-value pairs.
    ///   If `None`, default configuration values are used.
    ///
    /// # Returns
    ///
    /// A new `LogKeeper` instance ready for logging operations.
    ///
    /// # Configuration Keys
    ///
    /// The following keys are recognized in the configuration HashMap:
    /// - `log_level`: Log level filter ("debug", "info", "warn", "error")
    /// - `log_file_path`: Path to the log file
    /// - `flush_interval`: Buffer flush interval in milliseconds
    /// - `log_file_buffer_capacity`: File buffer size in number of messages
    /// - `stdout_buffer_capacity`: Stdout buffer size in number of messages
    /// - `log_to_file`: Enable/disable file logging ("true"/"false")
    /// - `log_to_stdout`: Enable/disable stdout logging ("true"/"false")
    ///
    /// # Examples
    ///
    /// ```rust
    /// /// Default configuration
    /// use log_keeper::LogKeeper;
    /// use std::collections::HashMap;
    ///
    /// /// Custom configuration
    /// let mut config = HashMap::new();
    /// config.insert("log_level".to_string(), "debug".to_string());
    /// config.insert("flush_interval".to_string(), "500".to_string());
    /// let logger = LogKeeper::new(Some(config));
    /// ```
    pub fn new(config: Option<HashMap<String, String>>) -> Self {
        Config::from_hashmap(config);

        let (sender, receiver) = futures::channel::mpsc::unbounded::<Log>();

        let sender_ref = sender.clone();
        let logger = JobLogger::new(sender);
        let worker = Worker::start_logging(Config::worker_id(), receiver);

        LogKeeper {
            logger,
            worker: Some(worker),
            sender: sender_ref,
        }
    }

    /// Waits for the worker thread to complete without sending a shutdown signal.
    ///
    /// This method consumes the `LogKeeper` instance and blocks until the worker
    /// thread finishes processing. It should typically be called after manually
    /// sending a shutdown signal through the channel.
    ///
    /// # Usage
    ///
    /// This method is useful when you want to control the shutdown process manually
    /// or when the worker thread terminates due to channel closure.
    ///
    /// # Note
    ///
    /// Unlike [`shutdown()`](Self::shutdown), this method does not send a shutdown
    /// signal to the worker thread. The worker will continue processing until
    /// the channel is closed or a `Log::Shutdown` message is received.
    pub fn join(self) {
        if let Some(worker) = self.worker {
            worker.join();
        }
    }

    /// Gracefully shuts down the logging system.
    ///
    /// This method performs a complete shutdown sequence:
    /// 1. Sends a final informational message about the shutdown
    /// 2. Sends a `Log::Shutdown` signal to the worker thread
    /// 3. Closes the channel to prevent further message sending
    /// 4. Waits for the worker thread to complete and cleanup
    ///
    /// # Behavior
    ///
    /// - All pending log messages are processed before shutdown
    /// - Buffers are flushed to ensure no messages are lost
    /// - The worker thread terminates gracefully
    /// - Resources are cleaned up properly
    ///
    /// # Panics
    ///
    /// This method will panic if the shutdown signal cannot be sent through the
    /// channel, which typically indicates a programming error or system issue.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use log_keeper::LogKeeper;
    ///
    /// let logger = LogKeeper::new(None);
    /// logger.info("Application running");
    /// // ... application logic ...
    /// logger.shutdown(); // Graceful shutdown
    /// ```
    pub fn shutdown(mut self) {
        self.force_send(Log::Info("Logger shutting down".into()));
        self.force_send(Log::Shutdown);
        drop(self.sender);

        if let Some(worker) = self.worker.take() {
            worker.join();
        }
    }

    /// Sends a log message through the channel, panicking if the send fails.
    ///
    /// This is an internal method used for critical messages (like shutdown signals)
    /// that must be delivered successfully. It converts send errors into shutdown
    /// errors and panics if the send operation fails.
    ///
    /// # Arguments
    ///
    /// * `log` - The log message to send
    ///
    /// # Panics
    ///
    /// Panics if the message cannot be sent through the channel, typically
    /// indicating that the worker thread has terminated unexpectedly.
    fn force_send(&self, log: Log) {
        if let Err(e) = self.sender.unbounded_send(log) {
            panic!("Failed to send log during shutdown: {e}");
        }
    }

    /// Logs a debug message.
    ///
    /// Debug messages are used for detailed diagnostic information that is
    /// typically only of interest during development or troubleshooting.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to log (anything that can be converted to a string reference)
    ///
    /// # Filtering
    ///
    /// Debug messages are only processed if the configured log level is set to
    /// `Filter::Debug`. They are ignored at higher log levels.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use log_keeper::LogKeeper;
    ///
    /// let value = 42;
    /// let logger = LogKeeper::new(None);
    /// logger.debug("Entering function foo()");
    /// logger.debug(format!("Variable value: {}", value));
    /// ```
    pub fn debug<S: AsRef<str>>(&self, message: S) {
        self.logger.debug(message);
    }

    /// Logs an informational message.
    ///
    /// Info messages are used for general application events, status updates,
    /// and other informational content that might be useful for monitoring
    /// or auditing purposes.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to log (anything that can be converted to a string reference)
    ///
    /// # Filtering
    ///
    /// Info messages are processed if the configured log level is set to
    /// `Filter::Info` or lower. They are ignored at higher log levels.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use log_keeper::LogKeeper;
    ///
    /// let logger = LogKeeper::new(None);
    /// logger.info("Application started successfully");
    /// logger.info("Processing 1000 records");
    /// ```
    pub fn info<S: AsRef<str>>(&self, message: S) {
        self.logger.info(message);
    }

    /// Logs a warning message.
    ///
    /// Warning messages indicate potentially problematic situations that don't
    /// prevent the application from continuing but may require attention.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to log (anything that can be converted to a string reference)
    ///
    /// # Filtering
    ///
    /// Warning messages are processed if the configured log level is set to
    /// `Filter::Warn` or lower. They are ignored only at the `Filter::Error` level.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use log_keeper::LogKeeper;
    ///
    /// let logger = LogKeeper::new(None);
    /// logger.warn("Database connection is slow");
    /// logger.warn("Memory usage is high");
    /// ```
    pub fn warn<S: AsRef<str>>(&self, message: S) {
        self.logger.warn(message);
    }

    /// Logs an error message.
    ///
    /// Error messages indicate serious problems that affect application
    /// functionality but don't necessarily cause the application to terminate.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to log (anything that can be converted to a string reference)
    ///
    /// # Filtering
    ///
    /// Error messages are processed at all log levels since they represent
    /// the highest severity level in the logging hierarchy.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use log_keeper::LogKeeper;
    ///
    /// let logger = LogKeeper::new(None);
    /// logger.error("Database connection failed");
    /// logger.error("Failed to process user request");
    /// ```
    pub fn error<S: AsRef<str>>(&self, message: S) {
        self.logger.error(message);
    }
}
