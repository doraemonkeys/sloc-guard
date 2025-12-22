//! Threshold checker tests organized by domain.

mod check_result_tests;
mod content_exclude_tests;
mod file_filter_tests;
mod override_matching_tests;
mod rule_matching_tests;
mod skip_settings_tests;
mod warn_threshold_tests;

use crate::checker::Checker;
use crate::checker::explain::ContentRuleMatch;
use crate::checker::result::CheckResult;
use crate::checker::threshold::ThresholdChecker;
use crate::config::Config;
use crate::counter::LineStats;

/// Create a default config with standard defaults.
fn default_config() -> Config {
    Config::default()
}

/// Create `LineStats` with specific code line count.
/// Total = code + 10, comment = 5, blank = 5.
fn stats_with_code(code: usize) -> LineStats {
    LineStats {
        total: code + 10,
        code,
        comment: 5,
        blank: 5,
        ignored: 0,
    }
}
