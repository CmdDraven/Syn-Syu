/*============================================================
  Synavera Project: Syn-Syu
  Module: synsyu_core::aur
  Etiquette: Synavera Script Etiquette — Rust Profile v1.1
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
use std::sync::Arc;
use std::time::Duration;

use reqwest::header::CONTENT_LENGTH;
use reqwest::StatusCode;
use serde::Deserialize;
use tokio::sync::Semaphore;
use tokio::time::sleep;
use urlencoding::encode;

use crate::config::AurConfig;
use crate::error::{Result, SynsyuError};
use crate::package_info::VersionInfo;

/// Client for interacting with the AUR RPC API.
#[derive(Clone)]
pub struct AurClient {
    client: reqwest::Client,
    base_url: String,
    max_args: usize,
    max_retries: usize,
    max_parallel_requests: usize,
    max_kib_per_sec: u64,
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
            max_parallel_requests: config.max_parallel_requests.max(1),
            max_kib_per_sec: config.max_kib_per_sec,
        })
    }

    /// Fetch version information for the provided packages.
    pub async fn fetch_versions(
        &self,
        packages: &[String],
    ) -> Result<HashMap<String, VersionInfo>> {
        let mut versions = HashMap::new();
        if packages.is_empty() {
            return Ok(versions);
        }

        let chunks: Vec<Vec<String>> = packages
            .chunks(self.max_args)
            .map(|chunk| chunk.to_vec())
            .collect();
        let semaphore = Arc::new(Semaphore::new(self.max_parallel_requests));
        let mut tasks = Vec::new();

        for chunk in chunks {
            let client = self.clone();
            let semaphore = semaphore.clone();
            tasks.push(tokio::spawn(async move {
                let _permit = semaphore
                    .acquire_owned()
                    .await
                    .map_err(|_| SynsyuError::Runtime("AUR semaphore closed".into()))?;
                client.fetch_chunk(chunk).await
            }));
        }

        for task in tasks {
            let chunk_result = task
                .await
                .map_err(|err| SynsyuError::Runtime(format!("AUR task failed: {err}")))?;
            let chunk_map = chunk_result?;
            versions.extend(chunk_map);
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

    async fn fetch_chunk(&self, chunk: Vec<String>) -> Result<HashMap<String, VersionInfo>> {
        let mut attempt = 0;
        let url = self.compose_url(&chunk);
        loop {
            let response = self.client.get(&url).send().await.map_err(|err| {
                SynsyuError::Network(format!("AUR request to {url} failed: {err}"))
            })?;
            let content_len = response.content_length();

            if response.status() == StatusCode::OK {
                let payload = response.json::<AurResponse>().await.map_err(|err| {
                    SynsyuError::Serialization(format!("Failed to decode AUR response: {err}"))
                })?;

                if let Some(error) = payload.error {
                    return Err(SynsyuError::Network(format!(
                        "AUR responded with error for {url}: {error}"
                    )));
                }

                self.enforce_rate_limit(content_len).await;

                let mut results = HashMap::new();
                for entry in payload.results.into_iter() {
                    let download_size = match (entry.compressed_size, entry.url_path.as_deref()) {
                        (Some(size), _) => Some(size),
                        (None, Some(path)) => self.fetch_tarball_size(path).await,
                        (None, None) => None,
                    };
                    let installed_size = entry.installed_size;
                    results.insert(
                        entry.name,
                        VersionInfo::new(entry.version, download_size, installed_size),
                    );
                }
                return Ok(results);
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

    fn aur_base_url(&self) -> String {
        // Trim trailing /rpc to derive the host root for tarball fetches.
        let mut base = self.base_url.trim_end_matches('/').to_string();
        if let Some(idx) = base.rfind("/rpc") {
            base.truncate(idx);
        }
        base
    }

    async fn fetch_tarball_size(&self, path: &str) -> Option<u64> {
        let url = if path.starts_with("http://") || path.starts_with("https://") {
            path.to_string()
        } else {
            format!("{}{}", self.aur_base_url(), path)
        };
        let response = self.client.head(url).send().await.ok()?;
        let content_length = response.content_length();
        if !response.status().is_success() {
            return None;
        }
        let header_size = response
            .headers()
            .get(CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
        self.enforce_rate_limit(content_length.or(header_size))
            .await;
        header_size
    }

    async fn enforce_rate_limit(&self, content_length: Option<u64>) {
        if self.max_kib_per_sec == 0 {
            return;
        }
        if let Some(bytes) = content_length {
            if let Some(delay) = throttle_delay(bytes, self.max_kib_per_sec) {
                sleep(delay).await;
            }
        }
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
    #[serde(rename = "URLPath")]
    pub url_path: Option<String>,
    #[serde(rename = "CompressedSize")]
    pub compressed_size: Option<u64>,
    #[serde(rename = "InstalledSize")]
    pub installed_size: Option<u64>,
}

fn throttle_delay(bytes: u64, kib_per_sec: u64) -> Option<Duration> {
    if kib_per_sec == 0 {
        return None;
    }
    let denominator = kib_per_sec.saturating_mul(1024);
    if denominator == 0 {
        return None;
    }
    // Ceil division to avoid exceeding the requested rate.
    let millis = bytes.saturating_mul(1000).saturating_add(denominator - 1) / denominator;
    if millis == 0 {
        None
    } else {
        Some(Duration::from_millis(millis))
    }
}

/// Placeholder for future expansion (e.g., changelog retrieval).
#[allow(dead_code)]
pub async fn fetch_future_metadata(_packages: &[String]) -> Result<()> {
    // Future hook: integrate changelog or plugin metadata.
    Ok(())
}
