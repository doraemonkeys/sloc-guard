//! Filesystem abstraction for testability.
//!
//! Provides a trait for filesystem operations that can be mocked in tests.

use std::path::{Path, PathBuf};

/// Trait for filesystem operations (for testability).
pub trait FileSystem {
    /// Read file contents as a string.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read.
    fn read_to_string(&self, path: &Path) -> std::io::Result<String>;

    /// Check if a path exists.
    fn exists(&self, path: &Path) -> bool;

    /// Get the current working directory.
    ///
    /// # Errors
    /// Returns an error if the current directory cannot be determined.
    fn current_dir(&self) -> std::io::Result<PathBuf>;

    /// Get the platform-specific configuration directory for sloc-guard.
    ///
    /// Returns the appropriate config directory based on platform conventions:
    /// - Windows: `%APPDATA%\sloc-guard` (e.g., `C:\Users\xxx\AppData\Roaming\sloc-guard`)
    /// - macOS: `~/Library/Application Support/sloc-guard`
    /// - Linux: `~/.config/sloc-guard` (XDG)
    fn config_dir(&self) -> Option<PathBuf>;

    /// Canonicalize a path to its absolute, normalized form.
    ///
    /// # Errors
    /// Returns an error if the path cannot be canonicalized (e.g., file doesn't exist).
    fn canonicalize(&self, path: &Path) -> std::io::Result<PathBuf>;
}

/// Real filesystem implementation.
#[derive(Debug, Default, Clone, Copy)]
pub struct RealFileSystem;

impl FileSystem for RealFileSystem {
    fn read_to_string(&self, path: &Path) -> std::io::Result<String> {
        std::fs::read_to_string(path)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn current_dir(&self) -> std::io::Result<PathBuf> {
        std::env::current_dir()
    }

    fn config_dir(&self) -> Option<PathBuf> {
        directories::ProjectDirs::from("", "", "sloc-guard")
            .map(|dirs| dirs.config_dir().to_path_buf())
    }

    fn canonicalize(&self, path: &Path) -> std::io::Result<PathBuf> {
        path.canonicalize()
    }
}
