use std::collections::HashSet;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobMatcher};

use crate::analyzer::SplitSuggestion;
use crate::config::Config;
use crate::counter::LineStats;
use crate::path_utils::path_matches_override;

use super::Checker;
use super::explain::{ContentExplanation, ContentRuleCandidate, ContentRuleMatch, MatchStatus};

/// Result of checking a file against configured thresholds.
///
/// Each variant represents a distinct check outcome. The `suggestions` field is only
/// available on `Warning` and `Failed` variants, making it impossible to have suggestions
/// on passed or grandfathered results.
#[derive(Debug, Clone)]
pub enum CheckResult {
    Passed {
        path: PathBuf,
        stats: LineStats,
        limit: usize,
        override_reason: Option<String>,
    },
    Warning {
        path: PathBuf,
        stats: LineStats,
        limit: usize,
        override_reason: Option<String>,
        suggestions: Option<SplitSuggestion>,
    },
    Failed {
        path: PathBuf,
        stats: LineStats,
        limit: usize,
        override_reason: Option<String>,
        suggestions: Option<SplitSuggestion>,
    },
    Grandfathered {
        path: PathBuf,
        stats: LineStats,
        limit: usize,
        override_reason: Option<String>,
    },
}

impl CheckResult {
    // Accessor methods

    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            Self::Passed { path, .. }
            | Self::Warning { path, .. }
            | Self::Failed { path, .. }
            | Self::Grandfathered { path, .. } => path,
        }
    }

    #[must_use]
    pub const fn stats(&self) -> &LineStats {
        match self {
            Self::Passed { stats, .. }
            | Self::Warning { stats, .. }
            | Self::Failed { stats, .. }
            | Self::Grandfathered { stats, .. } => stats,
        }
    }

    #[must_use]
    pub const fn limit(&self) -> usize {
        match self {
            Self::Passed { limit, .. }
            | Self::Warning { limit, .. }
            | Self::Failed { limit, .. }
            | Self::Grandfathered { limit, .. } => *limit,
        }
    }

    #[must_use]
    pub fn override_reason(&self) -> Option<&str> {
        match self {
            Self::Passed {
                override_reason, ..
            }
            | Self::Warning {
                override_reason, ..
            }
            | Self::Failed {
                override_reason, ..
            }
            | Self::Grandfathered {
                override_reason, ..
            } => override_reason.as_deref(),
        }
    }

    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Accessing option reference isn't const
    pub fn suggestions(&self) -> Option<&SplitSuggestion> {
        match self {
            Self::Warning { suggestions, .. } | Self::Failed { suggestions, .. } => {
                suggestions.as_ref()
            }
            Self::Passed { .. } | Self::Grandfathered { .. } => None,
        }
    }

    // Predicate methods

    #[must_use]
    pub const fn is_passed(&self) -> bool {
        matches!(self, Self::Passed { .. })
    }

    #[must_use]
    pub const fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    #[must_use]
    pub const fn is_warning(&self) -> bool {
        matches!(self, Self::Warning { .. })
    }

    #[must_use]
    pub const fn is_grandfathered(&self) -> bool {
        matches!(self, Self::Grandfathered { .. })
    }

    // Transformation methods

    /// Convert a Failed result to Grandfathered (used for baseline comparison).
    /// Returns self unchanged if not Failed.
    #[must_use]
    pub fn into_grandfathered(self) -> Self {
        match self {
            Self::Failed {
                path,
                stats,
                limit,
                override_reason,
                ..
            } => Self::Grandfathered {
                path,
                stats,
                limit,
                override_reason,
            },
            other => other,
        }
    }

    /// Add split suggestions to a Warning or Failed result.
    /// Returns self unchanged if Passed or Grandfathered.
    #[must_use]
    pub fn with_suggestions(self, new_suggestions: SplitSuggestion) -> Self {
        match self {
            Self::Warning {
                path,
                stats,
                limit,
                override_reason,
                ..
            } => Self::Warning {
                path,
                stats,
                limit,
                override_reason,
                suggestions: Some(new_suggestions),
            },
            Self::Failed {
                path,
                stats,
                limit,
                override_reason,
                ..
            } => Self::Failed {
                path,
                stats,
                limit,
                override_reason,
                suggestions: Some(new_suggestions),
            },
            other => other,
        }
    }

    #[must_use]
    #[allow(clippy::cast_precision_loss)] // Precision loss is acceptable for usage percentage
    pub fn usage_percent(&self) -> f64 {
        let limit = self.limit();
        if limit == 0 {
            return 0.0;
        }
        (self.stats().sloc() as f64 / limit as f64) * 100.0
    }
}

struct CompiledPathRule {
    matcher: GlobMatcher,
    max_lines: usize,
    warn_threshold: Option<f64>,
    skip_comments: Option<bool>,
    skip_blank: Option<bool>,
}

pub struct ThresholdChecker {
    config: Config,
    warning_threshold: f64,
    path_rules: Vec<CompiledPathRule>,
    /// Extensions to process (from content.extensions).
    /// Empty set means process all files.
    allowed_extensions: HashSet<String>,
}

impl ThresholdChecker {
    #[must_use]
    pub fn new(config: Config) -> Self {
        let path_rules = Self::build_path_rules(&config);
        let allowed_extensions: HashSet<String> =
            config.content.extensions.iter().cloned().collect();
        Self {
            config,
            warning_threshold: 0.9,
            path_rules,
            allowed_extensions,
        }
    }

    #[must_use]
    pub const fn with_warning_threshold(mut self, threshold: f64) -> Self {
        self.warning_threshold = threshold;
        self
    }

    /// Check if a file should be processed based on extension or rule match.
    ///
    /// A file is processed if:
    /// - `content.extensions` is empty (no filter), OR
    /// - File extension is in `content.extensions`, OR
    /// - File matches any content override or rule pattern
    ///
    /// This ensures extension-less files (Dockerfile, Jenkinsfile, etc.) can be
    /// checked if there's an explicit rule targeting them.
    #[must_use]
    pub fn should_process(&self, path: &Path) -> bool {
        if self.allowed_extensions.is_empty() {
            return true; // No filter = process all files
        }

        // Check if extension matches
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| self.allowed_extensions.contains(ext))
        {
            return true;
        }

        // Check if file matches any content override
        for override_config in &self.config.content.overrides {
            if path_matches_override(path, &override_config.path) {
                return true;
            }
        }

        // Check if file matches any legacy override
        for override_config in &self.config.overrides {
            if path_matches_override(path, &override_config.path) {
                return true;
            }
        }

        // Check if file matches any rule pattern
        for rule in &self.path_rules {
            if rule.matcher.is_match(path) {
                return true;
            }
        }

        false
    }

    fn build_path_rules(config: &Config) -> Vec<CompiledPathRule> {
        let mut rules = Vec::new();

        // Process content.rules (V2 format)
        for rule in &config.content.rules {
            if let Ok(glob) = Glob::new(&rule.pattern) {
                rules.push(CompiledPathRule {
                    matcher: glob.compile_matcher(),
                    max_lines: rule.max_lines,
                    warn_threshold: rule.warn_threshold,
                    skip_comments: rule.skip_comments,
                    skip_blank: rule.skip_blank,
                });
            }
        }

        rules
    }

    /// Returns (`max_lines`, `override_reason`) for a path.
    fn get_limit_for_path(&self, path: &Path) -> (usize, Option<String>) {
        // 1. Check content.overrides first (highest priority, V2)
        for override_config in &self.config.content.overrides {
            if path_matches_override(path, &override_config.path) {
                return (
                    override_config.max_lines,
                    Some(override_config.reason.clone()),
                );
            }
        }

        // 2. Check legacy overrides (for V1 migration)
        for override_config in &self.config.overrides {
            if path_matches_override(path, &override_config.path) {
                return (override_config.max_lines, override_config.reason.clone());
            }
        }

        // 3. Check path_rules (glob patterns) - last match wins
        // Iterate in reverse to find the last matching rule
        for path_rule in self.path_rules.iter().rev() {
            if path_rule.matcher.is_match(path) {
                return (path_rule.max_lines, None);
            }
        }

        // 4. Fall back to content.max_lines (V2) with legacy fallback
        (self.config.content.max_lines, None)
    }

    fn get_warn_threshold_for_path(&self, path: &Path) -> f64 {
        // 1. Check path_rules for custom warn_threshold (last match wins)
        for path_rule in self.path_rules.iter().rev() {
            if path_rule.matcher.is_match(path)
                && let Some(threshold) = path_rule.warn_threshold
            {
                return threshold;
            }
        }

        // 2. Fall back to instance warning_threshold
        // (set via with_warning_threshold or from config during initialization)
        self.warning_threshold
    }

    /// Returns (`skip_comments`, `skip_blank`) settings for a path.
    /// Priority: `path_rules` (last match) > global defaults
    #[must_use]
    pub fn get_skip_settings_for_path(&self, path: &Path) -> (bool, bool) {
        // Check path_rules (last match wins)
        for path_rule in self.path_rules.iter().rev() {
            if path_rule.matcher.is_match(path) {
                let skip_comments = path_rule
                    .skip_comments
                    .unwrap_or(self.config.content.skip_comments);
                let skip_blank = path_rule
                    .skip_blank
                    .unwrap_or(self.config.content.skip_blank);
                return (skip_comments, skip_blank);
            }
        }

        // Fall back to global defaults
        (
            self.config.content.skip_comments,
            self.config.content.skip_blank,
        )
    }

    /// Explain which rule matches a given file path.
    ///
    /// Returns a detailed breakdown of all evaluated rules and which one won.
    #[must_use]
    pub fn explain(&self, path: &Path) -> ContentExplanation {
        let mut rule_chain = Vec::new();
        let mut matched_rule = ContentRuleMatch::Default;
        let mut found_match = false;

        // 1. Check content.overrides (highest priority)
        for (i, ovr) in self.config.content.overrides.iter().enumerate() {
            let matches = path_matches_override(path, &ovr.path);
            let status = if matches && !found_match {
                found_match = true;
                matched_rule = ContentRuleMatch::Override {
                    index: i,
                    reason: ovr.reason.clone(),
                };
                MatchStatus::Matched
            } else if matches {
                MatchStatus::Superseded
            } else {
                MatchStatus::NoMatch
            };

            rule_chain.push(ContentRuleCandidate {
                source: format!("content.overrides[{i}]"),
                pattern: Some(ovr.path.clone()),
                limit: ovr.max_lines,
                status,
            });
        }

        // 2. Check legacy overrides (for V1 migration)
        for (i, ovr) in self.config.overrides.iter().enumerate() {
            let matches = path_matches_override(path, &ovr.path);
            let status = if matches && !found_match {
                found_match = true;
                matched_rule = ContentRuleMatch::Override {
                    index: i,
                    reason: ovr
                        .reason
                        .clone()
                        .unwrap_or_else(|| "legacy override".to_string()),
                };
                MatchStatus::Matched
            } else if matches {
                MatchStatus::Superseded
            } else {
                MatchStatus::NoMatch
            };

            rule_chain.push(ContentRuleCandidate {
                source: format!("overrides[{i}] (legacy)"),
                pattern: Some(ovr.path.clone()),
                limit: ovr.max_lines,
                status,
            });
        }

        // 3. Check content.rules (last match wins - iterate in reverse for display,
        //    but track the actual last match)
        // First, find the last matching rule index
        let mut last_match_idx: Option<usize> = None;
        for (i, rule) in self.path_rules.iter().enumerate() {
            if rule.matcher.is_match(path) {
                last_match_idx = Some(i);
            }
        }

        // Now build the chain with correct status
        for (i, rule) in self.path_rules.iter().enumerate() {
            let matches = rule.matcher.is_match(path);
            let is_selected = !found_match && last_match_idx == Some(i);

            let status = if is_selected {
                found_match = true;
                MatchStatus::Matched
            } else if matches {
                MatchStatus::Superseded
            } else {
                MatchStatus::NoMatch
            };

            let pattern = self.config.content.rules[i].pattern.clone();
            let source = format!("content.rules[{i}]");

            if is_selected {
                matched_rule = ContentRuleMatch::Rule {
                    index: i,
                    pattern: pattern.clone(),
                };
            }

            rule_chain.push(ContentRuleCandidate {
                source,
                pattern: Some(pattern),
                limit: rule.max_lines,
                status,
            });
        }

        // 4. Add default
        rule_chain.push(ContentRuleCandidate {
            source: "content.max_lines (default)".to_string(),
            pattern: None,
            limit: self.config.content.max_lines,
            status: if found_match {
                MatchStatus::Superseded
            } else {
                MatchStatus::Matched
            },
        });

        let (skip_comments, skip_blank) = self.get_skip_settings_for_path(path);

        ContentExplanation {
            path: path.to_path_buf(),
            matched_rule,
            effective_limit: self.get_limit_for_path(path).0,
            warn_threshold: self.get_warn_threshold_for_path(path),
            skip_comments,
            skip_blank,
            rule_chain,
        }
    }
}

impl Checker for ThresholdChecker {
    fn check(&self, path: &Path, line_stats: &LineStats) -> CheckResult {
        let (limit, override_reason) = self.get_limit_for_path(path);
        let warn_threshold = self.get_warn_threshold_for_path(path);
        let sloc = line_stats.sloc();
        let path = path.to_path_buf();
        let stats = line_stats.clone();

        #[allow(clippy::cast_precision_loss)]
        // Precision loss is acceptable for threshold comparison
        if sloc > limit {
            CheckResult::Failed {
                path,
                stats,
                limit,
                override_reason,
                suggestions: None,
            }
        } else if sloc as f64 >= limit as f64 * warn_threshold {
            CheckResult::Warning {
                path,
                stats,
                limit,
                override_reason,
                suggestions: None,
            }
        } else {
            CheckResult::Passed {
                path,
                stats,
                limit,
                override_reason,
            }
        }
    }
}

#[cfg(test)]
#[path = "threshold_tests.rs"]
mod tests;
