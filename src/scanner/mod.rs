mod filter;
mod gitignore;

pub use filter::{FileFilter, GlobFilter};
pub use gitignore::GitAwareScanner;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};
use walkdir::WalkDir;

use crate::SlocGuardError;
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

/// A compiled allowlist rule for checking allowed file types in a directory.
#[derive(Debug, Clone)]
pub struct AllowlistRule {
    /// Glob pattern matching directories where this rule applies.
    pub pattern: String,
    matcher: globset::GlobMatcher,
    /// Validated extensions (with leading dot, e.g., ".rs").
    pub allow_extensions: Vec<String>,
    /// Compiled patterns for allowlist matching.
    pub allow_patterns: GlobSet,
}

impl AllowlistRule {
    /// Check if a file matches this allowlist (extensions OR patterns).
    fn file_matches(&self, file_path: &Path) -> bool {
        // Check extensions first (OR logic)
        if !self.allow_extensions.is_empty()
            && let Some(ext) = file_path.extension()
        {
            let ext_with_dot = format!(".{}", ext.to_string_lossy());
            if self.allow_extensions.contains(&ext_with_dot) {
                return true;
            }
        }

        // Check patterns (OR logic with extensions)
        let file_name = file_path.file_name().unwrap_or_default();
        if self.allow_patterns.is_match(file_name) || self.allow_patterns.is_match(file_path) {
            return true;
        }

        false
    }

    /// Check if a directory path matches this rule's pattern.
    fn matches_directory(&self, dir: &Path) -> bool {
        self.matcher.is_match(dir)
    }
}

/// Configuration for structure-aware scanning.
#[derive(Debug, Clone, Default)]
pub struct StructureScanConfig {
    /// Patterns to exclude from file/dir counting (`structure.count_exclude`).
    pub count_exclude: GlobSet,
    /// Scanner exclude patterns (scanner.exclude) - skip entirely.
    pub scanner_exclude: GlobSet,
    /// Directory names extracted from scanner.exclude patterns ending with "/**".
    pub scanner_exclude_dir_names: Vec<String>,
    /// Allowlist rules from structure.rules with `allow_extensions`/`allow_patterns`.
    pub allowlist_rules: Vec<AllowlistRule>,
}

impl StructureScanConfig {
    /// Build from config components.
    ///
    /// # Errors
    /// Returns an error if any pattern is invalid.
    pub fn new(
        count_exclude_patterns: &[String],
        scanner_exclude_patterns: &[String],
        allowlist_rules: Vec<AllowlistRule>,
    ) -> Result<Self> {
        let count_exclude = Self::build_glob_set(count_exclude_patterns)?;
        let scanner_exclude = Self::build_glob_set(scanner_exclude_patterns)?;
        let scanner_exclude_dir_names = Self::extract_dir_names(scanner_exclude_patterns);

        Ok(Self {
            count_exclude,
            scanner_exclude,
            scanner_exclude_dir_names,
            allowlist_rules,
        })
    }

    fn build_glob_set(patterns: &[String]) -> Result<GlobSet> {
        let mut builder = GlobSetBuilder::new();
        for pattern in patterns {
            let glob = Glob::new(pattern).map_err(|e| SlocGuardError::InvalidPattern {
                pattern: pattern.clone(),
                source: e,
            })?;
            builder.add(glob);
        }
        builder.build().map_err(|e| SlocGuardError::InvalidPattern {
            pattern: "combined patterns".to_string(),
            source: e,
        })
    }

    /// Extract directory names from patterns ending with "/**".
    fn extract_dir_names(patterns: &[String]) -> Vec<String> {
        patterns
            .iter()
            .filter_map(|p| {
                let trimmed = p.trim_end_matches("/**").trim_end_matches("\\**");
                if trimmed.len() < p.len() {
                    let last_component = trimmed
                        .rsplit(['/', '\\'])
                        .next()
                        .filter(|s| !s.is_empty() && !s.contains('*'));
                    last_component.map(String::from)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Check if a path should be excluded from scanning entirely.
    fn is_scanner_excluded(&self, path: &Path, is_dir: bool) -> bool {
        let file_name = path.file_name().unwrap_or_default();
        let file_name_str = file_name.to_string_lossy();

        if self.scanner_exclude.is_match(file_name) || self.scanner_exclude.is_match(path) {
            return true;
        }

        // For directories, check against extracted dir names
        if is_dir {
            for dir_name in &self.scanner_exclude_dir_names {
                if file_name_str == *dir_name {
                    return true;
                }
            }
        }

        false
    }

    /// Check if a path should be excluded from counting (but still traversed).
    fn is_count_excluded(&self, path: &Path) -> bool {
        let file_name = path.file_name().unwrap_or_default();
        self.count_exclude.is_match(file_name) || self.count_exclude.is_match(path)
    }

    /// Find the first allowlist rule matching a directory.
    fn find_matching_allowlist_rule(&self, dir: &Path) -> Option<&AllowlistRule> {
        self.allowlist_rules.iter().find(|r| r.matches_directory(dir))
    }
}

/// Builder for creating `AllowlistRule` instances.
pub struct AllowlistRuleBuilder {
    pattern: String,
    allow_extensions: Vec<String>,
    allow_patterns: Vec<String>,
}

impl AllowlistRuleBuilder {
    #[must_use]
    pub const fn new(pattern: String) -> Self {
        Self {
            pattern,
            allow_extensions: Vec::new(),
            allow_patterns: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_extensions(mut self, extensions: Vec<String>) -> Self {
        self.allow_extensions = extensions;
        self
    }

    #[must_use]
    pub fn with_patterns(mut self, patterns: Vec<String>) -> Self {
        self.allow_patterns = patterns;
        self
    }

    /// Build the `AllowlistRule`.
    ///
    /// # Errors
    /// Returns an error if any pattern is invalid.
    pub fn build(self) -> Result<AllowlistRule> {
        let glob = Glob::new(&self.pattern).map_err(|e| SlocGuardError::InvalidPattern {
            pattern: self.pattern.clone(),
            source: e,
        })?;

        let mut pattern_builder = GlobSetBuilder::new();
        for p in &self.allow_patterns {
            let g = Glob::new(p).map_err(|e| SlocGuardError::InvalidPattern {
                pattern: p.clone(),
                source: e,
            })?;
            pattern_builder.add(g);
        }
        let allow_patterns = pattern_builder
            .build()
            .map_err(|e| SlocGuardError::InvalidPattern {
                pattern: "allow_patterns".to_string(),
                source: e,
            })?;

        Ok(AllowlistRule {
            pattern: self.pattern,
            matcher: glob.compile_matcher(),
            allow_extensions: self.allow_extensions,
            allow_patterns,
        })
    }
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
            combined.allowlist_violations.extend(result.allowlist_violations);
        }
        Ok(combined)
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

    fn scan_with_structure_impl(
        &self,
        root: &Path,
        structure_config: Option<&StructureScanConfig>,
    ) -> ScanResult {
        let mut result = ScanResult::default();
        let mut dir_entries: HashMap<PathBuf, DirStats> = HashMap::new();

        let walker = WalkDir::new(root).into_iter();

        for entry in walker {
            let Ok(entry) = entry else {
                continue;
            };

            let path = entry.path();
            let depth = entry.depth();
            let file_type = entry.file_type();

            // Check scanner_exclude - skip entry entirely
            if let Some(cfg) = structure_config
                && cfg.is_scanner_excluded(path, file_type.is_dir())
            {
                continue;
            }

            // Check count_exclude - don't count but continue traversing
            let is_count_excluded = structure_config.is_some_and(|cfg| cfg.is_count_excluded(path));

            if file_type.is_file() {
                // Add to files list if filter allows
                if self.filter.should_include(path) {
                    result.files.push(path.to_path_buf());
                }

                // Count for parent directory (if not excluded)
                if !is_count_excluded
                    && let Some(parent) = path.parent()
                {
                    let parent_stats = dir_entries
                        .entry(parent.to_path_buf())
                        .or_insert_with(|| DirStats {
                            depth: if depth > 0 { depth - 1 } else { 0 },
                            ..Default::default()
                        });
                    parent_stats.file_count += 1;

                    // Check allowlist violations
                    if let Some(cfg) = structure_config
                        && let Some(rule) = cfg.find_matching_allowlist_rule(parent)
                        && !rule.file_matches(path)
                    {
                        result.allowlist_violations.push(StructureViolation::disallowed_file(
                            path.to_path_buf(),
                            rule.pattern.clone(),
                        ));
                    }
                }
            } else if file_type.is_dir() {
                // Initialize this directory's stats
                dir_entries
                    .entry(path.to_path_buf())
                    .or_insert_with(|| DirStats {
                        depth,
                        ..Default::default()
                    });

                // Count as subdirectory for parent (if not excluded and not root)
                if depth > 0 && !is_count_excluded
                    && let Some(parent) = path.parent()
                {
                    let parent_stats = dir_entries
                        .entry(parent.to_path_buf())
                        .or_insert_with(|| DirStats {
                            depth: depth - 1,
                            ..Default::default()
                        });
                    parent_stats.dir_count += 1;
                }
            }
        }

        result.dir_stats = dir_entries;
        result
    }
}

impl<F: FileFilter + Send + Sync> FileScanner for DirectoryScanner<F> {
    fn scan(&self, root: &Path) -> Result<Vec<PathBuf>> {
        Ok(self.scan_impl(root))
    }

    fn scan_with_structure(
        &self,
        root: &Path,
        structure_config: Option<&StructureScanConfig>,
    ) -> Result<ScanResult> {
        Ok(self.scan_with_structure_impl(root, structure_config))
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
                Err(SlocGuardError::Git(_)) => {
                    // Fallback to directory scanner if not in git repo
                    let filter = GlobFilter::new(Vec::new(), &self.exclude_patterns)?;
                    let scanner = DirectoryScanner::new(filter);
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
                        combined.allowlist_violations.extend(result.allowlist_violations);
                    }
                    Err(SlocGuardError::Git(_)) => {
                        // Fallback to non-git scanning for all paths
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
        let scanner = DirectoryScanner::new(filter);
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
        let scanner = DirectoryScanner::new(filter);
        let mut combined = ScanResult::default();
        for path in paths {
            let result = scanner.scan_with_structure(path, structure_config)?;
            combined.files.extend(result.files);
            combined.dir_stats.extend(result.dir_stats);
            combined.allowlist_violations.extend(result.allowlist_violations);
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
