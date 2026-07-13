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
    /// Concrete children the pattern excludes. `None` when no inventory was
    /// supplied (the field is then omitted from JSON); `Some` with empty
    /// lists when an inventory existed but the pattern excluded nothing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hits: Option<CountExcludeHits>,
}

/// Immediate children a count-exclusion pattern removes from the counts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct CountExcludeHits {
    /// Excluded child files (sorted).
    pub files: Vec<String>,
    /// Excluded child directories (sorted).
    pub dirs: Vec<String>,
}

/// Raw vs effective child counts for an explained directory.
///
/// Raw counts describe the supplied inventory; effective counts result from
/// applying the active `count_exclude` union to that inventory and are what
/// limits compare against for the same roster. The CLI supplies the
/// config-driven scan's roster (see [`DirInventorySource`]), so these are the
/// numbers `check` enforces unless check-time CLI flags alter its scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct StructureCounts {
    pub raw_file_count: usize,
    pub raw_dir_count: usize,
    pub effective_file_count: usize,
    pub effective_dir_count: usize,
}

/// Child inventory supplied to [`crate::checker::StructureChecker::explain`].
///
/// The CLI inventories a directory by scanning under the loaded
/// configuration's exclusion regime (`scanner.exclude`, gitignore, no-follow
/// symlink handling) — the same regime `check` scans under. Check-time CLI
/// flags (`--exclude`, `--no-gitignore`, scan roots) can still change what
/// `check` sees; that residual divergence is not knowable from configuration
/// and is labeled, not resolved, in the output.
#[derive(Debug, Clone, Copy)]
pub enum DirInventorySource<'a> {
    /// Roster produced by the config-driven scan.
    ConfiguredScan(&'a super::DirStats),
    /// The config-driven scan never reaches this directory (scanner.exclude,
    /// gitignore, or symlink handling prunes it), so `check` has nothing to
    /// count here.
    ExcludedFromScan,
    /// No scan was performed; counts and per-pattern hits are unknowable.
    NotScanned,
}

/// Directory child inventory outcome in a structure explanation.
///
/// Mirrors [`DirInventorySource`], with counts materialized for the scanned
/// case. The serialized `basis` tag tells JSON consumers whether the counts
/// exist and where the roster came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "basis", rename_all = "snake_case")]
pub enum DirInventory {
    /// Counts measured from the config-driven scan's roster.
    ConfiguredScan { counts: StructureCounts },
    /// The config-driven scan excludes this directory; `check` does not
    /// evaluate limits for it under this configuration.
    ExcludedFromScan,
    /// No inventory was supplied; counts and per-pattern hits are omitted.
    NotScanned,
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
    /// Child inventory outcome; counts are present when a roster was supplied
    pub inventory: DirInventory,
    /// All candidates evaluated (for debugging)
    pub rule_chain: Vec<StructureRuleCandidate>,
}
