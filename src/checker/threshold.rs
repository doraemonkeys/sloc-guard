use std::collections::HashSet;
use std::path::Path;

use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::config::Config;
use crate::counter::LineStats;

use super::Checker;
use super::explain::{
    ContentExplanation, ContentRuleCandidate, ContentRuleMatch, MatchStatus, WarnAtSource,
};
use super::result::CheckResult;

/// Compiled rule data (pattern metadata stored separately from matcher).
struct CompiledPathRule {
    max_lines: usize,
    warn_threshold: Option<f64>,
    warn_at: Option<usize>,
    skip_comments: Option<bool>,
    skip_blank: Option<bool>,
    reason: Option<String>,
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
    /// Creates a new `ThresholdChecker` from configuration.
    ///
    /// # Errors
    /// Returns `SlocGuardError::InvalidPattern` if any glob pattern in
    /// `content.exclude` or `content.rules[].pattern` is invalid.
    pub fn new(config: Config) -> crate::Result<Self> {
        let (path_rules, path_rules_set) = Self::build_path_rules(&config)?;
        let allowed_extensions: HashSet<String> =
            config.content.extensions.iter().cloned().collect();
        let content_exclude = Self::build_content_exclude(&config)?;
        Ok(Self {
            config,
            warning_threshold: 0.9,
            path_rules,
            path_rules_set,
            allowed_extensions,
            content_exclude,
        })
    }

    /// Build glob set for content exclusion patterns.
    /// First invalid pattern fails immediately (fail-fast).
    fn build_content_exclude(config: &Config) -> crate::Result<GlobSet> {
        let mut builder = GlobSetBuilder::new();
        for pattern in &config.content.exclude {
            let glob = Glob::new(pattern).map_err(|source| {
                crate::error::SlocGuardError::InvalidPattern {
                    pattern: pattern.clone(),
                    source,
                }
            })?;
            builder.add(glob);
        }
        builder.build().map_err(|source| {
            // GlobSet build error includes pattern info in its message
            crate::error::SlocGuardError::InvalidPattern {
                pattern: "<combined globset>".to_string(),
                source,
            }
        })
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
    ///   File matches any rule pattern)
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
    /// First invalid pattern fails immediately (fail-fast).
    fn build_path_rules(config: &Config) -> crate::Result<(Vec<CompiledPathRule>, GlobSet)> {
        let mut rules = Vec::new();
        let mut builder = GlobSetBuilder::new();

        // Process content.rules (V2 format)
        for rule in &config.content.rules {
            let glob = Glob::new(&rule.pattern).map_err(|source| {
                crate::error::SlocGuardError::InvalidPattern {
                    pattern: rule.pattern.clone(),
                    source,
                }
            })?;
            builder.add(glob);
            rules.push(CompiledPathRule {
                max_lines: rule.max_lines,
                warn_threshold: rule.warn_threshold,
                warn_at: rule.warn_at,
                skip_comments: rule.skip_comments,
                skip_blank: rule.skip_blank,
                reason: rule.reason.clone(),
            });
        }

        let globset =
            builder
                .build()
                .map_err(|source| crate::error::SlocGuardError::InvalidPattern {
                    pattern: "<combined globset>".to_string(),
                    source,
                })?;
        Ok((rules, globset))
    }

    /// Returns (`max_lines`, `override_reason`) for a path.
    fn get_limit_for_path(&self, path: &Path) -> (usize, Option<String>) {
        // Check path_rules (glob patterns) - last match wins
        // Use GlobSet::matches() for O(n) matching where n = path length
        let matches = self.path_rules_set.matches(path);
        if let Some(&last_idx) = matches.last() {
            let rule = &self.path_rules[last_idx];
            return (rule.max_lines, rule.reason.clone());
        }

        // Fall back to content.max_lines
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

    /// Calculate the effective warn limit (absolute line count) for a path.
    ///
    /// Priority:
    /// 1. `rule.warn_at` → absolute value
    /// 2. `rule.warn_threshold` → calculate `rule.max_lines * threshold`
    /// 3. `global.warn_at` → absolute value
    /// 4. `global.warn_threshold` → calculate `effective_max_lines * threshold`
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    fn get_warn_limit_for_path(&self, path: &Path, effective_limit: usize) -> usize {
        self.get_warn_limit_with_source(path, effective_limit).0
    }

    /// Calculate the effective warn limit and its source for a path.
    ///
    /// Returns `(warn_limit, WarnAtSource)` for debugging/explain output.
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    fn get_warn_limit_with_source(
        &self,
        path: &Path,
        effective_limit: usize,
    ) -> (usize, WarnAtSource) {
        // Check path_rules (last match wins) via GlobSet
        let matches = self.path_rules_set.matches(path);
        if let Some(&last_idx) = matches.last() {
            let rule = &self.path_rules[last_idx];

            // 1. rule.warn_at takes highest precedence
            if let Some(warn_at) = rule.warn_at {
                return (warn_at, WarnAtSource::RuleAbsolute { index: last_idx });
            }

            // 2. rule.warn_threshold (percentage of rule's max_lines)
            if let Some(threshold) = rule.warn_threshold {
                return (
                    (rule.max_lines as f64 * threshold).ceil() as usize,
                    WarnAtSource::RulePercentage {
                        index: last_idx,
                        threshold,
                    },
                );
            }
        }

        // 3. global.warn_at (content.warn_at)
        if let Some(warn_at) = self.config.content.warn_at {
            return (warn_at, WarnAtSource::GlobalAbsolute);
        }

        // 4. global.warn_threshold (calculated from effective limit)
        (
            (effective_limit as f64 * self.warning_threshold).ceil() as usize,
            WarnAtSource::GlobalPercentage {
                threshold: self.warning_threshold,
            },
        )
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
                effective_warn_at: 0,
                warn_at_source: WarnAtSource::GlobalPercentage {
                    threshold: self.warning_threshold,
                },
                warn_threshold: self.get_warn_threshold_for_path(path),
                skip_comments,
                skip_blank,
                rule_chain: Vec::new(),
            };
        }

        let mut rule_chain = Vec::new();
        let mut matched_rule = ContentRuleMatch::Default;
        let mut found_match = false;

        // Check content.rules (last match wins) via GlobSet
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
                    reason: rule.reason.clone(),
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
        let effective_limit = self.get_limit_for_path(path).0;
        let (effective_warn_at, warn_at_source) =
            self.get_warn_limit_with_source(path, effective_limit);

        ContentExplanation {
            path: path.to_path_buf(),
            is_excluded: false,
            matched_rule,
            effective_limit,
            effective_warn_at,
            warn_at_source,
            warn_threshold: self.get_warn_threshold_for_path(path),
            skip_comments,
            skip_blank,
            rule_chain,
        }
    }

    /// Find the first matching content.exclude pattern for a path.
    /// Uses pre-compiled `GlobSet` for O(1) lookup.
    fn find_matching_exclude_pattern(&self, path: &Path) -> Option<String> {
        let matches = self.content_exclude.matches(path);
        matches
            .first()
            .map(|&idx| self.config.content.exclude[idx].clone())
    }
}

impl Checker for ThresholdChecker {
    fn check(
        &self,
        path: &Path,
        line_stats: &LineStats,
        raw_stats: Option<&LineStats>,
    ) -> CheckResult {
        let (limit, override_reason) = self.get_limit_for_path(path);
        let warn_limit = self.get_warn_limit_for_path(path, limit);
        let sloc = line_stats.sloc();
        let path = path.to_path_buf();
        let stats = line_stats.clone();
        let raw_stats = raw_stats.cloned();

        if sloc > limit {
            CheckResult::Failed {
                path,
                stats,
                raw_stats,
                limit,
                override_reason,
                suggestions: None,
                violation_category: None, // Content violations don't need explicit category
            }
        } else if sloc >= warn_limit {
            CheckResult::Warning {
                path,
                stats,
                raw_stats,
                limit,
                override_reason,
                suggestions: None,
                violation_category: None,
            }
        } else {
            CheckResult::Passed {
                path,
                stats,
                raw_stats,
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
