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

use crate::config::{StructureConfig, UNLIMITED};
use crate::error::Result;

use super::explain::{
    MatchStatus, StructureExplanation, StructureRuleCandidate, StructureRuleMatch,
};
pub use violation::{DirStats, StructureViolation, ViolationType};

use builder::{build_rules, build_sibling_rules};
use compiled_rules::{CompiledSiblingRule, CompiledStructureRule, StructureLimits};
use validation::{validate_allow_deny_mutual_exclusion, validate_limits, validate_sibling_rules};

/// Checker for directory structure limits.
pub struct StructureChecker {
    max_files: Option<i64>,
    max_dirs: Option<i64>,
    max_depth: Option<i64>,
    warn_threshold: Option<f64>,
    warn_files_at: Option<i64>,
    warn_dirs_at: Option<i64>,
    warn_files_threshold: Option<f64>,
    warn_dirs_threshold: Option<f64>,
    rules: Vec<CompiledStructureRule>,
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
        validate_allow_deny_mutual_exclusion(config)?;
        let rules = build_rules(&config.rules)?;
        let sibling_rules = build_sibling_rules(&config.rules)?;

        Ok(Self {
            max_files: config.max_files,
            max_dirs: config.max_dirs,
            max_depth: config.max_depth,
            warn_threshold: config.warn_threshold,
            warn_files_at: config.warn_files_at,
            warn_dirs_at: config.warn_dirs_at,
            warn_files_threshold: config.warn_files_threshold,
            warn_dirs_threshold: config.warn_dirs_threshold,
            rules,
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
    }

    /// Get limits for a directory path.
    /// Returns a `StructureLimits` struct with all applicable limits.
    /// A limit of `-1` (UNLIMITED) means no check should be performed.
    ///
    /// # Priority Chain (high → low)
    ///
    /// 1. `[[structure.rules]]` - glob pattern, last match wins
    /// 2. `[structure]` defaults (lowest)
    ///
    /// # Glob Semantics (structure rules only match directories)
    ///
    /// - `src/components/*`  — matches DIRECT children only (e.g., `Button/`, `Icon/`)
    /// - `src/components/**` — matches ALL descendants recursively
    /// - `src/features`      — exact directory match only
    fn resolve_limits(&self, path: &Path) -> StructureLimits {
        // Check rules (glob patterns) - last match wins
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
                    warn_files_at: rule.warn_files_at.or(self.warn_files_at),
                    warn_dirs_at: rule.warn_dirs_at.or(self.warn_dirs_at),
                    warn_files_threshold: rule.warn_files_threshold.or(self.warn_files_threshold),
                    warn_dirs_threshold: rule.warn_dirs_threshold.or(self.warn_dirs_threshold),
                    override_reason: rule.reason.clone(),
                };
            }
        }

        // Fall back to global defaults
        StructureLimits {
            max_files: self.max_files,
            max_dirs: self.max_dirs,
            max_depth: self.max_depth,
            relative_depth: false,
            base_depth: 0,
            warn_threshold: self.warn_threshold,
            warn_files_at: self.warn_files_at,
            warn_dirs_at: self.warn_dirs_at,
            warn_files_threshold: self.warn_files_threshold,
            warn_dirs_threshold: self.warn_dirs_threshold,
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
        /// Default warn threshold when none specified.
        const DEFAULT_WARN_THRESHOLD: f64 = 0.8;

        let mut violations = Vec::new();

        for (path, stats) in dir_stats {
            let limits = self.resolve_limits(path);

            // Check file count (skip if unlimited)
            if let Some(limit) = limits.max_files
                && limit != UNLIMITED
            {
                let limit_usize = limit as usize;
                // Warn threshold fallback: absolute → percentage → global → default 0.8
                let warn_limit = Self::calculate_warn_limit(
                    limit,
                    limits.warn_files_at,
                    limits.warn_files_threshold,
                    limits.warn_threshold,
                    DEFAULT_WARN_THRESHOLD,
                );

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
                // Warn threshold fallback: absolute → percentage → global → default 0.8
                let warn_limit = Self::calculate_warn_limit(
                    limit,
                    limits.warn_dirs_at,
                    limits.warn_dirs_threshold,
                    limits.warn_threshold,
                    DEFAULT_WARN_THRESHOLD,
                );

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
            // Note: depth uses global warn_threshold only (no granular threshold)
            if let Some(limit) = limits.max_depth
                && limit != UNLIMITED
            {
                let limit_usize = limit as usize;
                let warn_limit = limits.warn_threshold.unwrap_or(DEFAULT_WARN_THRESHOLD);
                let warn_limit = ((limit as f64) * warn_limit).ceil() as usize;

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

    /// Calculate the warn limit using the fallback chain:
    /// absolute → percentage → global → default
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    fn calculate_warn_limit(
        limit: i64,
        absolute: Option<i64>,
        percentage: Option<f64>,
        global_threshold: Option<f64>,
        default_threshold: f64,
    ) -> usize {
        // 1. Absolute threshold takes highest precedence
        if let Some(abs) = absolute {
            return abs as usize;
        }

        // 2. Per-metric percentage threshold
        if let Some(pct) = percentage {
            return ((limit as f64) * pct).ceil() as usize;
        }

        // 3. Global warn_threshold
        if let Some(global) = global_threshold {
            return ((limit as f64) * global).ceil() as usize;
        }

        // 4. Default threshold (0.8)
        ((limit as f64) * default_threshold).ceil() as usize
    }

    /// Check files for missing siblings.
    ///
    /// Supports two rule types:
    /// - **Directed**: If a file matches the pattern, require specific sibling(s).
    /// - **Group**: If ANY file in the group exists, ALL must exist.
    ///
    /// # Returns
    /// A vector of sibling violations (`MissingSibling` or `GroupIncomplete`).
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
                match rule {
                    CompiledSiblingRule::Directed {
                        dir_scope,
                        dir_matcher,
                        file_matcher,
                        sibling_templates,
                        is_warning,
                    } => {
                        // Check if parent directory matches the rule's directory pattern
                        if !dir_matcher.is_match(parent) {
                            continue;
                        }

                        // Check if filename matches the rule's file pattern
                        if !file_matcher.is_match(file_name) {
                            continue;
                        }

                        // Check each required sibling
                        for template in sibling_templates {
                            if let Some(expected_sibling) =
                                Self::derive_sibling_path(file_path, template)
                                && !file_set.contains(&expected_sibling)
                            {
                                let violation = if *is_warning {
                                    StructureViolation::missing_sibling_warning(
                                        file_path.clone(),
                                        dir_scope.clone(),
                                        template.clone(),
                                    )
                                } else {
                                    StructureViolation::missing_sibling(
                                        file_path.clone(),
                                        dir_scope.clone(),
                                        template.clone(),
                                    )
                                };
                                violations.push(violation);
                            }
                        }
                    }
                    #[allow(clippy::literal_string_with_formatting_args)]
                    // {stem} is template syntax, not a format arg
                    CompiledSiblingRule::Group {
                        dir_scope,
                        dir_matcher,
                        group_patterns,
                        is_warning,
                    } => {
                        // Check if parent directory matches the rule's directory pattern
                        if !dir_matcher.is_match(parent) {
                            continue;
                        }

                        // Try ALL patterns to extract possible stems, then find the best one
                        // (the one that results in the most complete group)
                        let stems: Vec<String> = group_patterns
                            .iter()
                            .filter_map(|pattern| {
                                Self::extract_stem_from_pattern(file_name, pattern)
                            })
                            .collect();

                        if stems.is_empty() {
                            // File doesn't match any pattern in the group
                            continue;
                        }

                        // Find the stem that results in the most complete group
                        // (fewest missing files, prefer complete groups)
                        let best_stem = stems.iter().min_by_key(|stem| {
                            group_patterns
                                .iter()
                                .filter(|pattern| {
                                    let expected_name = pattern.replace("{stem}", stem);
                                    let expected_path = parent.join(&expected_name);
                                    !file_set.contains(&expected_path)
                                })
                                .count()
                        });

                        let Some(stem) = best_stem else {
                            continue;
                        };

                        // This file triggers the group check - find missing members using the best stem
                        let missing: Vec<String> = group_patterns
                            .iter()
                            .filter(|pattern| {
                                let expected_name = pattern.replace("{stem}", stem);
                                let expected_path = parent.join(&expected_name);
                                !file_set.contains(&expected_path)
                            })
                            .cloned()
                            .collect();

                        if !missing.is_empty() {
                            let violation = if *is_warning {
                                StructureViolation::group_incomplete_warning(
                                    file_path.clone(),
                                    dir_scope.clone(),
                                    group_patterns.clone(),
                                    missing,
                                )
                            } else {
                                StructureViolation::group_incomplete(
                                    file_path.clone(),
                                    dir_scope.clone(),
                                    group_patterns.clone(),
                                    missing,
                                )
                            };
                            violations.push(violation);
                        }
                    }
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
    #[allow(clippy::literal_string_with_formatting_args)] // {stem} is template syntax, not a format arg
    fn derive_sibling_path(source: &Path, template: &str) -> Option<PathBuf> {
        let parent = source.parent()?;
        let stem = source.file_stem()?.to_str()?;

        let sibling_name = template.replace("{stem}", stem);
        Some(parent.join(sibling_name))
    }

    /// Extract the stem value from a file name matching a pattern.
    ///
    /// Template syntax: `{stem}` is a placeholder. The function checks if the file name
    /// matches the pattern and extracts what `{stem}` represents.
    ///
    /// Examples:
    /// - `Button.tsx` matching `{stem}.tsx` → `Some("Button")`
    /// - `Button.test.tsx` matching `{stem}.test.tsx` → `Some("Button")`
    /// - `Button.tsx` matching `{stem}.test.tsx` → `None` (no match)
    #[allow(clippy::literal_string_with_formatting_args)] // {stem} is template syntax, not a format arg
    fn extract_stem_from_pattern(file_name: &str, pattern: &str) -> Option<String> {
        // Split the pattern by {stem} to get prefix and suffix
        let parts: Vec<&str> = pattern.split("{stem}").collect();
        if parts.len() != 2 {
            // Pattern must contain exactly one {stem}
            return None;
        }
        let (prefix, suffix) = (parts[0], parts[1]);

        // Check if file_name starts with prefix and ends with suffix
        if !file_name.starts_with(prefix) {
            return None;
        }
        if !file_name.ends_with(suffix) {
            return None;
        }

        // Extract the stem (the part between prefix and suffix)
        let stem_start = prefix.len();
        let stem_end = file_name.len() - suffix.len();

        if stem_start > stem_end {
            return None;
        }

        Some(file_name[stem_start..stem_end].to_string())
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

        // Check rules (last match wins for consistency with content rules)
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
                override_reason.clone_from(&rule.reason);
                matched_rule = StructureRuleMatch::Rule {
                    index: i,
                    pattern: rule.scope.clone(),
                    reason: rule.reason.clone(),
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

        // Add default
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
mod check_tests;
