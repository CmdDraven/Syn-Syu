/*============================================================
  Synavera Project: Syn-Syu
  Module: synsyu_core::aur
  Etiquette: Synavera Script Etiquette — Rust Profile v1.1.1
  ------------------------------------------------------------
  Purpose:
    Query the Arch User Repository RPC API to gather version
    metadata for installed packages.

  Security / Safety Notes:
    Performs read-only HTTPS requests to the public AUR API.
    No credentials are transmitted.

  Dependencies:
    reqwest for HTTP, serde for response parsing.

  Operational Scope:
    Supplies candidate versions for packages absent from the
    official repositories.

  Revision History:
    2024-11-04 COD  Implemented asynchronous AUR client.
  ------------------------------------------------------------
  SSE Principles Observed:
    - Defensive retry logic with exponential backoff
    - Structured response parsing with explicit error paths
    - Configurable timeouts and batching
============================================================*/

use std::collections::HashMap;
use std::time::Duration;

use reqwest::StatusCode;
use serde::Deserialize;
use tokio::time::sleep;
use urlencoding::encode;

use crate::config::AurConfig;
use crate::error::{Result, SynsyuError};
use crate::package_info::VersionInfo;

/// Client for interacting with the AUR RPC API.
pub struct AurClient {
    client: reqwest::Client,
    base_url: String,
    max_args: usize,
    max_retries: usize,
}

impl AurClient {
    /// Construct a new client from configuration.
    pub fn new(config: &AurConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout))
            .user_agent("Syn-Syu-Core/0.13 (linux)")
            .build()
            .map_err(|err| SynsyuError::Network(format!("Failed to build HTTP client: {err}")))?;

        Ok(Self {
            client,
            base_url: config.base_url.trim_end_matches('/').to_string(),
            max_args: config.max_args.max(1),
            max_retries: config.max_retries.max(1),
        })
    }

    /// Fetch version information for the provided packages.
    pub async fn fetch_versions(
        &self,
        packages: &[String],
    ) -> Result<HashMap<String, VersionInfo>> {
        let mut versions = HashMap::new();

        for chunk in packages.chunks(self.max_args) {
            let url = self.compose_url(chunk);
            let mut attempt = 0;
            loop {
                let response = self.client.get(&url).send().await.map_err(|err| {
                    SynsyuError::Network(format!("AUR request to {url} failed: {err}"))
                })?;

                if response.status() == StatusCode::OK {
                    let payload = response.json::<AurResponse>().await.map_err(|err| {
                        SynsyuError::Serialization(format!("Failed to decode AUR response: {err}"))
                    })?;

                    if let Some(error) = payload.error {
                        return Err(SynsyuError::Network(format!(
                            "AUR responded with error for {url}: {error}"
                        )));
                    }

                    for entry in payload.results.into_iter() {
                        let download_size = entry.compressed_size;
                        let installed_size = entry.installed_size;
                        versions.insert(
                            entry.name,
                            VersionInfo::new(entry.version, download_size, installed_size),
                        );
                    }
                    break;
                } else {
                    attempt += 1;
                    if attempt >= self.max_retries {
                        return Err(SynsyuError::Network(format!(
                            "AUR request {url} failed with status {} after {attempt} retries",
                            response.status()
                        )));
                    }
                    let exponent = (attempt as u32).min(8);
                    let backoff = Duration::from_millis(200_u64.saturating_mul(1_u64 << exponent));
                    sleep(backoff).await;
                }
            }
        }

        Ok(versions)
    }

    fn compose_url(&self, packages: &[String]) -> String {
        let mut url = format!("{}?v=5&type=info", self.base_url);
        for pkg in packages {
            url.push_str("&arg[]=");
            url.push_str(&encode(pkg));
        }
        url
    }
}

#[derive(Debug, Deserialize)]
struct AurResponse {
    #[serde(rename = "resultcount")]
    #[allow(dead_code)]
    pub result_count: Option<u32>,
    #[serde(default)]
    pub results: Vec<AurEntry>,
    #[serde(rename = "error")]
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AurEntry {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Version")]
    pub version: String,
    #[serde(rename = "CompressedSize")]
    pub compressed_size: Option<u64>,
    #[serde(rename = "InstalledSize")]
    pub installed_size: Option<u64>,
}

/// Placeholder for future expansion (e.g., changelog retrieval).
#[allow(dead_code)]
pub async fn fetch_future_metadata(_packages: &[String]) -> Result<()> {
    // Future hook: integrate changelog or plugin metadata.
    Ok(())
}
