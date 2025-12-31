use std::fs;
use std::io::BufReader;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::config::TrendConfig;
use crate::git::GitContext;
use crate::output::ProjectStatistics;
use crate::state::{
    DEFAULT_LOCK_TIMEOUT_MS, SaveOutcome, SharedLockGuard, atomic_write_with_lock,
    current_unix_timestamp,
};
use crate::{Result, SlocGuardError};

/// Seconds per day for age calculation.
const SECONDS_PER_DAY: u64 = 86400;

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
    /// Git commit hash (short form, e.g., "a1b2c3d") at the time of snapshot.
    /// None if not in a git repository.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,
    /// Git branch name at the time of snapshot.
    /// None if not in a git repository or in detached HEAD state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,
}

impl TrendEntry {
    /// Creates a new trend entry from current project statistics.
    ///
    /// Git context (commit hash and branch) is not set by this constructor.
    /// Use `with_git_context` to add git information after creation.
    ///
    /// # Panics
    /// Panics if the system clock is set to a time before the UNIX epoch.
    #[must_use]
    pub fn new(stats: &ProjectStatistics) -> Self {
        Self {
            timestamp: current_unix_timestamp(),
            total_files: stats.total_files,
            total_lines: stats.total_lines,
            code: stats.total_code,
            comment: stats.total_comment,
            blank: stats.total_blank,
            git_ref: None,
            git_branch: None,
        }
    }

    #[must_use]
    pub const fn with_timestamp(mut self, timestamp: u64) -> Self {
        self.timestamp = timestamp;
        self
    }

    /// Set git context (commit hash and branch name).
    #[must_use]
    pub fn with_git_context(mut self, git_ref: Option<String>, git_branch: Option<String>) -> Self {
        self.git_ref = git_ref;
        self.git_branch = git_branch;
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
    /// Git commit hash from the previous entry (for display context)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_git_ref: Option<String>,
    /// Git branch from the previous entry
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_git_branch: Option<String>,
}

/// Default threshold for significant code changes.
/// Changes of 10 lines or fewer are considered trivial unless files were added/removed.
pub const DEFAULT_MIN_CODE_DELTA: u64 = 10;

impl TrendDelta {
    /// Compute delta from previous entry to current stats.
    #[must_use]
    #[allow(clippy::cast_possible_wrap)] // Delta values can be negative and fit in i64
    pub fn compute(previous: &TrendEntry, current: &ProjectStatistics) -> Self {
        Self {
            files_delta: current.total_files as i64 - previous.total_files as i64,
            lines_delta: current.total_lines as i64 - previous.total_lines as i64,
            code_delta: current.total_code as i64 - previous.code as i64,
            comment_delta: current.total_comment as i64 - previous.comment as i64,
            blank_delta: current.total_blank as i64 - previous.blank as i64,
            previous_timestamp: Some(previous.timestamp),
            previous_git_ref: previous.git_ref.clone(),
            previous_git_branch: previous.git_branch.clone(),
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

    /// Check if the delta is significant enough to display.
    ///
    /// A delta is significant if:
    /// - Any files were added or removed (`files_delta` != 0), OR
    /// - The absolute code delta exceeds the configured threshold
    ///
    /// Use this to suppress noise from trivial changes (e.g., ±5 lines of code
    /// with no file changes).
    #[must_use]
    pub fn is_significant(&self, config: &TrendConfig) -> bool {
        // File changes are always significant
        if self.files_delta != 0 {
            return true;
        }

        // Check code delta against threshold
        let threshold = config.min_code_delta.unwrap_or(DEFAULT_MIN_CODE_DELTA);
        self.code_delta.unsigned_abs() > threshold
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
        let file = fs::File::open(path).map_err(|e| SlocGuardError::FileAccess {
            path: path.to_path_buf(),
            source: e,
        })?;

        // Acquire shared lock for reading (allows multiple readers)
        // Guard automatically unlocks on drop
        let _lock_guard =
            SharedLockGuard::try_acquire(&file, DEFAULT_LOCK_TIMEOUT_MS, "history file", path);

        let reader = BufReader::new(&file);
        Ok(serde_json::from_reader(reader)?)
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

    /// Save history to a JSON file using atomic write pattern.
    ///
    /// Uses atomic write (temp file + rename) to prevent data loss:
    /// 1. Serialize JSON to memory
    /// 2. Write to temporary file
    /// 3. Acquire exclusive lock
    /// 4. Atomically rename temp → target
    ///
    /// Returns `SaveOutcome::Saved` on success, `SaveOutcome::Skipped` if lock times out.
    /// The original file is preserved on any failure.
    ///
    /// # Errors
    /// Returns an error if the file cannot be written (except lock timeout → `Skipped`).
    #[must_use = "check if save was skipped due to lock timeout"]
    pub fn save(&self, path: &Path) -> Result<SaveOutcome> {
        let json = serde_json::to_string_pretty(self)?;
        atomic_write_with_lock(path, json.as_bytes(), "history file")
    }

    /// Get the most recent entry.
    #[must_use]
    pub fn latest(&self) -> Option<&TrendEntry> {
        self.entries.last()
    }

    /// Add a new entry from current statistics.
    ///
    /// **Note**: This method does not capture git context. Use `add_with_context`
    /// if you need to record the current git commit and branch.
    pub fn add(&mut self, stats: &ProjectStatistics) {
        self.entries.push(TrendEntry::new(stats));
    }

    /// Add a new entry from current statistics with git context.
    ///
    /// The git context (commit hash and branch name) is recorded in the entry
    /// for later reference when computing deltas.
    pub fn add_with_context(
        &mut self,
        stats: &ProjectStatistics,
        git_context: Option<&GitContext>,
    ) {
        let entry = TrendEntry::new(stats).with_git_context(
            git_context.map(|ctx| ctx.commit.clone()),
            git_context.and_then(|ctx| ctx.branch.clone()),
        );
        self.entries.push(entry);
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

    /// Find the nearest entry at or before a given timestamp.
    ///
    /// Searches backwards through entries (newest to oldest) to find the first
    /// entry with a timestamp <= `at_or_before`.
    ///
    /// Returns `None` if no entry exists at or before the specified time.
    #[must_use]
    pub fn find_entry_at_or_before(&self, at_or_before: u64) -> Option<&TrendEntry> {
        // Entries are chronological (oldest first), so search backwards
        self.entries
            .iter()
            .rev()
            .find(|entry| entry.timestamp <= at_or_before)
    }

    /// Compute delta from an entry at a specific time ago to current stats.
    ///
    /// Finds the nearest entry at or before `(current_time - duration_secs)` and
    /// computes the delta to the current statistics. This allows comparing against
    /// a snapshot from a specific point in time.
    ///
    /// # Arguments
    /// * `duration_secs` - How far back to look (in seconds)
    /// * `current` - Current project statistics
    /// * `current_time` - Current timestamp (seconds since epoch)
    ///
    /// Returns `None` if no entry exists at or before the specified time point.
    #[must_use]
    pub fn compute_delta_since(
        &self,
        duration_secs: u64,
        current: &ProjectStatistics,
        current_time: u64,
    ) -> Option<TrendDelta> {
        let target_time = current_time.saturating_sub(duration_secs);
        self.find_entry_at_or_before(target_time)
            .map(|prev| TrendDelta::compute(prev, current))
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

    // ========================================================================
    // Retention Policy Methods
    // ========================================================================

    /// Check if a new entry should be added based on `min_interval_secs`.
    ///
    /// Returns `true` if:
    /// - History is empty, or
    /// - No `min_interval_secs` configured, or
    /// - Enough time has elapsed since the last entry
    #[must_use]
    pub fn should_add(&self, config: &TrendConfig, current_time: u64) -> bool {
        let Some(min_interval) = config.min_interval_secs else {
            return true;
        };

        let Some(latest) = self.latest() else {
            return true;
        };

        current_time.saturating_sub(latest.timestamp) >= min_interval
    }

    /// Apply retention policy: remove old entries based on config.
    ///
    /// Applies in order:
    /// 1. Remove entries older than `max_age_days`
    /// 2. Remove oldest entries exceeding `max_entries`
    ///
    /// Returns the number of entries removed.
    pub fn apply_retention(&mut self, config: &TrendConfig, current_time: u64) -> usize {
        let original_count = self.entries.len();

        // 1. Remove entries older than max_age_days
        if let Some(max_age_days) = config.max_age_days {
            let cutoff = current_time.saturating_sub(max_age_days * SECONDS_PER_DAY);
            self.entries.retain(|e| e.timestamp >= cutoff);
        }

        // 2. Trim to max_entries (keep newest, remove oldest)
        if let Some(max_entries) = config.max_entries
            && self.entries.len() > max_entries
        {
            // Entries are chronological (oldest first), so drain from the front
            let excess = self.entries.len() - max_entries;
            self.entries.drain(0..excess);
        }

        original_count - self.entries.len()
    }

    /// Add a new entry only if retention policy allows (respects `min_interval_secs`).
    ///
    /// **Note**: This method does not capture git context. Use `add_if_allowed_with_context`
    /// if you need to record the current git commit and branch.
    ///
    /// Returns `true` if the entry was added, `false` if skipped due to interval.
    ///
    /// # Panics
    /// Panics if the system clock is set to a time before the UNIX epoch.
    pub fn add_if_allowed(&mut self, stats: &ProjectStatistics, config: &TrendConfig) -> bool {
        self.add_if_allowed_with_context(stats, config, None)
    }

    /// Add a new entry with git context only if retention policy allows.
    ///
    /// The git context (commit hash and branch name) is recorded in the entry
    /// for later reference when computing deltas.
    ///
    /// Returns `true` if the entry was added, `false` if skipped due to interval.
    ///
    /// # Panics
    /// Panics if the system clock is set to a time before the UNIX epoch.
    pub fn add_if_allowed_with_context(
        &mut self,
        stats: &ProjectStatistics,
        config: &TrendConfig,
        git_context: Option<&GitContext>,
    ) -> bool {
        let current_time = current_unix_timestamp();

        if !self.should_add(config, current_time) {
            return false;
        }

        let entry = TrendEntry::new(stats).with_git_context(
            git_context.map(|ctx| ctx.commit.clone()),
            git_context.and_then(|ctx| ctx.branch.clone()),
        );
        self.entries.push(entry);
        true
    }

    /// Save history with retention policy applied.
    ///
    /// Applies cleanup before writing to disk:
    /// 1. Remove entries older than `max_age_days`
    /// 2. Trim to `max_entries`
    ///
    /// # Panics
    /// Panics if the system clock is set to a time before the UNIX epoch.
    ///
    /// # Errors
    /// Returns an error if the file cannot be written.
    #[must_use = "check if save was skipped due to lock timeout"]
    pub fn save_with_retention(
        &mut self,
        path: &Path,
        config: &TrendConfig,
    ) -> Result<SaveOutcome> {
        self.apply_retention(config, current_unix_timestamp());
        self.save(path)
    }
}

#[cfg(test)]
#[path = "trend_tests/mod.rs"]
mod tests;
