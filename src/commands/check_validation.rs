use std::collections::HashMap;
use std::path::PathBuf;

use crate::checker::DirStats;
use crate::config::{ContentOverride, StructureOverride};
use crate::path_utils::path_matches_override;

/// Validate that override paths are correctly configured.
///
/// - `ContentOverride` paths must point to files, not directories
/// - `StructureOverride` paths must point to directories, not files
///
/// Returns an error if any override path is misconfigured.
pub(crate) fn validate_override_paths(
    content_overrides: &[ContentOverride],
    structure_overrides: &[StructureOverride],
    files: &[PathBuf],
    directories: &HashMap<PathBuf, DirStats>,
) -> crate::Result<()> {
    // Check ContentOverrides don't match directories
    for (i, ovr) in content_overrides.iter().enumerate() {
        for dir_path in directories.keys() {
            if path_matches_override(dir_path, &ovr.path) {
                return Err(crate::SlocGuardError::Config(format!(
                    "content.override[{}] path '{}' matches directory '{}', \
                     but content overrides only apply to files. \
                     Use [[structure.override]] for directory overrides.",
                    i,
                    ovr.path,
                    dir_path.display()
                )));
            }
        }
    }

    // Check StructureOverrides don't match files
    for (i, ovr) in structure_overrides.iter().enumerate() {
        for file_path in files {
            if path_matches_override(file_path, &ovr.path) {
                return Err(crate::SlocGuardError::Config(format!(
                    "structure.override[{}] path '{}' matches file '{}', \
                     but structure overrides only apply to directories. \
                     Use [[content.override]] for file overrides.",
                    i,
                    ovr.path,
                    file_path.display()
                )));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
#[path = "check_validation_tests.rs"]
mod tests;

