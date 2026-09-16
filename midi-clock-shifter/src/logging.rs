//! In-memory ring buffer of recent log lines, backing the UI's Log panel
//! (ARCHITECTURE.md §9: "last 200 lines, timestamped").
//!
//! A single global logger is installed once at startup (`main.rs`, before
//! anything else can log) and shared with the UI via the [`LogBuffer`]
//! handle returned by [`init`]. Every thread (UI, engine, port watcher,
//! input callbacks) logs through the ordinary `log` macros; nothing on the
//! engine's hot path does — logging there only ever happens on error
//! conditions, already rare by construction (timing-and-jitter skill).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use log::{Level, Log, Metadata, Record};

/// ARCHITECTURE.md §9: "last 200 lines".
const CAPACITY: usize = 200;

#[derive(Debug, Clone)]
pub struct LogLine {
    /// Time since the logger was installed (i.e. since app startup) — avoids
    /// pulling in a date/time crate just to timestamp a debug panel.
    pub elapsed: Duration,
    pub level: Level,
    pub message: String,
}

/// Fixed-capacity ring buffer of the most recent log lines, shared between
/// the [`log::Log`] backend (which pushes) and the UI (which reads a
/// snapshot). Locking is only ever contended between the UI's ~30 Hz poll
/// and whichever thread just logged something — never the engine's hot path.
#[derive(Default)]
pub struct LogBuffer {
    lines: Mutex<VecDeque<LogLine>>,
}

impl LogBuffer {
    fn push(&self, line: LogLine) {
        let mut lines = self
            .lines
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if lines.len() >= CAPACITY {
            lines.pop_front();
        }
        lines.push_back(line);
    }

    /// A snapshot of the current lines, oldest first.
    pub fn snapshot(&self) -> Vec<LogLine> {
        let lines = self
            .lines
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        lines.iter().cloned().collect()
    }
}

struct RingLogger {
    buffer: Arc<LogBuffer>,
    start: Instant,
}

impl Log for RingLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let elapsed = self.start.elapsed();
        let message = format!("{}", record.args());
        eprintln!(
            "[{:>8.3}s] {:<5} {message}",
            elapsed.as_secs_f64(),
            record.level(),
        );
        self.buffer.push(LogLine {
            elapsed,
            level: record.level(),
            message,
        });
    }

    fn flush(&self) {}
}

/// Installs the global logger and returns the shared buffer for the UI to
/// read. Must be called exactly once, before any other module logs
/// (`main.rs`, first thing).
pub fn init() -> Arc<LogBuffer> {
    let buffer = Arc::new(LogBuffer::default());
    let logger = RingLogger {
        buffer: Arc::clone(&buffer),
        start: Instant::now(),
    };
    if log::set_boxed_logger(Box::new(logger)).is_err() {
        // Only possible if `init` is called twice; keep the first logger and
        // the caller's buffer handle simply won't receive anything.
        log::warn!("logger already initialized, ignoring second init() call");
    }
    log::set_max_level(log::LevelFilter::Info);
    buffer
}
