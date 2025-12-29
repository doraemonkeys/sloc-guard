mod collection;
mod formatting;
mod history;
mod report;
mod runner;

pub use runner::run_stats;

// Re-export internal items for tests
#[cfg(test)]
pub(crate) use formatting::format_stats_output;
#[cfg(test)]
pub(crate) use history::{
    days_to_ymd, format_history_json, format_history_text, format_timestamp, is_leap_year,
};
#[cfg(test)]
pub(crate) use report::{build_exclude_set, parse_breakdown_by};

#[cfg(test)]
mod stats_collection_tests;
#[cfg(test)]
mod stats_formatting_tests;
#[cfg(test)]
mod stats_history_tests;
#[cfg(test)]
mod stats_report_tests;
#[cfg(test)]
mod stats_runner_tests;
