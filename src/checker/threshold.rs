use std::collections::HashMap;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobMatcher};

use crate::config::Config;
use crate::counter::LineStats;

use super::Checker;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckStatus {
    Passed,
    Warning,
    Failed,
}

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub path: PathBuf,
    pub status: CheckStatus,
    pub stats: LineStats,
    pub limit: usize,
}

impl CheckResult {
    #[must_use]
    pub const fn is_passed(&self) -> bool {
        matches!(self.status, CheckStatus::Passed)
    }

    #[must_use]
    pub const fn is_failed(&self) -> bool {
        matches!(self.status, CheckStatus::Failed)
    }

    #[must_use]
    pub const fn is_warning(&self) -> bool {
        matches!(self.status, CheckStatus::Warning)
    }

    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn usage_percent(&self) -> f64 {
        if self.limit == 0 {
            return 0.0;
        }
        (self.stats.sloc() as f64 / self.limit as f64) * 100.0
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
    path_rules: Vec<CompiledPathRule>,
}

impl ThresholdChecker {
    #[must_use]
    pub fn new(config: Config) -> Self {
        let extension_limits = Self::build_extension_index(&config);
        let path_rules = Self::build_path_rules(&config);
        Self {
            config,
            warning_threshold: 0.9,
            extension_limits,
            path_rules,
        }
    }

    #[must_use]
    pub const fn with_warning_threshold(mut self, threshold: f64) -> Self {
        self.warning_threshold = threshold;
        self
    }

    fn build_extension_index(config: &Config) -> HashMap<String, usize> {
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

    fn get_limit_for_path(&self, path: &Path) -> usize {
        // 1. Check overrides first (highest priority)
        for override_config in &self.config.overrides {
            if Self::path_matches_override(path, &override_config.path) {
                return override_config.max_lines;
            }
        }

        // 2. Check path_rules (glob patterns)
        for path_rule in &self.path_rules {
            if path_rule.matcher.is_match(path) {
                return path_rule.max_lines;
            }
        }

        // 3. Check extension rules
        if let Some(ext) = path.extension().and_then(|e| e.to_str())
            && let Some(&limit) = self.extension_limits.get(ext)
        {
            return limit;
        }

        // 4. Fall back to default
        self.config.default.max_lines
    }

    fn get_warn_threshold_for_path(&self, path: &Path) -> f64 {
        // Check path_rules for custom warn_threshold
        for path_rule in &self.path_rules {
            if path_rule.matcher.is_match(path)
                && let Some(threshold) = path_rule.warn_threshold
            {
                return threshold;
            }
        }
        self.warning_threshold
    }
}

impl Checker for ThresholdChecker {
    fn check(&self, path: &Path, line_stats: &LineStats) -> CheckResult {
        let limit = self.get_limit_for_path(path);
        let warn_threshold = self.get_warn_threshold_for_path(path);
        let sloc = line_stats.sloc();

        #[allow(clippy::cast_precision_loss)]
        let check_status = if sloc > limit {
            CheckStatus::Failed
        } else if sloc as f64 >= limit as f64 * warn_threshold {
            CheckStatus::Warning
        } else {
            CheckStatus::Passed
        };

        CheckResult {
            path: path.to_path_buf(),
            status: check_status,
            stats: line_stats.clone(),
            limit,
        }
    }
}

#[cfg(test)]
#[path = "threshold_tests.rs"]
mod tests;
