use std::collections::HashMap;
use std::path::{Path, PathBuf};

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

pub struct ThresholdChecker {
    config: Config,
    warning_threshold: f64,
    extension_limits: HashMap<String, usize>,
}

impl ThresholdChecker {
    #[must_use]
    pub fn new(config: Config) -> Self {
        let extension_limits = Self::build_extension_index(&config);
        Self {
            config,
            warning_threshold: 0.9,
            extension_limits,
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

    fn get_limit_for_path(&self, path: &Path) -> usize {
        let path_str = path.to_string_lossy();
        for override_config in &self.config.overrides {
            if path_str.ends_with(&override_config.path) {
                return override_config.max_lines;
            }
        }

        if let Some(ext) = path.extension().and_then(|e| e.to_str())
            && let Some(&limit) = self.extension_limits.get(ext)
        {
            return limit;
        }

        self.config.default.max_lines
    }
}

impl Checker for ThresholdChecker {
    fn check(&self, path: &Path, line_stats: &LineStats) -> CheckResult {
        let limit = self.get_limit_for_path(path);
        let sloc = line_stats.sloc();

        #[allow(clippy::cast_precision_loss)]
        let check_status = if sloc > limit {
            CheckStatus::Failed
        } else if sloc as f64 >= limit as f64 * self.warning_threshold {
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
