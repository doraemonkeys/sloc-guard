mod filter;
mod gitignore;

pub use filter::{FileFilter, GlobFilter};
pub use gitignore::GitAwareScanner;

use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::SlocGuardError;
use crate::error::Result;

/// Trait for scanning directories and finding files.
///
/// Implementations must be thread-safe (`Send + Sync`) for parallel processing.
pub trait FileScanner: Send + Sync {
    /// Scan a directory and return all matching file paths.
    ///
    /// # Errors
    /// Returns an error if the directory cannot be read.
    fn scan(&self, root: &Path) -> Result<Vec<PathBuf>>;

    /// Scan multiple directories and return all matching file paths.
    ///
    /// Default implementation calls `scan` for each path.
    ///
    /// # Errors
    /// Returns an error if any directory cannot be read.
    fn scan_all(&self, paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
        let mut all_files = Vec::new();
        for path in paths {
            all_files.extend(self.scan(path)?);
        }
        Ok(all_files)
    }
}

pub struct DirectoryScanner<F: FileFilter> {
    filter: F,
}

impl<F: FileFilter> DirectoryScanner<F> {
    #[must_use]
    pub const fn new(filter: F) -> Self {
        Self { filter }
    }

    fn scan_impl(&self, root: &Path) -> Vec<PathBuf> {
        WalkDir::new(root)
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter(|e| e.file_type().is_file() && self.filter.should_include(e.path()))
            .map(walkdir::DirEntry::into_path)
            .collect()
    }
}

impl<F: FileFilter + Send + Sync> FileScanner for DirectoryScanner<F> {
    fn scan(&self, root: &Path) -> Result<Vec<PathBuf>> {
        Ok(self.scan_impl(root))
    }
}

/// Composite scanner that handles git-aware and fallback scanning.
///
/// This scanner:
/// - Uses `GitAwareScanner` when `use_gitignore` is enabled
/// - Falls back to `DirectoryScanner` if not in a git repository
/// - Applies exclude patterns via `GlobFilter`
pub struct CompositeScanner {
    exclude_patterns: Vec<String>,
    use_gitignore: bool,
}

impl CompositeScanner {
    /// Create a new composite scanner with exclude patterns and gitignore setting.
    #[must_use]
    pub const fn new(exclude_patterns: Vec<String>, use_gitignore: bool) -> Self {
        Self {
            exclude_patterns,
            use_gitignore,
        }
    }
}

impl FileScanner for CompositeScanner {
    fn scan(&self, root: &Path) -> Result<Vec<PathBuf>> {
        if self.use_gitignore {
            let filter = GlobFilter::new(Vec::new(), &self.exclude_patterns)?;
            let scanner = GitAwareScanner::new(filter);
            match scanner.scan(root) {
                Ok(files) => Ok(files),
                Err(SlocGuardError::Git(_)) => {
                    // Fallback to directory scanner if not in git repo
                    let filter = GlobFilter::new(Vec::new(), &self.exclude_patterns)?;
                    let scanner = DirectoryScanner::new(filter);
                    scanner.scan(root)
                }
                Err(e) => Err(e),
            }
        } else {
            let filter = GlobFilter::new(Vec::new(), &self.exclude_patterns)?;
            let scanner = DirectoryScanner::new(filter);
            scanner.scan(root)
        }
    }

    fn scan_all(&self, paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
        // Override default to handle git fallback at the path-list level
        let mut all_files = Vec::new();

        if self.use_gitignore {
            let filter = GlobFilter::new(Vec::new(), &self.exclude_patterns)?;
            let scanner = GitAwareScanner::new(filter);
            for path in paths {
                match scanner.scan(path) {
                    Ok(files) => all_files.extend(files),
                    Err(SlocGuardError::Git(_)) => {
                        // Fallback to non-git scanning for all paths
                        return self.scan_all_without_git(paths);
                    }
                    Err(e) => return Err(e),
                }
            }
        } else {
            return self.scan_all_without_git(paths);
        }

        Ok(all_files)
    }
}

impl CompositeScanner {
    fn scan_all_without_git(&self, paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
        let filter = GlobFilter::new(Vec::new(), &self.exclude_patterns)?;
        let scanner = DirectoryScanner::new(filter);
        let mut all_files = Vec::new();
        for path in paths {
            all_files.extend(scanner.scan(path)?);
        }
        Ok(all_files)
    }
}

/// Scan files from paths using either `GitAwareScanner` or `DirectoryScanner`.
///
/// Scanner returns ALL files (respecting gitignore + exclude patterns only).
/// Extension filtering should be done by the caller (e.g., `ThresholdChecker`).
///
/// Uses `GitAwareScanner` (respects .gitignore) if `use_gitignore` is true and
/// falls back to `DirectoryScanner` if not in a git repository.
///
/// # Errors
/// Returns an error if the directory cannot be read or if glob patterns are invalid.
pub fn scan_files(
    paths: &[PathBuf],
    exclude_patterns: &[String],
    use_gitignore: bool,
) -> Result<Vec<PathBuf>> {
    let mut all_files = Vec::new();

    if use_gitignore {
        // Empty extensions = no extension filtering (accept all files)
        let filter = GlobFilter::new(Vec::new(), exclude_patterns)?;
        let scanner = GitAwareScanner::new(filter);
        for path in paths {
            match scanner.scan(path) {
                Ok(files) => all_files.extend(files),
                Err(SlocGuardError::Git(_)) => {
                    return scan_files(paths, exclude_patterns, false);
                }
                Err(e) => return Err(e),
            }
        }
    } else {
        let filter = GlobFilter::new(Vec::new(), exclude_patterns)?;
        let scanner = DirectoryScanner::new(filter);
        for path in paths {
            let files = scanner.scan(path)?;
            all_files.extend(files);
        }
    }

    Ok(all_files)
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
