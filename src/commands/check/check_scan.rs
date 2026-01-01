use std::path::{Path, PathBuf};

use crate::checker::CheckResult;
use crate::cli::{CheckArgs, Cli};
use crate::commands::context::{CheckContext, FileProcessError, resolve_scan_paths};
use crate::scanner::ScanResult;

use super::check_git_diff::filter_by_git_diff;
use super::check_processing::CheckFileResult;

/// Scan directories or filter provided files based on mode.
///
/// Returns:
/// - List of files to process
/// - Optional scan result with directory stats (None in --files mode)
/// - Whether to skip structure checks (true in --files mode)
pub fn scan_or_filter_files(
    args: &CheckArgs,
    cli: &Cli,
    paths: &[PathBuf],
    ctx: &CheckContext,
    project_root: &Path,
) -> crate::Result<(Vec<PathBuf>, Option<ScanResult>, bool)> {
    if args.files.is_empty() {
        // Normal mode: scan directories
        // 1. Determine paths to scan
        let paths_to_scan = resolve_scan_paths(paths, &args.include);

        // 2. Scan directories using unified traversal (collects files + dir stats in one pass)
        let scan_result = ctx
            .scanner
            .scan_all_with_structure(&paths_to_scan, ctx.structure_scan_config.as_ref())?;

        // 2.1 Filter by git diff if --diff or --staged is specified
        let files = filter_by_git_diff(
            scan_result.files.clone(),
            args.diff.as_deref(),
            args.staged,
            project_root,
        )?;
        Ok((files, Some(scan_result), false))
    } else {
        // Pure incremental mode: process only listed files, skip structure checks
        // Warn about non-existent files before filtering them out
        let mut existing_files = Vec::with_capacity(args.files.len());
        for file in &args.files {
            if file.exists() {
                existing_files.push(file.clone());
            } else if !cli.quiet {
                crate::output::print_warning(&format!("file not found: {}", file.display()));
            }
        }
        Ok((existing_files, None, true))
    }
}

/// Partition file processing results into successes, stats, and errors.
///
/// Returns three vectors:
/// - Check results from successfully processed files
/// - File statistics from successfully processed files
/// - Errors that occurred during file processing (IO failures, lock errors)
///
/// Skipped files (no extension, unrecognized extension, ignored by directive) are
/// silently filtered out as they are legitimate non-errors.
pub fn partition_file_results(
    results: Vec<CheckFileResult>,
) -> (
    Vec<CheckResult>,
    Vec<crate::output::FileStatistics>,
    Vec<FileProcessError>,
) {
    let mut check_results = Vec::new();
    let mut file_stats = Vec::new();
    let mut errors = Vec::new();

    for result in results {
        match result {
            CheckFileResult::Success {
                check_result,
                file_stats: stats,
            } => {
                check_results.push(*check_result);
                file_stats.push(stats);
            }
            CheckFileResult::Skipped(_) => {
                // Legitimately skipped files are not errors, just ignored
            }
            CheckFileResult::Error(error) => {
                errors.push(error);
            }
        }
    }

    (check_results, file_stats, errors)
}
