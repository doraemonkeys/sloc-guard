use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::output::OutputFormat;

#[derive(Parser, Debug)]
#[command(name = "sloc-guard")]
#[command(author, version, about = "Source Lines of Code guard - enforce code size limits", long_about = None)]
pub struct Cli {
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
}

#[derive(Parser, Debug)]
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

    /// File extensions to check (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub ext: Option<Vec<String>>,

    /// Output format
    #[arg(short, long, default_value = "text")]
    pub format: OutputFormat,

    /// Only warn, don't fail on threshold violations
    #[arg(long)]
    pub warn_only: bool,

    /// Compare against a git reference (branch or commit)
    #[arg(long)]
    pub diff: Option<String>,
}

#[derive(Parser, Debug)]
pub struct StatsArgs {
    /// Paths to analyze
    #[arg(default_value = ".")]
    pub paths: Vec<PathBuf>,

    /// File extensions to include (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub ext: Option<Vec<String>>,

    /// Output format
    #[arg(short, long, default_value = "text")]
    pub format: OutputFormat,
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


#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
