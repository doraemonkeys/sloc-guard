//! Integration tests for the `check` command.
//!
//! Organized into domain-focused submodules:
//! - `check_core_tests`: Basic behavior, CLI overrides, verbosity, error handling
//! - `check_output_tests`: Output formats (json, sarif, markdown, html)
//! - `check_rules_tests`: Content rules, structure checks, path normalization
//! - `check_state_tests`: Baseline, auto-snapshot, line counting

mod common;

mod check_integration_tests {
    mod check_core_tests;
    mod check_output_tests;
    mod check_rules_tests;
    mod check_state_tests;
}
