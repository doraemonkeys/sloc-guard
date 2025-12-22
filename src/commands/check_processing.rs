use std::path::Path;
use std::sync::Mutex;

use crate::cache::Cache;
use crate::checker::{CheckResult, Checker, ThresholdChecker};
use crate::counter::LineStats;
use crate::language::LanguageRegistry;
use crate::output::FileStatistics;

use super::context::{FileReader, process_file_with_cache};

pub(crate) fn process_file_for_check(
    file_path: &Path,
    registry: &LanguageRegistry,
    checker: &ThresholdChecker,
    cache: &Mutex<Cache>,
    reader: &dyn FileReader,
) -> Option<(CheckResult, FileStatistics)> {
    let (stats, language) = process_file_with_cache(file_path, registry, cache, reader)?;
    let (skip_comments, skip_blank) = checker.get_skip_settings_for_path(file_path);
    let effective_stats = compute_effective_stats(&stats, skip_comments, skip_blank);
    let check_result = checker.check(file_path, &effective_stats);
    let file_stats = FileStatistics {
        path: file_path.to_path_buf(),
        stats,
        language,
    };
    Some((check_result, file_stats))
}

#[must_use]
pub(crate) fn compute_effective_stats(
    stats: &LineStats,
    skip_comments: bool,
    skip_blank: bool,
) -> LineStats {
    let mut effective = stats.clone();

    // If not skipping comments, add them to code count
    if !skip_comments {
        effective.code += effective.comment;
        effective.comment = 0;
    }

    // If not skipping blanks, add them to code count
    if !skip_blank {
        effective.code += effective.blank;
        effective.blank = 0;
    }

    effective
}

#[cfg(test)]
#[path = "check_processing_tests.rs"]
mod tests;
