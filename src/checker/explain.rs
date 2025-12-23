use std::path::PathBuf;

use serde::Serialize;

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
    /// Source identifier (e.g., "content.overrides[0]", "content.rules[2]")
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
    /// Warning threshold (0.0-1.0)
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

/// A candidate rule evaluated during structure rule matching.
#[derive(Debug, Clone, Serialize)]
pub struct StructureRuleCandidate {
    /// Source identifier (e.g., "structure.overrides[0]", "structure.rules[1]")
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
    /// All candidates evaluated (for debugging)
    pub rule_chain: Vec<StructureRuleCandidate>,
}
