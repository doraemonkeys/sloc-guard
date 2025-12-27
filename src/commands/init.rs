use std::fs;

use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS, Result, SlocGuardError};

use super::detect;

#[must_use]
pub fn run_init(args: &crate::cli::InitArgs) -> i32 {
    match run_init_impl(args) {
        Ok(()) => EXIT_SUCCESS,
        Err(e) => {
            crate::output::print_error_full(
                e.error_type(),
                &e.message(),
                e.detail().as_deref(),
                None,
            );
            EXIT_CONFIG_ERROR
        }
    }
}

/// Initializes a new configuration file.
///
/// # Errors
/// Returns an error if the file already exists (without --force) or cannot be written.
pub(crate) fn run_init_impl(args: &crate::cli::InitArgs) -> Result<()> {
    let cwd = std::env::current_dir()?;
    run_init_with_cwd(args, &cwd)
}

pub(crate) fn run_init_with_cwd(args: &crate::cli::InitArgs, cwd: &std::path::Path) -> Result<()> {
    let output_path = &args.output;

    if output_path.exists() && !args.force {
        return Err(SlocGuardError::Config(format!(
            "Configuration file already exists: {}. Use --force to overwrite.",
            output_path.display()
        )));
    }

    let template = if args.detect {
        detect::generate_detected_config_from_dir(cwd)?
    } else {
        generate_config_template()
    };

    fs::write(output_path, template)?;

    println!("Created configuration file: {}", output_path.display());
    Ok(())
}

#[must_use]
pub(crate) fn generate_config_template() -> String {
    r##"# sloc-guard configuration file
# See: https://github.com/doraemonkeys/sloc-guard for documentation
version = "2"

# Optional: inherit from presets or remote configs
# extends = "preset:rust-strict"
# extends = "https://example.com/shared-config.toml"

# =============================================================================
# Scanner: How to discover files
# =============================================================================
[scanner]
gitignore = true                          # Respect .gitignore (default: true)
exclude = [".git/**", "**/target/**", "**/node_modules/**", "**/vendor/**", "**/dist/**"]

# =============================================================================
# Content: SLOC (Source Lines of Code) limits
# =============================================================================
[content]
# File types to analyze (major programming languages)
extensions = [
    "rs", "go", "c", "cpp",           # Systems
    "java", "kt", "scala",            # JVM
    "cs",                             # .NET
    "js", "ts", "tsx", "jsx", "vue",  # Web/Frontend
    "swift", "dart",                  # Mobile
    "py", "rb", "php", "lua", "sh",   # Scripting
]
max_lines = 600                           # Default max lines per file
warn_threshold = 0.9                      # Warn at 90% of limit (450 lines)
skip_comments = true                      # Don't count comment lines
skip_blank = true                         # Don't count blank lines
# exclude = ["**/*_test.go"]              # Exclude from SLOC check (still visible to structure)

# Content Rules: Override limits for specific paths (last match wins)
# [[content.rules]]
# pattern = "src/generated/**"
# max_lines = 2000
# reason = "Auto-generated code"

# [[content.rules]]
# pattern = "**/*_test.rs"
# max_lines = 800
# reason = "Test files need more space for fixtures"

# Temporary exemption with expiration
# [[content.rules]]
# pattern = "src/legacy/parser.rs"
# max_lines = 1500
# reason = "Refactoring in progress - JIRA-1234"
# expires = "2025-06-01"

# =============================================================================
# Structure: Directory organization limits
# =============================================================================
[structure]
max_files = 30                            # Max files per directory
max_dirs = 10                             # Max subdirectories per directory
max_depth = 8                             # Max nesting depth
warn_threshold = 0.8                      # Warn at 80% of limits
# count_exclude = ["*.md", ".gitkeep"]    # Don't count these toward limits

# Global deny lists
# deny_extensions = [".exe", ".dll", ".so"]
# deny_files = ["*.bak", "*.tmp", ".DS_Store"]

# Structure Rules: Override for specific directories
# [[structure.rules]]
# scope = "src/components/**"
# max_files = 50
# allow_extensions = [".tsx", ".ts", ".css"]
# reason = "React components directory"

# [[structure.rules]]
# scope = "tests/**"
# max_files = -1                          # -1 = unlimited
# reason = "No file limit for test directories"

# =============================================================================
# Baseline: Grandfather existing violations
# =============================================================================
[baseline]
# ratchet = "warn"                        # warn|auto|strict - enforce violations can only decrease

# =============================================================================
# Trend: History tracking settings
# =============================================================================
[trend]
max_entries = 100                         # Keep last 100 snapshots
max_age_days = 90                         # Delete entries older than 90 days
min_interval_secs = 3600                  # At most one entry per hour
# min_code_delta = 10                     # Ignore changes < 10 lines

# =============================================================================
# Custom Languages: Define comment syntax for unsupported languages
# =============================================================================
# [languages.hcl]
# extensions = ["tf", "hcl"]
# single_line_comments = ["#", "//"]
# multi_line_comments = [["/*", "*/"]]
"##
    .to_string()
}

#[cfg(test)]
#[path = "init_tests.rs"]
mod tests;
