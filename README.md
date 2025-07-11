# log_keeper

**A lightweight, thread-safe, async logging system for Rust apps.**

[![crates.io](https://img.shields.io/crates/v/log_keeper.svg)](https://crates.io/crates/log_keeper)
[![Docs.rs](https://docs.rs/log_keeper/badge.svg)](https://docs.rs/log_keeper)
![License: Apache-2.0](https://img.shields.io/crates/l/log_keeper)

---

## ✨ Features

- 🔒 Thread-safe with `Arc<Buffer>` sharing
- ⚙️ Buffered logging with zero-cost macros
- 🧵 Asynchronous logging via background worker
- ✅ Graceful shutdown (`.join()`)
- 📦 No runtime dependencies outside of `futures`

---

## 🧠 Overview

`log_keeper` provides a fast, zero-alloc logging pipeline designed for async and multi-threaded environments. Logs are pushed into a buffer and processed asynchronously on a dedicated worker thread, reducing I/O blocking in your main thread.

---

## 📐 Architecture

```text
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
| - logger: Arc<Mutex<JobLogger>>
| - handle: Option<JoinHandle<()>>
+---------------------+
        |
        v
  [ Worker Thread ]
     - Loops & processes log events
     - Writes logs via JobLogger
```

---

## 🚀 Getting Started

Add it to your `Cargo.toml`:

```toml
[dependencies]
log_keeper = "0.1.0"
```

Then use it like so:

```rust
use log_keeper::{LogKeeper, Filter};

fn main() {
    let log = LogKeeper::new(Filter::Info);

    log.info("Server started");
    log.debug("This will not be shown with Filter::Info");
    log.warn("Low disk space!");
    log.error("Unexpected error occurred");

    // Optional: Waits for all logs to flush before exiting
    log.join();
}
```

---

## 🧪 Log Levels

`log_keeper` provides the following levels via the `Filter` enum:

- `Filter::Error`
- `Filter::Warn`
- `Filter::Info`
- `Filter::Debug`

Only messages at or above the current `Filter` level will be logged.

---

## 🔁 Graceful Shutdown

Ensure all logs are flushed before exiting:

```rust
log_keeper.join();
```

This blocks until the background worker finishes processing queued log entries.

---

## 🔧 Configuration

Advanced config is available via the internal `config` module (e.g. buffer size, max retries, etc.), though the public API is kept intentionally minimal for now. Feel free to contribute improvements or submit feature requests.

---

## 📚 Related Projects

- [`macro_keeper`](https://crates.io/crates/macro_keeper) — optional macro utilities used internally

---

## 🤝 Contributing

PRs, suggestions, and issues are welcome! Just fork this repo and submit a PR or open an issue.

---

## 📄 License

Apache-2.0 © [EnvKeeper](https://github.com/envkeeper)
