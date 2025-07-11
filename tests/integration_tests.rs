#[cfg(test)]
mod integration_tests {
    use log_keeper::Log;
    use log_keeper::LogKeeper;
    use std::collections::HashMap;
    use std::fs;
    use std::io::Read;
    use std::path::Path;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    // Helper function to ensure test log directory exists
    fn ensure_test_log_dir() {
        if fs::create_dir_all("tmp").is_err() {
            // Directory might already exist, which is fine
        }
    }

    // Helper function to clean up test log files
    fn cleanup_test_log(path: &str) {
        let _ = fs::remove_file(path);
        println!("Cleaned up test log file: {path}");
    }

    // Helper function to read log file contents
    fn read_log_file(path: &str) -> Result<String, std::io::Error> {
        let mut file = fs::File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Ok(contents)
    }

    // Helper function to wait for async operations to complete
    fn wait_for_flush() {
        thread::sleep(Duration::from_millis(100));
    }

    #[test]
    fn test_default_configuration() {
        let logger = LogKeeper::new(None);

        // Test all log levels with default configuration
        logger.debug("Debug message");
        logger.info("Info message");
        logger.warn("Warning message");
        logger.error("Error message");

        wait_for_flush();
        logger.shutdown();
    }

    #[test]
    fn test_log_level_filtering_info() {
        ensure_test_log_dir();

        let mut config = HashMap::new();
        config.insert("log_level".to_string(), "info".to_string());
        config.insert("log_file_path".to_string(), "./tmp/app.log".to_string());
        config.insert("log_to_file".to_string(), "true".to_string());
        config.insert("log_to_stdout".to_string(), "false".to_string());

        let logger = LogKeeper::new(Some(config));

        logger.debug("Debug should NOT appear");
        logger.info("Info should appear");
        logger.warn("Warn should appear");
        logger.error("Error should appear");

        wait_for_flush();
        logger.shutdown();

        let contents = read_log_file("./tmp/app.log")
            .unwrap_or_else(|_| panic!("Failed to read log file: {}", "./tmp/app.log"));
        assert!(!contents.contains("Debug should NOT appear"));
        assert!(contents.contains("Info should appear"));
        assert!(contents.contains("Warn should appear"));
        assert!(contents.contains("Error should appear"));

        cleanup_test_log("./tmp/app.log");
    }

    #[test]
    fn test_file_only_logging() {
        ensure_test_log_dir();

        let mut config = HashMap::new();
        config.insert("log_file_path".to_string(), "./tmp/app.log".to_string());
        config.insert("log_to_file".to_string(), "true".to_string());
        config.insert("log_to_stdout".to_string(), "false".to_string());

        let logger = LogKeeper::new(Some(config));

        logger.info("File only message");
        logger.error("File only error");

        wait_for_flush();
        logger.shutdown();

        assert!(Path::new("./tmp/app.log").exists());
        let contents = read_log_file("./tmp/app.log")
            .unwrap_or_else(|_| panic!("Failed to read log file: {}", "./tmp/app.log"));
        assert!(contents.contains("File only message"));
        assert!(contents.contains("File only error"));

        cleanup_test_log("./tmp/app.log");
    }

    #[test]
    fn test_stdout_only_logging() {
        let mut config = HashMap::new();
        config.insert("log_to_file".to_string(), "false".to_string());
        config.insert("log_to_stdout".to_string(), "true".to_string());

        let logger = LogKeeper::new(Some(config));

        logger.info("Stdout only message");
        logger.error("Stdout only error");

        wait_for_flush();
        logger.shutdown();

        // Note: Cannot easily test stdout output in unit tests,
        // but this verifies the configuration doesn't crash
    }

    #[test]
    fn test_no_logging_outputs() {
        let mut config = HashMap::new();
        config.insert("log_to_file".to_string(), "false".to_string());
        config.insert("log_to_stdout".to_string(), "false".to_string());

        let logger = LogKeeper::new(Some(config));

        logger.info("Message that goes nowhere");
        logger.error("Error that goes nowhere");

        wait_for_flush();
        logger.shutdown();

        // This should not crash even with no outputs enabled
    }

    #[test]
    fn test_buffer_capacity_settings() {
        ensure_test_log_dir();

        let mut config = HashMap::new();
        config.insert("log_file_path".to_string(), "./tmp/app.log".to_string());
        config.insert("log_file_buffer_capacity".to_string(), "2".to_string());
        config.insert("stdout_buffer_capacity".to_string(), "2".to_string());
        config.insert("flush_interval".to_string(), "50".to_string());
        config.insert("log_to_file".to_string(), "true".to_string());

        let logger = LogKeeper::new(Some(config));

        // Send more messages than buffer capacity to trigger flushing
        for i in 0..5 {
            logger.info(format!("Buffer test message {i}"));
        }

        wait_for_flush();
        logger.shutdown();

        let contents = read_log_file("./tmp/app.log")
            .unwrap_or_else(|_| panic!("Failed to read log file: {}", "./tmp/app.log"));
        assert!(contents.contains("Buffer test message 0"));
        assert!(contents.contains("Buffer test message 4"));

        cleanup_test_log("./tmp/app.log");
    }

    #[test]
    fn test_fast_flush_interval() {
        ensure_test_log_dir();

        let mut config = HashMap::new();
        config.insert("log_file_path".to_string(), "./tmp/app.log".to_string());
        config.insert("flush_interval".to_string(), "10".to_string()); // Very fast flush
        config.insert("log_to_file".to_string(), "true".to_string());

        let logger = LogKeeper::new(Some(config));

        logger.info("Fast flush test");

        // Even with fast flush, give it a moment
        thread::sleep(Duration::from_millis(50));
        logger.shutdown();

        let contents = read_log_file("./tmp/app.log")
            .unwrap_or_else(|_| panic!("Failed to read log file: {}", "./tmp/app.log"));
        assert!(contents.contains("Fast flush test"));

        cleanup_test_log("./tmp/app.log");
    }

    #[test]
    fn test_multithreaded_logging() {
        ensure_test_log_dir();

        let mut config = HashMap::new();
        config.insert("log_file_path".to_string(), "./tmp/app.log".to_string());
        config.insert("log_to_file".to_string(), "true".to_string());
        config.insert("log_to_stdout".to_string(), "false".to_string());

        let logger = Arc::new(LogKeeper::new(Some(config)));
        let mut handles = vec![];

        // Spawn multiple threads that log concurrently
        for thread_id in 0..5 {
            let logger_clone = Arc::clone(&logger);
            let handle = thread::spawn(move || {
                for i in 0..10 {
                    logger_clone.info(format!("Thread {thread_id} message {i}"));
                    logger_clone.warn(format!("Thread {thread_id} warning {i}"));
                }
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().expect("Thread failed");
        }

        wait_for_flush();

        // Extract logger from Arc and shutdown
        let logger = Arc::try_unwrap(logger).expect("Failed to unwrap Arc");
        logger.shutdown();

        let contents = read_log_file("./tmp/app.log")
            .unwrap_or_else(|_| panic!("Failed to read log file: {}", "./tmp/app.log"));

        // Verify messages from all threads are present
        for thread_id in 0..5 {
            assert!(contents.contains(&format!("Thread {thread_id} message 0")));
            assert!(contents.contains(&format!("Thread {thread_id} warning 9")));
        }

        cleanup_test_log("./tmp/app.log");
    }

    #[test]
    fn test_high_volume_logging() {
        ensure_test_log_dir();

        let mut config = HashMap::new();
        config.insert("log_file_path".to_string(), "./tmp/app.log".to_string());
        config.insert("log_to_file".to_string(), "true".to_string());
        config.insert("log_to_stdout".to_string(), "false".to_string());
        config.insert("log_file_buffer_capacity".to_string(), "50".to_string());

        let logger = LogKeeper::new(Some(config));

        // Log a large number of messages
        for i in 0..1000 {
            logger.info(format!("High volume message {i}"));
        }

        wait_for_flush();
        logger.shutdown();

        let contents = read_log_file("./tmp/app.log")
            .unwrap_or_else(|_| panic!("Failed to read log file: {}", "./tmp/app.log"));
        assert!(contents.contains("High volume message 0"));
        assert!(contents.contains("High volume message 999"));

        // Count the number of messages (rough check)
        let message_count = contents.matches("High volume message").count();
        assert_eq!(message_count, 1000);

        cleanup_test_log("./tmp/app.log");
    }

    #[test]
    fn test_join_without_shutdown() {
        ensure_test_log_dir();

        let mut config = HashMap::new();
        config.insert("log_file_path".to_string(), "./tmp/app.log".to_string());
        config.insert("log_to_file".to_string(), "true".to_string());

        let logger = LogKeeper::new(Some(config));

        logger.info("Message before join");

        // Manually send shutdown signal through the sender
        let _ = logger.sender.unbounded_send(Log::Shutdown);

        // Use join instead of shutdown
        logger.join();

        let contents = read_log_file("./tmp/app.log")
            .unwrap_or_else(|_| panic!("Failed to read log file: {}", "./tmp/app.log"));
        assert!(contents.contains("Message before join"));

        cleanup_test_log("./tmp/app.log");
    }

    #[test]
    fn test_string_types_and_formatting() {
        ensure_test_log_dir();

        let mut config = HashMap::new();
        config.insert("log_file_path".to_string(), "./tmp/app.log".to_string());
        config.insert("log_to_file".to_string(), "true".to_string());
        config.insert("log_to_stdout".to_string(), "false".to_string());

        let logger = LogKeeper::new(Some(config));

        // Test different string types
        logger.info("String literal");
        logger.info(String::from("String object"));
        logger.info(format!("Formatted string with number: {}", 42));

        let message = "Variable string";
        logger.info(message);

        let owned_message = String::from("Owned string");
        logger.info(&owned_message);
        logger.info(owned_message); // Move the string

        wait_for_flush();
        logger.shutdown();

        let contents = read_log_file("./tmp/app.log")
            .unwrap_or_else(|_| panic!("Failed to read log file: {}", "./tmp/app.log"));
        assert!(contents.contains("String literal"));
        assert!(contents.contains("String object"));
        assert!(contents.contains("Formatted string with number: 42"));
        assert!(contents.contains("Variable string"));
        assert!(contents.contains("Owned string"));

        cleanup_test_log("./tmp/app.log");
    }

    #[test]
    fn test_empty_and_special_messages() {
        ensure_test_log_dir();

        let mut config = HashMap::new();
        config.insert("log_file_path".to_string(), "./tmp/app.log".to_string());
        config.insert("log_to_file".to_string(), "true".to_string());
        config.insert("log_to_stdout".to_string(), "false".to_string());

        let logger = LogKeeper::new(Some(config));

        // Test edge cases
        logger.info(""); // Empty string
        logger.info("   "); // Whitespace only
        logger.info("Message with\nnewlines\nand\ttabs");
        logger.info("Message with special chars: !@#$%^&*()");
        logger.info("Unicode message: 🚀 Hello 世界");
        logger.info("Very long message: ".to_string() + &"x".repeat(1000));

        wait_for_flush();
        logger.shutdown();

        let contents = read_log_file("./tmp/app.log")
            .unwrap_or_else(|_| panic!("Failed to read log file: {}", "./tmp/app.log"));
        assert!(contents.contains("Message with\nnewlines\nand\ttabs"));
        assert!(contents.contains("Message with special chars: !@#$%^&*()"));
        assert!(contents.contains("Unicode message: 🚀 Hello 世界"));
        assert!(contents.contains(&"x".repeat(1000)));

        cleanup_test_log("./tmp/app.log");
    }

    #[test]
    fn test_invalid_configuration_values() {
        // Test that invalid configuration values don't crash the system
        let mut config = HashMap::new();
        config.insert("log_level".to_string(), "invalid_level".to_string());
        config.insert("flush_interval".to_string(), "not_a_number".to_string());
        config.insert("log_file_buffer_capacity".to_string(), "-1".to_string());
        config.insert("log_to_file".to_string(), "maybe".to_string());

        // This should not panic, but fall back to defaults
        let logger = LogKeeper::new(Some(config));

        logger.info("Message with invalid config");
        logger.shutdown();
    }

    #[test]
    fn test_logger_interface() {
        // Test that LogKeeper implements all Logger methods correctly
        let logger = LogKeeper::new(None);

        // Test all methods from the Logger trait
        logger.info("Interface info message");
        logger.warn("Interface warn message");
        logger.error("Interface error message");
        logger.debug("Interface debug message");

        logger.shutdown();
    }

    #[test]
    fn test_rapid_create_destroy() {
        // Test creating and destroying loggers rapidly
        for i in 0..10 {
            let logger = LogKeeper::new(None);
            logger.info(format!("Rapid test {i}"));
            logger.shutdown();
        }
    }

    #[test]
    fn test_configuration_edge_cases() {
        // Test various edge cases in configuration
        let test_cases = vec![
            // Empty values
            HashMap::from([
                ("log_level".to_string(), "".to_string()),
                ("log_file_path".to_string(), "".to_string()),
            ]),
            // Extreme values
            HashMap::from([
                ("flush_interval".to_string(), "0".to_string()),
                ("log_file_buffer_capacity".to_string(), "1".to_string()),
                ("stdout_buffer_capacity".to_string(), "1000000".to_string()),
            ]),
            // Case sensitivity
            HashMap::from([
                ("log_level".to_string(), "DEBUG".to_string()),
                ("log_to_file".to_string(), "TRUE".to_string()),
            ]),
        ];

        for config in test_cases {
            let logger = LogKeeper::new(Some(config));
            logger.info("Edge case test");
            logger.shutdown();
        }
    }

    #[test]
    fn test_direct_channel_communication() {
        ensure_test_log_dir();

        let mut config = HashMap::new();
        config.insert("log_file_path".to_string(), "./tmp/app.log".to_string());
        config.insert("log_to_file".to_string(), "true".to_string());
        config.insert("log_to_stdout".to_string(), "false".to_string());

        let logger = LogKeeper::new(Some(config));

        // Test direct channel communication
        logger.info("Before direct send");

        // Send messages directly through the channel
        let _ = logger
            .sender
            .unbounded_send(Log::Info("Direct channel message".to_string()));
        let _ = logger
            .sender
            .unbounded_send(Log::Warn("Direct channel warning".to_string()));

        logger.info("After direct send");

        wait_for_flush();
        logger.shutdown();

        let contents = read_log_file("./tmp/app.log")
            .unwrap_or_else(|_| panic!("Failed to read log file: {}", "./tmp/app.log"));
        assert!(contents.contains("Before direct send"));
        assert!(contents.contains("Direct channel message"));
        assert!(contents.contains("Direct channel warning"));
        assert!(contents.contains("After direct send"));

        cleanup_test_log("./tmp/app.log");
    }

    #[test]
    fn test_worker_thread_behavior() {
        ensure_test_log_dir();

        let mut config = HashMap::new();
        config.insert("log_file_path".to_string(), "./tmp/app.log".to_string());
        config.insert("log_to_file".to_string(), "true".to_string());
        config.insert("worker_id".to_string(), "custom_worker_123".to_string());

        let logger = LogKeeper::new(Some(config));

        // Test that worker is properly initialized
        assert!(logger.worker.is_some());

        logger.info("Testing worker thread");

        wait_for_flush();
        logger.shutdown();

        let contents = read_log_file("./tmp/app.log")
            .unwrap_or_else(|_| panic!("Failed to read log file: {}", "./tmp/app.log"));
        assert!(contents.contains("Testing worker thread"));

        cleanup_test_log("./tmp/app.log");
    }

    #[test]
    fn test_channel_capacity_stress() {
        ensure_test_log_dir();

        let mut config = HashMap::new();
        config.insert(
            "log_file_path".to_string(),
            "./tmp/app.log".to_string().clone(),
        );
        config.insert("log_to_file".to_string(), "true".to_string());
        config.insert("log_to_stdout".to_string(), "false".to_string());

        let logger = LogKeeper::new(Some(config));

        // Stress test the unbounded channel
        for i in 0..5000 {
            logger.info(format!("Stress message {i}"));
            if i % 100 == 0 {
                // Mix in other log levels
                logger.warn(format!("Stress warning {i}"));
                logger.error(format!("Stress error {i}"));
            }
        }

        wait_for_flush();
        logger.shutdown();

        let contents = read_log_file("./tmp/app.log")
            .unwrap_or_else(|_| panic!("Failed to read log file: {}", "./tmp/app.log"));
        assert!(contents.contains("Stress message 0"));
        assert!(contents.contains("Stress message 4999"));
        assert!(contents.contains("Stress warning 4900"));
        assert!(contents.contains("Stress error 4900"));

        cleanup_test_log("./tmp/app.log");
    }

    #[test]
    fn test_logger_drop_behavior() {
        ensure_test_log_dir();

        let mut config = HashMap::new();
        config.insert("log_file_path".to_string(), "./tmp/app.log".to_string());
        config.insert("log_to_file".to_string(), "true".to_string());

        {
            let logger = LogKeeper::new(Some(config));
            logger.info("Message before drop");

            // Logger will be dropped here without explicit shutdown
            // This tests the implicit drop behavior
        }

        // Give some time for any cleanup
        thread::sleep(Duration::from_millis(100));

        // Note: The behavior when dropping without shutdown depends on the implementation
        // This test mainly ensures no panic occurs

        cleanup_test_log("./tmp/app.log");
    }
}
