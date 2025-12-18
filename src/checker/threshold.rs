use std::collections::HashMap;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobMatcher};

use crate::analyzer::SplitSuggestion;
use crate::config::Config;
use crate::counter::LineStats;

use super::Checker;

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
            Self::Passed { override_reason, .. }
            | Self::Warning { override_reason, .. }
            | Self::Failed { override_reason, .. }
            | Self::Grandfathered { override_reason, .. } => override_reason.as_deref(),
        }
    }

    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
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
    #[allow(clippy::cast_precision_loss)]
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
}

pub struct ThresholdChecker {
    config: Config,
    warning_threshold: f64,
    extension_limits: HashMap<String, usize>,
    extension_warn_thresholds: HashMap<String, f64>,
    path_rules: Vec<CompiledPathRule>,
}

impl ThresholdChecker {
    #[must_use]
    pub fn new(config: Config) -> Self {
        let extension_limits = Self::build_extension_limits(&config);
        let extension_warn_thresholds = Self::build_extension_warn_thresholds(&config);
        let path_rules = Self::build_path_rules(&config);
        Self {
            config,
            warning_threshold: 0.9,
            extension_limits,
            extension_warn_thresholds,
            path_rules,
        }
    }

    #[must_use]
    pub const fn with_warning_threshold(mut self, threshold: f64) -> Self {
        self.warning_threshold = threshold;
        self
    }

    fn build_extension_limits(config: &Config) -> HashMap<String, usize> {
        let mut index = HashMap::new();
        for rule in config.rules.values() {
            if let Some(max_lines) = rule.max_lines {
                for ext in &rule.extensions {
                    index.insert(ext.clone(), max_lines);
                }
            }
        }
        index
    }

    fn build_extension_warn_thresholds(config: &Config) -> HashMap<String, f64> {
        let mut index = HashMap::new();
        for rule in config.rules.values() {
            if let Some(warn_threshold) = rule.warn_threshold {
                for ext in &rule.extensions {
                    index.insert(ext.clone(), warn_threshold);
                }
            }
        }
        index
    }

    fn build_path_rules(config: &Config) -> Vec<CompiledPathRule> {
        config
            .path_rules
            .iter()
            .filter_map(|rule| {
                Glob::new(&rule.pattern).ok().map(|glob| CompiledPathRule {
                    matcher: glob.compile_matcher(),
                    max_lines: rule.max_lines,
                    warn_threshold: rule.warn_threshold,
                })
            })
            .collect()
    }

    fn path_matches_override(file_path: &Path, override_path: &str) -> bool {
        let override_components: Vec<&str> = override_path
            .split(['/', '\\'])
            .filter(|s| !s.is_empty())
            .collect();

        let file_components: Vec<_> = file_path.components().collect();

        if override_components.is_empty() || override_components.len() > file_components.len() {
            return false;
        }

        file_components
            .iter()
            .rev()
            .zip(override_components.iter().rev())
            .all(|(file_comp, override_comp)| {
                file_comp.as_os_str().to_string_lossy() == *override_comp
            })
    }

    /// Returns (`max_lines`, `override_reason`) for a path.
    fn get_limit_for_path(&self, path: &Path) -> (usize, Option<String>) {
        // 1. Check overrides first (highest priority)
        for override_config in &self.config.overrides {
            if Self::path_matches_override(path, &override_config.path) {
                return (override_config.max_lines, override_config.reason.clone());
            }
        }

        // 2. Check path_rules (glob patterns)
        for path_rule in &self.path_rules {
            if path_rule.matcher.is_match(path) {
                return (path_rule.max_lines, None);
            }
        }

        // 3. Check extension rules
        if let Some(ext) = path.extension().and_then(|e| e.to_str())
            && let Some(&limit) = self.extension_limits.get(ext)
        {
            return (limit, None);
        }

        // 4. Fall back to default
        (self.config.default.max_lines, None)
    }

    fn get_warn_threshold_for_path(&self, path: &Path) -> f64 {
        // 1. Check path_rules for custom warn_threshold (higher priority)
        for path_rule in &self.path_rules {
            if path_rule.matcher.is_match(path)
                && let Some(threshold) = path_rule.warn_threshold
            {
                return threshold;
            }
        }

        // 2. Check extension rules
        if let Some(ext) = path.extension().and_then(|e| e.to_str())
            && let Some(&threshold) = self.extension_warn_thresholds.get(ext)
        {
            return threshold;
        }

        // 3. Fall back to default
        self.warning_threshold
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
