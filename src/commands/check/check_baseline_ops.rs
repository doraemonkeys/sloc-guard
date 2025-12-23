use std::collections::HashSet;
use std::path::Path;

use crate::baseline::{Baseline, StructureViolationType, compute_file_hash};
use crate::checker::{CheckResult, ViolationCategory, ViolationType};
use crate::cli::BaselineUpdateMode;
use crate::counter::LineStats;

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

pub fn apply_baseline_comparison(results: &mut [CheckResult], baseline: &Baseline) {
    for result in results.iter_mut() {
        if !result.is_failed() {
            continue;
        }

        let path_str = result.path().to_string_lossy().replace('\\', "/");
        if baseline.contains(&path_str) {
            // Replace the result with its grandfathered version
            let owned = std::mem::replace(
                result,
                CheckResult::Passed {
                    path: std::path::PathBuf::new(),
                    stats: LineStats::default(),
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
pub fn update_baseline_from_results(
    results: &[CheckResult],
    mode: BaselineUpdateMode,
    baseline_path: &Path,
    existing_baseline: Option<&Baseline>,
) -> crate::Result<()> {
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

        let path_str = result.path().to_string_lossy().replace('\\', "/");
        let is_structure = is_structure_violation_result(result);

        // Apply mode filtering
        let should_include = match mode {
            BaselineUpdateMode::All => true,
            BaselineUpdateMode::Content => !is_structure,
            BaselineUpdateMode::Structure => is_structure,
            BaselineUpdateMode::New => {
                // In new mode, only add if not already in baseline
                !new_baseline.contains(&path_str)
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
            let hash = compute_file_hash(result.path()).unwrap_or_default();
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
pub fn check_baseline_ratchet(results: &[CheckResult], baseline: &Baseline) -> RatchetResult {
    // Collect all current failure paths (normalized)
    let current_failures: HashSet<String> = results
        .iter()
        .filter(|r| r.is_failed() || r.is_grandfathered())
        .map(|r| r.path().to_string_lossy().replace('\\', "/"))
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
pub fn tighten_baseline(
    baseline: &mut Baseline,
    stale_paths: &[String],
    path: &Path,
) -> crate::Result<()> {
    for stale_path in stale_paths {
        baseline.remove(stale_path);
    }
    baseline.save(path)
}
