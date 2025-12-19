use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use crate::output::OutputFormat;

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

#[derive(Parser, Debug)]
#[command(name = "sloc-guard")]
#[command(
    author,
    version,
    about = "Source Lines of Code guard - enforce code size limits"
)]
#[command(
    long_about = "A tool to enforce source lines of code (SLOC) limits per file.\n\n\
    Exit codes:\n  \
    0 - All checks passed\n  \
    1 - Threshold violations found\n  \
    2 - Configuration or runtime error"
)]
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

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Check files against line count thresholds
    Check(CheckArgs),

    /// Display statistics without checking thresholds
    Stats(StatsArgs),

    /// Generate a default configuration file
    Init(InitArgs),

    /// Configuration file utilities
    Config(ConfigArgs),

    /// Baseline management for grandfathering violations
    Baseline(BaselineArgs),

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

    /// Compare against a git reference (branch/commit). Defaults to HEAD when no
    /// value provided. Only content (SLOC) checks are filtered; structure checks
    /// still count full directory state.
    #[arg(long, num_args = 0..=1, default_missing_value = "HEAD")]
    pub diff: Option<String>,

    /// Treat warnings as failures (exit code 1)
    #[arg(long)]
    pub strict: bool,

    /// Path to baseline file for grandfathering violations
    #[arg(long)]
    pub baseline: Option<PathBuf>,

    /// Update baseline after check [possible values: all, content, structure, new]
    #[arg(long, value_name = "MODE", num_args = 0..=1, default_missing_value = "all")]
    pub update_baseline: Option<BaselineUpdateMode>,

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

    /// Write statistics report to JSON file (avoids separate stats run in CI)
    #[arg(long, value_name = "PATH")]
    pub report_json: Option<PathBuf>,
}

#[derive(Parser, Debug)]
pub struct StatsArgs {
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

    /// Output format [possible values: text, json, sarif, markdown, html]
    #[arg(short, long, default_value = "text")]
    pub format: OutputFormat,

    /// Write output to file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Disable file hash caching
    #[arg(long)]
    pub no_cache: bool,

    /// Group results by category
    #[arg(long, value_enum, default_value = "none")]
    pub group_by: GroupBy,

    /// Show top N largest files by code lines
    #[arg(long)]
    pub top: Option<usize>,

    /// Disable .gitignore filtering (scan all files)
    #[arg(long)]
    pub no_gitignore: bool,

    /// Track and display trend (delta from previous run)
    #[arg(long)]
    pub trend: bool,

    /// Path to history file for trend tracking (default: .sloc-guard-history.json)
    #[arg(long, value_name = "PATH")]
    pub history_file: Option<PathBuf>,
}

#[derive(Parser, Debug)]
pub struct InitArgs {
    /// Output path for configuration file
    #[arg(short, long, default_value = ".sloc-guard.toml")]
    pub output: PathBuf,

    /// Overwrite existing configuration
    #[arg(long)]
    pub force: bool,
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

        /// Output format [possible values: text, json]
        #[arg(short, long, default_value = "text")]
        format: String,
    },
}

#[derive(Parser, Debug)]
pub struct BaselineArgs {
    #[command(subcommand)]
    pub action: BaselineAction,
}

#[derive(Subcommand, Debug)]
pub enum BaselineAction {
    /// Generate baseline from current violations
    Update(BaselineUpdateArgs),
}

#[derive(Parser, Debug)]
pub struct BaselineUpdateArgs {
    /// Paths to scan (files or directories)
    #[arg(default_value = ".")]
    pub paths: Vec<PathBuf>,

    /// Path to configuration file
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Output path for baseline file
    #[arg(short, long, default_value = ".sloc-guard-baseline.json")]
    pub output: PathBuf,

    /// File extensions to check (comma-separated, e.g., rs,go,py)
    #[arg(long, value_delimiter = ',')]
    pub ext: Option<Vec<String>>,

    /// Exclude patterns (glob syntax, can be specified multiple times)
    #[arg(long, short = 'x')]
    pub exclude: Vec<String>,

    /// Allowlist filter: only scan these directories
    #[arg(long, short = 'I')]
    pub include: Vec<String>,

    /// Disable .gitignore filtering (scan all files)
    #[arg(long)]
    pub no_gitignore: bool,
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
