use std::collections::HashMap;
use std::fs;
use std::io::{BufReader, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::Config;
use crate::counter::LineStats;
use crate::{Result, SlocGuardError};

const CACHE_VERSION: u32 = 1;

/// Cached line statistics for a single file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheEntry {
    pub hash: String,
    pub stats: CachedLineStats,
}

impl CacheEntry {
    #[must_use]
    pub fn new(hash: String, stats: &LineStats) -> Self {
        Self {
            hash,
            stats: CachedLineStats::from(stats),
        }
    }
}

/// Serializable version of `LineStats` for caching.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CachedLineStats {
    pub total: usize,
    pub code: usize,
    pub comment: usize,
    pub blank: usize,
}

impl From<&LineStats> for CachedLineStats {
    fn from(stats: &LineStats) -> Self {
        Self {
            total: stats.total,
            code: stats.code,
            comment: stats.comment,
            blank: stats.blank,
        }
    }
}

impl From<&CachedLineStats> for LineStats {
    fn from(cached: &CachedLineStats) -> Self {
        Self {
            total: cached.total,
            code: cached.code,
            comment: cached.comment,
            blank: cached.blank,
        }
    }
}

/// File cache for storing SLOC results.
///
/// Cache format:
/// ```json
/// {
///   "version": 1,
///   "config_hash": "abc123...",
///   "files": {
///     "src/main.rs": { "hash": "def456...", "stats": {...} }
///   }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Cache {
    version: u32,
    config_hash: String,
    files: HashMap<String, CacheEntry>,
}

impl Default for Cache {
    fn default() -> Self {
        Self::new(String::new())
    }
}

impl Cache {
    #[must_use]
    pub fn new(config_hash: String) -> Self {
        Self {
            version: CACHE_VERSION,
            config_hash,
            files: HashMap::new(),
        }
    }

    /// Load cache from a JSON file.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed.
    pub fn load(path: &Path) -> Result<Self> {
        let file = fs::File::open(path).map_err(|e| SlocGuardError::FileRead {
            path: path.to_path_buf(),
            source: e,
        })?;
        let reader = BufReader::new(file);
        let cache: Self = serde_json::from_reader(reader)?;
        Ok(cache)
    }

    /// Save cache to a JSON file.
    ///
    /// # Errors
    /// Returns an error if the file cannot be written.
    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        let mut file = fs::File::create(path).map_err(|e| SlocGuardError::FileRead {
            path: path.to_path_buf(),
            source: e,
        })?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    /// Check if the cache is valid for the given config hash.
    #[must_use]
    pub fn is_valid(&self, config_hash: &str) -> bool {
        self.version == CACHE_VERSION && self.config_hash == config_hash
    }

    /// Get cached entry for a file by its path.
    #[must_use]
    pub fn get(&self, path: &str) -> Option<&CacheEntry> {
        self.files.get(path)
    }

    /// Get cached entry if hash matches.
    #[must_use]
    pub fn get_if_valid(&self, path: &str, file_hash: &str) -> Option<&CacheEntry> {
        self.files
            .get(path)
            .filter(|entry| entry.hash == file_hash)
    }

    /// Add or update a cached entry.
    pub fn set(&mut self, path: &str, hash: String, stats: &LineStats) {
        self.files
            .insert(path.to_string(), CacheEntry::new(hash, stats));
    }

    /// Remove a cached entry.
    pub fn remove(&mut self, path: &str) -> Option<CacheEntry> {
        self.files.remove(path)
    }

    /// Get number of cached entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Check if cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Get the config hash.
    #[must_use]
    pub fn config_hash(&self) -> &str {
        &self.config_hash
    }

    /// Get the cache version.
    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }

    /// Get all cached files.
    #[must_use]
    pub const fn files(&self) -> &HashMap<String, CacheEntry> {
        &self.files
    }
}

/// Compute a hash of the config that affects line counting.
///
/// This hash is used to invalidate the cache when config changes.
#[must_use]
pub fn compute_config_hash(config: &Config) -> String {
    let json = serde_json::to_string(config).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(json.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
#[path = "types_tests.rs"]
mod tests;
