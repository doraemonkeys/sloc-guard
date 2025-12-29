// Collection module tests
//
// Tests for stats collection functionality are primarily covered by integration
// tests in the runner and other subcommand test modules. This module is reserved
// for unit tests specific to collection internals if needed.

use super::collection::{collect_file_stats, collect_stats, save_cache_if_enabled};

// Verify collection module exports are accessible
#[test]
fn collection_module_exports_are_accessible() {
    // The collection module is tested indirectly through runner tests
    // which exercise collect_stats and related functions.
    // This test ensures the module's public API is accessible.
    let _: fn(&crate::cli::CommonStatsArgs, &crate::cli::Cli) -> crate::Result<_> = collect_stats;
    let _: fn(
        &std::path::Path,
        &crate::language::LanguageRegistry,
        &std::sync::Mutex<crate::cache::Cache>,
        &dyn crate::commands::context::FileReader,
    ) -> Option<crate::output::FileStatistics> = collect_file_stats;
    let _: fn(
        &crate::cli::CommonStatsArgs,
        &std::sync::Mutex<crate::cache::Cache>,
        &std::path::Path,
    ) = save_cache_if_enabled;
}
