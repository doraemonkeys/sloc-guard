use std::collections::HashSet;
use std::path::Path;

use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::config::Config;
use crate::counter::LineStats;
use crate::path_utils::path_matches_override;

use super::Checker;
use super::explain::{ContentExplanation, ContentRuleCandidate, ContentRuleMatch, MatchStatus};
use super::result::CheckResult;

/// Compiled rule data (pattern metadata stored separately from matcher).
struct CompiledPathRule {
    max_lines: usize,
    warn_threshold: Option<f64>,
    skip_comments: Option<bool>,
    skip_blank: Option<bool>,
}

pub struct ThresholdChecker {
    config: Config,
    warning_threshold: f64,
    /// Rule data indexed by rule position.
    path_rules: Vec<CompiledPathRule>,
    /// Combined `GlobSet` for O(1) "any rule matches" check.
    /// Indices correspond to `path_rules` positions.
    path_rules_set: GlobSet,
    /// Extensions to process (from content.extensions).
    /// Empty set means process all files.
    allowed_extensions: HashSet<String>,
    /// Glob patterns for files to exclude from content checks.
    /// These files skip SLOC counting but remain visible for structure checks.
    content_exclude: GlobSet,
}

impl ThresholdChecker {
    #[must_use]
    pub fn new(config: Config) -> Self {
        let (path_rules, path_rules_set) = Self::build_path_rules(&config);
        let allowed_extensions: HashSet<String> =
            config.content.extensions.iter().cloned().collect();
        let content_exclude = Self::build_content_exclude(&config);
        Self {
            config,
            warning_threshold: 0.9,
            path_rules,
            path_rules_set,
            allowed_extensions,
            content_exclude,
        }
    }

    fn build_content_exclude(config: &Config) -> GlobSet {
        let mut builder = GlobSetBuilder::new();
        for pattern in &config.content.exclude {
            if let Ok(glob) = Glob::new(pattern) {
                builder.add(glob);
            }
        }
        builder.build().unwrap_or_else(|_| GlobSet::empty())
    }

    #[must_use]
    pub const fn with_warning_threshold(mut self, threshold: f64) -> Self {
        self.warning_threshold = threshold;
        self
    }

    /// Check if a file should be processed based on extension or rule match.
    ///
    /// A file is processed if:
    /// - NOT in `content.exclude` patterns, AND
    /// - (`content.extensions` is empty (no filter), OR
    ///   File extension is in `content.extensions`, OR
    ///   File matches any content override or rule pattern)
    ///
    /// This ensures extension-less files (Dockerfile, Jenkinsfile, etc.) can be
    /// checked if there's an explicit rule targeting them.
    #[must_use]
    pub fn should_process(&self, path: &Path) -> bool {
        // Check content exclusion FIRST - excluded files skip SLOC checks entirely
        if self.content_exclude.is_match(path) {
            return false;
        }

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

        // Check if file matches any rule pattern (O(1) via GlobSet)
        self.path_rules_set.is_match(path)
    }

    /// Check if a file is excluded from content checks via `content.exclude`.
    #[must_use]
    pub fn is_content_excluded(&self, path: &Path) -> bool {
        self.content_exclude.is_match(path)
    }

    /// Build path rules and a combined `GlobSet` for efficient matching.
    /// Returns `(rules_data, globset)` where `GlobSet` indices correspond to `rules_data` positions.
    fn build_path_rules(config: &Config) -> (Vec<CompiledPathRule>, GlobSet) {
        let mut rules = Vec::new();
        let mut builder = GlobSetBuilder::new();

        // Process content.rules (V2 format)
        for rule in &config.content.rules {
            if let Ok(glob) = Glob::new(&rule.pattern) {
                builder.add(glob);
                rules.push(CompiledPathRule {
                    max_lines: rule.max_lines,
                    warn_threshold: rule.warn_threshold,
                    skip_comments: rule.skip_comments,
                    skip_blank: rule.skip_blank,
                });
            }
        }

        let globset = builder.build().unwrap_or_else(|_| GlobSet::empty());
        (rules, globset)
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
        // Use GlobSet::matches() for O(n) matching where n = path length
        let matches = self.path_rules_set.matches(path);
        if let Some(&last_idx) = matches.last() {
            return (self.path_rules[last_idx].max_lines, None);
        }

        // 4. Fall back to content.max_lines (V2) with legacy fallback
        (self.config.content.max_lines, None)
    }

    fn get_warn_threshold_for_path(&self, path: &Path) -> f64 {
        // 1. Check path_rules (last match wins) via GlobSet
        let matches = self.path_rules_set.matches(path);
        if let Some(&last_idx) = matches.last() {
            return self.path_rules[last_idx]
                .warn_threshold
                .unwrap_or(self.warning_threshold);
        }

        // 2. Fall back to instance warning_threshold
        self.warning_threshold
    }

    /// Returns (`skip_comments`, `skip_blank`) settings for a path.
    /// Priority: `path_rules` (last match) > global defaults
    #[must_use]
    pub fn get_skip_settings_for_path(&self, path: &Path) -> (bool, bool) {
        // Check path_rules (last match wins) via GlobSet
        let matches = self.path_rules_set.matches(path);
        if let Some(&last_idx) = matches.last() {
            let path_rule = &self.path_rules[last_idx];
            let skip_comments = path_rule
                .skip_comments
                .unwrap_or(self.config.content.skip_comments);
            let skip_blank = path_rule
                .skip_blank
                .unwrap_or(self.config.content.skip_blank);
            return (skip_comments, skip_blank);
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
        // 0. Check content exclusion FIRST (highest priority)
        if let Some(pattern) = self.find_matching_exclude_pattern(path) {
            let (skip_comments, skip_blank) = self.get_skip_settings_for_path(path);
            return ContentExplanation {
                path: path.to_path_buf(),
                is_excluded: true,
                matched_rule: ContentRuleMatch::Excluded { pattern },
                effective_limit: 0,
                warn_threshold: self.get_warn_threshold_for_path(path),
                skip_comments,
                skip_blank,
                rule_chain: Vec::new(),
            };
        }

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

        // 3. Check content.rules (last match wins) via GlobSet
        // Get all matching indices in one pass
        let rule_matches = self.path_rules_set.matches(path);
        let matching_set: HashSet<usize> = rule_matches.iter().copied().collect();
        let last_match_idx = rule_matches.last().copied();

        // Now build the chain with correct status
        for (i, rule) in self.path_rules.iter().enumerate() {
            let matches = matching_set.contains(&i);
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
            is_excluded: false,
            matched_rule,
            effective_limit: self.get_limit_for_path(path).0,
            warn_threshold: self.get_warn_threshold_for_path(path),
            skip_comments,
            skip_blank,
            rule_chain,
        }
    }

    /// Find the first matching content.exclude pattern for a path.
    fn find_matching_exclude_pattern(&self, path: &Path) -> Option<String> {
        for pattern in &self.config.content.exclude {
            if let Ok(glob) = Glob::new(pattern)
                && glob.compile_matcher().is_match(path)
            {
                return Some(pattern.clone());
            }
        }
        None
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
                violation_category: None, // Content violations don't need explicit category
            }
        } else if sloc as f64 >= limit as f64 * warn_threshold {
            CheckResult::Warning {
                path,
                stats,
                limit,
                override_reason,
                suggestions: None,
                violation_category: None,
            }
        } else {
            CheckResult::Passed {
                path,
                stats,
                limit,
                override_reason,
                violation_category: None,
            }
        }
    }
}

#[cfg(test)]
#[path = "threshold_tests/mod.rs"]
mod tests;
