//! Logical-path normalization and glob matching.

use std::path::{Path, PathBuf};

use globset::{Glob, GlobMatcher, GlobSet};

use crate::error::{Result, SlocGuardError};

/// Compiled matcher for a directory-scope pattern (structure `scope` fields).
///
/// Scope patterns select directories with plain glob semantics — `X` exact,
/// `X/*` direct children, `X/**` strict descendants — plus one addition: a
/// trailing separator selects the *subtree*, i.e. `X/` matches the directory
/// itself and every descendant. The subtree form exists because globset's
/// `X/**` excludes the base directory, which would otherwise force every
/// subtree rule to be written twice (`X` and `X/**`).
#[derive(Debug, Clone)]
pub enum ScopeMatcher {
    /// Scope without a trailing separator: plain glob semantics.
    Pattern(GlobMatcher),
    /// Trailing-separator scope (`X/`): the base directory plus all descendants.
    Subtree {
        base: GlobMatcher,
        descendants: GlobMatcher,
    },
}

impl ScopeMatcher {
    /// Compile a configuration-root-relative directory-scope pattern.
    ///
    /// # Errors
    /// Returns an invalid-pattern error containing the original user-authored scope.
    pub fn compile(scope: &str) -> Result<Self> {
        let trimmed = scope.trim_end_matches(['/', '\\']);
        if trimmed.len() == scope.len() {
            return Ok(Self::Pattern(
                compile_logical_path_glob(scope)?.compile_matcher(),
            ));
        }

        let base = normalize_pattern_for_matching(trimmed);
        // `.` (the config root) normalizes to an empty base; its subtree is everything.
        let descendants = if base.is_empty() {
            "**".to_string()
        } else {
            format!("{base}/**")
        };
        let compile = |pattern: &str| {
            Glob::new(pattern).map_err(|source| SlocGuardError::InvalidPattern {
                pattern: scope.to_string(),
                source,
            })
        };
        Ok(Self::Subtree {
            base: compile(&base)?.compile_matcher(),
            descendants: compile(&descendants)?.compile_matcher(),
        })
    }

    /// Whether a normalized logical directory path falls inside this scope.
    pub fn is_match(&self, logical_path: impl AsRef<Path>) -> bool {
        let path = logical_path.as_ref();
        match self {
            Self::Pattern(matcher) => matcher.is_match(path),
            Self::Subtree { base, descendants } => {
                base.is_match(path) || descendants.is_match(path)
            }
        }
    }
}

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
    fn scope_without_trailing_separator_keeps_plain_glob_semantics() {
        let exact = ScopeMatcher::compile("web/src").unwrap();
        assert!(exact.is_match(Path::new("web/src")));
        assert!(!exact.is_match(Path::new("web/src/components")));

        let descendants = ScopeMatcher::compile("web/src/**").unwrap();
        assert!(!descendants.is_match(Path::new("web/src")));
        assert!(descendants.is_match(Path::new("web/src/components")));
    }

    #[test]
    fn trailing_separator_scope_matches_base_and_descendants() {
        for scope in ["web/src/", r"web\src\", "web/src//"] {
            let matcher = ScopeMatcher::compile(scope).unwrap();
            assert!(matcher.is_match(Path::new("web/src")), "base for {scope}");
            assert!(
                matcher.is_match(Path::new("web/src/components/button")),
                "descendant for {scope}"
            );
            assert!(
                !matcher.is_match(Path::new("web/src2")),
                "sibling prefix for {scope}"
            );
            assert!(!matcher.is_match(Path::new("web")), "parent for {scope}");
        }
    }

    #[test]
    fn trailing_separator_scope_supports_brace_alternation() {
        let matcher = ScopeMatcher::compile("web/{src,test}/").unwrap();
        assert!(matcher.is_match(Path::new("web/src")));
        assert!(matcher.is_match(Path::new("web/test")));
        assert!(matcher.is_match(Path::new("web/test/fixtures")));
        assert!(!matcher.is_match(Path::new("web/docs")));
    }

    #[test]
    fn config_root_subtree_scope_matches_everything() {
        let matcher = ScopeMatcher::compile("./").unwrap();
        assert!(matcher.is_match(normalize_for_matching(Path::new("."))));
        assert!(matcher.is_match(Path::new("src")));
        assert!(matcher.is_match(Path::new("src/nested/deep")));
    }

    #[test]
    fn invalid_subtree_scope_error_reports_authored_pattern() {
        let error = ScopeMatcher::compile("web/{src/").unwrap_err();
        match error {
            SlocGuardError::InvalidPattern { pattern, .. } => assert_eq!(pattern, "web/{src/"),
            other => panic!("expected InvalidPattern, got {other:?}"),
        }
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
