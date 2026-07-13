mod allowlist;
mod composite;
mod directory;
mod filter;
mod structure_config;

pub use allowlist::{AllowlistRule, AllowlistRuleBuilder};
pub use composite::{CompositeScanner, scan_files, scan_files_with_project_paths};
pub use directory::DirectoryScanner;
pub use filter::{DirectoryPruner, FileFilter, GlobFilter};
pub use structure_config::StructureScanConfig;

#[cfg(test)]
pub use structure_config::TestConfigParams;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::checker::{DirStats, StructureViolation};
use crate::error::Result;
use crate::project::{ProjectPaths, UNROOTED_PROJECT_PATHS};

/// Result of unified directory scan with structure stats.
#[derive(Debug, Clone, Default)]
pub struct ScanResult {
    /// All file paths discovered during scanning.
    pub files: Vec<PathBuf>,
    /// Directory statistics: raw immediate child inventory and depth.
    pub dir_stats: HashMap<PathBuf, DirStats>,
    /// Allowlist violations detected during scanning.
    pub allowlist_violations: Vec<StructureViolation>,
}

impl ScanResult {
    /// Rebase paths used for rules, output, and persistent identity to the configuration root.
    ///
    /// Directory traversal initially retains physical paths so files can be opened and filtered by
    /// git. Callers should clone the physical file list before rebasing it for structure checks.
    pub(crate) fn rebase_logical_paths(&mut self, project_paths: &ProjectPaths) {
        for file in &mut self.files {
            *file = project_paths.logical(file);
        }

        self.dir_stats = std::mem::take(&mut self.dir_stats)
            .into_iter()
            .map(|(path, mut stats)| {
                let logical = project_paths.logical(&path);
                if project_paths.config_root().is_some() {
                    stats.depth = project_paths.logical_depth(&path);
                }
                (logical, stats)
            })
            .collect();

        for violation in &mut self.allowlist_violations {
            violation.path = project_paths.logical(&violation.path);
        }
    }
}

/// Trait for scanning directories and finding files.
///
/// Implementations must be thread-safe (`Send + Sync`) for parallel processing.
pub trait FileScanner: Send + Sync {
    /// Path namespace used by this scanner's configured globs and logical conversion.
    fn project_paths(&self) -> &ProjectPaths {
        &UNROOTED_PROJECT_PATHS
    }

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
