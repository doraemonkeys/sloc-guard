use std::collections::HashMap;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobMatcher, GlobSet, GlobSetBuilder};

use crate::config::{StructureConfig, UNLIMITED};
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
    /// True if this is a warning (threshold exceeded but under hard limit).
    pub is_warning: bool,
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
            is_warning: false,
        }
    }

    #[must_use]
    pub const fn warning(
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
            is_warning: true,
        }
    }
}

struct CompiledStructureRule {
    matcher: GlobMatcher,
    max_files: Option<i64>,
    max_dirs: Option<i64>,
    warn_threshold: Option<f64>,
}

/// Checker for directory structure limits.
pub struct StructureChecker {
    max_files: Option<i64>,
    max_dirs: Option<i64>,
    warn_threshold: Option<f64>,
    ignore_patterns: GlobSet,
    rules: Vec<CompiledStructureRule>,
}

impl StructureChecker {
    /// Create a new structure checker from config.
    ///
    /// # Errors
    /// Returns an error if any ignore or rule pattern is invalid,
    /// or if any limit value is less than -1.
    pub fn new(config: &StructureConfig) -> Result<Self> {
        Self::validate_limits(config)?;

        let ignore_patterns = Self::build_ignore_patterns(&config.count_exclude)?;
        let rules = Self::build_rules(&config.rules)?;

        Ok(Self {
            max_files: config.max_files,
            max_dirs: config.max_dirs,
            warn_threshold: config.warn_threshold,
            ignore_patterns,
            rules,
        })
    }

    /// Validate that all limit values are >= -1.
    fn validate_limits(config: &StructureConfig) -> Result<()> {
        if let Some(limit) = config.max_files
            && limit < UNLIMITED
        {
            return Err(SlocGuardError::Config(format!(
                "Invalid max_files value: {limit}. Use -1 for unlimited, 0 for prohibited, or a positive number."
            )));
        }
        if let Some(limit) = config.max_dirs
            && limit < UNLIMITED
        {
            return Err(SlocGuardError::Config(format!(
                "Invalid max_dirs value: {limit}. Use -1 for unlimited, 0 for prohibited, or a positive number."
            )));
        }

        for (i, rule) in config.rules.iter().enumerate() {
            if let Some(limit) = rule.max_files
                && limit < UNLIMITED
            {
                return Err(SlocGuardError::Config(format!(
                    "Invalid max_files value in rule {}: {limit}. Use -1 for unlimited, 0 for prohibited, or a positive number.",
                    i + 1
                )));
            }
            if let Some(limit) = rule.max_dirs
                && limit < UNLIMITED
            {
                return Err(SlocGuardError::Config(format!(
                    "Invalid max_dirs value in rule {}: {limit}. Use -1 for unlimited, 0 for prohibited, or a positive number.",
                    i + 1
                )));
            }
        }

        Ok(())
    }

    /// Returns true if structure checking is enabled (any limit is set).
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Vec::is_empty() is not const
    pub fn is_enabled(&self) -> bool {
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
                    warn_threshold: rule.warn_threshold,
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
    /// Returns (`max_files`, `max_dirs`, `warn_threshold`).
    /// A limit of `-1` (UNLIMITED) means no check should be performed.
    fn get_limits(&self, path: &Path) -> (Option<i64>, Option<i64>, Option<f64>) {
        // Check rules first (higher priority)
        for rule in &self.rules {
            if rule.matcher.is_match(path) {
                // Rule can override individual limits; fall back to global for unset
                let max_files = rule.max_files.or(self.max_files);
                let max_dirs = rule.max_dirs.or(self.max_dirs);
                let warn_threshold = rule.warn_threshold.or(self.warn_threshold);
                return (max_files, max_dirs, warn_threshold);
            }
        }

        // Fall back to global defaults
        (self.max_files, self.max_dirs, self.warn_threshold)
    }

    /// Check directory stats against limits and return violations.
    /// Limits of `-1` (UNLIMITED) are skipped.
    #[must_use]
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    pub fn check(&self, dir_stats: &HashMap<PathBuf, DirStats>) -> Vec<StructureViolation> {
        let mut violations = Vec::new();

        for (path, stats) in dir_stats {
            let (max_files, max_dirs, warn_threshold) = self.get_limits(path);

            // Check file count (skip if unlimited)
            if let Some(limit) = max_files
                && limit != UNLIMITED
            {
                let limit_usize = limit as usize;
                let warn_limit = warn_threshold
                    .map_or(limit_usize, |t| ((limit as f64) * t).ceil() as usize);

                if stats.file_count > limit_usize {
                    violations.push(StructureViolation::new(
                        path.clone(),
                        ViolationType::FileCount,
                        stats.file_count,
                        limit_usize,
                    ));
                } else if stats.file_count > warn_limit {
                    violations.push(StructureViolation::warning(
                        path.clone(),
                        ViolationType::FileCount,
                        stats.file_count,
                        limit_usize,
                    ));
                }
            }

            // Check directory count (skip if unlimited)
            if let Some(limit) = max_dirs
                && limit != UNLIMITED
            {
                let limit_usize = limit as usize;
                let warn_limit = warn_threshold
                    .map_or(limit_usize, |t| ((limit as f64) * t).ceil() as usize);

                if stats.dir_count > limit_usize {
                    violations.push(StructureViolation::new(
                        path.clone(),
                        ViolationType::DirCount,
                        stats.dir_count,
                        limit_usize,
                    ));
                } else if stats.dir_count > warn_limit {
                    violations.push(StructureViolation::warning(
                        path.clone(),
                        ViolationType::DirCount,
                        stats.dir_count,
                        limit_usize,
                    ));
                }
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
