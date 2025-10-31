/*============================================================
  Synavera Project: Syn-Syu
  Module: synsyu_core::main
  Etiquette: Synavera Script Etiquette — Rust Profile v1.1.1
  ------------------------------------------------------------
  Purpose:
    Entry point for Syn-Syu Core. Enumerates installed packages,
    queries upstream sources, and emits a structured manifest
    describing update candidates for the Syn-Syu orchestrator.

  Security / Safety Notes:
    Operates within user privileges. Executes pacman/vercmp
    commands and performs HTTPS GET requests only.

  Dependencies:
    clap for CLI parsing, chrono for timestamps.

  Operational Scope:
    Invoked by the Syn-Syu Bash layer via `syn-syu core` or when
    operators require standalone manifest regeneration.

  Revision History:
    2025-10-28 COD  Authored Syn-Syu Core runtime.
  ------------------------------------------------------------
  SSE Principles Observed:
    - Result-first error handling with deterministic exits
    - Structured logging following Synavera cadence
    - Configurable execution via CLI and config file
============================================================*/

mod aur;
mod config;
mod error;
mod future;
mod logger;
mod manifest;
mod package_info;
mod pacman;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::ExitCode;

use chrono::Utc;
use clap::{ArgAction, Parser};

use aur::AurClient;
use config::SynsyuConfig;
use error::{Result, SynsyuError};
use logger::Logger;
use manifest::{build_manifest, write_manifest, ManifestDocument};
use package_info::VersionInfo;
use pacman::{enumerate_installed_packages, query_repo_versions, InstalledPackage};

/// Command-line arguments for Syn-Syu-Core.
#[derive(Debug, Parser)]
#[command(
    name = "Syn-Syu-Core",
    version,
    author = "Synavera Systems",
    about = "Conscious manifest builder for Syn-Syu"
)]
struct Cli {
    /// Override configuration file path.
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,
    /// Override manifest output path.
    #[arg(long, value_name = "PATH")]
    manifest: Option<PathBuf>,
    /// Explicit log file path.
    #[arg(long, value_name = "PATH")]
    log: Option<PathBuf>,
    /// Limit manifest to specific packages.
    #[arg(long = "package", value_name = "PKG", action = ArgAction::Append)]
    packages: Vec<String>,
    /// Skip AUR lookups.
    #[arg(long, action = ArgAction::SetTrue)]
    no_aur: bool,
    /// Skip repository lookups.
    #[arg(long, action = ArgAction::SetTrue)]
    no_repo: bool,
    /// Do not write manifest; emit summary only.
    #[arg(long, action = ArgAction::SetTrue)]
    dry_run: bool,
    /// Enable verbose logging to stderr.
    #[arg(long, action = ArgAction::SetTrue)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(code) => code,
        Err(err) => {
            eprintln!("[Syn-Syu-Core] {}", err);
            err.exit_code()
        }
    }
}

async fn run() -> Result<ExitCode> {
    let cli = Cli::parse();

    if cli.no_aur && cli.no_repo {
        return Err(SynsyuError::Config(
            "Cannot disable both repo and AUR resolution".into(),
        ));
    }

    let config_path = cli.config.as_deref();
    let config = SynsyuConfig::load_from_optional_path(config_path)?;

    let manifest_path = cli
        .manifest
        .clone()
        .unwrap_or_else(|| config.manifest_path());

    let session_stamp = Utc::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let log_path = cli
        .log
        .clone()
        .or_else(|| Some(config.log_dir().join(format!("core_{session_stamp}.log"))));
    let logger = Logger::new(log_path.clone(), cli.verbose)?;
    logger.info("INIT", "Syn-Syu Core awakening.");

    let mut installed = enumerate_installed_packages().await?;
    logger.info(
        "PACKAGES",
        format!("Detected {} installed packages", installed.len()),
    );

    let selected = filter_packages(&mut installed, &cli.packages, &logger)?;
    if selected.is_empty() {
        logger.warn(
            "EMPTY",
            "No packages selected for manifest generation; exiting",
        );
        logger.finalize()?;
        return Ok(ExitCode::SUCCESS);
    }

    let repo_versions: HashMap<String, VersionInfo> = if cli.no_repo {
        HashMap::new()
    } else {
        let repo_candidates: Vec<String> = selected
            .iter()
            .filter(|pkg| {
                pkg.repository
                    .as_deref()
                    .map(|r| r != "local")
                    .unwrap_or(false)
            })
            .map(|pkg| pkg.name.clone())
            .collect();
        if repo_candidates.is_empty() {
            HashMap::new()
        } else {
            query_repo_versions(&repo_candidates).await?
        }
    };

    let aur_versions: HashMap<String, VersionInfo> = if cli.no_aur {
        HashMap::new()
    } else {
        let aur_candidates: Vec<String> = selected
            .iter()
            .filter(|pkg| repo_versions.get(&pkg.name).is_none())
            .map(|pkg| pkg.name.clone())
            .collect();
        if aur_candidates.is_empty() {
            HashMap::new()
        } else {
            let aur_client = AurClient::new(&config.aur)?;
            aur_client.fetch_versions(&aur_candidates).await?
        }
    };

    logger.info(
        "SOURCES",
        format!(
            "Repo candidates={} AUR candidates={}",
            repo_versions.len(),
            aur_versions.len()
        ),
    );

    let document = build_manifest(&selected, &repo_versions, &aur_versions, &logger).await?;

    if cli.dry_run {
        print_summary(&document);
    } else {
        write_manifest(&document, &manifest_path)?;
        logger.info(
            "MANIFEST",
            format!("Manifest written to {}", manifest_path.display()),
        );
    }

    logger.info(
        "SUMMARY",
        format!(
            "packages={} updates={}",
            document.metadata.total_packages, document.metadata.updates_available
        ),
    );
    logger.info("COMPLETE", "Consciousness synchronised.");
    logger.finalize()?;

    Ok(ExitCode::SUCCESS)
}

fn filter_packages(
    installed: &mut Vec<InstalledPackage>,
    requested: &[String],
    logger: &Logger,
) -> Result<Vec<InstalledPackage>> {
    installed.sort_by(|a, b| a.name.cmp(&b.name));

    if requested.is_empty() {
        return Ok(installed.clone());
    }

    let mut requested_set: HashSet<String> = HashSet::new();
    for pkg in requested {
        requested_set.insert(pkg.to_string());
    }

    let mut selected = Vec::new();
    for pkg in installed.iter() {
        if requested_set.contains(&pkg.name) {
            selected.push(pkg.clone());
        }
    }

    let missing: Vec<String> = requested_set
        .into_iter()
        .filter(|name| !selected.iter().any(|pkg| &pkg.name == name))
        .collect();

    if !missing.is_empty() {
        logger.warn(
            "PKG404",
            format!("Requested packages not installed: {}", missing.join(", ")),
        );
    }

    Ok(selected)
}

fn print_summary(document: &ManifestDocument) {
    println!(
        "→ Manifest dry-run. Packages={} Updates={} (Repo candidates={} AUR candidates={})",
        document.metadata.total_packages,
        document.metadata.updates_available,
        document.metadata.repo_candidates,
        document.metadata.aur_candidates
    );
}
