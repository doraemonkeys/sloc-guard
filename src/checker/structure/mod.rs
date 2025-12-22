//! Structure checker for directory file/subdir/depth limits.
//!
//! This module provides [`StructureChecker`] which enforces directory structure
//! constraints including:
//! - Maximum files per directory
//! - Maximum subdirectories per directory
//! - Maximum directory depth
//! - File co-location (sibling) requirements

mod builder;
mod compiled_rules;
mod validation;
pub mod violation;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::config::{StructureConfig, StructureOverride, UNLIMITED};
use crate::error::Result;
use crate::path_utils::path_matches_override;

use super::explain::{
    MatchStatus, StructureExplanation, StructureRuleCandidate, StructureRuleMatch,
};
pub use violation::{DirStats, StructureViolation, ViolationType};

use builder::{build_rules, build_sibling_rules};
use compiled_rules::{CompiledSiblingRule, CompiledStructureRule, StructureLimits};
use validation::{validate_limits, validate_sibling_rules};

/// Checker for directory structure limits.
pub struct StructureChecker {
    max_files: Option<i64>,
    max_dirs: Option<i64>,
    max_depth: Option<i64>,
    warn_threshold: Option<f64>,
    rules: Vec<CompiledStructureRule>,
    overrides: Vec<StructureOverride>,
    sibling_rules: Vec<CompiledSiblingRule>,
}

impl StructureChecker {
    /// Create a new structure checker from config.
    ///
    /// # Errors
    /// Returns an error if any rule pattern is invalid,
    /// or if any limit value is less than -1.
    pub fn new(config: &StructureConfig) -> Result<Self> {
        validate_limits(config)?;
        validate_sibling_rules(&config.rules)?;
        let rules = build_rules(&config.rules)?;
        let sibling_rules = build_sibling_rules(&config.rules)?;

        Ok(Self {
            max_files: config.max_files,
            max_dirs: config.max_dirs,
            max_depth: config.max_depth,
            warn_threshold: config.warn_threshold,
            rules,
            overrides: config.overrides.clone(),
            sibling_rules,
        })
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
    #[cfg(test)]
    fn get_limits(&self, path: &Path) -> StructureLimits {
        self.resolve_limits(path)
    }

    fn resolve_limits(&self, path: &Path) -> StructureLimits {
        // 1. Check overrides first (highest priority)
        // Note: Overrides don't support relative_depth (they use absolute limits)
        for ovr in &self.overrides {
            if path_matches_override(path, &ovr.path) {
                return StructureLimits {
                    max_files: ovr.max_files.or(self.max_files),
                    max_dirs: ovr.max_dirs.or(self.max_dirs),
                    max_depth: ovr.max_depth.or(self.max_depth),
                    relative_depth: false,
                    base_depth: 0,
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
                    relative_depth: rule.relative_depth,
                    base_depth: rule.base_depth,
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
            relative_depth: false,
            base_depth: 0,
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
    )] // Limits are validated to be non-negative (if not UNLIMITED), so casting is safe.
    pub fn check(&self, dir_stats: &HashMap<PathBuf, DirStats>) -> Vec<StructureViolation> {
        let mut violations = Vec::new();

        for (path, stats) in dir_stats {
            let limits = self.resolve_limits(path);

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

                // Calculate effective depth: relative to base if relative_depth is set
                let effective_depth = if limits.relative_depth {
                    stats.depth.saturating_sub(limits.base_depth)
                } else {
                    stats.depth
                };

                if effective_depth > limit_usize {
                    violations.push(StructureViolation::new(
                        path.clone(),
                        ViolationType::MaxDepth,
                        effective_depth,
                        limit_usize,
                        limits.override_reason.clone(),
                    ));
                } else if effective_depth > warn_limit {
                    violations.push(StructureViolation::warning(
                        path.clone(),
                        ViolationType::MaxDepth,
                        effective_depth,
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

    /// Check files for missing siblings.
    ///
    /// For each file matching a sibling rule (directory pattern + file pattern),
    /// verifies that a sibling file matching the template exists in the same directory.
    ///
    /// # Returns
    /// A vector of `MissingSibling` violations for files without required siblings.
    #[must_use]
    pub fn check_siblings(&self, files: &[PathBuf]) -> Vec<StructureViolation> {
        if self.sibling_rules.is_empty() {
            return Vec::new();
        }

        // Build a HashSet of all file paths for O(1) sibling lookup
        let file_set: HashSet<&PathBuf> = files.iter().collect();

        let mut violations = Vec::new();

        for file_path in files {
            let Some(parent) = file_path.parent() else {
                continue;
            };
            let Some(file_name) = file_path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };

            for rule in &self.sibling_rules {
                // Check if parent directory matches the rule's directory pattern
                if !rule.dir_matcher.is_match(parent) {
                    continue;
                }

                // Check if filename matches the rule's file pattern
                if !rule.file_matcher.is_match(file_name) {
                    continue;
                }

                // Derive expected sibling path
                if let Some(expected_sibling) =
                    Self::derive_sibling_path(file_path, &rule.sibling_template)
                    && !file_set.contains(&expected_sibling)
                {
                    violations.push(StructureViolation::missing_sibling(
                        file_path.clone(),
                        rule.dir_scope.clone(),
                        rule.sibling_template.clone(),
                    ));
                }
            }
        }

        // Sort by path for consistent output
        violations.sort_by(|a, b| a.path.cmp(&b.path));
        violations
    }

    /// Derive sibling path from source file and template.
    ///
    /// Template syntax: `{stem}` is replaced with the source file's stem.
    #[allow(clippy::literal_string_with_formatting_args)] // {stem} is a template placeholder, not a format arg
    fn derive_sibling_path(source: &Path, template: &str) -> Option<PathBuf> {
        let parent = source.parent()?;
        let stem = source.file_stem()?.to_str()?;

        let sibling_name = template.replace("{stem}", stem);
        Some(parent.join(sibling_name))
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
            let matches = path_matches_override(path, &ovr.path);
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
                    pattern: rule.scope.clone(),
                };
                MatchStatus::Matched
            } else if matches {
                MatchStatus::Superseded
            } else {
                MatchStatus::NoMatch
            };

            rule_chain.push(StructureRuleCandidate {
                source: format!("structure.rules[{i}]"),
                pattern: Some(rule.scope.clone()),
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

        // Get effective limits using the same logic as resolve_limits
        let limits = self.resolve_limits(path);

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
mod tests;
