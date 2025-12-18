use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use sha2::{Digest, Sha256};

use crate::error::{Result, SlocGuardError};

const CACHE_DIR: &str = "sloc-guard/configs";
const CACHE_TTL_SECS: u64 = 3600; // 1 hour
const REQUEST_TIMEOUT_SECS: u64 = 30;

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

/// Get the cache directory path.
fn cache_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|p| p.join(CACHE_DIR))
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .map(|p| p.join(".cache").join(CACHE_DIR))
    }
}

/// Get the cache file path for a given URL.
fn cache_file_path(url: &str) -> Option<PathBuf> {
    cache_dir().map(|dir| dir.join(format!("{}.toml", hash_url(url))))
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
fn read_from_cache(url: &str) -> Option<String> {
    let cache_path = cache_file_path(url)?;
    if is_cache_valid(&cache_path) {
        fs::read_to_string(&cache_path).ok()
    } else {
        None
    }
}

/// Write config to cache.
fn write_to_cache(url: &str, content: &str) -> Option<()> {
    let cache_path = cache_file_path(url)?;

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
/// 2. If cache miss, fetches from the URL using the provided client
/// 3. Caches the result for future use
///
/// # Errors
///
/// Returns an error if:
/// - The URL is invalid
/// - The request times out (30 seconds)
/// - The server returns a non-2xx status code
/// - Network errors occur
pub fn fetch_remote_config_with_client(url: &str, client: &impl HttpClient) -> Result<String> {
    // Validate URL
    if !is_remote_url(url) {
        return Err(SlocGuardError::Config(format!(
            "Invalid remote config URL (must start with http:// or https://): {url}"
        )));
    }

    // Try cache first
    if let Some(cached) = read_from_cache(url) {
        return Ok(cached);
    }

    // Fetch from remote
    let content = client.get(url)?;

    // Cache the result (ignore cache write errors)
    let _ = write_to_cache(url, &content);

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
pub fn fetch_remote_config(url: &str) -> Result<String> {
    fetch_remote_config_with_client(url, &ReqwestClient)
}

/// Clear the remote config cache.
///
/// Returns the number of files deleted.
#[must_use]
pub fn clear_cache() -> usize {
    let Some(dir) = cache_dir() else {
        return 0;
    };

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

#[cfg(test)]
#[path = "remote_tests.rs"]
mod tests;
