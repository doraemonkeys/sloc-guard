use std::path::Path;
use std::sync::Mutex;

use crate::cache::Cache;
use crate::checker::{CheckResult, Checker, ThresholdChecker};
use crate::counter::LineStats;
use crate::language::LanguageRegistry;
use crate::output::FileStatistics;

use crate::commands::context::{
    FileProcessError, FileProcessResult, FileReader, FileSkipReason, process_file_with_cache,
};

/// Result of processing a file for the check command.
///
/// Uses `Box` for the large `Success` variant to keep enum size small.
#[derive(Debug)]
pub enum CheckFileResult {
    /// File was successfully processed.
    /// Boxed to reduce enum size (`CheckResult` + `FileStatistics` is ~424 bytes).
    Success {
        check_result: Box<CheckResult>,
        file_stats: FileStatistics,
    },
    /// File was legitimately skipped (not an error).
    /// The inner reason is read in tests for verification, but discarded in production
    /// (runner.rs matches with `_`). We preserve it for test assertions and future verbose logging.
    #[allow(dead_code)]
    Skipped(FileSkipReason),
    /// An error occurred while processing the file.
    Error(FileProcessError),
}

impl CheckFileResult {
    /// Returns true if this result represents a check failure.
    /// Used by `fail_fast` mode to detect when to stop processing.
    #[must_use]
    pub fn is_failure(&self) -> bool {
        match self {
            Self::Success { check_result, .. } => check_result.is_failed(),
            Self::Skipped(_) | Self::Error(_) => false,
        }
    }
}

pub fn process_file_for_check(
    file_path: &Path,
    registry: &LanguageRegistry,
    checker: &ThresholdChecker,
    cache: &Mutex<Cache>,
    reader: &dyn FileReader,
) -> CheckFileResult {
    let result = process_file_with_cache(file_path, registry, cache, reader);

    match result {
        FileProcessResult::Success { stats, language } => {
            let (skip_comments, skip_blank) = checker.get_skip_settings_for_path(file_path);
            let effective_stats = compute_effective_stats(&stats, skip_comments, skip_blank);
            let check_result = checker.check(file_path, &effective_stats, Some(&stats));
            let file_stats = FileStatistics {
                path: file_path.to_path_buf(),
                stats,
                language,
            };
            CheckFileResult::Success {
                check_result: Box::new(check_result),
                file_stats,
            }
        }
        FileProcessResult::Skipped(reason) => CheckFileResult::Skipped(reason),
        FileProcessResult::Error(error) => CheckFileResult::Error(error),
    }
}

#[must_use]
pub fn compute_effective_stats(
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
