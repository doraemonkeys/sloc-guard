use std::collections::HashSet;
use std::path::Path;

use crate::baseline::{Baseline, StructureViolationType, compute_file_hash};
use crate::checker::{CheckResult, ViolationCategory, ViolationType};
use crate::cli::{BaselineUpdateMode, CheckArgs};
use crate::config::{Config, RatchetMode};
use crate::counter::LineStats;
use crate::project::ProjectPaths;
use crate::state::{self, SaveOutcome};

/// Result of baseline ratchet check.
#[derive(Debug, Clone)]
pub struct RatchetResult {
    /// Number of baseline entries that no longer have corresponding violations.
    pub stale_entries: usize,
    /// Paths of stale entries for reporting.
    pub stale_paths: Vec<String>,
}

impl RatchetResult {
    /// Returns true if the baseline is outdated (has stale entries).
    #[must_use]
    pub const fn is_outdated(&self) -> bool {
        self.stale_entries > 0
    }
}

pub fn load_baseline(baseline_path: Option<&Path>) -> crate::Result<Option<Baseline>> {
    let Some(path) = baseline_path else {
        return Ok(None);
    };

    if !path.exists() {
        return Err(crate::SlocGuardError::Config(format!(
            "Baseline file not found: {}",
            path.display()
        )));
    }

    Ok(Some(Baseline::load(path)?))
}

/// Load baseline optionally - returns None if file doesn't exist (for update-baseline mode).
pub fn load_baseline_optional(baseline_path: Option<&Path>) -> crate::Result<Option<Baseline>> {
    let Some(path) = baseline_path else {
        return Ok(None);
    };

    if !path.exists() {
        return Ok(None);
    }

    Ok(Some(Baseline::load(path)?))
}

fn baseline_path_aliases(path: &Path, project_paths: &ProjectPaths) -> HashSet<String> {
    fn insert_alias(aliases: &mut HashSet<String>, path: &Path) {
        let normalized = path.to_string_lossy().replace('\\', "/");
        if normalized.is_empty() || normalized == "." || normalized == "./" {
            aliases.insert(String::new());
            aliases.insert(".".to_string());
            aliases.insert("./".to_string());
            return;
        }
        aliases.insert(normalized.clone());
        if !path.is_absolute() && !normalized.starts_with("./") {
            aliases.insert(format!("./{normalized}"));
        }
    }

    let mut aliases = HashSet::new();
    insert_alias(&mut aliases, path);

    let physical = project_paths.physical(path);
    insert_alias(&mut aliases, &physical);

    aliases
}

fn baseline_identity(path: &Path) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    if normalized.is_empty() {
        ".".to_string()
    } else {
        normalized
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn apply_baseline_comparison(results: &mut [CheckResult], baseline: &Baseline) {
    apply_baseline_comparison_with_project_paths(results, baseline, &ProjectPaths::unrooted());
}

/// Apply a baseline while accepting unambiguous legacy keys from older path semantics.
pub fn apply_baseline_comparison_with_project_paths(
    results: &mut [CheckResult],
    baseline: &Baseline,
    project_paths: &ProjectPaths,
) {
    for result in results.iter_mut() {
        if !result.is_failed() {
            continue;
        }

        if baseline_path_aliases(result.path(), project_paths)
            .iter()
            .any(|path| baseline.contains(path))
        {
            // Replace the result with its grandfathered version
            let owned = std::mem::replace(
                result,
                CheckResult::Passed {
                    path: std::path::PathBuf::new(),
                    stats: LineStats::default(),
                    raw_stats: None,
                    limit: 0,
                    override_reason: None,
                    violation_category: None,
                },
            );
            *result = owned.into_grandfathered();
        }
    }
}

/// Update baseline file from check results based on the specified mode.
///
/// Returns `SaveOutcome::Saved` on success, `SaveOutcome::Skipped` if lock times out.
#[allow(dead_code)] // Compatibility wrapper used by focused baseline callers/tests.
pub fn update_baseline_from_results(
    results: &[CheckResult],
    mode: BaselineUpdateMode,
    baseline_path: &Path,
    existing_baseline: Option<&Baseline>,
) -> crate::Result<SaveOutcome> {
    update_baseline_from_results_with_project_paths(
        results,
        mode,
        baseline_path,
        existing_baseline,
        &ProjectPaths::unrooted(),
    )
}

/// Update a baseline using logical identities while hashing the corresponding physical files.
pub fn update_baseline_from_results_with_project_paths(
    results: &[CheckResult],
    mode: BaselineUpdateMode,
    baseline_path: &Path,
    existing_baseline: Option<&Baseline>,
    project_paths: &ProjectPaths,
) -> crate::Result<SaveOutcome> {
    let mut new_baseline = match mode {
        BaselineUpdateMode::New => {
            // Start with existing baseline for add-only mode
            existing_baseline.cloned().unwrap_or_default()
        }
        _ => Baseline::new(),
    };

    for result in results {
        if !result.is_failed() {
            continue;
        }

        let path_str = baseline_identity(result.path());
        let is_structure = is_structure_violation_result(result);

        // Apply mode filtering
        let should_include = match mode {
            BaselineUpdateMode::All => true,
            BaselineUpdateMode::Content => !is_structure,
            BaselineUpdateMode::Structure => is_structure,
            BaselineUpdateMode::New => {
                // In new mode, only add if not already in baseline
                !baseline_path_aliases(result.path(), project_paths)
                    .iter()
                    .any(|path| new_baseline.contains(path))
            }
        };

        if !should_include {
            continue;
        }

        if is_structure {
            // Parse structure violation type using structured ViolationCategory
            if let Some((vtype, count)) = parse_structure_violation_from_result(result) {
                new_baseline.set_structure(&path_str, vtype, count);
            }
        } else {
            // Content violation - compute file hash
            let hash =
                compute_file_hash(&project_paths.physical(result.path())).unwrap_or_default();
            new_baseline.set_content(&path_str, result.stats().code, hash);
        }
    }

    new_baseline.save(baseline_path)
}

/// Check if a check result represents a structure violation.
pub fn is_structure_violation_result(result: &CheckResult) -> bool {
    matches!(
        result.violation_category(),
        Some(ViolationCategory::Structure { .. })
    )
}

/// Check if a check result represents a structure violation (legacy string-based check).
/// Used for backwards compatibility during migration and in tests.
#[cfg_attr(not(test), allow(dead_code))]
pub fn is_structure_violation(override_reason: Option<&str>) -> bool {
    override_reason.is_some_and(|r| r.starts_with("structure:"))
}

/// Parse structure violation type from `CheckResult`.
/// Uses the structured `ViolationCategory` when available, falling back to string parsing.
pub fn parse_structure_violation_from_result(
    result: &CheckResult,
) -> Option<(StructureViolationType, usize)> {
    match result.violation_category() {
        Some(ViolationCategory::Structure { violation_type, .. }) => {
            let vtype = match violation_type {
                ViolationType::FileCount => StructureViolationType::Files,
                ViolationType::DirCount => StructureViolationType::Dirs,
                // Other structure violations don't have baseline support yet
                _ => return None,
            };
            Some((vtype, result.stats().code))
        }
        _ => {
            // Fallback to legacy string parsing for backwards compatibility
            parse_structure_violation(result.override_reason(), result.stats().code)
        }
    }
}

/// Parse structure violation type from `override_reason`.
/// Returns (`StructureViolationType`, count) if parseable.
/// Deprecated: prefer `parse_structure_violation_from_result` for new code.
pub fn parse_structure_violation(
    override_reason: Option<&str>,
    count: usize,
) -> Option<(StructureViolationType, usize)> {
    let reason = override_reason?;
    if !reason.starts_with("structure:") {
        return None;
    }

    let vtype = if reason.contains("files") {
        StructureViolationType::Files
    } else if reason.contains("subdirs") {
        StructureViolationType::Dirs
    } else {
        return None;
    };

    Some((vtype, count))
}

/// Check if baseline entries are stale (violations have been resolved).
///
/// Compares current violations with baseline entries. Returns `RatchetResult`
/// containing the count of stale entries that can be removed from the baseline.
#[cfg_attr(not(test), allow(dead_code))]
pub fn check_baseline_ratchet(results: &[CheckResult], baseline: &Baseline) -> RatchetResult {
    check_baseline_ratchet_with_project_paths(results, baseline, &ProjectPaths::unrooted())
}

/// Check baseline ratchet state while recognizing unambiguous legacy baseline keys.
pub fn check_baseline_ratchet_with_project_paths(
    results: &[CheckResult],
    baseline: &Baseline,
    project_paths: &ProjectPaths,
) -> RatchetResult {
    // Collect all current failure paths (normalized)
    let current_failures: HashSet<String> = results
        .iter()
        .filter(|r| r.is_failed() || r.is_grandfathered())
        .flat_map(|r| baseline_path_aliases(r.path(), project_paths))
        .collect();

    // Find baseline entries that are no longer violations
    let mut stale_paths: Vec<String> = Vec::new();
    for baseline_path in baseline.files().keys() {
        if !current_failures.contains(baseline_path) {
            stale_paths.push(baseline_path.clone());
        }
    }

    stale_paths.sort();

    RatchetResult {
        stale_entries: stale_paths.len(),
        stale_paths,
    }
}

/// Remove stale entries from baseline and save (for `--ratchet=auto` mode).
///
/// Returns `SaveOutcome::Saved` on success, `SaveOutcome::Skipped` if lock times out.
pub fn tighten_baseline(
    baseline: &mut Baseline,
    stale_paths: &[String],
    path: &Path,
) -> crate::Result<SaveOutcome> {
    for stale_path in stale_paths {
        baseline.remove(stale_path);
    }
    baseline.save(path)
}

/// Handle baseline ratchet enforcement.
///
/// Returns `true` if ratchet check failed (for strict mode exit code).
pub fn handle_baseline_ratchet(
    args: &CheckArgs,
    config: &Config,
    results: &[CheckResult],
    baseline: &mut Option<Baseline>,
    project_root: &Path,
    project_paths: &ProjectPaths,
    quiet: bool,
) -> crate::Result<bool> {
    // Determine effective ratchet mode: CLI takes precedence over config
    let ratchet_mode: Option<RatchetMode> = match (&args.ratchet, &config.baseline.ratchet) {
        (Some(cli_mode), _) => Some((*cli_mode).into()),
        (None, config_mode) => *config_mode,
    };

    // If no ratchet mode is set, skip ratchet check
    let Some(mode) = ratchet_mode else {
        return Ok(false);
    };

    // Ratchet requires a baseline
    let Some(current_baseline) = baseline else {
        // If ratchet is enabled but no baseline exists, emit warning
        if !quiet {
            crate::output::print_warning_full(
                "--ratchet specified but no baseline found",
                None,
                Some("Use --baseline to specify a baseline file"),
            );
        }
        return Ok(false);
    };

    // Check for stale entries
    let ratchet_result =
        check_baseline_ratchet_with_project_paths(results, current_baseline, project_paths);

    if !ratchet_result.is_outdated() {
        return Ok(false);
    }

    let baseline_path = args
        .baseline
        .clone()
        .unwrap_or_else(|| state::baseline_path(project_root));

    match mode {
        RatchetMode::Warn => {
            if !quiet {
                let paths_str = ratchet_result
                    .stale_paths
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .join(", ");
                crate::output::print_warning_full(
                    &format!(
                        "Baseline can be tightened - {} violation(s) resolved",
                        ratchet_result.stale_entries
                    ),
                    Some(&paths_str),
                    Some("Run with --ratchet=auto to auto-update, or --update-baseline to replace"),
                );
            }
            Ok(false)
        }
        RatchetMode::Auto => {
            let outcome = tighten_baseline(
                current_baseline,
                &ratchet_result.stale_paths,
                &baseline_path,
            )?;
            if !quiet && outcome.is_saved() {
                eprintln!(
                    "Baseline tightened: {} stale entry/entries removed.",
                    ratchet_result.stale_entries
                );
            }
            Ok(false)
        }
        RatchetMode::Strict => {
            let paths_str = ratchet_result
                .stale_paths
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(", ");
            crate::output::print_error_full(
                "Baseline",
                &format!(
                    "outdated - {} violation(s) resolved but not removed",
                    ratchet_result.stale_entries
                ),
                Some(&paths_str),
                Some("Update the baseline with --update-baseline or use --ratchet=auto"),
            );
            Ok(true) // Signal failure
        }
    }
}
