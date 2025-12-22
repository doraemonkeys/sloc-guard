use serde::{Deserialize, Serialize};

/// Supported config version. Current version is "2".
pub const CONFIG_VERSION: &str = "2";

/// Legacy config version for migration support.
pub const CONFIG_VERSION_V1: &str = "1";

// ============================================================================
// V2 Config Types (Scanner/Content/Structure separation)
// ============================================================================

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
    #[serde(default)]
    pub exclude: Vec<String>,
}

impl Default for ScannerConfig {
    fn default() -> Self {
        Self {
            gitignore: true,
            exclude: Vec::new(),
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

    /// Skip comment lines in SLOC count.
    #[serde(default = "default_true")]
    pub skip_comments: bool,

    /// Skip blank lines in SLOC count.
    #[serde(default = "default_true")]
    pub skip_blank: bool,

    /// Strict mode: exit with error on first violation.
    #[serde(default)]
    pub strict: bool,

    /// Glob patterns for files to exclude from content (SLOC) checks.
    /// These files are still visible for structure checks.
    #[serde(default)]
    pub exclude: Vec<String>,

    /// Path-based rules [[content.rules]].
    #[serde(default)]
    pub rules: Vec<ContentRule>,

    /// Language-based shorthand rules [content.languages.X].
    #[serde(default)]
    pub languages: std::collections::HashMap<String, LanguageRule>,

    /// File-level overrides [[content.override]].
    #[serde(default, rename = "override")]
    pub overrides: Vec<ContentOverride>,
}

impl Default for ContentConfig {
    fn default() -> Self {
        Self {
            extensions: default_extensions(),
            max_lines: default_max_lines(),
            warn_threshold: default_warn_threshold(),
            skip_comments: true,
            skip_blank: true,
            strict: false,
            exclude: Vec::new(),
            rules: Vec::new(),
            languages: std::collections::HashMap::new(),
            overrides: Vec::new(),
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

    /// Override `skip_comments` for matched files.
    #[serde(default)]
    pub skip_comments: Option<bool>,

    /// Override `skip_blank` for matched files.
    #[serde(default)]
    pub skip_blank: Option<bool>,
}

/// Language-based shorthand rule [content.languages.X].
/// Equivalent to [[content.rules]] pattern = "**/*.X".
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct LanguageRule {
    /// Maximum lines for this language.
    #[serde(default)]
    pub max_lines: Option<usize>,

    /// Warning threshold for this language.
    #[serde(default)]
    pub warn_threshold: Option<f64>,

    /// Skip comment lines for this language.
    #[serde(default)]
    pub skip_comments: Option<bool>,

    /// Skip blank lines for this language.
    #[serde(default)]
    pub skip_blank: Option<bool>,
}

/// Content override for specific files [[content.override]].
/// Only for FILES. Use to grandfather legacy files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContentOverride {
    /// Exact path to the file.
    pub path: String,

    /// Maximum lines allowed for this file.
    pub max_lines: usize,

    /// Reason for the override (required for audit trail).
    pub reason: String,
}

// ============================================================================
// V1 Legacy Types (kept for backward compatibility during migration)
// ============================================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Config {
    /// Config schema version. "2" for new schema, "1" or missing for legacy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extends: Option<String>,

    /// SHA-256 hash of the remote config content for integrity verification.
    /// When provided, the fetched content's hash must match this value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extends_sha256: Option<String>,

    // ========== V2 Fields ==========
    /// Scanner configuration (file discovery).
    #[serde(default)]
    pub scanner: ScannerConfig,

    /// Content configuration (SLOC limits).
    #[serde(default)]
    pub content: ContentConfig,

    /// Structure configuration (directory limits).
    #[serde(default)]
    pub structure: StructureConfig,

    // ========== V1 Legacy Fields (for migration) ==========
    // These fields are deserialized but not serialized (skip_serializing).
    // The loader migrates them to V2 fields.
    /// Legacy: default config (migrated to scanner + content).
    #[serde(default, skip_serializing)]
    pub default: DefaultConfig,

    /// Legacy: extension-based rules (migrated to content.languages).
    #[serde(default, skip_serializing)]
    pub rules: std::collections::HashMap<String, RuleConfig>,

    /// Legacy: path-based rules (DEPRECATED - use content.rules instead).
    /// This field is only used for detection to emit a clear error message.
    #[serde(default, skip_serializing)]
    pub path_rules: Vec<PathRuleDeprecated>,

    /// Legacy: exclude config (migrated to scanner.exclude).
    #[serde(default, skip_serializing)]
    pub exclude: ExcludeConfig,

    /// Legacy: file overrides (migrated to content.overrides).
    #[serde(default, skip_serializing, rename = "override")]
    pub overrides: Vec<FileOverride>,

    /// Custom language definitions (comment syntax).
    /// Kept at top level for both V1 and V2.
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct DefaultConfig {
    #[serde(default = "default_max_lines")]
    pub max_lines: usize,

    #[serde(default = "default_extensions")]
    pub extensions: Vec<String>,

    #[serde(default)]
    pub include_paths: Vec<String>,

    #[serde(default = "default_true")]
    pub skip_comments: bool,

    #[serde(default = "default_true")]
    pub skip_blank: bool,

    #[serde(default = "default_warn_threshold")]
    pub warn_threshold: f64,

    #[serde(default)]
    pub strict: bool,

    #[serde(default = "default_true")]
    pub gitignore: bool,
}

impl Default for DefaultConfig {
    fn default() -> Self {
        Self {
            max_lines: default_max_lines(),
            extensions: default_extensions(),
            include_paths: Vec::new(),
            skip_comments: true,
            skip_blank: true,
            warn_threshold: default_warn_threshold(),
            strict: false,
            gitignore: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuleConfig {
    #[serde(default)]
    pub extensions: Vec<String>,

    pub max_lines: Option<usize>,

    #[serde(default)]
    pub skip_comments: Option<bool>,

    #[serde(default)]
    pub skip_blank: Option<bool>,

    #[serde(default)]
    pub warn_threshold: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExcludeConfig {
    #[serde(default)]
    pub patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileOverride {
    pub path: String,
    pub max_lines: usize,
    #[serde(default)]
    pub reason: Option<String>,
}

/// Deprecated: use `ContentRule` in `[[content.rules]]` instead.
/// This type exists only for deserializing V1 configs to emit clear error messages.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PathRuleDeprecated {
    #[serde(default)]
    pub pattern: String,
    #[serde(default)]
    pub max_lines: usize,
    #[serde(default)]
    pub warn_threshold: Option<f64>,
}

const fn default_max_lines() -> usize {
    500
}

fn default_extensions() -> Vec<String> {
    vec![
        "rs".to_string(),
        "go".to_string(),
        "py".to_string(),
        "js".to_string(),
        "ts".to_string(),
        "c".to_string(),
        "cpp".to_string(),
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

    /// Absolute file count at which to warn (takes precedence over percentage thresholds).
    /// Example: `max_files=50`, `warn_files_at=45` → warns at exactly 45 files.
    #[serde(default)]
    pub warn_files_at: Option<i64>,

    /// Absolute directory count at which to warn (takes precedence over percentage thresholds).
    /// Example: `max_dirs=10`, `warn_dirs_at=8` → warns at exactly 8 directories.
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

    /// Per-directory rules that override global limits.
    #[serde(default)]
    pub rules: Vec<StructureRule>,

    /// Directory-level overrides [[structure.override]].
    #[serde(default, rename = "override")]
    pub overrides: Vec<StructureOverride>,
}

/// Rule for overriding structure limits on specific directories.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StructureRule {
    /// Glob pattern defining the directory scope where this rule applies.
    /// Example: `scope = "src/**"` applies to all directories under `src/`.
    #[serde(alias = "pattern")]
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

    /// Absolute file count at which to warn (takes precedence over percentage thresholds).
    #[serde(default)]
    pub warn_files_at: Option<i64>,

    /// Absolute directory count at which to warn (takes precedence over percentage thresholds).
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

    /// Glob pattern for files that require a sibling file.
    /// Only files matching this pattern are checked for siblings.
    /// Must be used together with `require_sibling`.
    /// Example: `*.tsx` for React components.
    #[serde(default)]
    pub file_pattern: Option<String>,

    /// Template for the required sibling file.
    /// Use `{stem}` as placeholder for the source file's stem (filename without extension).
    /// Must be used together with `file_pattern`.
    /// Example: `{stem}.test.tsx` requires `Button.test.tsx` for `Button.tsx`.
    #[serde(default)]
    pub require_sibling: Option<String>,
}

/// Structure override for specific directories [[structure.override]].
/// Only for DIRECTORIES. Use to grandfather legacy directories.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StructureOverride {
    /// Exact path to the directory.
    pub path: String,

    /// Maximum files allowed (-1 = unlimited).
    #[serde(default)]
    pub max_files: Option<i64>,

    /// Maximum subdirs allowed (-1 = unlimited).
    #[serde(default)]
    pub max_dirs: Option<i64>,

    /// Maximum depth allowed (-1 = unlimited).
    #[serde(default)]
    pub max_depth: Option<i64>,

    /// Reason for the override (required for audit trail).
    pub reason: String,
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
