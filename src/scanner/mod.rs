mod filter;

pub use filter::{FileFilter, GlobFilter};

use std::path::{Path, PathBuf};

use walkdir::WalkDir;

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
            .filter(|e| e.file_type().is_file())
            .map(|e| e.path().to_path_buf())
            .filter(|p| self.filter.should_include(p))
            .collect()
    }
}

impl<F: FileFilter> FileScanner for DirectoryScanner<F> {
    fn scan(&self, root: &Path) -> Result<Vec<PathBuf>> {
        Ok(self.scan_impl(root))
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
