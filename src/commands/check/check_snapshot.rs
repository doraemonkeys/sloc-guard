use std::path::Path;

use crate::config::Config;
use crate::git::GitContext;
use crate::output::ProjectStatistics;
use crate::state;
use crate::stats::TrendHistory;

/// Perform auto-snapshot after a successful check.
///
/// Records current statistics to trend history, respecting retention policies.
/// Skips (with verbose log) if:
/// - `min_interval_secs` hasn't elapsed since last entry
/// - History file cannot be written (logs warning instead of failing)
pub fn perform_auto_snapshot(
    project_stats: &ProjectStatistics,
    config: &Config,
    project_root: &Path,
    quiet: bool,
    verbose: u8,
) {
    // Get git context for the snapshot
    let git_context = GitContext::from_path(project_root);

    // Load history
    let history_path = state::history_path(project_root);
    let mut history = TrendHistory::load_or_default(&history_path);

    // Check if we should add (respects min_interval_secs)
    let current_time = state::current_unix_timestamp();

    if !history.should_add(&config.trend, current_time) {
        if verbose > 0 && !quiet {
            eprintln!(
                "Skipping auto-snapshot: min_interval_secs ({}) not elapsed since last entry",
                config.trend.min_interval_secs.unwrap_or(0)
            );
        }
        return;
    }

    // Add entry with git context
    history.add_with_context(project_stats, git_context.as_ref());

    // Save with retention policy applied
    if let Err(e) = history.save_with_retention(&history_path, &config.trend) {
        // Log warning but don't fail the check
        if !quiet {
            crate::output::print_warning_full(
                "Auto-snapshot failed to save",
                Some(&format!("{}: {e}", history_path.display())),
                None,
            );
        }
        return;
    }

    if !quiet {
        eprintln!("Auto-snapshot recorded to {}", history_path.display());
    }
}
