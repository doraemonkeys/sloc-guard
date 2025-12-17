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

#[derive(Parser, Debug)]
#[command(name = "sloc-guard")]
#[command(author, version, about = "Source Lines of Code guard - enforce code size limits")]
#[command(long_about = "A tool to enforce source lines of code (SLOC) limits per file.\n\n\
    Exit codes:\n  \
    0 - All checks passed\n  \
    1 - Threshold violations found\n  \
    2 - Configuration or runtime error")]
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
}

#[derive(Parser, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct CheckArgs {
    /// Paths to check (files or directories)
    #[arg(default_value = ".")]
    pub paths: Vec<PathBuf>,

    /// Path to configuration file
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Maximum lines per file (overrides config)
    #[arg(long)]
    pub max_lines: Option<usize>,

    /// File extensions to check (comma-separated, e.g., rs,go,py)
    #[arg(long, value_delimiter = ',')]
    pub ext: Option<Vec<String>>,

    /// Exclude patterns (glob syntax, can be specified multiple times)
    #[arg(long, short = 'x')]
    pub exclude: Vec<String>,

    /// Include only these directories (overrides config `include_paths`)
    #[arg(long, short = 'I')]
    pub include: Vec<String>,

    /// Count comment lines as code
    #[arg(long)]
    pub no_skip_comments: bool,

    /// Count blank lines as code
    #[arg(long)]
    pub no_skip_blank: bool,

    /// Warning threshold (0.0-1.0, warn when usage exceeds this ratio)
    #[arg(long)]
    pub warn_threshold: Option<f64>,

    /// Output format [possible values: text, json, sarif, markdown]
    #[arg(short, long, default_value = "text")]
    pub format: OutputFormat,

    /// Write output to file instead of stdout
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Only warn, don't fail on threshold violations
    #[arg(long)]
    pub warn_only: bool,

    /// Compare against a git reference (branch or commit)
    #[arg(long)]
    pub diff: Option<String>,

    /// Treat warnings as failures (exit code 1)
    #[arg(long)]
    pub strict: bool,

    /// Path to baseline file for grandfathering violations
    #[arg(long)]
    pub baseline: Option<PathBuf>,

    /// Disable file hash caching
    #[arg(long)]
    pub no_cache: bool,
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

    /// Include only these directories
    #[arg(long, short = 'I')]
    pub include: Vec<String>,

    /// Output format [possible values: text, json, sarif, markdown]
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

    /// Include only these directories (overrides config `include_paths`)
    #[arg(long, short = 'I')]
    pub include: Vec<String>,
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
