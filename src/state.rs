//! State file path resolution with git-awareness.
//!
//! This module provides functions to resolve paths for cache and history files.
//! When running in a git repository root, state files are stored in `.git/sloc-guard/`
//! (automatically gitignored). Otherwise, they fall back to `.sloc-guard/`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const STATE_DIR_NAME: &str = "sloc-guard";
const FALLBACK_STATE_DIR: &str = ".sloc-guard";
const CACHE_FILENAME: &str = "cache.json";
const HISTORY_FILENAME: &str = "history.json";
const BASELINE_FILENAME: &str = ".sloc-guard-baseline.json";
const CONFIG_FILENAME: &str = ".sloc-guard.toml";

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
    let abs_start = fs::canonicalize(start).unwrap_or_else(|_| start.to_path_buf());

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

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
