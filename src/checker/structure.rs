use std::collections::HashMap;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobMatcher, GlobSet, GlobSetBuilder};

use crate::config::StructureConfig;
use crate::error::{Result, SlocGuardError};

/// Counts of immediate children in a directory.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DirStats {
    pub file_count: usize,
    pub dir_count: usize,
}

/// Type of structure violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationType {
    FileCount,
    DirCount,
}

/// A structure limit violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureViolation {
    pub path: PathBuf,
    pub violation_type: ViolationType,
    pub actual: usize,
    pub limit: usize,
}

impl StructureViolation {
    #[must_use]
    pub const fn new(
        path: PathBuf,
        violation_type: ViolationType,
        actual: usize,
        limit: usize,
    ) -> Self {
        Self {
            path,
            violation_type,
            actual,
            limit,
        }
    }
}

struct CompiledStructureRule {
    matcher: GlobMatcher,
    max_files: Option<usize>,
    max_dirs: Option<usize>,
}

/// Checker for directory structure limits.
pub struct StructureChecker {
    max_files: Option<usize>,
    max_dirs: Option<usize>,
    ignore_patterns: GlobSet,
    rules: Vec<CompiledStructureRule>,
}

impl StructureChecker {
    /// Create a new structure checker from config.
    ///
    /// # Errors
    /// Returns an error if any ignore or rule pattern is invalid.
    pub fn new(config: &StructureConfig) -> Result<Self> {
        let ignore_patterns = Self::build_ignore_patterns(&config.count_exclude)?;
        let rules = Self::build_rules(&config.rules)?;

        Ok(Self {
            max_files: config.max_files,
            max_dirs: config.max_dirs,
            ignore_patterns,
            rules,
        })
    }

    /// Returns true if structure checking is enabled (any limit is set).
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.max_files.is_some() || self.max_dirs.is_some() || !self.rules.is_empty()
    }

    fn build_ignore_patterns(patterns: &[String]) -> Result<GlobSet> {
        let mut builder = GlobSetBuilder::new();
        for pattern in patterns {
            let glob = Glob::new(pattern).map_err(|e| SlocGuardError::InvalidPattern {
                pattern: pattern.clone(),
                source: e,
            })?;
            builder.add(glob);
        }
        builder.build().map_err(|e| SlocGuardError::InvalidPattern {
            pattern: "combined ignore patterns".to_string(),
            source: e,
        })
    }

    fn build_rules(rules: &[crate::config::StructureRule]) -> Result<Vec<CompiledStructureRule>> {
        rules
            .iter()
            .map(|rule| {
                let glob =
                    Glob::new(&rule.pattern).map_err(|e| SlocGuardError::InvalidPattern {
                        pattern: rule.pattern.clone(),
                        source: e,
                    })?;
                Ok(CompiledStructureRule {
                    matcher: glob.compile_matcher(),
                    max_files: rule.max_files,
                    max_dirs: rule.max_dirs,
                })
            })
            .collect()
    }

    /// Scan directories and collect stats for each.
    ///
    /// # Errors
    /// Returns an error if a directory cannot be read.
    pub fn collect_dir_stats(&self, root: &Path) -> Result<HashMap<PathBuf, DirStats>> {
        let mut stats = HashMap::new();
        self.collect_dir_stats_recursive(root, &mut stats)?;
        Ok(stats)
    }

    fn collect_dir_stats_recursive(
        &self,
        dir: &Path,
        stats: &mut HashMap<PathBuf, DirStats>,
    ) -> Result<()> {
        let entries = std::fs::read_dir(dir).map_err(|e| SlocGuardError::FileRead {
            path: dir.to_path_buf(),
            source: e,
        })?;

        let mut dir_stats = DirStats::default();

        for entry in entries {
            let entry = entry.map_err(SlocGuardError::Io)?;
            let path = entry.path();
            let file_name = path.file_name().unwrap_or_default();

            // Check if this entry should be ignored
            if self.ignore_patterns.is_match(file_name) || self.ignore_patterns.is_match(&path) {
                continue;
            }

            let file_type = entry.file_type().map_err(SlocGuardError::Io)?;

            if file_type.is_file() {
                dir_stats.file_count += 1;
            } else if file_type.is_dir() {
                dir_stats.dir_count += 1;
                // Recurse into subdirectory
                self.collect_dir_stats_recursive(&path, stats)?;
            }
        }

        stats.insert(dir.to_path_buf(), dir_stats);
        Ok(())
    }

    /// Get limits for a directory path (rule takes priority over global default).
    fn get_limits(&self, path: &Path) -> (Option<usize>, Option<usize>) {
        // Check rules first (higher priority)
        for rule in &self.rules {
            if rule.matcher.is_match(path) {
                // Rule can override individual limits; fall back to global for unset
                let max_files = rule.max_files.or(self.max_files);
                let max_dirs = rule.max_dirs.or(self.max_dirs);
                return (max_files, max_dirs);
            }
        }

        // Fall back to global defaults
        (self.max_files, self.max_dirs)
    }

    /// Check directory stats against limits and return violations.
    #[must_use]
    pub fn check(&self, dir_stats: &HashMap<PathBuf, DirStats>) -> Vec<StructureViolation> {
        let mut violations = Vec::new();

        for (path, stats) in dir_stats {
            let (max_files, max_dirs) = self.get_limits(path);

            if let Some(limit) = max_files
                && stats.file_count > limit
            {
                violations.push(StructureViolation::new(
                    path.clone(),
                    ViolationType::FileCount,
                    stats.file_count,
                    limit,
                ));
            }

            if let Some(limit) = max_dirs
                && stats.dir_count > limit
            {
                violations.push(StructureViolation::new(
                    path.clone(),
                    ViolationType::DirCount,
                    stats.dir_count,
                    limit,
                ));
            }
        }

        // Sort by path for consistent output
        violations.sort_by(|a, b| a.path.cmp(&b.path));
        violations
    }

    /// Convenience method: collect stats and check in one call.
    ///
    /// # Errors
    /// Returns an error if directories cannot be read.
    pub fn check_directory(&self, root: &Path) -> Result<Vec<StructureViolation>> {
        let stats = self.collect_dir_stats(root)?;
        Ok(self.check(&stats))
    }
}

#[cfg(test)]
#[path = "structure_tests.rs"]
mod tests;
