use serde::{Deserialize, Serialize};

/// Supported config version. Current version is "2".
pub const CONFIG_VERSION: &str = "2";

// ============================================================================
// V2 Config Types (Scanner/Content/Structure separation)
// ============================================================================

/// Ratchet mode for baseline enforcement.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RatchetMode {
    /// Emit warning if baseline can be tightened (default)
    #[default]
    Warn,
    /// Auto-update baseline when violations decrease
    Auto,
    /// Fail CI if baseline is outdated
    Strict,
}

impl From<crate::cli::RatchetMode> for RatchetMode {
    fn from(cli_mode: crate::cli::RatchetMode) -> Self {
        match cli_mode {
            crate::cli::RatchetMode::Warn => Self::Warn,
            crate::cli::RatchetMode::Auto => Self::Auto,
            crate::cli::RatchetMode::Strict => Self::Strict,
        }
    }
}

/// Baseline configuration for grandfathering violations.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BaselineConfig {
    /// Ratchet enforcement mode.
    /// - `warn`: emit warning when baseline can be tightened (default when enabled)
    /// - `auto`: auto-update baseline when violations decrease
    /// - `strict`: fail CI if baseline is outdated
    #[serde(default)]
    pub ratchet: Option<RatchetMode>,
}

/// Trend tracking configuration for history retention policy.
///
/// Controls how many historical entries are kept and how often new entries
/// are recorded. This prevents unbounded growth of the history file.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrendConfig {
    /// Maximum number of entries to keep. Oldest entries are removed when exceeded.
    /// Default: None (unlimited).
    #[serde(default)]
    pub max_entries: Option<usize>,

    /// Maximum age of entries in days. Entries older than this are removed.
    /// Default: None (no age limit).
    #[serde(default)]
    pub max_age_days: Option<u64>,

    /// Minimum seconds between consecutive entries. New entries within this
    /// interval are skipped (deduplicated).
    /// Default: None (no minimum interval).
    #[serde(default)]
    pub min_interval_secs: Option<u64>,

    /// Minimum absolute code line delta to be considered significant.
    /// Trend output is suppressed when changes are below this threshold
    /// AND no files were added/removed. Reduces noise from trivial changes.
    ///
    /// Default: [`crate::stats::DEFAULT_MIN_CODE_DELTA`] (changes at or below this
    /// threshold without file changes are hidden).
    ///
    /// **Note**: Setting this to `0` means changes must *exceed* 0 lines (i.e., any
    /// non-zero change is significant). A delta of exactly `0` is never significant
    /// when this is set to `0`, because `0 > 0` is false.
    #[serde(default)]
    pub min_code_delta: Option<u64>,

    /// Automatically record a snapshot after a successful `check` command.
    /// Respects `min_interval_secs` to avoid duplicate entries.
    /// Default: false (disabled).
    #[serde(default)]
    pub auto_snapshot_on_check: Option<bool>,
}

/// Check command behavior configuration.
///
/// Controls how the `check` command handles warnings and failures.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckConfig {
    /// Treat warnings as errors (exit code 1).
    /// Equivalent to `--warnings-as-errors` CLI flag.
    #[serde(default)]
    pub warnings_as_errors: bool,

    /// Stop processing on first failure for faster feedback.
    /// When enabled, short-circuits file processing after detecting a failure.
    #[serde(default)]
    pub fail_fast: bool,
}

/// Stats command configuration, specifically for report subcommand defaults.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatsConfig {
    /// Report subcommand configuration.
    #[serde(default)]
    pub report: StatsReportConfig,
}

/// Configuration for `stats report` subcommand defaults.
///
/// Controls which sections to include in comprehensive reports and their defaults.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatsReportConfig {
    /// Sections to exclude from report output.
    /// Valid values: "summary", "files", "breakdown", "trend"
    #[serde(default)]
    pub exclude: Vec<String>,

    /// Number of top files to include in files section.
    /// Default: 20
    #[serde(default)]
    pub top_count: Option<usize>,

    /// Default grouping for breakdown section.
    /// Valid values: "lang", "dir"
    /// Default: "lang"
    #[serde(default)]
    pub breakdown_by: Option<String>,

    /// Maximum depth for directory grouping in breakdown section.
    /// Only applicable when `breakdown_by = "dir"`.
    /// 1 = top-level directories only, 2 = two levels, etc.
    /// Default: None (shows full path of each file's parent directory without grouping).
    #[serde(default)]
    pub depth: Option<usize>,

    /// Default comparison period for trend section (e.g., "7d", "1w", "30d").
    /// When set, uses this duration for trend comparison instead of latest entry.
    #[serde(default)]
    pub trend_since: Option<String>,
}

/// Scanner configuration for physical file discovery.
/// Scanner finds ALL files - no extension filtering here.
/// This ensures Structure Guard sees the complete directory structure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScannerConfig {
    /// Respect .gitignore rules (default: true)
    #[serde(default = "default_true")]
    pub gitignore: bool,

    /// Global exclude patterns (files/dirs to completely ignore by ALL checkers).
    /// These are ADDITIVE to .gitignore (union, not override).
    /// Default: `[".git/**"]` - always exclude .git directory.
    #[serde(default = "default_scanner_exclude")]
    pub exclude: Vec<String>,
}

/// Default scanner exclude patterns.
/// Always excludes `.git/**` to prevent structure checks on git internals.
fn default_scanner_exclude() -> Vec<String> {
    vec![".git/**".to_string()]
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            gitignore: true,
            exclude: default_scanner_exclude(),
        }
    }
}

/// Content configuration for SLOC limits.
/// Extensions filter is HERE (not in scanner) - only these files get SLOC analysis.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContentConfig {
    /// File extensions for SLOC counting.
    #[serde(default = "default_extensions")]
    pub extensions: Vec<String>,

    /// Maximum lines per file (global default).
    #[serde(default = "default_max_lines")]
    pub max_lines: usize,

    /// Warning threshold (0.0-1.0).
    #[serde(default = "default_warn_threshold")]
    pub warn_threshold: f64,

    /// Absolute line count at or above which warnings are triggered
    /// (takes precedence over percentage thresholds).
    /// Example: `max_lines=500`, `warn_at=450` → warns at 450+ lines.
    #[serde(default)]
    pub warn_at: Option<usize>,

    /// Skip comment lines in SLOC count.
    #[serde(default = "default_true")]
    pub skip_comments: bool,

    /// Skip blank lines in SLOC count.
    #[serde(default = "default_true")]
    pub skip_blank: bool,

    /// Glob patterns for files to exclude from content (SLOC) checks.
    /// These files are still visible for structure checks.
    #[serde(default)]
    pub exclude: Vec<String>,

    /// Path-based rules [[content.rules]].
    #[serde(default)]
    pub rules: Vec<ContentRule>,
}

impl Default for ContentConfig {
    fn default() -> Self {
        Self {
            extensions: default_extensions(),
            max_lines: default_max_lines(),
            warn_threshold: default_warn_threshold(),
            warn_at: None,
            skip_comments: true,
            skip_blank: true,
            exclude: Vec::new(),
            rules: Vec::new(),
        }
    }
}

/// Content rule for path-based SLOC limits [[content.rules]].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContentRule {
    /// Glob pattern for file matching.
    pub pattern: String,

    /// Maximum lines for matched files.
    pub max_lines: usize,

    /// Override warning threshold for matched files.
    #[serde(default)]
    pub warn_threshold: Option<f64>,

    /// Absolute line count at or above which warnings are triggered
    /// (takes precedence over percentage thresholds).
    /// Example: `max_lines=1000`, `warn_at=800` → warns at 800+ lines.
    #[serde(default)]
    pub warn_at: Option<usize>,

    /// Override `skip_comments` for matched files.
    #[serde(default)]
    pub skip_comments: Option<bool>,

    /// Override `skip_blank` for matched files.
    #[serde(default)]
    pub skip_blank: Option<bool>,

    /// Optional reason for this rule (audit trail, displayed in explain output).
    #[serde(default)]
    pub reason: Option<String>,

    /// Optional expiration date (YYYY-MM-DD). Past dates emit warnings.
    #[serde(default)]
    pub expires: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Config {
    /// Config schema version. Must be "2".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extends: Option<String>,

    /// SHA-256 hash of the remote config content for integrity verification.
    /// When provided, the fetched content's hash must match this value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extends_sha256: Option<String>,

    /// Scanner configuration (file discovery).
    #[serde(default)]
    pub scanner: ScannerConfig,

    /// Content configuration (SLOC limits).
    #[serde(default)]
    pub content: ContentConfig,

    /// Structure configuration (directory limits).
    #[serde(default)]
    pub structure: StructureConfig,

    /// Baseline configuration (grandfathering/ratchet).
    #[serde(default)]
    pub baseline: BaselineConfig,

    /// Trend tracking configuration (history retention policy).
    #[serde(default)]
    pub trend: TrendConfig,

    /// Stats command configuration.
    #[serde(default)]
    pub stats: StatsConfig,

    /// Check command behavior configuration.
    #[serde(default)]
    pub check: CheckConfig,

    /// Custom language definitions (comment syntax).
    #[serde(default)]
    pub languages: std::collections::HashMap<String, CustomLanguageConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomLanguageConfig {
    #[serde(default)]
    pub extensions: Vec<String>,

    #[serde(default)]
    pub single_line_comments: Vec<String>,

    #[serde(default)]
    pub multi_line_comments: Vec<(String, String)>,
}

/// Default maximum lines per file for SLOC checks.
pub const DEFAULT_MAX_LINES: usize = 600;

const fn default_max_lines() -> usize {
    DEFAULT_MAX_LINES
}

fn default_extensions() -> Vec<String> {
    vec![
        // Systems programming
        "rs".to_string(),
        "go".to_string(),
        "c".to_string(),
        "cpp".to_string(),
        // JVM languages
        "java".to_string(),
        "kt".to_string(),
        "scala".to_string(),
        // .NET
        "cs".to_string(),
        // Web/Frontend
        "js".to_string(),
        "ts".to_string(),
        "tsx".to_string(),
        "jsx".to_string(),
        "vue".to_string(),
        // Mobile
        "swift".to_string(),
        "dart".to_string(),
        // Scripting
        "py".to_string(),
        "rb".to_string(),
        "php".to_string(),
        "lua".to_string(),
        "sh".to_string(),
    ]
}

const fn default_true() -> bool {
    true
}

const fn default_warn_threshold() -> f64 {
    0.9
}

/// Sentinel value representing unlimited (no check).
/// Use `-1` in TOML to indicate no limit should be applied.
pub const UNLIMITED: i64 = -1;

/// Severity level for sibling violations (defaults to error if not specified).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SiblingSeverity {
    /// Treat as warning, not a hard failure.
    Warn,
    /// Treat as error (default).
    #[default]
    Error,
}

/// Sibling rule for file co-location checking.
///
/// Supports two rule types:
/// - **Directed**: If a file matches `match_pattern`, require specific sibling(s).
/// - **Group**: If ANY file in the group exists, ALL must exist (atomic group).
///
/// **Note**: Ambiguous configs containing both `match`/`require` AND `group` fields
/// are rejected during deserialization with a clear error message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum SiblingRule {
    /// Directed rule: if file matches, require sibling(s).
    /// Example: `{ match = "*.tsx", require = "{stem}.test.tsx" }`
    Directed {
        /// Glob pattern for files that trigger the rule.
        match_pattern: String,
        /// Required sibling pattern(s). Use `{stem}` for the source file's stem.
        require: SiblingRequire,
        /// Severity level (default: error).
        severity: SiblingSeverity,
    },
    /// Atomic group: if ANY file in the group exists, ALL must exist.
    /// Example: `{ group = ["{stem}.tsx", "{stem}.test.tsx", "{stem}.module.css"] }`
    Group {
        /// Patterns that form an atomic group. Use `{stem}` for the base name.
        group: Vec<String>,
        /// Severity level (default: error).
        severity: SiblingSeverity,
    },
}

/// Intermediate struct for deserializing `SiblingRule` with ambiguity detection.
#[derive(Deserialize)]
struct SiblingRuleHelper {
    #[serde(rename = "match")]
    match_pattern: Option<String>,
    require: Option<SiblingRequire>,
    group: Option<Vec<String>>,
    #[serde(default)]
    severity: SiblingSeverity,
}

impl<'de> serde::Deserialize<'de> for SiblingRule {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let helper = SiblingRuleHelper::deserialize(deserializer)?;

        let has_directed = helper.match_pattern.is_some() || helper.require.is_some();
        let has_group = helper.group.is_some();

        match (has_directed, has_group) {
            (true, true) => Err(serde::de::Error::custom(
                "Ambiguous sibling rule: cannot mix 'match'/'require' with 'group'. \
                 Use either Directed ({ match = \"...\", require = \"...\" }) \
                 OR Group ({ group = [...] }), not both.",
            )),
            (true, false) => {
                let match_pattern = helper.match_pattern.ok_or_else(|| {
                    serde::de::Error::custom("Directed sibling rule requires 'match' field.")
                })?;
                let require = helper.require.ok_or_else(|| {
                    serde::de::Error::custom("Directed sibling rule requires 'require' field.")
                })?;
                Ok(Self::Directed {
                    match_pattern,
                    require,
                    severity: helper.severity,
                })
            }
            (false, true) => {
                // Safe to unwrap: has_group is true
                let group = helper.group.unwrap();
                Ok(Self::Group {
                    group,
                    severity: helper.severity,
                })
            }
            (false, false) => Err(serde::de::Error::custom(
                "Invalid sibling rule: must specify either 'match'/'require' (Directed) \
                 or 'group' (Group).",
            )),
        }
    }
}

/// Sibling require field: single pattern or array of patterns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SiblingRequire {
    /// Single required sibling pattern.
    Single(String),
    /// Multiple required sibling patterns.
    Multiple(Vec<String>),
}

impl SiblingRequire {
    /// Convert to a vector of patterns.
    #[must_use]
    pub fn as_patterns(&self) -> Vec<&str> {
        match self {
            Self::Single(s) => vec![s.as_str()],
            Self::Multiple(v) => v.iter().map(String::as_str).collect(),
        }
    }
}

/// Configuration for directory structure limits.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct StructureConfig {
    /// Global default limit for files per directory.
    /// Use `-1` for unlimited (no check), `0` for prohibited, `>0` for limit.
    #[serde(default)]
    pub max_files: Option<i64>,

    /// Global default limit for subdirectories per directory.
    /// Use `-1` for unlimited (no check), `0` for prohibited, `>0` for limit.
    #[serde(default)]
    pub max_dirs: Option<i64>,

    /// Global default limit for maximum directory nesting depth.
    /// Depth 0 = scan root, depth 1 = direct subdirectories.
    /// Use `-1` for unlimited (no check), `>0` for limit.
    #[serde(default)]
    pub max_depth: Option<i64>,

    /// Threshold (0.0-1.0) at which warnings are issued before hitting hard limits.
    /// Example: `max_files=50`, `warn_threshold=0.9` → warns at 45 files.
    #[serde(default)]
    pub warn_threshold: Option<f64>,

    /// Absolute file count at or above which warnings are triggered
    /// (takes precedence over percentage thresholds).
    /// Example: `max_files=50`, `warn_files_at=45` → warns at 45+ files.
    #[serde(default)]
    pub warn_files_at: Option<i64>,

    /// Absolute directory count at or above which warnings are triggered
    /// (takes precedence over percentage thresholds).
    /// Example: `max_dirs=10`, `warn_dirs_at=8` → warns at 8+ directories.
    #[serde(default)]
    pub warn_dirs_at: Option<i64>,

    /// Percentage threshold (0.0-1.0) for file count warnings (overrides global `warn_threshold`).
    /// Example: `max_files=50`, `warn_files_threshold=0.8` → warns at 40 files.
    #[serde(default)]
    pub warn_files_threshold: Option<f64>,

    /// Percentage threshold (0.0-1.0) for directory count warnings (overrides global `warn_threshold`).
    /// Example: `max_dirs=10`, `warn_dirs_threshold=0.5` → warns at 5 directories.
    #[serde(default)]
    pub warn_dirs_threshold: Option<f64>,

    /// Glob patterns for items not counted in structure limits (e.g., "*.md", ".gitkeep").
    /// These items are still visible but don't count toward file/dir quotas.
    #[serde(default)]
    pub count_exclude: Vec<String>,

    /// Global deny list of file extensions (with leading dot, e.g., ".exe", ".dll").
    /// Files matching these extensions trigger immediate violations.
    #[serde(default)]
    pub deny_extensions: Vec<String>,

    /// Global deny list of file patterns (glob patterns, e.g., "*.bak", "temp_*").
    /// Files matching these patterns trigger immediate violations.
    #[serde(default)]
    pub deny_patterns: Vec<String>,

    /// Global deny list of file name patterns (glob patterns, e.g., "temp_*", "secrets.*").
    /// Matches file names (basenames) only, not full paths or directories.
    #[serde(default, alias = "deny_file_patterns")]
    pub deny_files: Vec<String>,

    /// Global deny list of directory name patterns (glob patterns, e.g., "`node_modules`", "`__pycache__`").
    /// Matches directory names (basenames) only, not full paths.
    #[serde(default)]
    pub deny_dirs: Vec<String>,

    /// Global allowlist of file name patterns (glob patterns, e.g., "README.md", "LICENSE").
    /// When set, only matching files are permitted; all others trigger violations.
    /// Mutually exclusive with deny_* fields at global level.
    #[serde(default)]
    pub allow_files: Vec<String>,

    /// Global allowlist of directory name patterns (glob patterns, e.g., "src", "tests").
    /// When set, only matching directories are permitted; all others trigger violations.
    /// Mutually exclusive with deny_* fields at global level.
    #[serde(default)]
    pub allow_dirs: Vec<String>,

    /// Global allowlist of file extensions (with leading dot, e.g., ".rs", ".py").
    /// When set, only files with matching extensions are permitted.
    /// Mutually exclusive with deny_* fields at global level.
    #[serde(default)]
    pub allow_extensions: Vec<String>,

    /// Per-directory rules that override global limits.
    #[serde(default)]
    pub rules: Vec<StructureRule>,
}

impl StructureConfig {
    /// Check if any structure checking is configured.
    ///
    /// Returns `true` if any limits, rules, or denylists are defined.
    #[must_use]
    pub const fn is_enabled(&self) -> bool {
        self.max_files.is_some()
            || self.max_dirs.is_some()
            || self.max_depth.is_some()
            || !self.rules.is_empty()
            // Global allowlist mode should still enable structure scanning so that
            // allowlist violations can be detected and reported even without limits.
            || !self.allow_extensions.is_empty()
            || !self.allow_files.is_empty()
            || !self.allow_dirs.is_empty()
            || !self.deny_extensions.is_empty()
            || !self.deny_patterns.is_empty()
            || !self.deny_files.is_empty()
            || !self.deny_dirs.is_empty()
    }
}

/// Rule for overriding structure limits on specific directories.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct StructureRule {
    /// Glob pattern defining the directory scope where this rule applies.
    /// Example: `scope = "src/**"` applies to all directories under `src/`.
    pub scope: String,

    /// Override limit for files in matched directories.
    /// Use `-1` for unlimited (no check), `0` for prohibited, `>0` for limit.
    #[serde(default)]
    pub max_files: Option<i64>,

    /// Override limit for subdirectories in matched directories.
    /// Use `-1` for unlimited (no check), `0` for prohibited, `>0` for limit.
    #[serde(default)]
    pub max_dirs: Option<i64>,

    /// Override limit for maximum depth in matched directories.
    /// Use `-1` for unlimited (no check), `>0` for limit.
    #[serde(default)]
    pub max_depth: Option<i64>,

    /// When true, `max_depth` is measured relative to the scope's base directory
    /// instead of from the scan root.
    /// Example: `scope="src/features/**"`, `relative_depth=true`, `max_depth=2`
    /// → checks depth within src/features/, not from project root.
    #[serde(default)]
    pub relative_depth: bool,

    /// Override threshold for warnings in matched directories.
    #[serde(default)]
    pub warn_threshold: Option<f64>,

    /// Absolute file count at or above which warnings are triggered
    /// (takes precedence over percentage thresholds).
    #[serde(default)]
    pub warn_files_at: Option<i64>,

    /// Absolute directory count at or above which warnings are triggered
    /// (takes precedence over percentage thresholds).
    #[serde(default)]
    pub warn_dirs_at: Option<i64>,

    /// Percentage threshold (0.0-1.0) for file count warnings (overrides global `warn_threshold`).
    #[serde(default)]
    pub warn_files_threshold: Option<f64>,

    /// Percentage threshold (0.0-1.0) for directory count warnings (overrides global `warn_threshold`).
    #[serde(default)]
    pub warn_dirs_threshold: Option<f64>,

    /// Allowlist of allowed file extensions (with leading dot, e.g., ".rs", ".go").
    /// Files NOT matching these extensions are violations.
    /// Combined with `allow_patterns` using OR logic.
    #[serde(default)]
    pub allow_extensions: Vec<String>,

    /// Allowlist of allowed file patterns (glob patterns, e.g., "*.rs", "mod.*").
    /// Files NOT matching these patterns are violations.
    /// Combined with `allow_extensions` using OR logic.
    #[serde(default)]
    pub allow_patterns: Vec<String>,

    /// Allowlist of file name patterns (glob patterns, e.g., "README.md", "config.*").
    /// When set, only matching file names are permitted in this scope.
    /// Combined with `allow_extensions` and `allow_patterns` using OR logic.
    /// Mutually exclusive with deny_* fields at rule level.
    #[serde(default)]
    pub allow_files: Vec<String>,

    /// Allowlist of directory name patterns (glob patterns, e.g., "utils", "helpers").
    /// When set, only matching directory names are permitted in this scope.
    /// Mutually exclusive with deny_* fields at rule level.
    #[serde(default)]
    pub allow_dirs: Vec<String>,

    /// Deny list of file extensions (with leading dot, e.g., ".exe", ".dll").
    /// Files matching these extensions trigger immediate violations.
    /// Checked BEFORE allowlist - if denied, file is rejected regardless of allowlist.
    #[serde(default)]
    pub deny_extensions: Vec<String>,

    /// Deny list of file patterns (glob patterns, e.g., "*.bak", "temp_*").
    /// Files matching these patterns trigger immediate violations.
    /// Checked BEFORE allowlist - if denied, file is rejected regardless of allowlist.
    #[serde(default)]
    pub deny_patterns: Vec<String>,

    /// Deny list of file name patterns (glob patterns, e.g., "temp_*", "secrets.*").
    /// Matches file names (basenames) only, not full paths.
    /// Checked BEFORE allowlist - if denied, file is rejected regardless of allowlist.
    #[serde(default, alias = "deny_file_patterns")]
    pub deny_files: Vec<String>,

    /// Deny list of directory name patterns (glob patterns, e.g., "`temp_*`", "`__pycache__`").
    /// Matches directory names (basenames) only.
    /// Checked BEFORE other directory processing.
    #[serde(default)]
    pub deny_dirs: Vec<String>,

    /// Regex pattern for filename validation.
    /// Files not matching this pattern trigger a `NamingConvention` violation.
    /// Example: `^[A-Z][a-zA-Z0-9]*\.tsx$` for `PascalCase` React components.
    #[serde(default)]
    pub file_naming_pattern: Option<String>,

    /// Sibling rules for file co-location checking.
    /// Supports two rule types:
    /// - Directed: `{ match = "*.tsx", require = "{stem}.test.tsx" }`
    /// - Group: `{ group = ["{stem}.tsx", "{stem}.test.tsx"] }`
    #[serde(default)]
    pub siblings: Vec<SiblingRule>,

    /// Optional reason for this rule (audit trail, displayed in explain output).
    #[serde(default)]
    pub reason: Option<String>,

    /// Optional expiration date (YYYY-MM-DD). Past dates emit warnings.
    #[serde(default)]
    pub expires: Option<String>,
}

#[cfg(test)]
#[path = "model_tests/mod.rs"]
mod model_tests;
