use std::fs;
use std::path::Path;

use crate::cli::{Cli, ConfigAction, ConfigOutputFormat};
use crate::config::{
    Config, ConfigLoader, FetchPolicy, FileConfigLoader, validate_config_semantics,
};
use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS, Result, SlocGuardError};

use super::context::print_preset_info;

#[must_use]
pub fn run_config(args: &crate::cli::ConfigArgs, cli: &Cli) -> i32 {
    match &args.action {
        ConfigAction::Validate { config } => run_config_validate(config),
        ConfigAction::Show { config, format } => run_config_show(config.as_deref(), *format, cli),
    }
}

fn run_config_validate(config_path: &Path) -> i32 {
    match run_config_validate_impl(config_path) {
        Ok(()) => {
            println!("Configuration is valid: {}", config_path.display());
            EXIT_SUCCESS
        }
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

/// Validates a configuration file.
///
/// Two-phase validation:
/// 1. Direct TOML parse - catches syntax errors with detailed error messages
/// 2. Full load with extends - validates inheritance chain and semantics
///
/// # Errors
/// Returns an error if the file doesn't exist, contains invalid TOML,
/// extends resolution fails, or has semantic errors.
pub(crate) fn run_config_validate_impl(config_path: &Path) -> Result<()> {
    if !config_path.exists() {
        return Err(SlocGuardError::Config(format!(
            "Configuration file not found: {}",
            config_path.display()
        )));
    }

    // Phase 1: Direct parse for better syntax error messages
    let content = fs::read_to_string(config_path)?;
    let _: Config = toml::from_str(&content)?;

    // Phase 2: Full load with extends chain and semantic validation
    super::context::load_config(Some(config_path), false, false, FetchPolicy::Normal)?;

    Ok(())
}

fn run_config_show(config_path: Option<&Path>, format: ConfigOutputFormat, cli: &Cli) -> i32 {
    match run_config_show_impl(config_path, format, cli) {
        Ok(output) => {
            print!("{output}");
            EXIT_SUCCESS
        }
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

/// Shows the effective configuration.
///
/// # Errors
/// Returns an error if the configuration file cannot be loaded or serialization fails.
pub(crate) fn run_config_show_impl(
    config_path: Option<&Path>,
    format: ConfigOutputFormat,
    cli: &Cli,
) -> Result<String> {
    let config = load_config(config_path, cli)?;

    match format {
        ConfigOutputFormat::Json => {
            let json = serde_json::to_string_pretty(&config)?;
            Ok(format!("{json}\n"))
        }
        ConfigOutputFormat::Text => Ok(format_config_text(&config)),
    }
}

fn load_config(config_path: Option<&Path>, cli: &Cli) -> Result<Config> {
    // Determine project root for consistent state file resolution
    let project_root = Some(super::context::resolve_project_root());

    let loader =
        FileConfigLoader::with_options(FetchPolicy::from_cli(cli.extends_policy), project_root);
    let load_result =
        config_path.map_or_else(|| loader.load(), |path| loader.load_from_path(path))?;

    // Validate semantic correctness after loading
    validate_config_semantics(&load_result.config)?;

    // Print preset info if a preset was used
    if let Some(ref preset_name) = load_result.preset_used {
        print_preset_info(preset_name);
    }

    Ok(load_result.config)
}

#[must_use]
pub(crate) fn format_config_text(config: &Config) -> String {
    use std::fmt::Write;

    let mut output = String::new();

    output.push_str("=== Effective Configuration ===\n\n");

    // Scanner section
    output.push_str("[scanner]\n");
    let _ = writeln!(output, "  gitignore = {}", config.scanner.gitignore);
    if !config.scanner.exclude.is_empty() {
        let _ = writeln!(output, "  exclude = {:?}", config.scanner.exclude);
    }

    // Content section
    output.push_str("\n[content]\n");
    let _ = writeln!(output, "  max_lines = {}", config.content.max_lines);
    let _ = writeln!(output, "  extensions = {:?}", config.content.extensions);
    let _ = writeln!(output, "  skip_comments = {}", config.content.skip_comments);
    let _ = writeln!(output, "  skip_blank = {}", config.content.skip_blank);
    let _ = writeln!(
        output,
        "  warn_threshold = {}",
        config.content.warn_threshold
    );
    if let Some(warn_at) = config.content.warn_at {
        let _ = writeln!(output, "  warn_at = {warn_at}");
    }
    if !config.content.exclude.is_empty() {
        let _ = writeln!(output, "  exclude = {:?}", config.content.exclude);
    }

    // Content rules
    if !config.content.rules.is_empty() {
        output.push('\n');
        for (i, rule) in config.content.rules.iter().enumerate() {
            let _ = writeln!(output, "[[content.rules]]  # rule {i}");
            let _ = writeln!(output, "  pattern = \"{}\"", rule.pattern);
            let _ = writeln!(output, "  max_lines = {}", rule.max_lines);
            if let Some(warn_threshold) = rule.warn_threshold {
                let _ = writeln!(output, "  warn_threshold = {warn_threshold}");
            }
            if let Some(warn_at) = rule.warn_at {
                let _ = writeln!(output, "  warn_at = {warn_at}");
            }
            if let Some(skip_comments) = rule.skip_comments {
                let _ = writeln!(output, "  skip_comments = {skip_comments}");
            }
            if let Some(skip_blank) = rule.skip_blank {
                let _ = writeln!(output, "  skip_blank = {skip_blank}");
            }
            if let Some(reason) = &rule.reason {
                let _ = writeln!(output, "  reason = \"{reason}\"");
            }
            if let Some(expires) = &rule.expires {
                let _ = writeln!(output, "  expires = \"{expires}\"");
            }
        }
    }

    // Structure section (if configured)
    if config.structure.max_files.is_some()
        || config.structure.max_dirs.is_some()
        || config.structure.max_depth.is_some()
        || !config.structure.rules.is_empty()
    {
        output.push_str("\n[structure]\n");
        if let Some(max_files) = config.structure.max_files {
            let _ = writeln!(output, "  max_files = {max_files}");
        }
        if let Some(max_dirs) = config.structure.max_dirs {
            let _ = writeln!(output, "  max_dirs = {max_dirs}");
        }
        if let Some(max_depth) = config.structure.max_depth {
            let _ = writeln!(output, "  max_depth = {max_depth}");
        }
        if let Some(warn_threshold) = config.structure.warn_threshold {
            let _ = writeln!(output, "  warn_threshold = {warn_threshold}");
        }
    }

    // Stats section (if configured)
    let report = &config.stats.report;
    if !report.exclude.is_empty()
        || report.top_count.is_some()
        || report.breakdown_by.is_some()
        || report.trend_since.is_some()
    {
        output.push_str("\n[stats.report]\n");
        if !report.exclude.is_empty() {
            let _ = writeln!(output, "  exclude = {:?}", report.exclude);
        }
        if let Some(top_count) = report.top_count {
            let _ = writeln!(output, "  top_count = {top_count}");
        }
        if let Some(breakdown_by) = &report.breakdown_by {
            let _ = writeln!(output, "  breakdown_by = \"{breakdown_by}\"");
        }
        if let Some(trend_since) = &report.trend_since {
            let _ = writeln!(output, "  trend_since = \"{trend_since}\"");
        }
    }

    // Check section (if non-default)
    if config.check.warnings_as_errors || config.check.fail_fast {
        output.push_str("\n[check]\n");
        if config.check.warnings_as_errors {
            let _ = writeln!(output, "  warnings_as_errors = true");
        }
        if config.check.fail_fast {
            let _ = writeln!(output, "  fail_fast = true");
        }
    }

    output
}

#[cfg(test)]
#[path = "config_tests/mod.rs"]
mod tests;
