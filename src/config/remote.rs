use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};

use sha2::{Digest, Sha256};

use crate::error::{Result, SlocGuardError};

const LOCAL_CACHE_DIR: &str = ".sloc-guard/remote-cache";
const CACHE_TTL_SECS: u64 = 3600; // 1 hour
const REQUEST_TIMEOUT_SECS: u64 = 30;

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

/// Get the cache directory path (project-local).
fn cache_dir(project_root: &Path) -> PathBuf {
    project_root.join(LOCAL_CACHE_DIR)
}

/// Get the cache file path for a given URL.
fn cache_file_path(url: &str, project_root: &Path) -> PathBuf {
    cache_dir(project_root).join(format!("{}.toml", hash_url(url)))
}

/// Check if cache file is still valid (within TTL).
fn is_cache_valid(cache_path: &PathBuf) -> bool {
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

/// Try to read config from cache.
fn read_from_cache(url: &str, project_root: Option<&Path>) -> Option<String> {
    let root = project_root?;
    let cache_path = cache_file_path(url, root);
    if is_cache_valid(&cache_path) {
        fs::read_to_string(&cache_path).ok()
    } else {
        None
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
/// 1. Checks the local cache first (1 hour TTL)
/// 2. If `expected_hash` is provided, verifies cached content matches
/// 3. If cache miss or hash mismatch, fetches from remote
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
pub fn fetch_remote_config_with_client(
    url: &str,
    client: &impl HttpClient,
    project_root: Option<&Path>,
    expected_hash: Option<&str>,
) -> Result<String> {
    // Validate URL
    if !is_remote_url(url) {
        return Err(SlocGuardError::Config(format!(
            "Invalid remote config URL (must start with http:// or https://): {url}"
        )));
    }

    // Try cache first
    if let Some(cached) = read_from_cache(url, project_root) {
        // If hash is specified, verify cached content matches
        if let Some(hash) = expected_hash {
            if compute_content_hash(&cached) == hash {
                return Ok(cached);
            }
            // Hash mismatch - cache is stale, fall through to fetch fresh content
        } else {
            // No hash specified - use cache as-is
            return Ok(cached);
        }
    }

    // Emit warning on first remote fetch per session
    if !WARNING_SHOWN.swap(true, Ordering::SeqCst) {
        eprintln!(
            "Warning: Fetching remote config from {url}. Consider using --offline or extends_sha256 for reproducible builds."
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
pub fn fetch_remote_config(
    url: &str,
    project_root: Option<&Path>,
    expected_hash: Option<&str>,
) -> Result<String> {
    fetch_remote_config_with_client(url, &ReqwestClient, project_root, expected_hash)
}

/// Fetch remote configuration from cache only (offline mode).
///
/// Returns cached content if available and valid (within TTL).
/// Does NOT fetch from the network.
///
/// # Errors
///
/// Returns an error if:
/// - The URL is invalid
/// - The cache is missing or expired
/// - Hash verification fails (if `expected_hash` is provided)
pub fn fetch_remote_config_offline(
    url: &str,
    project_root: Option<&Path>,
    expected_hash: Option<&str>,
) -> Result<String> {
    if !is_remote_url(url) {
        return Err(SlocGuardError::Config(format!(
            "Invalid remote config URL (must start with http:// or https://): {url}"
        )));
    }

    let content = read_from_cache(url, project_root).ok_or_else(|| {
        SlocGuardError::Config(format!(
            "Remote config cache miss in offline mode. Run without --offline first to cache: {url}"
        ))
    })?;

    // Verify cached content if hash is specified
    if let Some(hash) = expected_hash {
        verify_content_hash(&content, hash, url)?;
    }

    Ok(content)
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
#[path = "remote_tests.rs"]
mod tests;
