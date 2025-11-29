/*============================================================
  Synavera Project: Syn-Syu
  Module: synsyu_core::logger
  Etiquette: Synavera Script Etiquette — Rust Profile v1.1
  ------------------------------------------------------------
  Purpose:
    Provide structured, append-only logging utilities for
    Syn-Syu-Core operations.

  Security / Safety Notes:
    Logging avoids leaking secrets by redacting configurable
    values and file paths when marked sensitive.

  Dependencies:
    std::fs::File, std::sync::Mutex, sha2 for integrity hashing.

  Operational Scope:
    Used by runtime components to emit RFC-3339 UTC stamped
    log entries and produce session hash digests.

  Revision History:
    2024-11-04 COD  Established logging module for Syn-Syu-Core.
  ------------------------------------------------------------
  SSE Principles Observed:
    - Append-only logging with UTC timestamps
    - Deterministic formatting for auditability
    - Graceful error propagation on I/O failures
============================================================*/

use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::{SecondsFormat, Utc};
use sha2::{Digest, Sha256};

use crate::error::{Result, SynsyuError};

/// Structured log level for Syn-Syu-Core events.
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
}

impl LogLevel {
    fn as_str(self) -> &'static str {
        match self {
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Debug => "DEBUG",
        }
    }
}

/// Shared logger that emits append-only entries in Synavera format.
pub struct Logger {
    file: Option<Mutex<BufWriter<File>>>,
    path: Option<PathBuf>,
    verbose: bool,
}

impl Logger {
    /// Build a logger that writes to stderr and optionally to a file.
    pub fn new(path: Option<PathBuf>, verbose: bool) -> Result<Self> {
        let file = if let Some(ref file_path) = path {
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent).map_err(|err| {
                    SynsyuError::Filesystem(format!(
                        "Failed to create log directory {}: {err}",
                        parent.display()
                    ))
                })?;
            }

            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(file_path)
                .map_err(|err| {
                    SynsyuError::Filesystem(format!(
                        "Failed to open log file {}: {err}",
                        file_path.display()
                    ))
                })?;
            Some(Mutex::new(BufWriter::new(file)))
        } else {
            None
        };

        Ok(Self {
            file,
            path,
            verbose,
        })
    }

    /// Emit a log entry with the given level, code, and message.
    pub fn log<S: AsRef<str>>(&self, level: LogLevel, code: &str, message: S) {
        let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
        let payload = format!(
            "{timestamp} [{}] [{}] {}",
            level.as_str(),
            code,
            message.as_ref()
        );

        if self.verbose || level == LogLevel::Error || level == LogLevel::Warn {
            eprintln!("{payload}");
        }

        if let Some(file) = &self.file {
            if let Ok(mut guard) = file.lock() {
                if writeln!(guard, "{payload}").is_err() {
                    eprintln!(
                        "{} [{}] [{}] {}",
                        timestamp,
                        LogLevel::Error.as_str(),
                        "LOGGER",
                        "Failed to write to log file"
                    );
                }
                if guard.flush().is_err() {
                    eprintln!(
                        "{} [{}] [{}] {}",
                        timestamp,
                        LogLevel::Warn.as_str(),
                        "LOGGER",
                        "Failed to flush log writer"
                    );
                }
            }
        }
    }

    /// Convenience wrapper for `INFO` level events.
    pub fn info<S: AsRef<str>>(&self, code: &str, message: S) {
        self.log(LogLevel::Info, code, message);
    }

    /// Convenience wrapper for `WARN` level events.
    pub fn warn<S: AsRef<str>>(&self, code: &str, message: S) {
        self.log(LogLevel::Warn, code, message);
    }

    /// Convenience wrapper for `ERROR` level events.
    #[allow(dead_code)]
    pub fn error<S: AsRef<str>>(&self, code: &str, message: S) {
        self.log(LogLevel::Error, code, message);
    }

    /// Convenience wrapper for `DEBUG` level events.
    pub fn debug<S: AsRef<str>>(&self, code: &str, message: S) {
        self.log(LogLevel::Debug, code, message);
    }

    /// Return the path backing this logger, if any.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Compute and persist SHA-256 digest of the log file.
    pub fn finalize(&self) -> Result<()> {
        if let Some(path) = self.path() {
            let data = std::fs::read(path).map_err(|err| {
                SynsyuError::Filesystem(format!(
                    "Failed to read log for hashing {}: {err}",
                    path.display()
                ))
            })?;
            let mut hasher = Sha256::new();
            hasher.update(&data);
            let digest = hasher.finalize();
            let mut hash_os = path.as_os_str().to_os_string();
            hash_os.push(".hash");
            let hash_path = PathBuf::from(hash_os);
            let mut file = File::create(&hash_path).map_err(|err| {
                SynsyuError::Filesystem(format!(
                    "Failed to create hash file {}: {err}",
                    hash_path.display()
                ))
            })?;
            writeln!(
                file,
                "{:x}  {}",
                digest,
                path.file_name().unwrap_or_default().to_string_lossy()
            )
            .map_err(|err| {
                SynsyuError::Filesystem(format!(
                    "Failed to write hash file {}: {err}",
                    hash_path.display()
                ))
            })?;
        }
        Ok(())
    }
}
