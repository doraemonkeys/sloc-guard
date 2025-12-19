mod explain;
mod structure;
mod threshold;

pub use explain::{
    ContentExplanation, ContentRuleCandidate, ContentRuleMatch, MatchStatus,
    StructureExplanation, StructureRuleCandidate, StructureRuleMatch,
};
pub use structure::{DirStats, StructureChecker, StructureViolation, ViolationType};
pub use threshold::{CheckResult, ThresholdChecker};

use std::path::Path;

use crate::counter::LineStats;

pub trait Checker {
    fn check(&self, path: &Path, stats: &LineStats) -> CheckResult;
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
