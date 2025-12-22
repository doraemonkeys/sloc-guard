use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::git::GitDiff;

/// Represents a parsed diff range (base..target).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiffRange {
    pub base: String,
    pub target: String,
}

/// Parse a diff reference string into a `DiffRange`.
///
/// Supports:
/// - `ref` → base=ref, target=HEAD
/// - `base..target` → base=base, target=target
/// - `base..` → base=base, target=HEAD
///
/// # Errors
/// Returns an error if:
/// - Input starts with `..` (no base specified)
/// - Input is empty
pub(crate) fn parse_diff_range(diff_ref: &str) -> crate::Result<DiffRange> {
    if diff_ref.is_empty() {
        return Err(crate::SlocGuardError::Config(
            "--diff requires a git reference".to_string(),
        ));
    }

    // Check for range syntax (contains "..")
    if let Some(pos) = diff_ref.find("..") {
        let base = &diff_ref[..pos];
        let target = &diff_ref[pos + 2..];

        // Error if no base specified
        if base.is_empty() {
            return Err(crate::SlocGuardError::Config(
                "--diff range requires a base reference (e.g., 'main..feature', not '..feature')"
                    .to_string(),
            ));
        }

        // If target is empty, default to HEAD
        let target = if target.is_empty() {
            "HEAD".to_string()
        } else {
            target.to_string()
        };

        Ok(DiffRange {
            base: base.to_string(),
            target,
        })
    } else {
        // Single reference: compare to HEAD
        Ok(DiffRange {
            base: diff_ref.to_string(),
            target: "HEAD".to_string(),
        })
    }
}

pub(crate) fn filter_by_git_diff(
    files: Vec<PathBuf>,
    diff_ref: Option<&str>,
    staged_only: bool,
    project_root: &Path,
) -> crate::Result<Vec<PathBuf>> {
    if !staged_only && diff_ref.is_none() {
        return Ok(files);
    }

    // Discover git repository from project root
    let git_diff = GitDiff::discover(project_root)?;
    let changed_files = if staged_only {
        git_diff.get_staged_files()?
    } else {
        let range = parse_diff_range(diff_ref.expect("diff_ref checked above"))?;
        git_diff.get_changed_files_range(&range.base, &range.target)?
    };

    // Canonicalize paths for comparison
    let changed_canonical: HashSet<_> = changed_files
        .iter()
        .filter_map(|p| p.canonicalize().ok())
        .collect();

    // Filter to only include changed files
    let filtered: Vec<_> = files
        .into_iter()
        .filter(|f| {
            f.canonicalize()
                .ok()
                .is_some_and(|canon| changed_canonical.contains(&canon))
        })
        .collect();

    Ok(filtered)
}

#[cfg(test)]
#[path = "check_git_diff_tests.rs"]
mod tests;

