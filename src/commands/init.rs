use std::fs;

use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS, Result, SlocGuardError};

#[must_use]
pub fn run_init(args: &crate::cli::InitArgs) -> i32 {
    match run_init_impl(args) {
        Ok(()) => EXIT_SUCCESS,
        Err(e) => {
            eprintln!("Error: {e}");
            EXIT_CONFIG_ERROR
        }
    }
}

/// Initializes a new configuration file.
///
/// # Errors
/// Returns an error if the file already exists (without --force) or cannot be written.
pub fn run_init_impl(args: &crate::cli::InitArgs) -> Result<()> {
    let output_path = &args.output;

    if output_path.exists() && !args.force {
        return Err(SlocGuardError::Config(format!(
            "Configuration file already exists: {}. Use --force to overwrite.",
            output_path.display()
        )));
    }

    let template = generate_config_template();

    fs::write(output_path, template)?;

    println!("Created configuration file: {}", output_path.display());
    Ok(())
}

#[must_use]
pub fn generate_config_template() -> String {
    r#"# sloc-guard configuration file
# See: https://github.com/doraemonkeys/sloc-guard for documentation

[default]
# Maximum lines of code per file (default: 500)
max_lines = 500

# File extensions to check
extensions = ["rs", "go", "py", "js", "ts", "c", "cpp"]

# Directories to include (empty = scan from current directory)
# include_paths = ["src", "lib"]

# Skip comment lines when counting (default: true)
skip_comments = true

# Skip blank lines when counting (default: true)
skip_blank = true

# Warning threshold as ratio of max_lines (default: 0.9)
# Files exceeding this ratio but under max_lines will show warnings
warn_threshold = 0.9

# Strict mode: treat warnings as failures (default: false)
# strict = true

# Extension-based rules (override defaults for specific languages)
# [rules.rust]
# extensions = ["rs"]
# max_lines = 300

# [rules.python]
# extensions = ["py"]
# max_lines = 400

# Path-based rules (higher priority than extension rules)
# [[path_rules]]
# pattern = "src/generated/**"
# max_lines = 1000
# warn_threshold = 1.0  # Disable warnings for generated code

# Exclude patterns (glob syntax)
[exclude]
patterns = [
    "**/target/**",
    "**/node_modules/**",
    "**/.git/**",
    "**/vendor/**",
]

# Per-file overrides (highest priority)
# [[override]]
# path = "src/legacy/parser.rs"
# max_lines = 800
# reason = "Legacy code, scheduled for refactor"
"#
    .to_string()
}

#[cfg(test)]
#[path = "init_tests.rs"]
mod tests;
