//! # Buffered Log Writer

/// A buffered log writer that accumulates log messages in memory and flushes them
/// in batches to an underlying writer (e.g., file, stdout).
use std::collections::VecDeque;
use std::io::{BufWriter, Write};

pub struct LogBuffer {
    buffer: VecDeque<String>,
    capacity: usize,
    writer: BufWriter<Box<dyn Write + Send>>,
}

/// A buffered log writer that stores log messages in memory and flushes them
/// in batches to an underlying writer (e.g., file or stdout).
///
/// The main goal of `LogBuffer` is to **avoid frequent I/O locks** by batching
/// writes instead of writing logs immediately. This helps reduce contention
/// and improve performance in multi-threaded or high-frequency logging scenarios.
impl LogBuffer {
    /// Creates a new `LogBuffer` with a given capacity and writer.
    ///
    /// # Arguments
    ///
    /// * `capacity` - The number of messages to accumulate before flushing.
    /// * `writer` - A destination implementing `Write + Send`, such as a file, `stdout`, etc.
    ///
    /// # Purpose
    ///
    /// This setup allows you to delay I/O operations until the buffer is full,
    /// **reducing the number of locks and context switches required**.
    pub fn new(capacity: &usize, writer: Box<dyn Write + Send>) -> Self {
        LogBuffer {
            buffer: VecDeque::with_capacity(*capacity),
            capacity: *capacity,
            writer: BufWriter::new(writer),
        }
    }

    /// Adds a log message to the internal buffer.
    ///
    /// If the number of buffered messages reaches the configured `capacity`,
    /// this will automatically flush the buffer to the writer.
    ///
    /// # Panics
    ///
    /// Will panic if flushing fails internally. Consider using `flush()` directly
    /// if you need to handle errors gracefully.
    pub fn push(&mut self, msg: String) {
        self.buffer.push_back(msg);
        if self.buffer.len() >= self.capacity {
            self.flush().unwrap();
        }
    }

    /// Flushes all buffered log messages to the writer immediately.
    ///
    /// This writes each message in the order it was added and clears the buffer.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if the final flush operation fails.
    ///
    /// # Error Handling
    ///
    /// If writing a specific line fails, an error is printed to `stderr`
    /// but the flush will continue for remaining messages.
    pub fn flush(&mut self) -> std::io::Result<()> {
        while let Some(line) = self.buffer.pop_front() {
            writeln!(self.writer, "{line}").unwrap_or_else(|err| {
                eprintln!("Failed to write log: {err}");
            });
        }
        self.writer.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::sync::{Arc, Mutex};

    #[cfg(test)]
    struct MockWriter {
        buffer: Arc<Mutex<Vec<u8>>>,
    }

    #[cfg(test)]
    impl MockWriter {
        fn new() -> (Self, Arc<Mutex<Vec<u8>>>) {
            let buffer = Arc::new(Mutex::new(Vec::new()));
            (
                MockWriter {
                    buffer: buffer.clone(),
                },
                buffer,
            )
        }
    }

    #[cfg(test)]
    impl Write for MockWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let mut buffer = self.buffer.lock().unwrap();
            buffer.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    // Helper method to expose writer for testing
    #[cfg(test)]
    impl LogBuffer {
        pub fn into_writer(self) -> BufWriter<Box<dyn Write + Send>> {
            self.writer
        }

        pub fn buffer_len(&self) -> usize {
            self.buffer.len()
        }
    }

    #[test]
    fn test_new_creates_empty_buffer() {
        let (writer, _) = MockWriter::new();
        let capacity = 5;
        let log_buffer = LogBuffer::new(&capacity, Box::new(writer));

        assert_eq!(log_buffer.buffer_len(), 0);
        assert_eq!(log_buffer.capacity, capacity);
    }

    #[test]
    fn test_push_does_not_flush_until_capacity() {
        let (writer, output) = MockWriter::new();
        let capacity = 3;
        let mut log_buffer = LogBuffer::new(&capacity, Box::new(writer));

        // Push messages below capacity
        log_buffer.push("message 1".to_string());
        log_buffer.push("message 2".to_string());

        // Buffer should contain messages, but nothing written to output yet
        assert_eq!(log_buffer.buffer_len(), 2);
        assert!(output.lock().unwrap().is_empty());
    }

    #[test]
    fn test_push_flushes_when_capacity_reached() {
        let (writer, output) = MockWriter::new();
        let capacity = 2;
        let mut log_buffer = LogBuffer::new(&capacity, Box::new(writer));

        // Push messages up to capacity
        log_buffer.push("message 1".to_string());
        log_buffer.push("message 2".to_string());

        // Buffer should be empty after automatic flush
        assert_eq!(log_buffer.buffer_len(), 0);

        // Check that messages were written to output
        let output_str = String::from_utf8(output.lock().unwrap().clone()).unwrap();
        assert!(output_str.contains("message 1"));
        assert!(output_str.contains("message 2"));
    }

    #[test]
    fn test_manual_flush_empties_buffer() {
        let (writer, output) = MockWriter::new();
        let capacity = 5;
        let mut log_buffer = LogBuffer::new(&capacity, Box::new(writer));

        // Push some messages
        log_buffer.push("test message 1".to_string());
        log_buffer.push("test message 2".to_string());

        assert_eq!(log_buffer.buffer_len(), 2);

        // Manual flush
        log_buffer.flush().unwrap();

        // Buffer should be empty
        assert_eq!(log_buffer.buffer_len(), 0);

        // Messages should be in output
        let output_str = String::from_utf8(output.lock().unwrap().clone()).unwrap();
        assert!(output_str.contains("test message 1"));
        assert!(output_str.contains("test message 2"));
    }

    #[test]
    fn test_flush_preserves_message_order() {
        let (writer, output) = MockWriter::new();
        let capacity = 10;
        let mut log_buffer = LogBuffer::new(&capacity, Box::new(writer));

        // Push messages in order
        log_buffer.push("first".to_string());
        log_buffer.push("second".to_string());
        log_buffer.push("third".to_string());

        log_buffer.flush().unwrap();

        let output_str = String::from_utf8(output.lock().unwrap().clone()).unwrap();
        let lines: Vec<&str> = output_str.lines().collect();

        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "first");
        assert_eq!(lines[1], "second");
        assert_eq!(lines[2], "third");
    }

    #[test]
    fn test_multiple_push_flush_cycles() {
        let (writer, output) = MockWriter::new();
        let capacity = 2;
        let mut log_buffer = LogBuffer::new(&capacity, Box::new(writer));

        // First cycle
        log_buffer.push("msg1".to_string());
        log_buffer.push("msg2".to_string()); // Should trigger flush

        // Second cycle
        log_buffer.push("msg3".to_string());
        log_buffer.push("msg4".to_string()); // Should trigger flush

        let output_str = String::from_utf8(output.lock().unwrap().clone()).unwrap();
        let lines: Vec<&str> = output_str.lines().collect();

        assert_eq!(lines.len(), 4);
        assert!(lines.contains(&"msg1"));
        assert!(lines.contains(&"msg2"));
        assert!(lines.contains(&"msg3"));
        assert!(lines.contains(&"msg4"));
    }

    #[test]
    fn test_flush_empty_buffer() {
        let (writer, output) = MockWriter::new();
        let capacity = 5;
        let mut log_buffer = LogBuffer::new(&capacity, Box::new(writer));

        // Flush empty buffer should not panic
        let result = log_buffer.flush();
        assert!(result.is_ok());

        // Output should be empty
        assert!(output.lock().unwrap().is_empty());
    }

    #[test]
    fn test_capacity_one() {
        let (writer, output) = MockWriter::new();
        let capacity = 1;
        let mut log_buffer = LogBuffer::new(&capacity, Box::new(writer));

        // Each push should immediately flush
        log_buffer.push("immediate1".to_string());
        assert_eq!(log_buffer.buffer_len(), 0);

        log_buffer.push("immediate2".to_string());
        assert_eq!(log_buffer.buffer_len(), 0);

        let output_str = String::from_utf8(output.lock().unwrap().clone()).unwrap();
        let lines: Vec<&str> = output_str.lines().collect();

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "immediate1");
        assert_eq!(lines[1], "immediate2");
    }

    #[test]
    fn test_with_cursor_writer() {
        let cursor = Cursor::new(Vec::new());
        let capacity = 3;
        let mut log_buffer = LogBuffer::new(&capacity, Box::new(cursor));

        log_buffer.push("test with cursor".to_string());
        log_buffer.flush().unwrap();

        // This test ensures compatibility with different Write implementations
        // The actual cursor content verification would require accessing the inner writer
        assert_eq!(log_buffer.buffer_len(), 0);
    }

    #[test]
    fn test_large_capacity() {
        let (writer, output) = MockWriter::new();
        let capacity = 1000;
        let mut log_buffer = LogBuffer::new(&capacity, Box::new(writer));

        // Push many messages without triggering auto-flush
        for i in 0..999 {
            log_buffer.push(format!("message {i}"));
        }

        assert_eq!(log_buffer.buffer_len(), 999);
        assert!(output.lock().unwrap().is_empty());

        // One more push should trigger flush
        log_buffer.push("final message".to_string());
        assert_eq!(log_buffer.buffer_len(), 0);

        let output_str = String::from_utf8(output.lock().unwrap().clone()).unwrap();
        let lines: Vec<&str> = output_str.lines().collect();
        assert_eq!(lines.len(), 1000);
    }

    #[test]
    fn test_empty_string_messages() {
        let (writer, output) = MockWriter::new();
        let capacity = 2;
        let mut log_buffer = LogBuffer::new(&capacity, Box::new(writer));

        log_buffer.push("".to_string());
        log_buffer.push("non-empty".to_string());

        let output_str = String::from_utf8(output.lock().unwrap().clone()).unwrap();
        let lines: Vec<&str> = output_str.lines().collect();

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "");
        assert_eq!(lines[1], "non-empty");
    }

    #[test]
    fn test_multiline_messages() {
        let (writer, output) = MockWriter::new();
        let capacity = 1;
        let mut log_buffer = LogBuffer::new(&capacity, Box::new(writer));

        // Note: Each message is treated as a single line regardless of content
        log_buffer.push("line1\nline2".to_string());

        let output_str = String::from_utf8(output.lock().unwrap().clone()).unwrap();
        assert!(output_str.contains("line1\nline2"));
    }

    #[test]
    fn test_flush_error_propagation() {
        struct FlushFailingWriter;
        impl Write for FlushFailingWriter {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Err(std::io::Error::other("flush failed"))
            }
        }

        let capacity = 2;
        let mut log_buffer = LogBuffer::new(&capacity, Box::new(FlushFailingWriter));
        log_buffer.push("will be buffered".into());
        let result = log_buffer.flush();
        assert!(result.is_err());
    }
}
