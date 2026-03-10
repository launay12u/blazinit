#[cfg(not(debug_assertions))]
use std::{
    io::Write,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use log::{Level, LevelFilter, Log, Metadata, Record};

#[cfg(not(debug_assertions))]
fn timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// --- Dev logger: colored stderr -------------------------------------------

#[cfg(debug_assertions)]
struct DevLogger;

#[cfg(debug_assertions)]
impl Log for DevLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let level = match record.level() {
            Level::Error => "\x1b[31mERROR\x1b[0m",
            Level::Warn => "\x1b[33m WARN\x1b[0m",
            Level::Info => "\x1b[32m INFO\x1b[0m",
            Level::Debug => "\x1b[36mDEBUG\x1b[0m",
            Level::Trace => "TRACE",
        };
        eprintln!("[{}] {}", level, record.args());
    }

    fn flush(&self) {}
}

// --- Prod logger: append to file -------------------------------------------

#[cfg(not(debug_assertions))]
struct FileLogger {
    file: Mutex<std::fs::File>,
}

#[cfg(not(debug_assertions))]
impl Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        if let Ok(mut f) = self.file.lock() {
            let _ = writeln!(
                f,
                "[{}][{:5}] {}",
                timestamp_secs(),
                record.level(),
                record.args()
            );
        }
    }

    fn flush(&self) {
        if let Ok(mut f) = self.file.lock() {
            let _ = f.flush();
        }
    }
}

// --- Init ------------------------------------------------------------------

pub fn init_logger() {
    #[cfg(debug_assertions)]
    {
        log::set_boxed_logger(Box::new(DevLogger)).unwrap_or_default();
        let level = std::env::var("RUST_LOG")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(LevelFilter::Warn);
        log::set_max_level(level);
    }

    #[cfg(not(debug_assertions))]
    {
        use std::fs::{self, OpenOptions};
        let log_dir = crate::config::config_dir();
        if fs::create_dir_all(&log_dir).is_ok() {
            let log_path = log_dir.join("blazinit.log");
            if let Ok(file) =
                OpenOptions::new().create(true).append(true).open(&log_path)
            {
                log::set_boxed_logger(Box::new(FileLogger {
                    file: Mutex::new(file),
                }))
                .unwrap_or_default();
                log::set_max_level(LevelFilter::Info);
            }
        }
    }
}
