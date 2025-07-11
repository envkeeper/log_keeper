//! # Worker Module
//!
//! This module provides an asynchronous logging worker that processes log entries
//! from a channel and writes them to configured outputs (file and/or stdout) with
//! buffering and periodic flushing capabilities.

use std::{
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use futures::stream::StreamExt;
use futures::{channel::mpsc::UnboundedReceiver, executor::LocalPool, future::Either};

use crate::{buffer::LogBuffer, config::Config, level::Log};

/// A logging worker that processes log entries asynchronously.
///
/// The worker runs in its own thread and receives log entries through an
/// unbounded channel. It maintains separate buffers for file and stdout
/// output, flushing them periodically based on configuration.
#[derive(Debug)]
pub struct Worker {
    /// Unique identifier for this worker instance
    pub worker_id: String,
    /// Handle to the worker thread (None if not started)
    handle: Option<JoinHandle<()>>,
}

impl Worker {
    /// Creates and starts a new logging worker.
    ///
    /// # Arguments
    ///
    /// * `worker_id` - Unique identifier for this worker instance
    /// * `receiver` - Channel receiver for incoming log entries
    ///
    /// # Returns
    ///
    /// A new `Worker` instance with the logging thread started
    pub fn start_logging(worker_id: &str, receiver: UnboundedReceiver<Log>) -> Worker {
        let mut worker = Worker::new(worker_id.to_string());
        let handle = worker.loop_event(receiver);
        worker.handle = Some(handle);
        worker
    }

    /// Creates a new worker instance without starting it.
    ///
    /// # Arguments
    ///
    /// * `worker_id` - Unique identifier for this worker instance
    fn new(worker_id: String) -> Self {
        Self {
            worker_id,
            handle: None,
        }
    }

    /// Loads and configures the file buffer if file logging is enabled.
    ///
    /// # Returns
    ///
    /// `Some(LogBuffer)` if file logging is enabled, `None` otherwise
    ///
    /// # Panics
    ///
    /// Panics if the log file cannot be opened when file logging is enabled
    fn load_file_buffer(&self) -> Option<LogBuffer> {
        if !(*Config::log_to_file()) {
            return None;
        }

        let opened_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(Config::log_file_path())
            .unwrap_or_else(|_| panic!("Failed to open log file - {}", Config::log_file_path()));

        Some(LogBuffer::new(
            Config::log_file_buffer_capacity(),
            Box::new(opened_file),
        ))
    }

    /// Loads and configures the stdout buffer if stdout logging is enabled.
    ///
    /// # Returns
    ///
    /// `Some(LogBuffer)` if stdout logging is enabled, `None` otherwise
    fn load_stdout_buffer(&self) -> Option<LogBuffer> {
        if !(*Config::log_to_stdout()) {
            return None;
        }

        Some(LogBuffer::new(
            Config::stdout_buffer_capacity(),
            Box::new(std::io::stdout()),
        ))
    }

    /// Returns file and stdout log buffers as configured.
    /// Both buffers are optional depending on feature flags.
    ///
    /// # Returns
    ///
    /// A tuple containing `(file_buffer, stdout_buffer)` where each can be `None`
    /// if the corresponding output is disabled
    fn load_buffers(&self) -> (Option<LogBuffer>, Option<LogBuffer>) {
        (self.load_file_buffer(), self.load_stdout_buffer())
    }

    /// Gets the current time and flush interval for timing operations.
    ///
    /// # Returns
    ///
    /// A tuple containing `(current_time, flush_interval_duration)`
    pub fn loop_event_timing() -> (Instant, Duration) {
        let now = Instant::now();
        let flush_interval = Duration::from_millis(*Config::flush_interval());
        (now, flush_interval)
    }

    /// Main event loop for processing logs with a single buffer.
    ///
    /// This function continuously processes incoming log entries and flushes
    /// the buffer at regular intervals. It handles three types of events:
    /// - Incoming log entries (written to buffer)
    /// - Shutdown signals (breaks the loop)
    /// - Timeout events (triggers buffer flush)
    ///
    /// # Arguments
    ///
    /// * `receiver` - Mutable reference to the log receiver channel
    /// * `buffer` - Mutable reference to the log buffer
    ///
    /// # Behavior
    ///
    /// - Logs are immediately pushed to the buffer
    /// - Buffer is flushed when flush interval elapses
    /// - Loop exits on `Log::Shutdown` or channel closure
    /// - Final flush is performed before exit
    pub async fn loop_event_execution(
        receiver: &mut UnboundedReceiver<Log>,
        buffer: &mut LogBuffer,
    ) {
        let (mut last_flush, flush_interval) = Worker::loop_event_timing();

        loop {
            let next_log = receiver.next();
            let log_opt = futures::future::select(
                Box::pin(next_log),
                Box::pin(futures_timer::Delay::new(flush_interval)),
            )
            .await;

            match log_opt {
                Either::Left((Some(Log::Shutdown), _)) => break,
                Either::Left((Some(log), _)) => {
                    buffer.push(log.to_string());
                }
                Either::Left((None, _)) => break, // channel closed
                Either::Right((_, _)) => {
                    if last_flush.elapsed() >= flush_interval {
                        let _ = buffer.flush();
                        last_flush = Instant::now();
                    }
                }
            }
        }

        let _ = buffer.flush();
    }

    /// Main event loop for processing logs with dual buffers (file and stdout).
    ///
    /// Similar to `loop_event_execution` but writes to both file and stdout buffers
    /// simultaneously. This is more efficient than running separate loops when both
    /// outputs are enabled.
    ///
    /// # Arguments
    ///
    /// * `receiver` - Mutable reference to the log receiver channel
    /// * `file_buffer` - Mutable reference to the file log buffer
    /// * `stdout_buffer` - Mutable reference to the stdout log buffer
    ///
    /// # Behavior
    ///
    /// - Each log entry is written to both buffers
    /// - Both buffers are flushed simultaneously at intervals
    /// - Loop exits on `Log::Shutdown` or channel closure
    /// - Final flush is performed on both buffers before exit
    pub async fn dual_loop_event_execution(
        receiver: &mut UnboundedReceiver<Log>,
        file_buffer: &mut LogBuffer,
        stdout_buffer: &mut LogBuffer,
    ) {
        let (mut last_flush, flush_interval) = Worker::loop_event_timing();

        loop {
            let next_log = receiver.next();
            let log_opt = futures::future::select(
                Box::pin(next_log),
                Box::pin(futures_timer::Delay::new(flush_interval)),
            )
            .await;

            match log_opt {
                Either::Left((Some(Log::Shutdown), _)) => break,
                Either::Left((Some(log), _)) => {
                    file_buffer.push(log.to_string());
                    stdout_buffer.push(log.to_string());
                }
                Either::Left((None, _)) => break, // channel closed
                Either::Right((_, _)) => {
                    if last_flush.elapsed() >= flush_interval {
                        let _ = file_buffer.flush();
                        let _ = stdout_buffer.flush();
                        last_flush = Instant::now();
                    }
                }
            }
        }

        let _ = file_buffer.flush();
        let _ = stdout_buffer.flush();
    }

    /// Starts the worker thread and begins processing log entries.
    ///
    /// This method creates a new thread that runs an async executor to handle
    /// the log processing loop. The appropriate loop variant is selected based
    /// on which buffers are available:
    /// - Both file and stdout: uses `dual_loop_event_execution`
    /// - Only one buffer: uses `loop_event_execution`
    /// - No buffers: panics with an error message
    ///
    /// # Arguments
    ///
    /// * `receiver` - Channel receiver for incoming log entries
    ///
    /// # Returns
    ///
    /// `JoinHandle` for the spawned worker thread
    ///
    /// # Panics
    ///
    /// Panics if no buffers are available (both file and stdout logging disabled)
    /// or if the thread cannot be spawned
    fn loop_event(&self, receiver: UnboundedReceiver<Log>) -> thread::JoinHandle<()> {
        let worker_id = &self.worker_id;
        let (mut file_buffer, mut stdout_buffer) = self.load_buffers();
        thread::Builder::new()
            .name(format!("Worker {worker_id}"))
            .spawn(move || {
                let mut pool = LocalPool::new();
                pool.run_until(async move {
                    let mut receiver = receiver;

                    match (file_buffer.as_mut(), stdout_buffer.as_mut()) {
                        (Some(fb), Some(sb)) => {
                            Worker::dual_loop_event_execution(&mut receiver, fb, sb).await;
                        }
                        (Some(fb), None) => {
                            Worker::loop_event_execution(&mut receiver, fb).await;
                        }
                        (None, Some(sb)) => {
                            Worker::loop_event_execution(&mut receiver, sb).await;
                        }
                        (None, None) => panic!(
                            "At least one buffer should be available,
                            but both file and stdout buffers were disabled."
                        ),
                    }
                })
            })
            .expect("Failed to start worker thread")
    }

    /// Waits for the worker thread to complete.
    ///
    /// This method blocks until the worker thread finishes execution.
    /// It should typically be called after sending a `Log::Shutdown` message
    /// to gracefully terminate the worker.
    ///
    /// # Behavior
    ///
    /// - Blocks until the worker thread completes
    /// - Logs an error message if the thread panicked
    /// - Does nothing if the worker was never started
    pub fn join(self) {
        if let Some(handle) = self.handle {
            if let Err(e) = handle.join() {
                eprintln!("Worker {} failed to join: {:?}", self.worker_id, e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{buffer::LogBuffer, level::Log};
    use futures::channel::mpsc;
    use futures::executor::block_on;
    use futures::SinkExt;
    use std::io::Cursor;
    use std::thread;
    use std::time::Duration;

    // Test helper to create a LogBuffer with in-memory writer
    fn create_test_buffer() -> LogBuffer {
        let writer = Box::new(Cursor::new(Vec::new()));
        LogBuffer::new(&1024, writer)
    }

    // Test helper to create test Log instances
    fn create_info_log(msg: &str) -> Log {
        // You'll need to adjust this based on your actual Log enum variants
        // This is a placeholder - replace with actual Log construction
        Log::Info(msg.to_string())
    }

    fn create_error_log(msg: &str) -> Log {
        Log::Error(msg.to_string())
    }

    fn create_shutdown_log() -> Log {
        Log::Shutdown
    }

    #[test]
    fn test_worker_creation() {
        let worker = Worker::new("test-worker".to_string());
        assert_eq!(worker.worker_id, "test-worker");
        assert!(worker.handle.is_none());
    }

    #[test]
    fn test_loop_event_timing() {
        let (start_time, duration) = Worker::loop_event_timing();

        // Verify that we get a valid timestamp
        assert!(start_time.elapsed() < Duration::from_millis(10));

        // Verify duration is reasonable (this would depend on actual config)
        assert!(duration > Duration::from_millis(0));
    }

    #[test]
    fn test_single_buffer_execution() {
        let mut buffer = create_test_buffer();
        let (mut sender, mut receiver) = mpsc::unbounded();

        // Test in a separate thread to avoid blocking
        let handle = thread::spawn(move || {
            block_on(async {
                // Send some test logs
                sender
                    .send(create_info_log("Test message 1"))
                    .await
                    .unwrap();
                sender.send(create_error_log("Test error")).await.unwrap();
                sender.send(create_shutdown_log()).await.unwrap();

                // Close the sender
                sender.close().await.unwrap();
            });
        });

        // Run the event loop
        block_on(async {
            Worker::loop_event_execution(&mut receiver, &mut buffer).await;
        });

        handle.join().unwrap();

        // Test passes if no panics occurred
        // Content verification would require access to LogBuffer internals
    }

    #[test]
    fn test_dual_buffer_execution() {
        let mut file_buffer = create_test_buffer();
        let mut stdout_buffer = create_test_buffer();
        let (mut sender, mut receiver) = mpsc::unbounded();

        // Test in a separate thread
        let handle = thread::spawn(move || {
            block_on(async {
                sender
                    .send(create_info_log("Dual buffer test"))
                    .await
                    .unwrap();
                sender.send(create_shutdown_log()).await.unwrap();
                sender.close().await.unwrap();
            });
        });

        // Run the dual event loop
        block_on(async {
            Worker::dual_loop_event_execution(&mut receiver, &mut file_buffer, &mut stdout_buffer)
                .await;
        });

        handle.join().unwrap();

        // Test passes if no panics occurred
        // Content verification would require access to LogBuffer internals
    }

    #[test]
    fn test_channel_closed_handling() {
        let mut buffer = create_test_buffer();
        let (sender, mut receiver) = mpsc::unbounded::<Log>();

        // Close the sender immediately
        drop(sender);

        // The loop should exit gracefully when the channel is closed
        block_on(async {
            Worker::loop_event_execution(&mut receiver, &mut buffer).await;
        });

        // Test passes if no panics occurred
    }

    #[test]
    fn test_flush_timing() {
        let mut buffer = create_test_buffer();
        let (mut sender, mut receiver) = mpsc::unbounded();

        let start = std::time::Instant::now();

        let handle = thread::spawn(move || {
            block_on(async {
                sender.send(create_info_log("Before flush")).await.unwrap();

                // Wait longer than flush interval
                futures_timer::Delay::new(Duration::from_millis(150)).await;

                sender.send(create_info_log("After flush")).await.unwrap();
                sender.send(create_shutdown_log()).await.unwrap();
                sender.close().await.unwrap();
            });
        });

        // Run the event loop
        block_on(async {
            Worker::loop_event_execution(&mut receiver, &mut buffer).await;
        });

        handle.join().unwrap();

        // Verify some time has passed (indicating flush timing worked)
        assert!(start.elapsed() >= Duration::from_millis(100));
    }

    #[test]
    fn test_worker_start_and_join() {
        let (mut sender, receiver) = mpsc::unbounded();

        // Start the worker
        let worker = Worker::start_logging("test-worker", receiver);
        assert_eq!(worker.worker_id, "test-worker");
        assert!(worker.handle.is_some());

        // Send some logs and shutdown
        block_on(async {
            sender.send(create_info_log("Test log")).await.unwrap();
            sender.send(create_shutdown_log()).await.unwrap();
        });

        // Join should complete successfully
        worker.join();
    }

    #[test]
    fn test_worker_join_without_handle() {
        let worker = Worker::new("test-worker".to_string());

        // Should not panic when joining a worker without a handle
        worker.join();
    }

    #[test]
    fn test_multiple_log_messages() {
        let mut buffer = create_test_buffer();
        let (mut sender, mut receiver) = mpsc::unbounded();

        let handle = thread::spawn(move || {
            block_on(async {
                for i in 0..10 {
                    sender
                        .send(create_info_log(&format!("Message {i}")))
                        .await
                        .unwrap();
                }
                sender.send(create_shutdown_log()).await.unwrap();
                sender.close().await.unwrap();
            });
        });

        block_on(async {
            Worker::loop_event_execution(&mut receiver, &mut buffer).await;
        });

        handle.join().unwrap();

        // Test passes if no panics occurred
        // Content verification would require access to LogBuffer internals
    }

    #[test]
    fn test_error_and_info_logs() {
        let mut buffer = create_test_buffer();
        let (mut sender, mut receiver) = mpsc::unbounded();

        let handle = thread::spawn(move || {
            block_on(async {
                sender.send(create_info_log("Info message")).await.unwrap();
                sender
                    .send(create_error_log("Error message"))
                    .await
                    .unwrap();
                sender.send(create_info_log("Another info")).await.unwrap();
                sender.send(create_shutdown_log()).await.unwrap();
                sender.close().await.unwrap();
            });
        });

        block_on(async {
            Worker::loop_event_execution(&mut receiver, &mut buffer).await;
        });

        handle.join().unwrap();

        // Test passes if no panics occurred
        // Content verification would require access to LogBuffer internals
    }

    #[test]
    fn test_concurrent_dual_buffer_writes() {
        let mut file_buffer = create_test_buffer();
        let mut stdout_buffer = create_test_buffer();
        let (mut sender, mut receiver) = mpsc::unbounded();

        let handle = thread::spawn(move || {
            block_on(async {
                // Send multiple messages rapidly
                for i in 0..5 {
                    sender
                        .send(create_info_log(&format!("Rapid message {i}")))
                        .await
                        .unwrap();
                }
                sender.send(create_shutdown_log()).await.unwrap();
                sender.close().await.unwrap();
            });
        });

        block_on(async {
            Worker::dual_loop_event_execution(&mut receiver, &mut file_buffer, &mut stdout_buffer)
                .await;
        });

        handle.join().unwrap();

        // Test passes if no panics occurred
        // Content verification would require access to LogBuffer internals
    }

    // Integration test that simulates real usage
    #[test]
    fn test_worker_integration() {
        let (mut sender, receiver) = mpsc::unbounded();

        // Start worker
        let worker = Worker::start_logging("integration-test", receiver);

        // Send various log types
        let sender_handle = thread::spawn(move || {
            block_on(async {
                sender
                    .send(create_info_log("Application started"))
                    .await
                    .unwrap();
                sender
                    .send(create_error_log("Connection failed"))
                    .await
                    .unwrap();
                sender
                    .send(create_info_log("Retrying connection"))
                    .await
                    .unwrap();
                sender
                    .send(create_info_log("Connection successful"))
                    .await
                    .unwrap();

                // Wait a bit to test timing
                futures_timer::Delay::new(Duration::from_millis(50)).await;

                sender
                    .send(create_info_log("Processing complete"))
                    .await
                    .unwrap();
                sender.send(create_shutdown_log()).await.unwrap();
            });
        });

        // Wait for sender to finish
        sender_handle.join().unwrap();

        // Wait for worker to finish
        worker.join();

        // Test passes if no panics occurred
    }
}
