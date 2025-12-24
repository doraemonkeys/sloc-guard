//! Trend tracking tests organized by domain.

mod delta_tests;
mod entry_tests;
mod history_tests;
mod query_tests;
mod retention_tests;
mod significance_tests;

use std::path::PathBuf;

use super::*;
use crate::config::TrendConfig;
use crate::counter::LineStats;
use crate::output::{FileStatistics, ProjectStatistics};

/// Create sample project statistics for testing.
pub(super) fn sample_project_stats(total_files: usize, total_code: usize) -> ProjectStatistics {
    let files: Vec<FileStatistics> = (0..total_files)
        .map(|i| FileStatistics {
            path: PathBuf::from(format!("file{i}.rs")),
            stats: LineStats {
                total: total_code / total_files + 20,
                code: total_code / total_files,
                comment: 10,
                blank: 10,
                ignored: 0,
            },
            language: "Rust".to_string(),
        })
        .collect();
    ProjectStatistics::new(files)
}

/// Create a minimal `TrendEntry` with the given timestamp.
pub(super) fn make_entry(timestamp: u64) -> TrendEntry {
    TrendEntry {
        timestamp,
        total_files: 10,
        total_lines: 100,
        code: 50,
        comment: 30,
        blank: 20,
    }
}
