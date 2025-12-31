//! State file path resolution with git-awareness.
//!
//! This module provides functions to resolve paths for cache and history files.
//! When running in a git repository root, state files are stored in `.git/sloc-guard/`
//! (automatically gitignored). Otherwise, they fall back to `.sloc-guard/`.

use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use crate::{Result, SlocGuardError};

const STATE_DIR_NAME: &str = "sloc-guard";
const FALLBACK_STATE_DIR: &str = ".sloc-guard";
const CACHE_FILENAME: &str = "cache.json";
const HISTORY_FILENAME: &str = "history.json";
const BASELINE_FILENAME: &str = ".sloc-guard-baseline.json";
const CONFIG_FILENAME: &str = ".sloc-guard.toml";

/// Default lock timeout in milliseconds.
pub const DEFAULT_LOCK_TIMEOUT_MS: u64 = 5000;

/// Polling interval for lock acquisition in milliseconds.
const LOCK_POLL_INTERVAL_MS: u64 = 50;

/// Returns the current Unix timestamp in seconds.
///
/// # Panics
/// Panics if system time is before UNIX epoch (should never happen in practice).
#[must_use]
pub fn current_unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before UNIX_EPOCH")
        .as_secs()
}

/// Returns the current Unix timestamp in seconds, or `None` if system time is unavailable.
///
/// Use this variant in contexts where panicking is not acceptable (e.g., formatting).
#[must_use]
pub fn try_current_unix_timestamp() -> Option<u64> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

/// Detect the state directory for cache and history files.
///
/// Returns `.git/sloc-guard/` if the project root has a `.git` directory,
/// otherwise returns `.sloc-guard/`.
///
/// Note: This only checks for `.git` in the immediate project root, not parent directories.
/// This ensures state files are always relative to the project being scanned.
#[must_use]
pub fn detect_state_dir(project_root: &Path) -> PathBuf {
    let git_dir = project_root.join(".git");
    if git_dir.is_dir() {
        // Use .git/sloc-guard/ for state files
        git_dir.join(STATE_DIR_NAME)
    } else {
        // Fallback to .sloc-guard/ in project root
        project_root.join(FALLBACK_STATE_DIR)
    }
}

/// Get the cache file path for the given project root.
#[must_use]
pub fn cache_path(project_root: &Path) -> PathBuf {
    detect_state_dir(project_root).join(CACHE_FILENAME)
}

/// Get the history file path for the given project root.
#[must_use]
pub fn history_path(project_root: &Path) -> PathBuf {
    detect_state_dir(project_root).join(HISTORY_FILENAME)
}

/// Ensure the parent directory exists for a given path.
///
/// # Errors
/// Returns an error if the directory cannot be created.
pub fn ensure_parent_dir(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

/// Discover the project root by walking up from `start` looking for markers.
///
/// Markers (checked in order at each directory level):
///   1. `.git/` directory - git repository root
///   2. `.sloc-guard.toml` - explicit sloc-guard config
///
/// Returns `start` if no markers found (backward compatible).
/// If `start` cannot be canonicalized, returns it as-is.
#[must_use]
pub fn discover_project_root(start: &Path) -> PathBuf {
    // Use dunce::canonicalize to avoid \\?\ UNC prefix on Windows,
    // ensuring consistent path comparison with scanner output
    let abs_start = dunce::canonicalize(start).unwrap_or_else(|_| start.to_path_buf());

    for ancestor in abs_start.ancestors() {
        if ancestor.join(".git").is_dir() {
            return ancestor.to_path_buf();
        }
        if ancestor.join(CONFIG_FILENAME).is_file() {
            return ancestor.to_path_buf();
        }
    }

    abs_start
}

/// Get the baseline file path for the given project root.
#[must_use]
pub fn baseline_path(project_root: &Path) -> PathBuf {
    project_root.join(BASELINE_FILENAME)
}

// =============================================================================
// File Locking Utilities
// =============================================================================

/// Error type for lock acquisition failures.
#[derive(Debug)]
pub enum LockError {
    /// Lock acquisition timed out.
    Timeout,
    /// I/O error during lock operation.
    Io(io::Error),
}

/// Outcome of a save operation.
///
/// Distinguishes between successful save and skipped save (due to lock timeout).
/// This makes the result explicit rather than silently succeeding when nothing was saved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveOutcome {
    /// File was successfully written.
    Saved,
    /// Save was skipped due to lock timeout.
    /// The original file (if any) remains unchanged.
    Skipped,
}

impl SaveOutcome {
    /// Returns true if the file was saved.
    #[must_use]
    pub const fn is_saved(self) -> bool {
        matches!(self, Self::Saved)
    }

    /// Returns true if the save was skipped.
    #[must_use]
    pub const fn is_skipped(self) -> bool {
        matches!(self, Self::Skipped)
    }
}

impl From<io::Error> for LockError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl std::fmt::Display for LockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => write!(f, "lock acquisition timed out"),
            Self::Io(e) => write!(f, "lock I/O error: {e}"),
        }
    }
}

impl std::error::Error for LockError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Timeout => None,
            Self::Io(e) => Some(e),
        }
    }
}

/// Try to acquire an exclusive (write) lock on the file with timeout.
///
/// Uses polling with [`LOCK_POLL_INTERVAL_MS`] interval.
/// Returns `Ok(())` on success, `Err(LockError::Timeout)` if timeout exceeded.
///
/// # Errors
/// - `LockError::Timeout` if lock cannot be acquired within `timeout_ms`
/// - `LockError::Io` for other I/O errors
// Returns std::result::Result (not crate::Result) because this uses LockError, not SlocGuardError
pub fn try_lock_exclusive_with_timeout(
    file: &File,
    timeout_ms: u64,
) -> std::result::Result<(), LockError> {
    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let poll_interval = Duration::from_millis(LOCK_POLL_INTERVAL_MS);

    loop {
        match file.try_lock() {
            Ok(()) => return Ok(()),
            Err(TryLockError::WouldBlock) => {
                if start.elapsed() >= timeout {
                    return Err(LockError::Timeout);
                }
                thread::sleep(poll_interval);
            }
            Err(TryLockError::Error(e)) => return Err(LockError::Io(e)),
        }
    }
}

/// Try to acquire a shared (read) lock on the file with timeout.
///
/// Uses polling with [`LOCK_POLL_INTERVAL_MS`] interval.
/// Returns `Ok(())` on success, `Err(LockError::Timeout)` if timeout exceeded.
///
/// # Errors
/// - `LockError::Timeout` if lock cannot be acquired within `timeout_ms`
/// - `LockError::Io` for other I/O errors
// Returns std::result::Result (not crate::Result) because this uses LockError, not SlocGuardError
pub fn try_lock_shared_with_timeout(
    file: &File,
    timeout_ms: u64,
) -> std::result::Result<(), LockError> {
    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let poll_interval = Duration::from_millis(LOCK_POLL_INTERVAL_MS);

    loop {
        match file.try_lock_shared() {
            Ok(()) => return Ok(()),
            Err(TryLockError::WouldBlock) => {
                if start.elapsed() >= timeout {
                    return Err(LockError::Timeout);
                }
                thread::sleep(poll_interval);
            }
            Err(TryLockError::Error(e)) => return Err(LockError::Io(e)),
        }
    }
}

/// Unlock a file, releasing any held lock.
///
/// This should be called after finishing with a locked file.
/// Errors are silently ignored as unlock failures are non-critical.
pub fn unlock_file(file: &File) {
    let _ = file.unlock();
}

// =============================================================================
// Shared Lock Guard (RAII pattern for lock tracking)
// =============================================================================

/// RAII guard that unlocks on drop if lock was acquired.
///
/// This encapsulates the common pattern of acquiring a shared lock with timeout,
/// warning on failure, and ensuring cleanup on drop.
#[derive(Debug)]
pub struct SharedLockGuard<'a> {
    file: &'a File,
    locked: bool,
}

impl<'a> SharedLockGuard<'a> {
    /// Try to acquire a shared lock with timeout, warning on failure.
    ///
    /// If lock acquisition fails, a warning is printed and the guard
    /// tracks that no lock was acquired (no unlock on drop).
    #[must_use]
    pub fn try_acquire(file: &'a File, timeout_ms: u64, context: &str, path: &Path) -> Self {
        let locked = match try_lock_shared_with_timeout(file, timeout_ms) {
            Ok(()) => true,
            Err(e) => {
                crate::output::print_warning_full(
                    &format!("Failed to acquire read lock on {context}"),
                    Some(&format!("{}: {e}", path.display())),
                    Some(&format!("{context} may be stale if written concurrently")),
                );
                false
            }
        };
        Self { file, locked }
    }

    /// Returns true if the lock was successfully acquired.
    #[must_use]
    pub const fn is_locked(&self) -> bool {
        self.locked
    }
}

impl Drop for SharedLockGuard<'_> {
    fn drop(&mut self) {
        if self.locked {
            unlock_file(self.file);
        }
    }
}

// =============================================================================
// Atomic File Writing
// =============================================================================

/// RAII guard for temporary file cleanup.
///
/// Ensures the temp file is removed if the write operation fails.
/// Call `commit()` after successful write to prevent cleanup.
#[derive(Debug)]
struct TempFileGuard<'a> {
    path: &'a Path,
    should_remove: bool,
}

impl<'a> TempFileGuard<'a> {
    const fn new(path: &'a Path) -> Self {
        Self {
            path,
            should_remove: true,
        }
    }

    /// Mark the temp file as committed (don't remove on drop).
    const fn commit(&mut self) {
        self.should_remove = false;
    }
}

impl Drop for TempFileGuard<'_> {
    fn drop(&mut self) {
        if self.should_remove {
            let _ = fs::remove_file(self.path);
        }
    }
}

/// Atomically write content to a file with exclusive locking.
///
/// Uses the atomic write pattern to prevent data loss:
/// 1. Write content to a temporary file in the same directory
/// 2. Sync temp file to disk for durability
/// 3. Open (or create) the target file for locking
/// 4. Acquire exclusive lock on the target file
/// 5. Atomically rename temp → target
///
/// If any step before rename fails, the temp file is cleaned up automatically
/// via RAII guard. If lock acquisition times out, returns `Ok(SaveOutcome::Skipped)`.
/// The original file (if any) is preserved on any failure.
///
/// # Arguments
/// * `path` - Target file path
/// * `content` - Content to write
/// * `file_description` - Human-readable description for warning messages (e.g., "baseline file")
///
/// # Errors
/// Returns an error if the file cannot be written (except for lock timeout, which returns `Skipped`).
pub fn atomic_write_with_lock(
    path: &Path,
    content: &[u8],
    file_description: &str,
) -> Result<SaveOutcome> {
    atomic_write_with_lock_timeout(path, content, file_description, DEFAULT_LOCK_TIMEOUT_MS)
}

/// Internal implementation with configurable timeout for testing.
pub(crate) fn atomic_write_with_lock_timeout(
    path: &Path,
    content: &[u8],
    file_description: &str,
    timeout_ms: u64,
) -> Result<SaveOutcome> {
    // Ensure parent directory exists
    ensure_parent_dir(path).map_err(|e| {
        SlocGuardError::io_with_context(e, path.to_path_buf(), "create parent directory")
    })?;

    // Generate unique temp filename in same directory (required for atomic rename)
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_stem = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let temp_name = format!(".{file_stem}.tmp.{}", std::process::id());
    let temp_path = parent.join(&temp_name);

    // RAII guard ensures temp file cleanup on any early return
    let mut temp_guard = TempFileGuard::new(&temp_path);

    // Write to temp file first (preserves original on failure)
    {
        let temp_file = File::create(&temp_path).map_err(|e| {
            SlocGuardError::io_with_context(e, temp_path.clone(), "create temp file")
        })?;
        let mut writer = io::BufWriter::new(&temp_file);
        writer.write_all(content).map_err(|e| {
            SlocGuardError::io_with_context(e, temp_path.clone(), "write temp file")
        })?;
        writer.flush().map_err(|e| {
            SlocGuardError::io_with_context(e, temp_path.clone(), "flush temp file")
        })?;
        // Sync to disk before rename for durability
        temp_file
            .sync_all()
            .map_err(|e| SlocGuardError::io_with_context(e, temp_path.clone(), "sync temp file"))?;
    }

    // Acquire exclusive lock on target file (create if needed, don't truncate)
    let lock_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false) // Don't truncate - we're just using this for locking
        .open(path)
        .map_err(|e| SlocGuardError::io_with_context(e, path.to_path_buf(), "open for lock"))?;

    if let Err(e) = try_lock_exclusive_with_timeout(&lock_file, timeout_ms) {
        // temp_guard will clean up on drop
        crate::output::print_warning_full(
            &format!("Failed to acquire write lock on {file_description}"),
            Some(&format!("{}: {e}", path.display())),
            Some(&format!("{file_description} save skipped")),
        );
        return Ok(SaveOutcome::Skipped);
    }

    // Atomic rename: temp → target
    // On Unix this is truly atomic. On Windows, we need to remove target first.
    #[cfg(windows)]
    {
        // Windows: can't rename over existing file while it's open.
        // Drop the lock file handle first, then remove and rename.
        //
        // KNOWN LIMITATION: There is a small race window between unlock and rename
        // where another process could:
        //   1. Create the file after remove_file but before rename
        //   2. Open the file for writing between unlock and remove
        //
        // This is a fundamental Windows limitation. True atomic rename requires
        // platform-specific APIs (e.g., ReplaceFile) which are not used here.
        //
        // Worst case: data loss if another process writes between remove and rename.
        // This is acceptable for cache/history (regenerated on next run) but would
        // be problematic for user-authored files. All usages here are for
        // tool-managed state files that can be safely regenerated.
        //
        // Note: unlock_file is best-effort; dropping lock_file closes the handle
        // anyway, releasing the lock as a side effect.
        unlock_file(&lock_file);
        drop(lock_file);
        // Remove target (ignore error if it doesn't exist)
        let _ = fs::remove_file(path);
        fs::rename(&temp_path, path)
            .map_err(|e| SlocGuardError::io_with_context(e, path.to_path_buf(), "rename"))?;
    }
    #[cfg(not(windows))]
    {
        fs::rename(&temp_path, path)
            .map_err(|e| SlocGuardError::io_with_context(e, path.to_path_buf(), "rename"))?;
        // Note: unlock_file is best-effort; dropping lock_file closes the handle
        // anyway, releasing the lock as a side effect.
        unlock_file(&lock_file);
    }

    // Rename succeeded, don't remove the (now renamed) temp file
    temp_guard.commit();

    Ok(SaveOutcome::Saved)
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
