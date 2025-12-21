use std::fs;
use std::io::{BufReader, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::output::ProjectStatistics;
use crate::state::{
    DEFAULT_LOCK_TIMEOUT_MS, try_lock_exclusive_with_timeout, try_lock_shared_with_timeout,
    unlock_file,
};
use crate::{Result, SlocGuardError};

const HISTORY_VERSION: u32 = 1;

/// A single historical snapshot of project statistics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrendEntry {
    /// Unix timestamp (seconds since epoch)
    pub timestamp: u64,
    pub total_files: usize,
    pub total_lines: usize,
    pub code: usize,
    pub comment: usize,
    pub blank: usize,
}

impl TrendEntry {
    #[must_use]
    pub fn new(stats: &ProjectStatistics) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            timestamp,
            total_files: stats.total_files,
            total_lines: stats.total_lines,
            code: stats.total_code,
            comment: stats.total_comment,
            blank: stats.total_blank,
        }
    }

    #[must_use]
    pub const fn with_timestamp(mut self, timestamp: u64) -> Self {
        self.timestamp = timestamp;
        self
    }
}

/// Delta between current and previous statistics.
#[derive(Debug, Clone, Default, Serialize)]
pub struct TrendDelta {
    pub files_delta: i64,
    pub lines_delta: i64,
    pub code_delta: i64,
    pub comment_delta: i64,
    pub blank_delta: i64,
    /// Timestamp of the previous entry for context
    pub previous_timestamp: Option<u64>,
}

impl TrendDelta {
    /// Compute delta from previous entry to current stats.
    #[must_use]
    #[allow(clippy::cast_possible_wrap)] // Delta values can be negative and fit in i64
    pub const fn compute(previous: &TrendEntry, current: &ProjectStatistics) -> Self {
        Self {
            files_delta: current.total_files as i64 - previous.total_files as i64,
            lines_delta: current.total_lines as i64 - previous.total_lines as i64,
            code_delta: current.total_code as i64 - previous.code as i64,
            comment_delta: current.total_comment as i64 - previous.comment as i64,
            blank_delta: current.total_blank as i64 - previous.blank as i64,
            previous_timestamp: Some(previous.timestamp),
        }
    }

    /// Check if there are any changes.
    #[must_use]
    pub const fn has_changes(&self) -> bool {
        self.files_delta != 0
            || self.lines_delta != 0
            || self.code_delta != 0
            || self.comment_delta != 0
            || self.blank_delta != 0
    }
}

/// Historical statistics storage.
///
/// File format:
/// ```json
/// {
///   "version": 1,
///   "entries": [
///     { "timestamp": 1234567890, "total_files": 100, ... }
///   ]
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrendHistory {
    version: u32,
    entries: Vec<TrendEntry>,
}

impl Default for TrendHistory {
    fn default() -> Self {
        Self::new()
    }
}

impl TrendHistory {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            version: HISTORY_VERSION,
            entries: Vec::new(),
        }
    }

    /// Load history from a JSON file.
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
            eprintln!(
                "Warning: Failed to acquire read lock on history file '{}': {e}",
                path.display()
            );
            // Continue without lock - better than failing
        }

        let reader = BufReader::new(&file);
        let result = serde_json::from_reader(reader);

        unlock_file(&file);

        Ok(result?)
    }

    /// Load history if file exists, otherwise return empty history.
    #[must_use]
    pub fn load_or_default(path: &Path) -> Self {
        if path.exists() {
            Self::load(path).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    /// Save history to a JSON file.
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
            eprintln!(
                "Warning: Failed to acquire write lock on history file '{}': {e}. Skipping history save.",
                path.display()
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

    /// Get the most recent entry.
    #[must_use]
    pub fn latest(&self) -> Option<&TrendEntry> {
        self.entries.last()
    }

    /// Add a new entry from current statistics.
    pub fn add(&mut self, stats: &ProjectStatistics) {
        self.entries.push(TrendEntry::new(stats));
    }

    /// Add a new entry.
    pub fn add_entry(&mut self, entry: TrendEntry) {
        self.entries.push(entry);
    }

    /// Compute delta from the latest entry to current stats.
    #[must_use]
    pub fn compute_delta(&self, current: &ProjectStatistics) -> Option<TrendDelta> {
        self.latest().map(|prev| TrendDelta::compute(prev, current))
    }

    /// Get number of entries.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if history is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get all entries.
    #[must_use]
    pub fn entries(&self) -> &[TrendEntry] {
        &self.entries
    }

    /// Get the history version.
    #[must_use]
    pub const fn version(&self) -> u32 {
        self.version
    }
}

#[cfg(test)]
#[path = "trend_tests.rs"]
mod tests;
