use crate::checker::CheckResult;
use crate::{EXIT_SUCCESS, EXIT_THRESHOLD_EXCEEDED};

/// Determine exit code based on results and mode flags.
///
/// - `warn_only`: Always return success (exit 0)
/// - `strict`: Treat warnings as failures
/// - `ratchet_failed`: Baseline ratchet check failed
pub fn determine_exit_code(
    results: &[CheckResult],
    warn_only: bool,
    strict: bool,
    ratchet_failed: bool,
) -> i32 {
    if warn_only {
        return EXIT_SUCCESS;
    }
    let has_failures = results.iter().any(CheckResult::is_failed);
    let has_warnings = results.iter().any(CheckResult::is_warning);
    if has_failures || (strict && has_warnings) || ratchet_failed {
        EXIT_THRESHOLD_EXCEEDED
    } else {
        EXIT_SUCCESS
    }
}
