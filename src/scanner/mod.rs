mod composite;
mod directory;
mod filter;
mod gitignore;
mod types;

pub use composite::{CompositeScanner, scan_files};
pub use directory::DirectoryScanner;
pub use filter::{FileFilter, GlobFilter};
pub use gitignore::GitAwareScanner;
pub use types::{AllowlistRule, AllowlistRuleBuilder, ScanResult, StructureScanConfig};

use std::path::{Path, PathBuf};

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

    /// Scan a directory with structure-aware statistics collection.
    ///
    /// Returns files, directory statistics, and allowlist violations in a single traversal.
    ///
    /// # Errors
    /// Returns an error if the directory cannot be read.
    fn scan_with_structure(
        &self,
        root: &Path,
        structure_config: Option<&StructureScanConfig>,
    ) -> Result<ScanResult>;

    /// Scan multiple directories with structure-aware statistics collection.
    ///
    /// # Errors
    /// Returns an error if any directory cannot be read.
    fn scan_all_with_structure(
        &self,
        paths: &[PathBuf],
        structure_config: Option<&StructureScanConfig>,
    ) -> Result<ScanResult> {
        let mut combined = ScanResult::default();
        for path in paths {
            let result = self.scan_with_structure(path, structure_config)?;
            combined.files.extend(result.files);
            combined.dir_stats.extend(result.dir_stats);
            combined
                .allowlist_violations
                .extend(result.allowlist_violations);
        }
        Ok(combined)
    }
}

#[cfg(test)]
mod composite_tests;
#[cfg(test)]
mod deny_pattern_tests;
#[cfg(test)]
mod directory_tests;
#[cfg(test)]
mod naming_pattern_tests;
#[cfg(test)]
mod structure_scan_tests;
#[cfg(test)]
mod types_tests;
