//! Compiled rule models for structure checking.
//!
//! Contains pre-compiled glob matchers and resolved limit structures
//! used internally by the `StructureChecker` implementation.

use globset::GlobMatcher;

/// Compiled structure rule with precompiled glob matcher.
pub(super) struct CompiledStructureRule {
    pub scope: String,
    pub matcher: GlobMatcher,
    pub max_files: Option<i64>,
    pub max_dirs: Option<i64>,
    pub max_depth: Option<i64>,
    /// When true, `max_depth` is relative to the pattern's base directory.
    pub relative_depth: bool,
    /// Depth of the pattern's base directory (components before first glob).
    /// Only meaningful when `relative_depth` is true.
    pub base_depth: usize,
    pub warn_threshold: Option<f64>,
    /// Absolute file count at which to warn.
    pub warn_files_at: Option<i64>,
    /// Absolute directory count at which to warn.
    pub warn_dirs_at: Option<i64>,
    /// Percentage threshold for file count warnings.
    pub warn_files_threshold: Option<f64>,
    /// Percentage threshold for directory count warnings.
    pub warn_dirs_threshold: Option<f64>,
}

/// Compiled sibling rule for file co-location checking.
pub(super) struct CompiledSiblingRule {
    /// Original directory scope string (for violation messages).
    pub dir_scope: String,
    /// Matcher for directory (parent of files).
    pub dir_matcher: GlobMatcher,
    /// Matcher for files that require a sibling.
    pub file_matcher: GlobMatcher,
    /// Template for deriving sibling filename (e.g., "{stem}.test.tsx").
    pub sibling_template: String,
}

/// Resolved limits for a directory path.
#[derive(Debug, Clone, Default)]
pub(super) struct StructureLimits {
    pub max_files: Option<i64>,
    pub max_dirs: Option<i64>,
    pub max_depth: Option<i64>,
    /// When true, `max_depth` is relative to `base_depth`.
    pub relative_depth: bool,
    /// Depth of the matched rule's base directory.
    pub base_depth: usize,
    pub warn_threshold: Option<f64>,
    /// Absolute file count at which to warn.
    pub warn_files_at: Option<i64>,
    /// Absolute directory count at which to warn.
    pub warn_dirs_at: Option<i64>,
    /// Percentage threshold for file count warnings.
    pub warn_files_threshold: Option<f64>,
    /// Percentage threshold for directory count warnings.
    pub warn_dirs_threshold: Option<f64>,
    pub override_reason: Option<String>,
}
