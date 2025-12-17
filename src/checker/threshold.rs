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
}

impl ThresholdChecker {
    #[must_use]
    pub const fn new(config: Config) -> Self {
        Self {
            config,
            warning_threshold: 0.9,
        }
    }

    #[must_use]
    pub const fn with_warning_threshold(mut self, threshold: f64) -> Self {
        self.warning_threshold = threshold;
        self
    }

    fn get_limit_for_path(&self, path: &Path) -> usize {
        let path_str = path.to_string_lossy();
        for override_config in &self.config.overrides {
            if path_str.ends_with(&override_config.path) || path_str.contains(&override_config.path)
            {
                return override_config.max_lines;
            }
        }

        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            for rule in self.config.rules.values() {
                if rule.extensions.iter().any(|e| e == ext)
                    && let Some(max_lines) = rule.max_lines
                {
                    return max_lines;
                }
            }
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
