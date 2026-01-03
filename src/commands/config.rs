use std::fs;
use std::path::Path;

use crate::cli::{Cli, ConfigAction, ConfigOutputFormat};
use crate::config::{Config, ConfigLoader, FetchPolicy, FileConfigLoader};
use crate::stats::parse_duration;
use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS, Result, SlocGuardError};

use super::context::print_preset_info;

/// Valid section names for `stats.report.exclude`.
const VALID_REPORT_SECTIONS: &[&str] = &["summary", "files", "breakdown", "trend"];

/// Valid values for `stats.report.breakdown_by`.
const VALID_BREAKDOWN_BY: &[&str] = &["lang", "language", "dir", "directory"];

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
/// # Errors
/// Returns an error if the file doesn't exist, contains invalid TOML, or has semantic errors.
pub(crate) fn run_config_validate_impl(config_path: &Path) -> Result<()> {
    if !config_path.exists() {
        return Err(SlocGuardError::Config(format!(
            "Configuration file not found: {}",
            config_path.display()
        )));
    }

    let content = fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(&content)?;

    validate_config_semantics(&config)?;

    Ok(())
}

/// Validates semantic correctness of a configuration.
///
/// # Errors
/// Returns an error if `warn_threshold` is out of range, glob patterns are invalid,
/// `warn_at >= max_lines`, or rules are misconfigured.
pub(crate) fn validate_config_semantics(config: &Config) -> Result<()> {
    // Validate content.warn_threshold
    if !(0.0..=1.0).contains(&config.content.warn_threshold) {
        return Err(SlocGuardError::Config(format!(
            "content.warn_threshold must be between 0.0 and 1.0, got {}",
            config.content.warn_threshold
        )));
    }

    // Validate content.warn_at < content.max_lines
    if let Some(warn_at) = config.content.warn_at
        && warn_at >= config.content.max_lines
    {
        return Err(SlocGuardError::Config(format!(
            "content.warn_at ({}) must be less than content.max_lines ({})",
            warn_at, config.content.max_lines
        )));
    }

    // Validate content.rules[i].warn_at < content.rules[i].max_lines
    for (i, rule) in config.content.rules.iter().enumerate() {
        if let Some(warn_at) = rule.warn_at
            && warn_at >= rule.max_lines
        {
            return Err(SlocGuardError::Config(format!(
                "content.rules[{}].warn_at ({}) must be less than content.rules[{}].max_lines ({})",
                i, warn_at, i, rule.max_lines
            )));
        }
    }

    // Validate scanner.exclude patterns
    for pattern in &config.scanner.exclude {
        globset::Glob::new(pattern).map_err(|e| SlocGuardError::InvalidPattern {
            pattern: pattern.clone(),
            source: e,
        })?;
    }

    // Validate content.exclude patterns
    for pattern in &config.content.exclude {
        globset::Glob::new(pattern).map_err(|e| SlocGuardError::InvalidPattern {
            pattern: pattern.clone(),
            source: e,
        })?;
    }

    // Validate stats.report.exclude values
    for section in &config.stats.report.exclude {
        let normalized = section.to_lowercase();
        if !VALID_REPORT_SECTIONS.contains(&normalized.as_str()) {
            return Err(SlocGuardError::Config(format!(
                "stats.report.exclude contains invalid section '{section}'. Valid values: {}",
                VALID_REPORT_SECTIONS.join(", ")
            )));
        }
    }

    // Validate stats.report.breakdown_by value
    if let Some(breakdown_by) = &config.stats.report.breakdown_by {
        let normalized = breakdown_by.to_lowercase();
        if !VALID_BREAKDOWN_BY.contains(&normalized.as_str()) {
            return Err(SlocGuardError::Config(format!(
                "stats.report.breakdown_by has invalid value '{breakdown_by}'. Valid values: lang, dir"
            )));
        }
    }

    // Validate stats.report.trend_since format
    if let Some(trend_since) = &config.stats.report.trend_since {
        parse_duration(trend_since).map_err(|_| {
            SlocGuardError::Config(format!(
                "stats.report.trend_since has invalid duration format '{trend_since}'. Expected format: <number><unit> (e.g., 7d, 1w, 12h)"
            ))
        })?;
    }

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
    // Determine project root from config path or current directory
    let project_root = super::context::resolve_project_root(config_path)?;

    let loader =
        FileConfigLoader::with_options(FetchPolicy::from_offline(cli.offline), project_root);
    let load_result =
        config_path.map_or_else(|| loader.load(), |path| loader.load_from_path(path))?;

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
#[path = "config_tests.rs"]
mod tests;
