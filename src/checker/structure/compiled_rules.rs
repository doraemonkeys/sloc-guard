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
    /// Optional reason for this rule (audit trail).
    pub reason: Option<String>,
}

/// Compiled sibling rule for file co-location checking.
///
/// Contains pre-compiled glob matchers for efficient runtime matching.
/// Created from [`crate::config::SiblingRule`] during checker initialization.
pub(super) enum CompiledSiblingRule {
    /// Directed rule: if file matches, require sibling(s).
    ///
    /// When a file matches `file_matcher` in a directory matching `dir_matcher`,
    /// each template in `sibling_templates` is expanded using `{stem}` substitution
    /// to derive required sibling file paths.
    Directed {
        /// Original directory scope string from config (for violation messages).
        dir_scope: String,
        /// Pre-compiled matcher for directory (parent of files).
        dir_matcher: GlobMatcher,
        /// Pre-compiled matcher for files that trigger the rule.
        file_matcher: GlobMatcher,
        /// Templates for deriving sibling filename(s), e.g., `"{stem}.test.tsx"`.
        /// The `{stem}` placeholder is replaced with the source file's stem.
        sibling_templates: Vec<String>,
        /// When `true`, violations are warnings instead of errors.
        is_warning: bool,
    },
    /// Atomic group: if ANY file in the group exists, ALL must exist.
    ///
    /// Enforces file co-location by treating a set of patterns as an atomic unit.
    /// If any file matching a group pattern exists, all other group members must also exist.
    Group {
        /// Original directory scope string from config (for violation messages).
        dir_scope: String,
        /// Pre-compiled matcher for directory (parent of files).
        dir_matcher: GlobMatcher,
        /// Patterns that form an atomic set, e.g., `["{stem}.tsx", "{stem}.test.tsx"]`.
        /// Each pattern must contain `{stem}` for stem extraction and expansion.
        group_patterns: Vec<String>,
        /// When `true`, violations are warnings instead of errors.
        is_warning: bool,
    },
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
