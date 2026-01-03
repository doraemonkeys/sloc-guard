use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};

use sha2::{Digest, Sha256};

use crate::error::{Result, SlocGuardError};
use crate::state::detect_state_dir;

const REMOTE_CACHE_SUBDIR: &str = "remote-configs";
const CACHE_TTL_SECS: u64 = 3600; // 1 hour
const REQUEST_TIMEOUT_SECS: u64 = 30;

/// Policy for fetching remote configurations.
///
/// Controls how the cache is used when resolving remote config URLs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FetchPolicy {
    /// Normal mode: use cache if valid (within TTL), otherwise fetch.
    /// This is the default behavior.
    #[default]
    Normal,
    /// Offline mode: ignore TTL, use any existing cached content.
    /// Errors if cache is missing. Does not make network requests.
    Offline,
    /// Force refresh: skip cache entirely, always fetch from network.
    /// Updates cache with fresh content after successful fetch.
    ForceRefresh,
}

impl FetchPolicy {
    /// Create a `FetchPolicy` from the CLI `ExtendsPolicy` enum.
    #[must_use]
    pub const fn from_cli(cli_policy: crate::cli::ExtendsPolicy) -> Self {
        match cli_policy {
            crate::cli::ExtendsPolicy::Normal => Self::Normal,
            crate::cli::ExtendsPolicy::Offline => Self::Offline,
            crate::cli::ExtendsPolicy::Refresh => Self::ForceRefresh,
        }
    }
}

/// Flag to track whether the remote config fetch warning has been shown this session.
static WARNING_SHOWN: AtomicBool = AtomicBool::new(false);

/// HTTP client abstraction for dependency injection.
pub trait HttpClient {
    /// Perform a GET request and return the response body.
    fn get(&self, url: &str) -> Result<String>;
}

/// Production HTTP client using reqwest.
///
/// This implementation cannot be unit tested without a real HTTP server,
/// so it is excluded from coverage measurement.
#[derive(Debug, Default)]
pub struct ReqwestClient;

#[cfg(not(tarpaulin_include))]
impl HttpClient for ReqwestClient {
    fn get(&self, url: &str) -> Result<String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .map_err(|e| SlocGuardError::Config(format!("Failed to create HTTP client: {e}")))?;

        let response = client.get(url).send().map_err(|e| {
            if e.is_timeout() {
                SlocGuardError::Config(format!("Request timeout fetching remote config: {url}"))
            } else if e.is_connect() {
                SlocGuardError::Config(format!("Failed to connect to remote config URL: {url}"))
            } else {
                SlocGuardError::Config(format!("Failed to fetch remote config from {url}: {e}"))
            }
        })?;

        let status = response.status();
        if !status.is_success() {
            return Err(SlocGuardError::Config(format!(
                "Failed to fetch remote config from {url}: HTTP {status}"
            )));
        }

        response
            .text()
            .map_err(|e| SlocGuardError::Config(format!("Failed to read response from {url}: {e}")))
    }
}

/// Check if a string is a valid remote URL (http:// or https://).
#[must_use]
pub fn is_remote_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

/// Compute SHA-256 hash of URL for cache filename.
fn hash_url(url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Compute SHA-256 hash of content for integrity verification.
#[must_use]
pub fn compute_content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Verify that content hash matches expected value.
fn verify_content_hash(content: &str, expected: &str, url: &str) -> Result<()> {
    let actual = compute_content_hash(content);
    if actual != expected {
        return Err(SlocGuardError::RemoteConfigHashMismatch {
            url: url.to_string(),
            expected: expected.to_string(),
            actual,
        });
    }
    Ok(())
}

/// Get the cache directory path.
///
/// Uses the state directory (`.git/sloc-guard/remote-configs/` or `.sloc-guard/remote-configs/`).
fn cache_dir(project_root: &Path) -> PathBuf {
    detect_state_dir(project_root).join(REMOTE_CACHE_SUBDIR)
}

/// Get the cache file path for a given URL.
fn cache_file_path(url: &str, project_root: &Path) -> PathBuf {
    cache_dir(project_root).join(format!("{}.toml", hash_url(url)))
}

/// Check if cache file exists.
fn cache_exists(cache_path: &Path) -> bool {
    cache_path.exists()
}

/// Check if cache file is within TTL.
fn is_cache_within_ttl(cache_path: &Path) -> bool {
    if !cache_path.exists() {
        return false;
    }

    let Ok(metadata) = fs::metadata(cache_path) else {
        return false;
    };

    let Ok(modified) = metadata.modified() else {
        return false;
    };

    let Ok(elapsed) = SystemTime::now().duration_since(modified) else {
        return false;
    };

    elapsed.as_secs() < CACHE_TTL_SECS
}

/// Try to read config from cache based on fetch policy.
///
/// - `Normal`: only read if within TTL
/// - `Offline`: read any existing cache (ignores TTL)
/// - `ForceRefresh`: never read from cache
fn read_from_cache(url: &str, project_root: Option<&Path>, policy: FetchPolicy) -> Option<String> {
    let root = project_root?;
    let cache_path = cache_file_path(url, root);

    match policy {
        FetchPolicy::ForceRefresh => None,
        FetchPolicy::Offline => {
            // Offline: use any existing cache, ignore TTL
            if cache_exists(&cache_path) {
                fs::read_to_string(&cache_path).ok()
            } else {
                None
            }
        }
        FetchPolicy::Normal => {
            // Normal: only use cache if within TTL
            if is_cache_within_ttl(&cache_path) {
                fs::read_to_string(&cache_path).ok()
            } else {
                None
            }
        }
    }
}

/// Write config to cache.
fn write_to_cache(url: &str, content: &str, project_root: Option<&Path>) -> Option<()> {
    let root = project_root?;
    let cache_path = cache_file_path(url, root);

    // Create cache directory if it doesn't exist
    if let Some(parent) = cache_path.parent() {
        fs::create_dir_all(parent).ok()?;
    }

    // Write content to cache file
    let mut file = fs::File::create(&cache_path).ok()?;
    file.write_all(content.as_bytes()).ok()?;

    Some(())
}

/// Fetch remote configuration from URL using the provided HTTP client.
///
/// This function:
/// 1. Checks the local cache based on `policy`:
///    - `Normal`: use cache if within TTL
///    - `Offline`: use any existing cache (ignores TTL)
///    - `ForceRefresh`: skip cache entirely
/// 2. If `expected_hash` is provided, verifies content matches
/// 3. If cache miss (or `ForceRefresh`), fetches from remote (unless `Offline`)
/// 4. Caches the result for future use (only after verification passes)
///
/// # Errors
///
/// Returns an error if:
/// - The URL is invalid
/// - The request times out (30 seconds)
/// - The server returns a non-2xx status code
/// - Network errors occur
/// - Hash verification fails (if `expected_hash` is provided)
/// - Offline mode with cache miss
pub fn fetch_remote_config_with_client(
    url: &str,
    client: &impl HttpClient,
    project_root: Option<&Path>,
    expected_hash: Option<&str>,
    policy: FetchPolicy,
) -> Result<String> {
    // Validate URL
    if !is_remote_url(url) {
        return Err(SlocGuardError::Config(format!(
            "Invalid remote config URL (must start with http:// or https://): {url}"
        )));
    }

    // Try cache based on policy
    if let Some(cached) = read_from_cache(url, project_root, policy) {
        // If hash is specified, verify cached content matches
        if let Some(hash) = expected_hash {
            let actual = compute_content_hash(&cached);
            if actual == hash {
                return Ok(cached);
            }
            // Hash mismatch - cache is stale
            // In offline mode, this is an error (can't fetch fresh content)
            if policy == FetchPolicy::Offline {
                return Err(SlocGuardError::RemoteConfigHashMismatch {
                    url: url.to_string(),
                    expected: hash.to_string(),
                    actual,
                });
            }
            // Fall through to fetch fresh content
        } else {
            // No hash specified - use cache as-is
            return Ok(cached);
        }
    } else if policy == FetchPolicy::Offline {
        // Offline mode with cache miss
        return Err(SlocGuardError::Config(format!(
            "Remote config cache miss in offline mode. Run without --extends-policy=offline first to cache the config: {url}"
        )));
    }

    // Emit warning on first remote fetch per session
    if !WARNING_SHOWN.swap(true, Ordering::SeqCst) {
        crate::output::print_warning_full(
            &format!("Fetching remote config from {url}"),
            None,
            Some(
                "Consider using --extends-policy=offline or extends_sha256 for reproducible builds",
            ),
        );
    }

    // Fetch from remote
    let content = client.get(url)?;

    // Verify hash BEFORE caching (don't cache bad content)
    if let Some(hash) = expected_hash {
        verify_content_hash(&content, hash, url)?;
    }

    // Cache the result (ignore cache write errors)
    let _ = write_to_cache(url, &content, project_root);

    Ok(content)
}

/// Fetch remote configuration from URL using the default HTTP client.
///
/// Convenience wrapper around [`fetch_remote_config_with_client`] using [`ReqwestClient`].
///
/// # Errors
///
/// Returns an error if:
/// - The URL is invalid (must start with `http://` or `https://`)
/// - The request times out
/// - The server returns a non-2xx status code
/// - Network errors occur
/// - Hash verification fails (if `expected_hash` is provided)
/// - Offline mode with cache miss
pub fn fetch_remote_config(
    url: &str,
    project_root: Option<&Path>,
    expected_hash: Option<&str>,
    policy: FetchPolicy,
) -> Result<String> {
    fetch_remote_config_with_client(url, &ReqwestClient, project_root, expected_hash, policy)
}

/// Clear the remote config cache.
///
/// Returns the number of files deleted.
#[must_use]
pub fn clear_cache(project_root: Option<&Path>) -> usize {
    let Some(root) = project_root else {
        return 0;
    };
    let dir = cache_dir(root);

    if !dir.exists() {
        return 0;
    }

    let mut count = 0;
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "toml") && fs::remove_file(&path).is_ok() {
                count += 1;
            }
        }
    }
    count
}

/// Reset the warning flag for testing purposes.
#[cfg(test)]
pub fn reset_warning_flag() {
    WARNING_SHOWN.store(false, Ordering::SeqCst);
}

/// Check if the warning has been shown this session.
#[cfg(test)]
pub fn was_warning_shown() -> bool {
    WARNING_SHOWN.load(Ordering::SeqCst)
}

#[cfg(test)]
#[path = "remote_tests/mod.rs"]
mod tests;
