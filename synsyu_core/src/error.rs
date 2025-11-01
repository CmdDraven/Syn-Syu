/*============================================================
  Synvera Project: Syn-Syu
  Module: synsyu_core::error
  Etiquette: Synvera Script Etiquette — Rust Profile v1.1.1
  ------------------------------------------------------------
  Purpose:
    Centralise Syn-Syu-Core error types to provide consistent
    diagnostics and exit semantics.

  Security / Safety Notes:
    Error contexts redact potentially sensitive data such as
    credentials or tokens; only high-level paths are exposed.

  Dependencies:
    thiserror for ergonomic error definitions.

  Operational Scope:
    Used across modules to propagate recoverable failures and
    consolidate exit codes for the binary entry point.

  Revision History:
    2024-11-04 COD  Established shared error definitions.
  ------------------------------------------------------------
  SSE Principles Observed:
    - Explicit error taxonomy with actionable context
    - No silent failure paths
    - Stable exit codes for operational tooling
============================================================*/

use std::io;
use std::process::ExitCode;

use thiserror::Error;

/// Result alias for Syn-Syu-Core operations.
pub type Result<T> = std::result::Result<T, SynsyuError>;

/// Enumerates high-level error domains surfaced by Syn-Syu-Core.
#[derive(Debug, Error)]
pub enum SynsyuError {
    #[error("Required command `{command}` not found in PATH")]
    CommandMissing { command: String },
    #[error("Command `{command}` failed with status {status}: {stderr}")]
    CommandFailure {
        command: String,
        status: i32,
        stderr: String,
    },
    #[error("Configuration: {0}")]
    Config(String),
    #[error("Network: {0}")]
    Network(String),
    #[error("Serialization: {0}")]
    Serialization(String),
    #[error("Filesystem: {0}")]
    Filesystem(String),
    #[error("Runtime: {0}")]
    Runtime(String),
    #[error(transparent)]
    Io(#[from] io::Error),
}

impl SynsyuError {
    /// Map error category to a deterministic exit code.
    pub fn exit_code(&self) -> ExitCode {
        match self {
            SynsyuError::CommandMissing { .. } => ExitCode::from(10),
            SynsyuError::CommandFailure { .. } => ExitCode::from(11),
            SynsyuError::Config(_) => ExitCode::from(20),
            SynsyuError::Network(_) => ExitCode::from(30),
            SynsyuError::Serialization(_) => ExitCode::from(31),
            SynsyuError::Filesystem(_) => ExitCode::from(40),
            SynsyuError::Runtime(_) => ExitCode::from(50),
            SynsyuError::Io(_) => ExitCode::from(41),
        }
    }
}
