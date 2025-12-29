use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use crate::output::OutputFormat;

// Re-export FileSortOrder from output module for unified usage
pub use crate::output::FileSortOrder;

/// Grouping mode for stats command
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum GroupBy {
    /// No grouping (default)
    #[default]
    None,
    /// Group by language
    Lang,
    /// Group by directory
    Dir,
}

/// Color output control
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum ColorChoice {
    /// Auto-detect terminal capability
    #[default]
    Auto,
    /// Always use colors
    Always,
    /// Never use colors
    Never,
}

/// Output format for explain command
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum ExplainFormat {
    /// Human-readable text output
    #[default]
    Text,
    /// JSON output
    Json,
}

/// Output format for config show command
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum ConfigOutputFormat {
    /// Human-readable text output
    #[default]
    Text,
    /// JSON output
    Json,
}

/// Baseline update mode for `check --update-baseline`
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum BaselineUpdateMode {
    /// Replace baseline with all current violations (content + structure)
    #[default]
    All,
    /// Update only SLOC (content) violations
    Content,
    /// Update only directory structure violations
    Structure,
    /// Add-only: preserve existing entries, only add new violations
    New,
}

/// Ratchet mode for `check --ratchet`
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum RatchetMode {
    /// Emit warning if baseline can be tightened (default)
    #[default]
    Warn,
    /// Auto-update baseline when violations decrease
    Auto,
    /// Fail CI if baseline is outdated (violations decreased but not updated)
    Strict,
}

#[derive(Parser, Debug)]
#[command(name = "sloc-guard")]
#[command(
    author,
    version,
    about = "Enforce code size (SLOC) and directory structure limits",
    long_about = "Enforce source lines of code (SLOC) limits per file and directory structure \
    limits (file/folder counts). Counts code lines excluding comments and blanks by default."
)]
#[allow(clippy::struct_excessive_bools)] // CLI flags are inherently boolean
pub struct Cli {
    /// Increase output verbosity (-v, -vv for more)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Suppress non-essential output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Control color output
    #[arg(long, value_enum, default_value = "auto", global = true)]
    pub color: ColorChoice,

    /// Skip loading configuration file
    #[arg(long, global = true)]
    pub no_config: bool,

    /// Skip resolving extends in configuration (ignore remote/local inheritance)
    #[arg(long, global = true)]
    pub no_extends: bool,

    /// Use cached remote configs only, error if cache miss
    #[arg(long, global = true)]
    pub offline: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Check files against line count thresholds
    Check(CheckArgs),

    /// Display statistics without checking thresholds
    Stats(StatsArgs),

    /// Record a statistics snapshot to trend history
    Snapshot(SnapshotArgs),

    /// Generate a default configuration file
    Init(InitArgs),

    /// Configuration file utilities
    Config(ConfigArgs),

    /// Explain which rules apply to a path
    Explain(ExplainArgs),
}

#[derive(Parser, Debug)]
#[allow(clippy::struct_excessive_bools)] // CLI arguments often require many boolean flags
pub struct CheckArgs {
    /// Scan roots: directories or files to check. Defaults to current directory.
    /// These are the starting points for file discovery. Use --include to filter
    /// which subdirectories are actually scanned. Required when using --max-files
    /// or --max-dirs.
    #[arg()]
    pub paths: Vec<PathBuf>,

    /// Path to configuration file
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Maximum lines per file (overrides [content] default; rules take precedence)
    #[arg(long)]
    pub max_lines: Option<usize>,

    /// File extensions to check (comma-separated, e.g., rs,go,py)
    #[arg(long, value_delimiter = ',')]
    pub ext: Option<Vec<String>>,

    /// Exclude patterns (glob syntax, can be specified multiple times)
    #[arg(long, short = 'x')]
    pub exclude: Vec<String>,

    /// Allowlist filter: only scan these directories (overrides <PATH> arguments
    /// and config `include_paths`). Use to restrict scanning to specific subdirs.
    #[arg(long, short = 'I')]
    pub include: Vec<String>,

    /// Count comment lines as code (disables `skip_comments`)
    #[arg(long)]
    pub count_comments: bool,

    /// Count blank lines as code (disables `skip_blank`)
    #[arg(long)]
    pub count_blank: bool,

    /// Warning threshold (0.0-1.0, warn when usage exceeds this ratio)
    #[arg(long)]
    pub warn_threshold: Option<f64>,

    /// Output format [possible values: text, json, sarif, markdown, html]
    #[arg(short, long, default_value = "text")]
    pub format: OutputFormat,

    /// Write output to file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Only warn, don't fail on threshold violations
    #[arg(long)]
    pub warn_only: bool,

    /// Compare files changed between git references.
    /// Supports single ref (compared to HEAD) or explicit range (base..target).
    /// Examples: --diff main, --diff origin/main..HEAD, --diff v1.0..v2.0
    /// Defaults to HEAD when no value provided. Note: compares committed trees only,
    /// not the working directory. Use --staged for uncommitted staged files.
    /// Structure checks use full directory state.
    #[arg(long, num_args = 0..=1, default_missing_value = "HEAD")]
    pub diff: Option<String>,

    /// Check only staged files (git staging area). Mutually exclusive with --diff.
    #[arg(long, conflicts_with = "diff")]
    pub staged: bool,

    /// Treat warnings as failures (exit code 1)
    #[arg(long)]
    pub strict: bool,

    /// Path to baseline file for grandfathering violations
    #[arg(long)]
    pub baseline: Option<PathBuf>,

    /// Update baseline after check [possible values: all, content, structure, new]
    #[arg(long, value_name = "MODE", num_args = 0..=1, default_missing_value = "all")]
    pub update_baseline: Option<BaselineUpdateMode>,

    /// Enforce baseline ratchet: violation count can only decrease.
    /// - warn (default): emit warning when baseline can be tightened
    /// - auto: auto-update baseline when violations decrease
    /// - strict: fail CI if baseline is outdated
    #[arg(long, value_name = "MODE", num_args = 0..=1, default_missing_value = "warn")]
    pub ratchet: Option<RatchetMode>,

    /// Disable file hash caching
    #[arg(long)]
    pub no_cache: bool,

    /// Disable .gitignore filtering (scan all files)
    #[arg(long)]
    pub no_gitignore: bool,

    /// Show split suggestions for files exceeding thresholds
    #[arg(long)]
    pub suggest: bool,

    /// Maximum files per directory (overrides config [structure] defaults; rules take precedence)
    #[arg(long, value_name = "COUNT")]
    pub max_files: Option<i64>,

    /// Maximum subdirectories per directory (overrides config [structure] defaults; rules take precedence)
    #[arg(long, value_name = "COUNT")]
    pub max_dirs: Option<i64>,

    /// Maximum directory depth (overrides config [structure] defaults; rules take precedence)
    #[arg(long, value_name = "DEPTH")]
    pub max_depth: Option<i64>,

    /// Write statistics report to JSON file (avoids separate stats run in CI)
    #[arg(long, value_name = "PATH")]
    pub report_json: Option<PathBuf>,

    /// Explicit file list for pre-commit hooks. When provided, skips directory
    /// scanning and processes only the specified files. Structure checks are
    /// disabled in this mode. Files are separated by commas or spaces.
    #[arg(long, value_delimiter = ',', num_args = 1..)]
    pub files: Vec<PathBuf>,
}

/// Output format for stats subcommands (subset without SARIF)
#[derive(Debug, Clone, Copy, Default, ValueEnum, PartialEq, Eq)]
pub enum StatsOutputFormat {
    /// Human-readable text output
    #[default]
    Text,
    /// JSON output
    Json,
    /// Markdown output
    #[value(name = "md")]
    Markdown,
}

/// Output format for stats report command (includes HTML)
#[derive(Debug, Clone, Copy, Default, ValueEnum, PartialEq, Eq)]
pub enum ReportOutputFormat {
    /// Human-readable text output
    #[default]
    Text,
    /// JSON output
    Json,
    /// Markdown output
    #[value(name = "md")]
    Markdown,
    /// HTML output
    Html,
}

/// Output format for stats history command
#[derive(Debug, Clone, Copy, Default, ValueEnum, PartialEq, Eq)]
pub enum HistoryOutputFormat {
    /// Human-readable text output
    #[default]
    Text,
    /// JSON output
    Json,
}

/// Grouping mode for breakdown subcommand
#[derive(Debug, Clone, Copy, Default, ValueEnum, PartialEq, Eq)]
pub enum BreakdownBy {
    /// Group by language (default)
    #[default]
    Lang,
    /// Group by directory
    Dir,
}

/// Common scanning arguments shared across stats subcommands
#[derive(clap::Args, Debug, Clone)]
pub struct CommonStatsArgs {
    /// Paths to analyze
    #[arg(default_value = ".")]
    pub paths: Vec<PathBuf>,

    /// Path to configuration file
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// File extensions to include (comma-separated, e.g., rs,go,py)
    #[arg(long, value_delimiter = ',')]
    pub ext: Option<Vec<String>>,

    /// Exclude patterns (glob syntax, can be specified multiple times)
    #[arg(long, short = 'x')]
    pub exclude: Vec<String>,

    /// Allowlist filter: only scan these directories
    #[arg(long, short = 'I')]
    pub include: Vec<String>,

    /// Disable file hash caching
    #[arg(long)]
    pub no_cache: bool,

    /// Disable .gitignore filtering (scan all files)
    #[arg(long)]
    pub no_gitignore: bool,
}

/// Arguments for the `snapshot` command
#[derive(Parser, Debug)]
pub struct SnapshotArgs {
    #[command(flatten)]
    pub common: CommonStatsArgs,

    /// Path to history file (defaults to .git/sloc-guard/history.json or .sloc-guard/history.json)
    #[arg(long)]
    pub history_file: Option<PathBuf>,

    /// Force snapshot even if `min_interval_secs` hasn't elapsed
    #[arg(long)]
    pub force: bool,

    /// Dry-run: show what would be recorded without saving
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Parser, Debug)]
pub struct StatsArgs {
    /// Stats subcommand (required)
    #[command(subcommand)]
    pub action: StatsAction,
}

#[derive(Subcommand, Debug)]
pub enum StatsAction {
    /// Project-level summary totals (files, code, comments, blanks, averages)
    Summary(SummaryArgs),

    /// File list with sorting and filtering options
    Files(FilesArgs),

    /// Grouped statistics by language or directory
    Breakdown(BreakdownArgs),

    /// Delta comparison with historical snapshots
    Trend(TrendArgs),

    /// List recent history entries
    History(HistoryArgs),

    /// Comprehensive report combining summary, files, breakdown, and trend
    Report(ReportArgs),
}

#[derive(Parser, Debug)]
pub struct SummaryArgs {
    #[command(flatten)]
    pub common: CommonStatsArgs,

    /// Output format
    #[arg(short, long, value_enum, default_value = "text")]
    pub format: StatsOutputFormat,
}

#[derive(Parser, Debug)]
pub struct FilesArgs {
    #[command(flatten)]
    pub common: CommonStatsArgs,

    /// Show top N largest files (default: all files)
    #[arg(long)]
    pub top: Option<usize>,

    /// Sort order for files
    #[arg(long, value_enum, default_value = "code")]
    pub sort: FileSortOrder,

    /// Output format
    #[arg(short, long, value_enum, default_value = "text")]
    pub format: StatsOutputFormat,
}

#[derive(Parser, Debug)]
pub struct BreakdownArgs {
    #[command(flatten)]
    pub common: CommonStatsArgs,

    /// Group by language or directory
    #[arg(long, value_enum, default_value = "lang")]
    pub by: BreakdownBy,

    /// Maximum depth for directory grouping (only with --by dir)
    #[arg(long)]
    pub depth: Option<usize>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "text")]
    pub format: StatsOutputFormat,
}

#[derive(Parser, Debug)]
pub struct TrendArgs {
    #[command(flatten)]
    pub common: CommonStatsArgs,

    /// Compare against a specific time ago (e.g., 7d, 30d, 1w, 12h).
    /// Finds the nearest entry before the specified time point.
    /// Supported units: s, m, h, d, w.
    #[arg(long, value_name = "DURATION")]
    pub since: Option<String>,

    /// Path to history file (default: auto-discovered in project state dir)
    #[arg(long, value_name = "PATH")]
    pub history_file: Option<PathBuf>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "text")]
    pub format: StatsOutputFormat,
}

#[derive(Parser, Debug)]
pub struct HistoryArgs {
    /// Maximum number of entries to display
    #[arg(short, long, default_value = "10")]
    pub limit: usize,

    /// Output format
    #[arg(short, long, value_enum, default_value = "text")]
    pub format: HistoryOutputFormat,

    /// Path to history file (default: auto-discovered in project state dir)
    #[arg(long, value_name = "PATH")]
    pub history_file: Option<PathBuf>,
}

#[derive(Parser, Debug)]
pub struct ReportArgs {
    #[command(flatten)]
    pub common: CommonStatsArgs,

    /// Output format
    #[arg(short, long, value_enum, default_value = "text")]
    pub format: ReportOutputFormat,

    /// Write output to file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Path to history file for trend data (default: auto-discovered in project state dir)
    #[arg(long, value_name = "PATH")]
    pub history_file: Option<PathBuf>,

    /// Sections to exclude from report (can be specified multiple times).
    /// Valid values: summary, files, breakdown, trend
    #[arg(long = "exclude-section", value_name = "SECTION")]
    pub exclude_sections: Vec<String>,

    /// Number of top files to include (overrides config)
    #[arg(long, value_name = "N")]
    pub top: Option<usize>,

    /// Grouping mode for breakdown section (overrides config)
    #[arg(long, value_enum)]
    pub breakdown_by: Option<BreakdownBy>,

    /// Comparison period for trend section (e.g., 7d, 1w, 30d) (overrides config)
    #[arg(long, value_name = "DURATION")]
    pub since: Option<String>,
}

#[derive(Parser, Debug)]
pub struct InitArgs {
    /// Output path for configuration file
    #[arg(short, long, default_value = ".sloc-guard.toml")]
    pub output: PathBuf,

    /// Overwrite existing configuration
    #[arg(long)]
    pub force: bool,

    /// Auto-detect project type and generate appropriate config
    #[arg(long)]
    pub detect: bool,
}

#[derive(Parser, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// Validate configuration file syntax
    Validate {
        /// Path to configuration file (default: .sloc-guard.toml)
        #[arg(short, long, default_value = ".sloc-guard.toml")]
        config: PathBuf,
    },

    /// Display the effective configuration (merged from all sources)
    Show {
        /// Path to configuration file
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// Output format
        #[arg(short, long, value_enum, default_value = "text")]
        format: ConfigOutputFormat,
    },
}

#[derive(Parser, Debug)]
pub struct ExplainArgs {
    /// Path to explain (file or directory)
    #[arg(value_name = "PATH")]
    pub path: PathBuf,

    /// Path to configuration file
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Output format
    #[arg(short, long, value_enum, default_value = "text")]
    pub format: ExplainFormat,
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
