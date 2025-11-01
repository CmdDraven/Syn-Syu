/*============================================================
  Synavera Project: Syn-Syu
  Module: synsyu_core::pacman
  Etiquette: Synavera Script Etiquette — Rust Profile v1.1.1
  ------------------------------------------------------------
  Purpose:
    Interface with pacman utilities to enumerate installed
    packages, query repository metadata, and compare versions.

  Security / Safety Notes:
    Executes pacman/vercmp binaries with user privileges only;
    no privilege escalation is attempted.

  Dependencies:
    tokio::process for async command execution.

  Operational Scope:
    Supplies Syn-Syu-Core with local inventory data and version
    comparisons against repo sources.

  Revision History:
    2024-11-04 COD  Crafted pacman integration layer.
  ------------------------------------------------------------
  SSE Principles Observed:
    - Deterministic command invocation with explicit checks
    - Structured parsing with clear failure modes
    - Reusable helpers for external command diagnostics
============================================================*/

use std::collections::HashMap;
use std::io;
use std::process::Stdio;
use std::str::FromStr;

use tokio::process::Command;

use crate::error::{Result, SynsyuError};
use crate::package_info::VersionInfo;

/// Represents a package currently installed on the system.
#[derive(Debug, Clone)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
    pub repository: Option<String>,
}

/// Enumerate all installed packages via `pacman -Qi`.
pub async fn enumerate_installed_packages() -> Result<Vec<InstalledPackage>> {
    let output = Command::new("pacman")
        .arg("-Qi")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|err| map_spawn_error(err, "pacman"))?;

    if !output.status.success() {
        return Err(SynsyuError::CommandFailure {
            command: "pacman -Qi".into(),
            status: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    let stdout = String::from_utf8(output.stdout).map_err(|err| {
        SynsyuError::Serialization(format!("pacman -Qi emitted invalid UTF-8: {err}"))
    })?;

    let mut packages = Vec::new();
    for block in stdout.split("\n\n") {
        let mut name: Option<String> = None;
        let mut version: Option<String> = None;
        let mut repository: Option<String> = None;

        for line in block.lines() {
            if let Some((raw_key, raw_value)) = line.split_once(':') {
                let key = raw_key.trim();
                let value = raw_value.trim();
                match key {
                    "Name" => name = Some(value.to_string()),
                    "Version" => version = Some(value.to_string()),
                    "Repository" => repository = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        if let (Some(name), Some(version)) = (name, version) {
            packages.push(InstalledPackage {
                name,
                version,
                repository,
            });
        }
    }

    packages.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(packages)
}

/// Retrieve remote repository versions for the specified packages via `pacman -Si`.
pub async fn query_repo_versions(packages: &[String]) -> Result<HashMap<String, VersionInfo>> {
    let mut versions = HashMap::new();
    if packages.is_empty() {
        return Ok(versions);
    }

    const CHUNK_SIZE: usize = 64;
    for chunk in packages.chunks(CHUNK_SIZE) {
        let output = Command::new("pacman")
            .arg("-Si")
            .args(chunk)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|err| map_spawn_error(err, "pacman"))?;

        if !output.status.success() {
            return Err(SynsyuError::CommandFailure {
                command: format!("pacman -Si {}", chunk.join(" ")),
                status: output.status.code().unwrap_or(-1),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }

        let stdout = String::from_utf8(output.stdout).map_err(|err| {
            SynsyuError::Serialization(format!("pacman -Si emitted invalid UTF-8: {err}"))
        })?;

        let mut current: Option<String> = None;
        let mut current_version: Option<String> = None;
        let mut download_size: Option<u64> = None;
        let mut installed_size: Option<u64> = None;
        for line in stdout.lines() {
            if let Some((raw_key, raw_value)) = line.split_once(':') {
                let key = raw_key.trim();
                let value = raw_value.trim();
                match key {
                    "Name" => {
                        current = Some(value.to_string());
                        current_version = None;
                        download_size = None;
                        installed_size = None;
                    }
                    "Version" => {
                        current_version = Some(value.to_string());
                    }
                    "Download Size" => {
                        download_size = parse_pacman_size(value);
                    }
                    "Installed Size" => {
                        installed_size = parse_pacman_size(value);
                    }
                    _ => {}
                }
            } else if line.trim().is_empty() {
                if let (Some(name), Some(ver)) = (current.take(), current_version.take()) {
                    versions.insert(name, VersionInfo::new(ver, download_size, installed_size));
                }
                download_size = None;
                installed_size = None;
            }
        }
        if let (Some(name), Some(ver)) = (current.take(), current_version.take()) {
            versions.insert(name, VersionInfo::new(ver, download_size, installed_size));
        }
    }

    Ok(versions)
}

/// Compare two package versions using `vercmp`.
pub async fn compare_versions(local: &str, remote: &str) -> Result<std::cmp::Ordering> {
    let output = Command::new("vercmp")
        .arg(local)
        .arg(remote)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|err| map_spawn_error(err, "vercmp"))?;

    if !output.status.success() {
        return Err(SynsyuError::CommandFailure {
            command: format!("vercmp {local} {remote}"),
            status: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    let stdout = String::from_utf8(output.stdout).map_err(|err| {
        SynsyuError::Serialization(format!("vercmp emitted invalid UTF-8: {err}"))
    })?;
    let verdict = stdout.trim();
    let ordering = i32::from_str(verdict).map_err(|err| {
        SynsyuError::Serialization(format!("Failed to parse vercmp output `{verdict}`: {err}"))
    })?;

    Ok(ordering.cmp(&0))
}

fn parse_pacman_size(value: &str) -> Option<u64> {
    let mut parts = value.trim().split_whitespace();
    let number = parts.next()?.replace(',', "");
    let unit = parts.next().unwrap_or("B");
    let magnitude = number.parse::<f64>().ok()?;
    let multiplier = match unit {
        "B" => 1_f64,
        "KiB" => 1024_f64,
        "MiB" => 1024_f64.powi(2),
        "GiB" => 1024_f64.powi(3),
        "TiB" => 1024_f64.powi(4),
        _ => 1_f64,
    };
    let bytes = magnitude * multiplier;
    if bytes.is_finite() && bytes >= 0.0 {
        Some(bytes.round() as u64)
    } else {
        None
    }
}

fn map_spawn_error(err: io::Error, command: &str) -> SynsyuError {
    if err.kind() == io::ErrorKind::NotFound {
        SynsyuError::CommandMissing {
            command: command.into(),
        }
    } else {
        SynsyuError::Runtime(format!("Failed to spawn {command}: {err}"))
    }
}
