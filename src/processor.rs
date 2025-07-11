//! # Processor Job Logger

use futures::channel::mpsc::UnboundedSender;

use crate::{
    config::Config,
    level::{Filter, Log},
    Logger,
};

/// A logger implementation that sends log messages through an unbounded channel.
///
/// `JobLogger` is designed for asynchronous logging scenarios where log messages
/// need to be processed by a separate consumer. It respects the configured log level
/// and only sends messages that meet or exceed the minimum severity threshold.
#[derive(Debug)]
pub struct JobLogger {
    /// Channel sender for transmitting log messages
    sender: UnboundedSender<Log>,
    /// Minimum log level filter - messages below this level are ignored
    level: Filter,
}

impl JobLogger {
    /// Creates a new `JobLogger` with the specified sender channel.
    ///
    /// The log level is automatically set from the global configuration.
    ///
    /// # Arguments
    ///
    /// * `sender` - An unbounded sender channel for transmitting log messages
    ///
    /// # Returns
    ///
    /// A new `JobLogger` instance configured with the global log level
    pub fn new(sender: UnboundedSender<Log>) -> Self {
        Self {
            sender,
            level: *Config::log_level(),
        }
    }

    /// Determines whether a message at the given level should be logged.
    ///
    /// Messages are logged if their level is greater than or equal to the
    /// configured minimum log level.
    ///
    /// # Arguments
    ///
    /// * `level` - The log level to check
    ///
    /// # Returns
    ///
    /// `true` if the message should be logged, `false` otherwise
    fn should_log(&self, level: Filter) -> bool {
        self.level <= level
    }

    /// Logs a message at the specified level if it meets the minimum threshold.
    ///
    /// This method checks if the message should be logged based on the configured
    /// level, creates the appropriate `Log` variant, and sends it through the channel.
    /// If the channel send fails (e.g., receiver is dropped), the error is silently
    /// ignored to prevent logging from crashing the application.
    ///
    /// # Arguments
    ///
    /// * `level` - The severity level of the message
    /// * `message` - The message content to log
    fn log(&self, level: Filter, message: String) {
        if self.should_log(level) {
            let log_message: Log = match level {
                Filter::Debug => Log::Debug(message),
                Filter::Info => Log::Info(message),
                Filter::Warn => Log::Warn(message),
                Filter::Error => Log::Error(message),
            };
            let _ = self.sender.unbounded_send(log_message);
        }
    }
}

impl Logger for JobLogger {
    /// Logs a debug message.
    ///
    /// Debug messages are typically used for detailed diagnostic information
    /// that is of interest when diagnosing problems.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to log (anything that can be converted to a string reference)
    fn debug<S: AsRef<str>>(&self, message: S) {
        self.log(Filter::Debug, message.as_ref().into());
    }

    /// Logs an informational message.
    ///
    /// Info messages are used to record general information about application
    /// execution, typically for audit trails or general status updates.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to log (anything that can be converted to a string reference)
    fn info<S: AsRef<str>>(&self, message: S) {
        self.log(Filter::Info, message.as_ref().into());
    }

    /// Logs a warning message.
    ///
    /// Warning messages indicate potentially harmful situations or deprecated
    /// usage that doesn't prevent the application from continuing.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to log (anything that can be converted to a string reference)
    fn warn<S: AsRef<str>>(&self, message: S) {
        self.log(Filter::Warn, message.as_ref().into());
    }

    /// Logs an error message.
    ///
    /// Error messages indicate serious problems that might still allow the
    /// application to continue running.
    ///
    /// # Arguments
    ///
    /// * `message` - The message to log (anything that can be converted to a string reference)
    fn error<S: AsRef<str>>(&self, message: S) {
        self.log(Filter::Error, message.as_ref().into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::channel::mpsc;

    /// Helper function to create a logger with a specific log level
    #[cfg(test)]
    fn create_logger_with_level(level: Filter) -> (JobLogger, mpsc::UnboundedReceiver<Log>) {
        let (sender, receiver) = mpsc::unbounded();

        // Note: In a real implementation, you might need to mock Config::log_level()
        // For this example, we'll assume the JobLogger constructor can be modified
        // to accept a level parameter for testing purposes
        let mut logger = JobLogger::new(sender);
        logger.level = level; // Override for testing

        (logger, receiver)
    }

    /// Helper function to collect messages from receiver synchronously
    #[cfg(test)]
    fn collect_messages(
        mut receiver: mpsc::UnboundedReceiver<Log>,
        expected_count: usize,
    ) -> Vec<Log> {
        use std::sync::mpsc;
        use std::thread;
        use std::time::Duration;

        let (tx, rx) = mpsc::channel();

        // Spawn a thread to collect messages
        thread::spawn(move || {
            let mut messages = Vec::new();

            // Use a simple polling approach with timeout
            for _ in 0..expected_count {
                match receiver.try_next() {
                    Ok(Some(msg)) => messages.push(msg),
                    Ok(None) => break, // Channel closed
                    Err(_) => {
                        // Channel is empty, wait a bit and try again
                        thread::sleep(Duration::from_millis(1));
                        match receiver.try_next() {
                            Ok(Some(msg)) => messages.push(msg),
                            _ => break,
                        }
                    }
                }
            }

            tx.send(messages).unwrap();
        });

        // Wait for the thread to complete with timeout
        rx.recv_timeout(Duration::from_millis(100))
            .unwrap_or_else(|_| Vec::new())
    }

    #[test]
    fn test_new_logger_creation() {
        let (sender, _receiver) = mpsc::unbounded();
        let logger = JobLogger::new(sender);

        // Logger should be created successfully
        // The level should be set from Config::log_level()
        assert_eq!(logger.level, *Config::log_level());
    }

    #[test]
    fn test_should_log_with_different_levels() {
        let (sender, _receiver) = mpsc::unbounded();
        let mut logger = JobLogger::new(sender);

        // Test with Info level configuration
        logger.level = Filter::Info;

        assert!(!logger.should_log(Filter::Debug)); // Debug < Info
        assert!(logger.should_log(Filter::Info)); // Info == Info
        assert!(logger.should_log(Filter::Warn)); // Warn > Info
        assert!(logger.should_log(Filter::Error)); // Error > Info
    }

    #[test]
    fn test_should_log_with_debug_level() {
        let (sender, _receiver) = mpsc::unbounded();
        let mut logger = JobLogger::new(sender);

        logger.level = Filter::Debug;

        // All levels should be logged when set to Debug
        assert!(logger.should_log(Filter::Debug));
        assert!(logger.should_log(Filter::Info));
        assert!(logger.should_log(Filter::Warn));
        assert!(logger.should_log(Filter::Error));
    }

    #[test]
    fn test_should_log_with_error_level() {
        let (sender, _receiver) = mpsc::unbounded();
        let mut logger = JobLogger::new(sender);

        logger.level = Filter::Error;

        // Only Error level should be logged
        assert!(!logger.should_log(Filter::Debug));
        assert!(!logger.should_log(Filter::Info));
        assert!(!logger.should_log(Filter::Warn));
        assert!(logger.should_log(Filter::Error));
    }

    #[test]
    fn test_debug_logging() {
        let (logger, receiver) = create_logger_with_level(Filter::Debug);

        logger.debug("Debug message");

        let messages = collect_messages(receiver, 1);
        assert_eq!(messages.len(), 1);

        match &messages[0] {
            Log::Debug(msg) => assert_eq!(msg, "Debug message"),
            _ => panic!("Expected Debug log variant"),
        }
    }

    #[test]
    fn test_info_logging() {
        let (logger, receiver) = create_logger_with_level(Filter::Info);

        logger.info("Info message");

        let messages = collect_messages(receiver, 1);
        assert_eq!(messages.len(), 1);

        match &messages[0] {
            Log::Info(msg) => assert_eq!(msg, "Info message"),
            _ => panic!("Expected Info log variant"),
        }
    }

    #[test]
    fn test_warn_logging() {
        let (logger, receiver) = create_logger_with_level(Filter::Warn);

        logger.warn("Warning message");

        let messages = collect_messages(receiver, 1);
        assert_eq!(messages.len(), 1);

        match &messages[0] {
            Log::Warn(msg) => assert_eq!(msg, "Warning message"),
            _ => panic!("Expected Warn log variant"),
        }
    }

    #[test]
    fn test_error_logging() {
        let (logger, receiver) = create_logger_with_level(Filter::Error);

        logger.error("Error message");

        let messages = collect_messages(receiver, 1);
        assert_eq!(messages.len(), 1);

        match &messages[0] {
            Log::Error(msg) => assert_eq!(msg, "Error message"),
            _ => panic!("Expected Error log variant"),
        }
    }

    #[test]
    fn test_log_filtering() {
        let (logger, receiver) = create_logger_with_level(Filter::Warn);

        // These should be filtered out
        logger.debug("Debug message");
        logger.info("Info message");

        // These should pass through
        logger.warn("Warning message");
        logger.error("Error message");

        // Should only receive 2 messages (the filtered ones)
        let messages = collect_messages(receiver, 2);
        assert_eq!(messages.len(), 2);

        match &messages[0] {
            Log::Warn(msg) => assert_eq!(msg, "Warning message"),
            _ => panic!("Expected Warn log variant"),
        }

        match &messages[1] {
            Log::Error(msg) => assert_eq!(msg, "Error message"),
            _ => panic!("Expected Error log variant"),
        }
    }

    #[test]
    fn test_string_types() {
        let (logger, _receiver) = create_logger_with_level(Filter::Debug);

        // Test different string types
        logger.debug("String literal");
        logger.info(String::from("Owned String"));
        logger.warn("String reference");

        let owned_string = String::from("Another owned string");
        logger.error(&owned_string);

        // All of these should compile and work due to AsRef<str> bound
    }

    #[test]
    fn test_dropped_receiver() {
        let (sender, receiver) = mpsc::unbounded();
        let logger = JobLogger::new(sender);

        // Drop the receiver
        drop(receiver);

        // Logging should not panic even when receiver is dropped
        logger.info("This should not panic");
        logger.error("Neither should this");
    }

    #[test]
    fn test_multiple_messages() {
        let (logger, receiver) = create_logger_with_level(Filter::Debug);

        // Send multiple messages
        logger.debug("Message 1");
        logger.info("Message 2");
        logger.warn("Message 3");
        logger.error("Message 4");

        // Collect all messages
        let messages = collect_messages(receiver, 4);
        assert_eq!(messages.len(), 4);

        // Verify message order and content
        match &messages[0] {
            Log::Debug(msg) => assert_eq!(msg, "Message 1"),
            _ => panic!("Expected Debug log variant"),
        }

        match &messages[1] {
            Log::Info(msg) => assert_eq!(msg, "Message 2"),
            _ => panic!("Expected Info log variant"),
        }

        match &messages[2] {
            Log::Warn(msg) => assert_eq!(msg, "Message 3"),
            _ => panic!("Expected Warn log variant"),
        }

        match &messages[3] {
            Log::Error(msg) => assert_eq!(msg, "Message 4"),
            _ => panic!("Expected Error log variant"),
        }
    }

    #[test]
    fn test_empty_messages() {
        let (logger, _receiver) = create_logger_with_level(Filter::Debug);

        // Empty messages should work
        logger.debug("");
        logger.info("");
        logger.warn("");
        logger.error("");
    }

    #[test]
    fn test_unicode_messages() {
        let (logger, receiver) = create_logger_with_level(Filter::Info);

        logger.info("Hello 世界! 🌍");

        let messages = collect_messages(receiver, 1);
        assert_eq!(messages.len(), 1);

        match &messages[0] {
            Log::Info(msg) => assert_eq!(msg, "Hello 世界! 🌍"),
            _ => panic!("Expected Info log variant"),
        }
    }

    #[test]
    fn test_large_messages() {
        let (logger, receiver) = create_logger_with_level(Filter::Error);

        let large_message = "x".repeat(10000);
        logger.error(&large_message);

        let messages = collect_messages(receiver, 1);
        assert_eq!(messages.len(), 1);

        match &messages[0] {
            Log::Error(msg) => assert_eq!(msg.len(), 10000),
            _ => panic!("Expected Error log variant"),
        }
    }
}
