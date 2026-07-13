use std::path::PathBuf;

use serde::Serialize;

/// Source of the effective `warn_at` value for debugging.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WarnAtSource {
    /// Absolute value from a content rule's `warn_at` field.
    RuleAbsolute { index: usize },
    /// Percentage threshold from a content rule.
    RulePercentage { index: usize, threshold: f64 },
    /// Absolute value from global `content.warn_at`.
    GlobalAbsolute,
    /// Percentage threshold from global `content.warn_threshold`.
    GlobalPercentage { threshold: f64 },
}

/// Match status for a rule candidate in the evaluation chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchStatus {
    /// This rule was selected (highest priority match)
    Matched,
    /// Pattern matched but superseded by higher priority rule
    Superseded,
    /// Pattern did not match the path
    NoMatch,
}

/// Which type of content rule matched for a file.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentRuleMatch {
    /// File excluded from content checks via `content.exclude`
    Excluded { pattern: String },
    /// Matched a `[[content.rules]]` pattern
    Rule {
        index: usize,
        pattern: String,
        reason: Option<String>,
    },
    /// No explicit rule matched; using global defaults
    Default,
}

/// A candidate rule evaluated during content rule matching.
#[derive(Debug, Clone, Serialize)]
pub struct ContentRuleCandidate {
    /// Source identifier (e.g., "content.rules[0]", "content.rules[2]")
    pub source: String,
    /// Glob pattern or path (if applicable)
    pub pattern: Option<String>,
    /// Line limit for this rule
    pub limit: usize,
    /// Match status
    pub status: MatchStatus,
}

/// Explanation of which content rule matched for a file.
#[derive(Debug, Clone, Serialize)]
pub struct ContentExplanation {
    /// Path being explained
    pub path: PathBuf,
    /// Whether file is excluded from content checks via `content.exclude`
    pub is_excluded: bool,
    /// Which rule was ultimately selected
    pub matched_rule: ContentRuleMatch,
    /// Effective line limit applied (0 if excluded)
    pub effective_limit: usize,
    /// Effective line count at which warnings are triggered
    pub effective_warn_at: usize,
    /// Source of the effective `warn_at` value (for debugging).
    pub warn_at_source: WarnAtSource,
    /// Warning threshold (0.0-1.0) - retained for reference/debugging
    pub warn_threshold: f64,
    /// Whether comments are skipped
    pub skip_comments: bool,
    /// Whether blank lines are skipped
    pub skip_blank: bool,
    /// All candidates evaluated (for debugging)
    pub rule_chain: Vec<ContentRuleCandidate>,
}

/// Which type of structure rule matched for a directory.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StructureRuleMatch {
    /// Matched a `[[structure.rules]]` pattern
    Rule {
        index: usize,
        pattern: String,
        reason: Option<String>,
    },
    /// No explicit rule matched; using global defaults
    Default,
}

/// Provenance of an active `count_exclude` pattern.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CountExcludeSource {
    /// From global `structure.count_exclude`.
    Global,
    /// From the winning rule's `count_exclude`.
    Rule { index: usize, scope: String },
}

/// An active count-exclusion pattern with provenance and its concrete hits.
///
/// The counting caliber is the union of the global set and the winning
/// (last-match) rule's set; superseded rules contribute nothing, even when
/// their patterns would match.
#[derive(Debug, Clone, Serialize)]
pub struct CountExcludePattern {
    /// The glob pattern as authored in the configuration.
    pub pattern: String,
    /// Where the pattern comes from (global section or the winning rule).
    pub source: CountExcludeSource,
    /// Immediate child files this pattern excludes from counting (sorted).
    /// Empty when no directory inventory was supplied.
    pub excluded_files: Vec<String>,
    /// Immediate child directories this pattern excludes from counting (sorted).
    pub excluded_dirs: Vec<String>,
}

/// Raw vs effective child counts for an explained directory.
///
/// Raw counts describe the supplied inventory; effective counts result from
/// applying the active `count_exclude` union and are what limits compare
/// against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct StructureCounts {
    pub raw_file_count: usize,
    pub raw_dir_count: usize,
    pub effective_file_count: usize,
    pub effective_dir_count: usize,
}

/// A candidate rule evaluated during structure rule matching.
#[derive(Debug, Clone, Serialize)]
pub struct StructureRuleCandidate {
    /// Source identifier (e.g., "structure.rules[0]", "structure.rules[1]")
    pub source: String,
    /// Glob pattern or path (if applicable)
    pub pattern: Option<String>,
    /// Max files limit (-1 for unlimited)
    pub max_files: Option<i64>,
    /// Max directories limit (-1 for unlimited)
    pub max_dirs: Option<i64>,
    /// Max depth limit (-1 for unlimited)
    pub max_depth: Option<i64>,
    /// Match status
    pub status: MatchStatus,
}

/// Explanation of which structure rule matched for a directory.
#[derive(Debug, Clone, Serialize)]
pub struct StructureExplanation {
    /// Path being explained
    pub path: PathBuf,
    /// Which rule was ultimately selected
    pub matched_rule: StructureRuleMatch,
    /// Effective max files limit (-1 for unlimited)
    pub effective_max_files: Option<i64>,
    /// Effective max directories limit (-1 for unlimited)
    pub effective_max_dirs: Option<i64>,
    /// Effective max depth limit (-1 for unlimited)
    pub effective_max_depth: Option<i64>,
    /// Warning threshold (0.0-1.0)
    pub warn_threshold: f64,
    /// Override reason if applicable
    pub override_reason: Option<String>,
    /// Active count-exclusion patterns: the global set first, then the
    /// winning rule's (their union defines the counting caliber)
    pub count_exclude: Vec<CountExcludePattern>,
    /// Raw vs effective counts; `None` when no child inventory was supplied
    pub counts: Option<StructureCounts>,
    /// All candidates evaluated (for debugging)
    pub rule_chain: Vec<StructureRuleCandidate>,
}
