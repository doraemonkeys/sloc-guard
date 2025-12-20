use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use globset::{Glob, GlobMatcher};

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
    /// Depth relative to scan root (root = 0).
    pub depth: usize,
}

/// Type of structure violation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationType {
    FileCount,
    DirCount,
    MaxDepth,
    /// File type not allowed by allowlist (`allow_extensions`/`allow_patterns`).
    DisallowedFile,
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
    /// Pattern of the rule that triggered this violation (for `DisallowedFile`).
    pub triggering_rule_pattern: Option<String>,
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
            triggering_rule_pattern: None,
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
            triggering_rule_pattern: None,
        }
    }

    /// Create a disallowed file violation.
    #[must_use]
    pub const fn disallowed_file(path: PathBuf, rule_pattern: String) -> Self {
        Self {
            path,
            violation_type: ViolationType::DisallowedFile,
            actual: 1,
            limit: 0,
            is_warning: false,
            override_reason: None,
            triggering_rule_pattern: Some(rule_pattern),
        }
    }
}

struct CompiledStructureRule {
    pattern: String,
    matcher: GlobMatcher,
    max_files: Option<i64>,
    max_dirs: Option<i64>,
    max_depth: Option<i64>,
    warn_threshold: Option<f64>,
}

/// Resolved limits for a directory path.
#[derive(Debug, Clone, Default)]
struct StructureLimits {
    max_files: Option<i64>,
    max_dirs: Option<i64>,
    max_depth: Option<i64>,
    warn_threshold: Option<f64>,
    override_reason: Option<String>,
}

/// Checker for directory structure limits.
pub struct StructureChecker {
    max_files: Option<i64>,
    max_dirs: Option<i64>,
    max_depth: Option<i64>,
    warn_threshold: Option<f64>,
    rules: Vec<CompiledStructureRule>,
    overrides: Vec<StructureOverride>,
}

impl StructureChecker {
    /// Create a new structure checker from config.
    ///
    /// # Errors
    /// Returns an error if any rule pattern is invalid,
    /// or if any limit value is less than -1.
    pub fn new(config: &StructureConfig) -> Result<Self> {
        Self::validate_limits(config)?;
        let rules = Self::build_rules(&config.rules)?;

        Ok(Self {
            max_files: config.max_files,
            max_dirs: config.max_dirs,
            max_depth: config.max_depth,
            warn_threshold: config.warn_threshold,
            rules,
            overrides: config.overrides.clone(),
        })
    }

    /// Create a new structure checker with scanner exclude patterns.
    ///
    /// Note: Scanner exclude patterns are now handled by the scanner during traversal,
    /// not by the `StructureChecker`. This method is kept for backward compatibility
    /// and simply ignores the `scanner_exclude` parameter.
    ///
    /// # Errors
    /// Returns an error if any pattern is invalid or any limit value is less than -1.
    #[allow(clippy::needless_pass_by_value)]
    pub fn with_scanner_exclude(
        config: &StructureConfig,
        _scanner_exclude: &[String],
    ) -> Result<Self> {
        Self::new(config)
    }

    /// Returns true if structure checking is enabled (any limit is set).
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Vec::is_empty() is not const
    pub fn is_enabled(&self) -> bool {
        self.max_files.is_some()
            || self.max_dirs.is_some()
            || self.max_depth.is_some()
            || !self.rules.is_empty()
            || !self.overrides.is_empty()
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
        if let Some(limit) = config.max_depth
            && limit < UNLIMITED
        {
            return Err(SlocGuardError::Config(format!(
                "Invalid max_depth value: {limit}. Use -1 for unlimited, or a positive number."
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
            if let Some(limit) = rule.max_depth
                && limit < UNLIMITED
            {
                return Err(SlocGuardError::Config(format!(
                    "Invalid max_depth value in rule {}: {limit}. Use -1 for unlimited, or a positive number.",
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
            if let Some(limit) = ovr.max_depth
                && limit < UNLIMITED
            {
                return Err(SlocGuardError::Config(format!(
                    "Invalid max_depth value in override {}: {limit}. Use -1 for unlimited, or a positive number.",
                    i + 1
                )));
            }
            // Require at least one limit to be set
            if ovr.max_files.is_none() && ovr.max_dirs.is_none() && ovr.max_depth.is_none() {
                return Err(SlocGuardError::Config(format!(
                    "Override {} for path '{}' must specify at least one of max_files, max_dirs, or max_depth.",
                    i + 1,
                    ovr.path
                )));
            }
        }

        Ok(())
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
                    max_depth: rule.max_depth,
                    warn_threshold: rule.warn_threshold,
                })
            })
            .collect()
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
    /// Returns a `StructureLimits` struct with all applicable limits.
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
    fn get_limits(&self, path: &Path) -> StructureLimits {
        // 1. Check overrides first (highest priority)
        for ovr in &self.overrides {
            if Self::path_matches_override(path, &ovr.path) {
                return StructureLimits {
                    max_files: ovr.max_files.or(self.max_files),
                    max_dirs: ovr.max_dirs.or(self.max_dirs),
                    max_depth: ovr.max_depth.or(self.max_depth),
                    warn_threshold: self.warn_threshold,
                    override_reason: Some(ovr.reason.clone()),
                };
            }
        }

        // 2. Check rules (glob patterns) - last match wins
        // Iterate in reverse to find the last matching rule
        for rule in self.rules.iter().rev() {
            if rule.matcher.is_match(path) {
                return StructureLimits {
                    max_files: rule.max_files.or(self.max_files),
                    max_dirs: rule.max_dirs.or(self.max_dirs),
                    max_depth: rule.max_depth.or(self.max_depth),
                    warn_threshold: rule.warn_threshold.or(self.warn_threshold),
                    override_reason: None,
                };
            }
        }

        // 3. Fall back to global defaults
        StructureLimits {
            max_files: self.max_files,
            max_dirs: self.max_dirs,
            max_depth: self.max_depth,
            warn_threshold: self.warn_threshold,
            override_reason: None,
        }
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
            let limits = self.get_limits(path);

            // Check file count (skip if unlimited)
            if let Some(limit) = limits.max_files
                && limit != UNLIMITED
            {
                let limit_usize = limit as usize;
                let warn_limit = limits
                    .warn_threshold
                    .map_or(limit_usize, |t| ((limit as f64) * t).ceil() as usize);

                if stats.file_count > limit_usize {
                    violations.push(StructureViolation::new(
                        path.clone(),
                        ViolationType::FileCount,
                        stats.file_count,
                        limit_usize,
                        limits.override_reason.clone(),
                    ));
                } else if stats.file_count > warn_limit {
                    violations.push(StructureViolation::warning(
                        path.clone(),
                        ViolationType::FileCount,
                        stats.file_count,
                        limit_usize,
                        limits.override_reason.clone(),
                    ));
                }
            }

            // Check directory count (skip if unlimited)
            if let Some(limit) = limits.max_dirs
                && limit != UNLIMITED
            {
                let limit_usize = limit as usize;
                let warn_limit = limits
                    .warn_threshold
                    .map_or(limit_usize, |t| ((limit as f64) * t).ceil() as usize);

                if stats.dir_count > limit_usize {
                    violations.push(StructureViolation::new(
                        path.clone(),
                        ViolationType::DirCount,
                        stats.dir_count,
                        limit_usize,
                        limits.override_reason.clone(),
                    ));
                } else if stats.dir_count > warn_limit {
                    violations.push(StructureViolation::warning(
                        path.clone(),
                        ViolationType::DirCount,
                        stats.dir_count,
                        limit_usize,
                        limits.override_reason.clone(),
                    ));
                }
            }

            // Check depth (skip if unlimited)
            if let Some(limit) = limits.max_depth
                && limit != UNLIMITED
            {
                let limit_usize = limit as usize;
                let warn_limit = limits
                    .warn_threshold
                    .map_or(limit_usize, |t| ((limit as f64) * t).ceil() as usize);

                if stats.depth > limit_usize {
                    violations.push(StructureViolation::new(
                        path.clone(),
                        ViolationType::MaxDepth,
                        stats.depth,
                        limit_usize,
                        limits.override_reason.clone(),
                    ));
                } else if stats.depth > warn_limit {
                    violations.push(StructureViolation::warning(
                        path.clone(),
                        ViolationType::MaxDepth,
                        stats.depth,
                        limit_usize,
                        limits.override_reason.clone(),
                    ));
                }
            }
        }

        // Sort by path for consistent output
        violations.sort_by(|a, b| a.path.cmp(&b.path));
        violations
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
                max_depth: ovr.max_depth,
                status,
            });
        }

        // 2. Check rules (last match wins for consistency with content rules)
        // First find the index of the last matching rule
        let last_matching_rule_idx = self
            .rules
            .iter()
            .enumerate()
            .rev()
            .find(|(_, rule)| rule.matcher.is_match(path))
            .map(|(i, _)| i);

        // Then iterate forward to build rule chain with correct statuses
        for (i, rule) in self.rules.iter().enumerate() {
            let matches = rule.matcher.is_match(path);
            let is_last_match = last_matching_rule_idx == Some(i);
            let status = if is_last_match && !found_match {
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
                max_depth: rule.max_depth,
                status,
            });
        }

        // 3. Add default
        rule_chain.push(StructureRuleCandidate {
            source: "structure (default)".to_string(),
            pattern: None,
            max_files: self.max_files,
            max_dirs: self.max_dirs,
            max_depth: self.max_depth,
            status: if found_match {
                MatchStatus::Superseded
            } else {
                MatchStatus::Matched
            },
        });

        // Get effective limits using the same logic as get_limits
        let limits = self.get_limits(path);

        StructureExplanation {
            path: path.to_path_buf(),
            matched_rule,
            effective_max_files: limits.max_files,
            effective_max_dirs: limits.max_dirs,
            effective_max_depth: limits.max_depth,
            warn_threshold: limits.warn_threshold.unwrap_or(1.0),
            override_reason,
            rule_chain,
        }
    }
}

#[cfg(test)]
#[path = "structure_tests.rs"]
mod tests;
