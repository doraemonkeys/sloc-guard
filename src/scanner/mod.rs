mod allowlist;
mod composite;
mod directory;
mod filter;
mod gitignore;
mod structure_config;

pub use allowlist::{AllowlistRule, AllowlistRuleBuilder};
pub use composite::{CompositeScanner, scan_files};
pub use directory::DirectoryScanner;
pub use filter::{FileFilter, GlobFilter};
pub use gitignore::GitAwareScanner;
pub use structure_config::StructureScanConfig;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::checker::{DirStats, StructureViolation};
use crate::error::Result;

/// Result of unified directory scan with structure stats.
#[derive(Debug, Clone, Default)]
pub struct ScanResult {
    /// All file paths discovered during scanning.
    pub files: Vec<PathBuf>,
    /// Directory statistics: immediate children counts and depth.
    pub dir_stats: HashMap<PathBuf, DirStats>,
    /// Allowlist violations detected during scanning.
    pub allowlist_violations: Vec<StructureViolation>,
}

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
mod allowlist_tests;
#[cfg(test)]
mod composite_tests;
#[cfg(test)]
mod directory_tests;
#[cfg(test)]
mod global_allow_tests;
#[cfg(test)]
mod global_deny_tests;
#[cfg(test)]
mod naming_pattern_tests;
#[cfg(test)]
mod structure_config_tests;
#[cfg(test)]
mod structure_scan_tests;
