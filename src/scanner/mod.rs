mod filter;
mod gitignore;

pub use filter::{FileFilter, GlobFilter};
pub use gitignore::GitAwareScanner;

use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::SlocGuardError;
use crate::error::Result;

/// Trait for scanning directories and finding files.
pub trait FileScanner {
    /// Scan a directory and return all matching file paths.
    ///
    /// # Errors
    /// Returns an error if the directory cannot be read.
    fn scan(&self, root: &Path) -> Result<Vec<PathBuf>>;
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

impl<F: FileFilter> FileScanner for DirectoryScanner<F> {
    fn scan(&self, root: &Path) -> Result<Vec<PathBuf>> {
        Ok(self.scan_impl(root))
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
