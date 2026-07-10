//! Logical-path normalization and glob matching.

use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet};

use crate::error::{Result, SlocGuardError};

/// Normalize a logical path for consistent glob pattern matching.
///
/// Leading `./` is removed and separators are converted to `/`. A bare `.` becomes an empty path,
/// representing the configuration root.
#[must_use]
pub fn normalize_for_matching(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();
    let stripped = path_str
        .strip_prefix("./")
        .or_else(|| path_str.strip_prefix(".\\"))
        .unwrap_or(&path_str);

    if stripped.is_empty() || stripped == "." {
        return PathBuf::new();
    }

    if stripped.contains('\\') {
        PathBuf::from(stripped.replace('\\', "/"))
    } else {
        PathBuf::from(stripped)
    }
}

/// Normalize a configuration glob into the same namespace as logical path candidates.
///
/// This removes one leading `./` (or `.\`) and normalizes path separators while preserving
/// parent components such as `../`. Basename-only patterns should not use this helper.
#[must_use]
pub fn normalize_pattern_for_matching(pattern: &str) -> String {
    normalize_for_matching(Path::new(pattern))
        .to_string_lossy()
        .into_owned()
}

/// Compile a configuration-root-relative path glob with shared logical-path normalization.
///
/// # Errors
/// Returns an invalid-pattern error containing the original user-authored pattern.
pub fn compile_logical_path_glob(pattern: &str) -> Result<Glob> {
    Glob::new(&normalize_pattern_for_matching(pattern)).map_err(|source| {
        SlocGuardError::InvalidPattern {
            pattern: pattern.to_string(),
            source,
        }
    })
}

/// Whether a pattern explicitly addresses a path rather than an unqualified basename.
#[must_use]
fn pattern_is_path_qualified(pattern: &str) -> bool {
    let pattern = pattern
        .strip_suffix('/')
        .or_else(|| pattern.strip_suffix('\\'))
        .unwrap_or(pattern);
    pattern.starts_with("./")
        || pattern.starts_with(r".\")
        || pattern.contains('/')
        || pattern.contains('\\')
}

/// Return matching pattern indices for a logical path while preserving basename semantics.
///
/// Every pattern is tested against the full normalized logical path. Patterns without an explicit
/// path component are additionally tested against the basename, preserving legacy patterns such as
/// `temp_*`. Explicit patterns such as `./root.rs` never gain basename semantics after their leading
/// dot is normalized away.
#[must_use]
pub fn matching_logical_path_globs(
    matcher: &GlobSet,
    original_patterns: &[String],
    path: &Path,
) -> Vec<usize> {
    let normalized = normalize_for_matching(path);
    // Missing metadata can occur when older library callers manually populate a public GlobSet.
    // Treat such entries as legacy unqualified patterns instead of silently disabling them.
    let mut match_flags = vec![false; matcher.len()];

    for index in matcher.matches(&normalized) {
        if let Some(value) = match_flags.get_mut(index) {
            *value = true;
        }
    }

    if let Some(file_name) = normalized.file_name() {
        for index in matcher.matches(file_name) {
            if !original_patterns
                .get(index)
                .is_some_and(|pattern| pattern_is_path_qualified(pattern))
                && let Some(value) = match_flags.get_mut(index)
            {
                *value = true;
            }
        }
    }

    match_flags
        .into_iter()
        .enumerate()
        .filter_map(|(index, matches)| matches.then_some(index))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_path_globs_share_candidate_normalization() {
        assert_eq!(normalize_pattern_for_matching("."), "");
        assert_eq!(normalize_pattern_for_matching("./src/**"), "src/**");
        assert_eq!(normalize_pattern_for_matching(r".\src\**"), "src/**");
        assert_eq!(
            normalize_pattern_for_matching("../shared/**"),
            "../shared/**"
        );

        let root = compile_logical_path_glob(".").unwrap().compile_matcher();
        assert!(root.is_match(normalize_for_matching(Path::new("."))));
        assert!(!root.is_match(normalize_for_matching(Path::new("src"))));

        let src = compile_logical_path_glob("./src/**")
            .unwrap()
            .compile_matcher();
        assert!(src.is_match(normalize_for_matching(Path::new("./src/lib"))));
    }

    #[test]
    fn explicit_paths_do_not_gain_basename_semantics() {
        assert!(!pattern_is_path_qualified("node_modules/"));
        assert!(pattern_is_path_qualified("./node_modules/"));
        assert!(pattern_is_path_qualified("src/node_modules/"));

        let patterns = vec!["temp_*".to_string(), "./root.rs".to_string()];
        let mut builder = globset::GlobSetBuilder::new();
        for pattern in &patterns {
            builder.add(compile_logical_path_glob(pattern).unwrap());
        }
        let matcher = builder.build().unwrap();

        assert_eq!(
            matching_logical_path_globs(&matcher, &patterns, Path::new("nested/temp_cache")),
            vec![0]
        );
        assert_eq!(
            matching_logical_path_globs(&matcher, &[], Path::new("nested/temp_cache")),
            vec![0]
        );
        assert_eq!(
            matching_logical_path_globs(&matcher, &patterns, Path::new("root.rs")),
            vec![1]
        );
        assert!(
            matching_logical_path_globs(&matcher, &patterns, Path::new("nested/root.rs"))
                .is_empty()
        );
    }
}
