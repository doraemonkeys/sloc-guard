use std::collections::HashMap;
use std::fs;
use std::io::{BufReader, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::Config;
use crate::counter::LineStats;
use crate::state::{
    DEFAULT_LOCK_TIMEOUT_MS, try_lock_exclusive_with_timeout, try_lock_shared_with_timeout,
    unlock_file,
};
use crate::{Result, SlocGuardError};

const CACHE_VERSION: u32 = 3;

/// Cached line statistics for a single file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheEntry {
    pub hash: String,
    pub stats: CachedLineStats,
    /// File modification time (seconds since epoch)
    #[serde(default)]
    pub mtime: u64,
    /// File size in bytes
    #[serde(default)]
    pub size: u64,
}

impl CacheEntry {
    #[must_use]
    pub fn new(hash: String, stats: &LineStats, mtime: u64, size: u64) -> Self {
        Self {
            hash,
            stats: CachedLineStats::from(stats),
            mtime,
            size,
        }
    }

    /// Check if metadata (mtime + size) matches.
    #[must_use]
    pub const fn metadata_matches(&self, mtime: u64, size: u64) -> bool {
        self.mtime == mtime && self.size == size
    }
}

/// Serializable version of `LineStats` for caching.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CachedLineStats {
    pub total: usize,
    pub code: usize,
    pub comment: usize,
    pub blank: usize,
    #[serde(default)]
    pub ignored: usize,
}

impl From<&LineStats> for CachedLineStats {
    fn from(stats: &LineStats) -> Self {
        Self {
            total: stats.total,
            code: stats.code,
            comment: stats.comment,
            blank: stats.blank,
            ignored: stats.ignored,
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
            ignored: cached.ignored,
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
    /// Acquires a shared lock on the file before reading.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed.
    pub fn load(path: &Path) -> Result<Self> {
        let file = fs::File::open(path).map_err(|e| SlocGuardError::FileRead {
            path: path.to_path_buf(),
            source: e,
        })?;

        // Acquire shared lock for reading (allows multiple readers)
        if let Err(e) = try_lock_shared_with_timeout(&file, DEFAULT_LOCK_TIMEOUT_MS) {
            crate::output::print_warning_full(
                "Failed to acquire read lock on cache file",
                Some(&format!("{}: {e}", path.display())),
                None,
            );
            // Continue without lock - better than failing
        }

        let reader = BufReader::new(&file);
        let result = serde_json::from_reader(reader);

        unlock_file(&file);

        Ok(result?)
    }

    /// Save cache to a JSON file.
    ///
    /// Acquires an exclusive lock on the file before writing.
    /// If lock acquisition times out, logs a warning and skips the save.
    ///
    /// # Errors
    /// Returns an error if the file cannot be written.
    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        let file = fs::File::create(path).map_err(|e| SlocGuardError::FileRead {
            path: path.to_path_buf(),
            source: e,
        })?;

        // Acquire exclusive lock for writing
        if let Err(e) = try_lock_exclusive_with_timeout(&file, DEFAULT_LOCK_TIMEOUT_MS) {
            crate::output::print_warning_full(
                "Failed to acquire write lock on cache file",
                Some(&format!("{}: {e}", path.display())),
                Some("Cache save skipped"),
            );
            // Drop file without writing to avoid corruption
            return Ok(());
        }

        let mut writer = std::io::BufWriter::new(&file);
        writer.write_all(json.as_bytes())?;
        writer.flush()?;

        unlock_file(&file);
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

    /// Get cached entry if metadata (mtime + size) matches.
    /// This is a fast check that avoids reading file content.
    #[must_use]
    pub fn get_if_metadata_matches(
        &self,
        path: &str,
        mtime: u64,
        size: u64,
    ) -> Option<&CacheEntry> {
        self.files
            .get(path)
            .filter(|entry| entry.metadata_matches(mtime, size))
    }

    /// Get cached entry if hash matches.
    #[must_use]
    pub fn get_if_valid(&self, path: &str, file_hash: &str) -> Option<&CacheEntry> {
        self.files.get(path).filter(|entry| entry.hash == file_hash)
    }

    /// Add or update a cached entry.
    pub fn set(&mut self, path: &str, hash: String, stats: &LineStats, mtime: u64, size: u64) {
        self.files
            .insert(path.to_string(), CacheEntry::new(hash, stats, mtime, size));
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
/// Only hashes the parts of config that affect `LineStats` computation:
/// - Custom language definitions (comment syntax)
///
/// Excludes (changes to these do NOT invalidate cache):
/// - `warn_threshold`, `max_lines` (thresholds are checked after counting)
/// - structure rules (directory limits don't affect line counting)
/// - exclude patterns (affect file discovery, not line parsing)
/// - extensions filter (affects which files are processed, not how)
#[must_use]
pub fn compute_config_hash(config: &Config) -> String {
    // Only hash custom language definitions - these define comment syntax
    // which directly affects how LineStats are computed.
    // Predefined languages in LanguageRegistry are constant across versions.
    let json = serde_json::to_string(&config.languages).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(json.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
#[path = "cache_tests.rs"]
mod tests;
