/*============================================================
  Synavera Project: Syn-Syu
  Module: synsyu_core::package_info
  Etiquette: Synavera Script Etiquette — Rust Profile v1.1.1
  ------------------------------------------------------------
  Purpose:
    Shared structures describing version metadata retrieved
    from pacman and the AUR (including size information).

  Security / Safety Notes:
    Pure data container; no I/O performed in this module.

  Dependencies:
    None beyond std.

  Operational Scope:
    Used across query modules and manifest construction to pass
    version strings and size metrics.

  Revision History:
    2024-11-04 COD  Introduced shared VersionInfo type.
  ------------------------------------------------------------
  SSE Principles Observed:
    - Clear data contracts between modules
    - Serializable structures for manifest output
============================================================*/

use serde::Serialize;

/// Captures version metadata for a package source (repo or AUR).
#[derive(Debug, Clone, Serialize)]
pub struct VersionInfo {
    pub version: String,
    pub download_size: Option<u64>,
    pub installed_size: Option<u64>,
}

impl VersionInfo {
    pub fn new(version: String, download_size: Option<u64>, installed_size: Option<u64>) -> Self {
        Self {
            version,
            download_size,
            installed_size,
        }
    }
}
