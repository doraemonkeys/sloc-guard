use std::path::{Path, PathBuf};

use super::FileScanner;
use super::directory::DirectoryScanner;
use super::filter::GlobFilter;
use super::{ScanResult, StructureScanConfig};
use crate::error::Result;

/// Composite scanner that handles gitignore-aware and regular scanning.
///
/// This scanner:
/// - Uses `DirectoryScanner::with_gitignore` when `use_gitignore` is enabled
/// - Uses regular `DirectoryScanner` otherwise
/// - Applies exclude patterns via `GlobFilter`
pub struct CompositeScanner {
    exclude_patterns: Vec<String>,
    pub(crate) use_gitignore: bool,
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
        let filter = GlobFilter::new(Vec::new(), &self.exclude_patterns)?;
        if self.use_gitignore {
            let scanner = DirectoryScanner::with_gitignore(filter, true);
            scanner.scan(root)
        } else {
            let scanner = DirectoryScanner::new(filter);
            scanner.scan(root)
        }
    }

    fn scan_all(&self, paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
        let filter = GlobFilter::new(Vec::new(), &self.exclude_patterns)?;
        let scanner = if self.use_gitignore {
            DirectoryScanner::with_gitignore(filter, true)
        } else {
            DirectoryScanner::new(filter)
        };

        let mut all_files = Vec::new();
        for path in paths {
            all_files.extend(scanner.scan(path)?);
        }
        Ok(all_files)
    }

    fn scan_with_structure(
        &self,
        root: &Path,
        structure_config: Option<&StructureScanConfig>,
    ) -> Result<ScanResult> {
        let filter = GlobFilter::new(Vec::new(), &self.exclude_patterns)?;
        if self.use_gitignore {
            let scanner = DirectoryScanner::with_gitignore(filter, true);
            scanner.scan_with_structure(root, structure_config)
        } else {
            let scanner = DirectoryScanner::new(filter);
            scanner.scan_with_structure(root, structure_config)
        }
    }

    fn scan_all_with_structure(
        &self,
        paths: &[PathBuf],
        structure_config: Option<&StructureScanConfig>,
    ) -> Result<ScanResult> {
        let filter = GlobFilter::new(Vec::new(), &self.exclude_patterns)?;
        let scanner = if self.use_gitignore {
            DirectoryScanner::with_gitignore(filter, true)
        } else {
            DirectoryScanner::new(filter)
        };

        let mut combined = ScanResult::default();
        for path in paths {
            let result = scanner.scan_with_structure(path, structure_config)?;
            combined.files.extend(result.files);
            combined.dir_stats.extend(result.dir_stats);
            combined
                .allowlist_violations
                .extend(result.allowlist_violations);
        }
        Ok(combined)
    }
}

/// Scan files from paths using `DirectoryScanner`.
///
/// Scanner returns ALL files (respecting gitignore + exclude patterns only).
/// Extension filtering should be done by the caller (e.g., `ThresholdChecker`).
///
/// Uses `DirectoryScanner::with_gitignore` when `use_gitignore` is true to
/// respect .gitignore patterns (works even outside git repositories).
///
/// # Errors
/// Returns an error if the directory cannot be read or if glob patterns are invalid.
pub fn scan_files(
    paths: &[PathBuf],
    exclude_patterns: &[String],
    use_gitignore: bool,
) -> Result<Vec<PathBuf>> {
    let filter = GlobFilter::new(Vec::new(), exclude_patterns)?;
    let scanner = if use_gitignore {
        DirectoryScanner::with_gitignore(filter, true)
    } else {
        DirectoryScanner::new(filter)
    };

    let mut all_files = Vec::new();
    for path in paths {
        all_files.extend(scanner.scan(path)?);
    }
    Ok(all_files)
}
