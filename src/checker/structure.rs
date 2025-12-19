use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use globset::{Glob, GlobMatcher, GlobSet, GlobSetBuilder};

use crate::config::{StructureConfig, StructureOverride, UNLIMITED};
use crate::error::{Result, SlocGuardError};

use super::explain::{
    MatchStatus, StructureExplanation, StructureRuleCandidate, StructureRuleMatch,
};

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
    /// Reason for override if one was applied.
    pub override_reason: Option<String>,
}

impl StructureViolation {
    #[must_use]
    pub const fn new(
        path: PathBuf,
        violation_type: ViolationType,
        actual: usize,
        limit: usize,
        override_reason: Option<String>,
    ) -> Self {
        Self {
            path,
            violation_type,
            actual,
            limit,
            is_warning: false,
            override_reason,
        }
    }

    #[must_use]
    pub const fn warning(
        path: PathBuf,
        violation_type: ViolationType,
        actual: usize,
        limit: usize,
        override_reason: Option<String>,
    ) -> Self {
        Self {
            path,
            violation_type,
            actual,
            limit,
            is_warning: true,
            override_reason,
        }
    }
}

struct CompiledStructureRule {
    pattern: String,
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
    overrides: Vec<StructureOverride>,
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
            overrides: config.overrides.clone(),
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

        // Validate overrides
        for (i, ovr) in config.overrides.iter().enumerate() {
            if let Some(limit) = ovr.max_files
                && limit < UNLIMITED
            {
                return Err(SlocGuardError::Config(format!(
                    "Invalid max_files value in override {}: {limit}. Use -1 for unlimited, 0 for prohibited, or a positive number.",
                    i + 1
                )));
            }
            if let Some(limit) = ovr.max_dirs
                && limit < UNLIMITED
            {
                return Err(SlocGuardError::Config(format!(
                    "Invalid max_dirs value in override {}: {limit}. Use -1 for unlimited, 0 for prohibited, or a positive number.",
                    i + 1
                )));
            }
            // Require at least one limit to be set
            if ovr.max_files.is_none() && ovr.max_dirs.is_none() {
                return Err(SlocGuardError::Config(format!(
                    "Override {} for path '{}' must specify at least one of max_files or max_dirs.",
                    i + 1,
                    ovr.path
                )));
            }
        }

        Ok(())
    }

    /// Returns true if structure checking is enabled (any limit is set).
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Vec::is_empty() is not const
    pub fn is_enabled(&self) -> bool {
        self.max_files.is_some()
            || self.max_dirs.is_some()
            || !self.rules.is_empty()
            || !self.overrides.is_empty()
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
                    pattern: rule.pattern.clone(),
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

    /// Check if a directory path matches an override path.
    /// Uses suffix matching similar to `ThresholdChecker`.
    fn path_matches_override(dir_path: &Path, override_path: &str) -> bool {
        let override_components: Vec<&str> = override_path
            .split(['/', '\\'])
            .filter(|s| !s.is_empty())
            .collect();

        let dir_components: Vec<_> = dir_path.components().collect();

        if override_components.is_empty() || override_components.len() > dir_components.len() {
            return false;
        }

        dir_components
            .iter()
            .rev()
            .zip(override_components.iter().rev())
            .all(|(dir_comp, override_comp)| {
                if let Component::Normal(os_str) = dir_comp {
                    os_str.to_string_lossy() == *override_comp
                } else {
                    false
                }
            })
    }

    /// Get limits for a directory path.
    /// Returns (`max_files`, `max_dirs`, `warn_threshold`, `override_reason`).
    /// A limit of `-1` (UNLIMITED) means no check should be performed.
    ///
    /// # Priority Chain (high → low)
    ///
    /// 1. `[[structure.override]]` - exact path match (highest)
    /// 2. `[[structure.rules]]` - glob pattern, last match wins
    /// 3. `[structure]` defaults (lowest)
    ///
    /// # Glob Semantics (structure rules only match directories)
    ///
    /// - `src/components/*`  — matches DIRECT children only (e.g., `Button/`, `Icon/`)
    /// - `src/components/**` — matches ALL descendants recursively
    /// - `src/features`      — exact directory match only
    fn get_limits(&self, path: &Path) -> (Option<i64>, Option<i64>, Option<f64>, Option<String>) {
        // 1. Check overrides first (highest priority)
        for ovr in &self.overrides {
            if Self::path_matches_override(path, &ovr.path) {
                let max_files = ovr.max_files.or(self.max_files);
                let max_dirs = ovr.max_dirs.or(self.max_dirs);
                return (
                    max_files,
                    max_dirs,
                    self.warn_threshold,
                    Some(ovr.reason.clone()),
                );
            }
        }

        // 2. Check rules (glob patterns)
        for rule in &self.rules {
            if rule.matcher.is_match(path) {
                let max_files = rule.max_files.or(self.max_files);
                let max_dirs = rule.max_dirs.or(self.max_dirs);
                let warn_threshold = rule.warn_threshold.or(self.warn_threshold);
                return (max_files, max_dirs, warn_threshold, None);
            }
        }

        // 3. Fall back to global defaults
        (self.max_files, self.max_dirs, self.warn_threshold, None)
    }

    /// Check directory stats against limits and return violations.
    ///
    /// Only directories are checked (files are not tracked in `dir_stats`).
    /// Each directory's immediate children counts are compared against applicable limits.
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
            let (max_files, max_dirs, warn_threshold, override_reason) = self.get_limits(path);

            // Check file count (skip if unlimited)
            if let Some(limit) = max_files
                && limit != UNLIMITED
            {
                let limit_usize = limit as usize;
                let warn_limit =
                    warn_threshold.map_or(limit_usize, |t| ((limit as f64) * t).ceil() as usize);

                if stats.file_count > limit_usize {
                    violations.push(StructureViolation::new(
                        path.clone(),
                        ViolationType::FileCount,
                        stats.file_count,
                        limit_usize,
                        override_reason.clone(),
                    ));
                } else if stats.file_count > warn_limit {
                    violations.push(StructureViolation::warning(
                        path.clone(),
                        ViolationType::FileCount,
                        stats.file_count,
                        limit_usize,
                        override_reason.clone(),
                    ));
                }
            }

            // Check directory count (skip if unlimited)
            if let Some(limit) = max_dirs
                && limit != UNLIMITED
            {
                let limit_usize = limit as usize;
                let warn_limit =
                    warn_threshold.map_or(limit_usize, |t| ((limit as f64) * t).ceil() as usize);

                if stats.dir_count > limit_usize {
                    violations.push(StructureViolation::new(
                        path.clone(),
                        ViolationType::DirCount,
                        stats.dir_count,
                        limit_usize,
                        override_reason.clone(),
                    ));
                } else if stats.dir_count > warn_limit {
                    violations.push(StructureViolation::warning(
                        path.clone(),
                        ViolationType::DirCount,
                        stats.dir_count,
                        limit_usize,
                        override_reason.clone(),
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

    /// Explain which rule matches a given directory path.
    ///
    /// Returns a detailed breakdown of all evaluated rules and which one won.
    #[must_use]
    pub fn explain(&self, path: &Path) -> StructureExplanation {
        let mut rule_chain = Vec::new();
        let mut matched_rule = StructureRuleMatch::Default;
        let mut found_match = false;
        let mut override_reason = None;

        // 1. Check overrides first (highest priority)
        for (i, ovr) in self.overrides.iter().enumerate() {
            let matches = Self::path_matches_override(path, &ovr.path);
            let status = if matches && !found_match {
                found_match = true;
                override_reason = Some(ovr.reason.clone());
                matched_rule = StructureRuleMatch::Override {
                    index: i,
                    reason: ovr.reason.clone(),
                };
                MatchStatus::Matched
            } else if matches {
                MatchStatus::Superseded
            } else {
                MatchStatus::NoMatch
            };

            rule_chain.push(StructureRuleCandidate {
                source: format!("structure.overrides[{i}]"),
                pattern: Some(ovr.path.clone()),
                max_files: ovr.max_files,
                max_dirs: ovr.max_dirs,
                status,
            });
        }

        // 2. Check rules (first match wins based on actual code behavior)
        for (i, rule) in self.rules.iter().enumerate() {
            let matches = rule.matcher.is_match(path);
            let status = if matches && !found_match {
                found_match = true;
                matched_rule = StructureRuleMatch::Rule {
                    index: i,
                    pattern: rule.pattern.clone(),
                };
                MatchStatus::Matched
            } else if matches {
                MatchStatus::Superseded
            } else {
                MatchStatus::NoMatch
            };

            rule_chain.push(StructureRuleCandidate {
                source: format!("structure.rules[{i}]"),
                pattern: Some(rule.pattern.clone()),
                max_files: rule.max_files,
                max_dirs: rule.max_dirs,
                status,
            });
        }

        // 3. Add default
        rule_chain.push(StructureRuleCandidate {
            source: "structure (default)".to_string(),
            pattern: None,
            max_files: self.max_files,
            max_dirs: self.max_dirs,
            status: if found_match {
                MatchStatus::Superseded
            } else {
                MatchStatus::Matched
            },
        });

        // Get effective limits using the same logic as get_limits
        let (effective_max_files, effective_max_dirs, warn_threshold, _) = self.get_limits(path);

        StructureExplanation {
            path: path.to_path_buf(),
            matched_rule,
            effective_max_files,
            effective_max_dirs,
            warn_threshold: warn_threshold.unwrap_or(1.0),
            override_reason,
            rule_chain,
        }
    }
}

#[cfg(test)]
#[path = "structure_tests.rs"]
mod tests;
