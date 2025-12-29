use std::path::Path;

use crate::EXIT_SUCCESS;
use crate::cli::{Cli, HistoryArgs, HistoryOutputFormat};
use crate::state;
use crate::stats::{TrendEntry, TrendHistory};

/// Run the history subcommand: list recent trend history entries.
///
/// Note: `_cli` is reserved for future options like `--verbose`, `--quiet`, or `--color` support
pub fn run_history(args: &HistoryArgs, _cli: &Cli) -> crate::Result<i32> {
    // Discover project root for history file resolution
    let project_root = state::discover_project_root(Path::new("."));

    // Determine history file path
    let default_path = state::history_path(&project_root);
    let history_path = args.history_file.as_ref().unwrap_or(&default_path);

    // Load history
    let history = TrendHistory::load_or_default(history_path);

    // Get entries (most recent first, limited by --limit)
    let entries = history.entries();
    let total_entries = entries.len();
    let display_entries: Vec<_> = entries.iter().rev().take(args.limit).collect();

    // Format output
    let output = match args.format {
        HistoryOutputFormat::Text => format_history_text(&display_entries, total_entries),
        HistoryOutputFormat::Json => format_history_json(&display_entries)?,
    };

    println!("{output}");

    Ok(EXIT_SUCCESS)
}

/// Format history entries as human-readable text.
pub fn format_history_text(entries: &[&TrendEntry], total_entries: usize) -> String {
    use std::fmt::Write;

    if entries.is_empty() {
        return "No history entries found.\n\nRecord a snapshot with: sloc-guard snapshot"
            .to_string();
    }

    let mut output = String::new();
    let _ = writeln!(
        output,
        "History ({} of {} entries)\n",
        entries.len(),
        total_entries
    );

    for (i, entry) in entries.iter().enumerate() {
        // Format timestamp as ISO 8601 datetime
        let datetime = format_timestamp(entry.timestamp);

        // Format git context
        let git_info = match (&entry.git_ref, &entry.git_branch) {
            (Some(commit), Some(branch)) => format!(" - {commit} ({branch})"),
            (Some(commit), None) => format!(" - {commit}"),
            (None, Some(branch)) => format!(" ({branch})"),
            (None, None) => String::new(),
        };

        let _ = writeln!(output, "{}. {datetime}{git_info}", i + 1);
        let _ = writeln!(
            output,
            "   Files: {}  Total: {}  Code: {}  Comment: {}  Blank: {}",
            entry.total_files, entry.total_lines, entry.code, entry.comment, entry.blank
        );

        // Add empty line between entries (except for the last one)
        if i < entries.len() - 1 {
            output.push('\n');
        }
    }

    output
}

/// Format history entries as JSON.
pub fn format_history_json(entries: &[&TrendEntry]) -> crate::Result<String> {
    // Create a struct for JSON serialization
    #[derive(serde::Serialize)]
    struct HistoryOutput<'a> {
        count: usize,
        entries: &'a [&'a TrendEntry],
    }

    let output = HistoryOutput {
        count: entries.len(),
        entries,
    };

    serde_json::to_string_pretty(&output).map_err(crate::SlocGuardError::from)
}

/// Format Unix timestamp as ISO 8601 datetime string.
///
/// Uses manual UTC calculation to avoid adding a datetime dependency (chrono/time).
/// This is acceptable for simple UTC formatting; complex timezone handling would warrant a crate.
pub fn format_timestamp(timestamp: u64) -> String {
    // Convert to date components (simplified UTC implementation)
    let days_since_epoch = timestamp / 86400;
    let secs_in_day = timestamp % 86400;

    let hours = secs_in_day / 3600;
    let minutes = (secs_in_day % 3600) / 60;
    let seconds = secs_in_day % 60;

    // Calculate year/month/day from days since epoch (1970-01-01)
    let (year, month, day) = days_to_ymd(days_since_epoch);

    format!("{year:04}-{month:02}-{day:02} {hours:02}:{minutes:02}:{seconds:02}")
}

/// Convert days since Unix epoch to (year, month, day).
#[allow(clippy::cast_possible_wrap)]
pub fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Simplified algorithm for UTC date calculation
    // Safe cast: days since 1970 won't exceed i64::MAX for foreseeable dates
    let mut remaining_days = days as i64;
    let mut year = 1970i64;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let leap = is_leap_year(year);
    let days_in_month = if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    for &days_in_m in &days_in_month {
        if remaining_days < days_in_m {
            break;
        }
        remaining_days -= days_in_m;
        month += 1;
    }

    let day = remaining_days + 1;

    // Safe cast: year >= 1970 and day >= 1 are guaranteed by the algorithm above
    #[allow(clippy::cast_sign_loss)]
    (year as u64, month, day as u64)
}

/// Check if a year is a leap year.
pub const fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}
