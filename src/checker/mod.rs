mod explain;
mod result;
mod structure;
mod threshold;

pub use explain::{
    ContentExplanation, ContentRuleCandidate, ContentRuleMatch, MatchStatus, StructureExplanation,
    StructureRuleCandidate, StructureRuleMatch, WarnAtSource,
};
pub use result::CheckResult;
pub use structure::StructureChecker;
pub use structure::violation::{DirStats, StructureViolation, ViolationCategory, ViolationType};
pub use threshold::ThresholdChecker;

use std::path::Path;

use crate::counter::LineStats;

pub trait Checker {
    /// Check a file against configured thresholds.
    ///
    /// - `stats`: Effective line stats (after `skip_comments`/`skip_blank` adjustments)
    /// - `raw_stats`: Original line stats before adjustments (for display in breakdown)
    fn check(&self, path: &Path, stats: &LineStats, raw_stats: Option<&LineStats>) -> CheckResult;
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
