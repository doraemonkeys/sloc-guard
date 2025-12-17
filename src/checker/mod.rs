mod threshold;

pub use threshold::{CheckResult, CheckStatus, ThresholdChecker};

use std::path::Path;

use crate::counter::LineStats;

pub trait Checker {
    fn check(&self, path: &Path, stats: &LineStats) -> CheckResult;
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
