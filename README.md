# log_keeper

A modern, thread-safe, asynchronous logging system for Rust.

## Overview

`log_keeper` provides a buffered, multi-threaded logging runtime. Log messages are sent from your main application thread to a background worker thread, which processes and outputs them asynchronously. This design ensures minimal blocking in your main code and safe, graceful shutdown.

## Architecture

```
Main Thread
    |
    v
+---------------------+
|   LogKeeper         |  (main thread)
|---------------------|
| - buffer: Arc<Buffer>  <-------------------+
| - logger: JobLogger                        |
| - worker: Option<Worker>                   |
+---------------------+                      |
        |                                    |
        v                                    |
+---------------------+                      |
|    JobLogger        |                      |
|---------------------|                      |
| - buffer: Arc<Buffer>  --------------------+
| - log_level: Filter  |
+---------------------+
        |
        |  (calls .debug(), .info(), etc.)
        v
+---------------------+
|   Buffer            |  (shared, thread-safe)
+---------------------+
        ^
        |  (Arc clone)
        |
+---------------------+
|   Worker            |  (worker thread)
|---------------------|
| - id: usize         |
| - logger: Arc<Mutex<JobLogger>>
| - handle: Option<JoinHandle<()>>
+---------------------+
        |
        |  (loop_event: runs in a new thread)
        v
+---------------------+
| Worker Thread       |  (background thread)
|---------------------|
| - Receives log messages
| - Calls logger methods
+---------------------+
```

## Threading Model

- **Main Thread:**
  - Owns the `LogKeeper`, which manages the buffer, logger, and worker.
  - All log calls (e.g., `log_keeper.info("msg")`) are made here.

- **Worker Thread:**
  - Created by `Worker::start_logging`.
  - Runs in the background, processing log messages from the buffer.

- **Total Threads:**
  - 1 main thread (your application)
  - 1 worker thread (for logging)

## Usage

```rust
use log_keeper::{LogKeeper, Filter};

let log_keeper = LogKeeper::new(Filter::Info);
log_keeper.info("Hello, world!");
log_keeper.debug("Debug message");
log_keeper.warn("Warning!");
log_keeper.error("Error!");
log_keeper.join(); // Waits for all logs to be processed before exit
```

## Graceful Shutdown

Call `log_keeper.join()` before your program exits to ensure all log messages are processed and the worker thread is cleanly shut down.

## License

MIT
