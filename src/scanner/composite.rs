use std::path::{Path, PathBuf};

use super::FileScanner;
use super::directory::DirectoryScanner;
use super::filter::GlobFilter;
use super::gitignore::GitAwareScanner;
use super::{ScanResult, StructureScanConfig};
use crate::SlocGuardError;
use crate::error::Result;

/// Composite scanner that handles git-aware and fallback scanning.
///
/// This scanner:
/// - Uses `GitAwareScanner` when `use_gitignore` is enabled
/// - Falls back to `DirectoryScanner` if not in a git repository
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
        if self.use_gitignore {
            let filter = GlobFilter::new(Vec::new(), &self.exclude_patterns)?;
            let scanner = GitAwareScanner::new(filter);
            match scanner.scan(root) {
                Ok(files) => Ok(files),
                Err(SlocGuardError::GitRepoNotFound(_)) => {
                    // Silent fallback - not in a git repo, but still respect .gitignore
                    let filter = GlobFilter::new(Vec::new(), &self.exclude_patterns)?;
                    let scanner = DirectoryScanner::with_gitignore(filter, true);
                    scanner.scan(root)
                }
                Err(SlocGuardError::Git(msg)) => {
                    // Warn user about git error and fallback
                    crate::output::print_warning_full(
                        "Git error occurred, falling back to filesystem scanner",
                        Some(&msg),
                        None,
                    );
                    let filter = GlobFilter::new(Vec::new(), &self.exclude_patterns)?;
                    let scanner = DirectoryScanner::with_gitignore(filter, true);
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
                    Err(SlocGuardError::GitRepoNotFound(_)) => {
                        // Silent fallback - not in a git repo
                        return self.scan_all_without_git(paths);
                    }
                    Err(SlocGuardError::Git(msg)) => {
                        // Warn user about git error and fallback
                        crate::output::print_warning_full(
                            "Git error occurred, falling back to filesystem scanner",
                            Some(&msg),
                            None,
                        );
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

    fn scan_with_structure(
        &self,
        root: &Path,
        structure_config: Option<&StructureScanConfig>,
    ) -> Result<ScanResult> {
        if self.use_gitignore {
            let filter = GlobFilter::new(Vec::new(), &self.exclude_patterns)?;
            let scanner = GitAwareScanner::new(filter);
            match scanner.scan_with_structure(root, structure_config) {
                Ok(result) => Ok(result),
                Err(SlocGuardError::GitRepoNotFound(_)) => {
                    // Silent fallback - not in a git repo, but still respect .gitignore
                    let filter = GlobFilter::new(Vec::new(), &self.exclude_patterns)?;
                    let scanner = DirectoryScanner::with_gitignore(filter, true);
                    scanner.scan_with_structure(root, structure_config)
                }
                Err(SlocGuardError::Git(msg)) => {
                    // Warn user about git error and fallback
                    crate::output::print_warning_full(
                        "Git error occurred, falling back to filesystem scanner",
                        Some(&msg),
                        None,
                    );
                    let filter = GlobFilter::new(Vec::new(), &self.exclude_patterns)?;
                    let scanner = DirectoryScanner::with_gitignore(filter, true);
                    scanner.scan_with_structure(root, structure_config)
                }
                Err(e) => Err(e),
            }
        } else {
            let filter = GlobFilter::new(Vec::new(), &self.exclude_patterns)?;
            let scanner = DirectoryScanner::new(filter);
            scanner.scan_with_structure(root, structure_config)
        }
    }

    fn scan_all_with_structure(
        &self,
        paths: &[PathBuf],
        structure_config: Option<&StructureScanConfig>,
    ) -> Result<ScanResult> {
        // Override default to handle git fallback at the path-list level
        let mut combined = ScanResult::default();

        if self.use_gitignore {
            let filter = GlobFilter::new(Vec::new(), &self.exclude_patterns)?;
            let scanner = GitAwareScanner::new(filter);
            for path in paths {
                match scanner.scan_with_structure(path, structure_config) {
                    Ok(result) => {
                        combined.files.extend(result.files);
                        combined.dir_stats.extend(result.dir_stats);
                        combined
                            .allowlist_violations
                            .extend(result.allowlist_violations);
                    }
                    Err(SlocGuardError::GitRepoNotFound(_)) => {
                        // Silent fallback - not in a git repo
                        return self.scan_all_with_structure_without_git(paths, structure_config);
                    }
                    Err(SlocGuardError::Git(msg)) => {
                        // Warn user about git error and fallback
                        crate::output::print_warning_full(
                            "Git error occurred, falling back to filesystem scanner",
                            Some(&msg),
                            None,
                        );
                        return self.scan_all_with_structure_without_git(paths, structure_config);
                    }
                    Err(e) => return Err(e),
                }
            }
        } else {
            return self.scan_all_with_structure_without_git(paths, structure_config);
        }

        Ok(combined)
    }
}

impl CompositeScanner {
    fn scan_all_without_git(&self, paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
        let filter = GlobFilter::new(Vec::new(), &self.exclude_patterns)?;
        let scanner = DirectoryScanner::with_gitignore(filter, self.use_gitignore);
        let mut all_files = Vec::new();
        for path in paths {
            all_files.extend(scanner.scan(path)?);
        }
        Ok(all_files)
    }

    fn scan_all_with_structure_without_git(
        &self,
        paths: &[PathBuf],
        structure_config: Option<&StructureScanConfig>,
    ) -> Result<ScanResult> {
        let filter = GlobFilter::new(Vec::new(), &self.exclude_patterns)?;
        let scanner = DirectoryScanner::with_gitignore(filter, self.use_gitignore);
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

/// Scan files from paths using either `GitAwareScanner` or `DirectoryScanner`.
///
/// Scanner returns ALL files (respecting gitignore + exclude patterns only).
/// Extension filtering should be done by the caller (e.g., `ThresholdChecker`).
///
/// Uses `GitAwareScanner` (respects .gitignore) if `use_gitignore` is true and
/// falls back to `DirectoryScanner` if not in a git repository.
///
/// If git operations fail (other than "not a git repo"), emits a warning and falls back.
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
                Err(SlocGuardError::GitRepoNotFound(_)) => {
                    // Silent fallback - not in a git repo
                    return scan_files(paths, exclude_patterns, false);
                }
                Err(SlocGuardError::Git(msg)) => {
                    // Warn user about git error and fallback
                    crate::output::print_warning_full(
                        "Git error occurred, falling back to filesystem scanner",
                        Some(&msg),
                        None,
                    );
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
