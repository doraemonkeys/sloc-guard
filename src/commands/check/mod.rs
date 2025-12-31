mod check_baseline_ops;
mod check_git_diff;
mod check_output;
mod check_processing;
mod runner;

pub use runner::run_check;

// Re-export internal items for tests
#[cfg(test)]
pub(crate) use check_baseline_ops::{
    apply_baseline_comparison, check_baseline_ratchet, is_structure_violation, load_baseline,
    parse_structure_violation,
};
#[cfg(test)]
pub(crate) use check_git_diff::{DiffRange, parse_diff_range};
#[cfg(test)]
pub(crate) use check_output::{format_output, structure_violation_to_check_result};
#[cfg(test)]
pub(crate) use check_processing::{compute_effective_stats, process_file_for_check};
#[cfg(test)]
pub(crate) use runner::{
    CheckOptions, apply_cli_overrides, run_check_impl, run_check_with_context,
    validate_and_resolve_paths,
};

#[cfg(test)]
mod check_baseline_tests;
#[cfg(test)]
mod check_context_structure_tests;
#[cfg(test)]
mod check_git_diff_tests;
#[cfg(test)]
mod check_output_tests;
#[cfg(test)]
mod check_processing_tests;
#[cfg(test)]
mod check_run_auto_snapshot_tests;
#[cfg(test)]
mod check_run_exit_code_tests;
#[cfg(test)]
mod check_run_filter_tests;
#[cfg(test)]
mod check_run_output_format_tests;
#[cfg(test)]
mod check_run_sidecar_output_tests;
#[cfg(test)]
mod check_run_strict_warn_tests;
#[cfg(test)]
mod check_tests;
