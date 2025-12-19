use std::collections::{HashMap, HashSet};
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
    /// Extensions to process (from content.extensions).
    /// Empty set means process all files.
    allowed_extensions: HashSet<String>,
}

impl ThresholdChecker {
    #[must_use]
    pub fn new(config: Config) -> Self {
        let extension_limits = Self::build_extension_limits(&config);
        let extension_warn_thresholds = Self::build_extension_warn_thresholds(&config);
        let path_rules = Self::build_path_rules(&config);
        let allowed_extensions: HashSet<String> =
            config.content.extensions.iter().cloned().collect();
        Self {
            config,
            warning_threshold: 0.9,
            extension_limits,
            extension_warn_thresholds,
            path_rules,
            allowed_extensions,
        }
    }

    #[must_use]
    pub const fn with_warning_threshold(mut self, threshold: f64) -> Self {
        self.warning_threshold = threshold;
        self
    }

    /// Check if a file should be processed based on its extension.
    /// Files with extensions not in `content.extensions` are skipped.
    #[must_use]
    pub fn should_process(&self, path: &Path) -> bool {
        if self.allowed_extensions.is_empty() {
            return true; // No filter = process all files
        }

        path.extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| self.allowed_extensions.contains(ext))
    }

    fn build_extension_limits(config: &Config) -> HashMap<String, usize> {
        let mut index = HashMap::new();

        // First, process legacy rules (lower priority)
        for rule in config.rules.values() {
            if let Some(max_lines) = rule.max_lines {
                for ext in &rule.extensions {
                    index.insert(ext.clone(), max_lines);
                }
            }
        }

        // Then, process content.languages (higher priority for V2)
        for (ext, lang_rule) in &config.content.languages {
            if let Some(max_lines) = lang_rule.max_lines {
                index.insert(ext.clone(), max_lines);
            }
        }

        index
    }

    fn build_extension_warn_thresholds(config: &Config) -> HashMap<String, f64> {
        let mut index = HashMap::new();

        // First, process legacy rules (lower priority)
        for rule in config.rules.values() {
            if let Some(warn_threshold) = rule.warn_threshold {
                for ext in &rule.extensions {
                    index.insert(ext.clone(), warn_threshold);
                }
            }
        }

        // Then, process content.languages (higher priority for V2)
        for (ext, lang_rule) in &config.content.languages {
            if let Some(warn_threshold) = lang_rule.warn_threshold {
                index.insert(ext.clone(), warn_threshold);
            }
        }

        index
    }

    fn build_path_rules(config: &Config) -> Vec<CompiledPathRule> {
        let mut rules = Vec::new();

        // First, process legacy path_rules (lower priority)
        for rule in &config.path_rules {
            if let Ok(glob) = Glob::new(&rule.pattern) {
                rules.push(CompiledPathRule {
                    matcher: glob.compile_matcher(),
                    max_lines: rule.max_lines,
                    warn_threshold: rule.warn_threshold,
                });
            }
        }

        // Then, process content.rules (higher priority for V2)
        for rule in &config.content.rules {
            if let Ok(glob) = Glob::new(&rule.pattern) {
                rules.push(CompiledPathRule {
                    matcher: glob.compile_matcher(),
                    max_lines: rule.max_lines,
                    warn_threshold: rule.warn_threshold,
                });
            }
        }

        rules
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
        // 1. Check content.overrides first (highest priority, V2)
        for override_config in &self.config.content.overrides {
            if Self::path_matches_override(path, &override_config.path) {
                return (override_config.max_lines, Some(override_config.reason.clone()));
            }
        }

        // 2. Check legacy overrides (for V1 migration)
        for override_config in &self.config.overrides {
            if Self::path_matches_override(path, &override_config.path) {
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

        // 4. Check extension rules
        if let Some(ext) = path.extension().and_then(|e| e.to_str())
            && let Some(&limit) = self.extension_limits.get(ext)
        {
            return (limit, None);
        }

        // 5. Fall back to content.max_lines (V2) with legacy fallback
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

        // 2. Check extension rules
        if let Some(ext) = path.extension().and_then(|e| e.to_str())
            && let Some(&threshold) = self.extension_warn_thresholds.get(ext)
        {
            return threshold;
        }

        // 3. Fall back to instance warning_threshold
        // (set via with_warning_threshold or from config during initialization)
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
