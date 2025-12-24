use std::fs;
use std::path::Path;

use crate::cli::{Cli, ConfigAction, ConfigOutputFormat};
use crate::config::{Config, ConfigLoader, FileConfigLoader};
use crate::{EXIT_CONFIG_ERROR, EXIT_SUCCESS, Result, SlocGuardError};

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
/// override paths are empty, or rules are misconfigured.
pub(crate) fn validate_config_semantics(config: &Config) -> Result<()> {
    if !(0.0..=1.0).contains(&config.default.warn_threshold) {
        return Err(SlocGuardError::Config(format!(
            "warn_threshold must be between 0.0 and 1.0, got {}",
            config.default.warn_threshold
        )));
    }

    for pattern in &config.exclude.patterns {
        globset::Glob::new(pattern).map_err(|e| SlocGuardError::InvalidPattern {
            pattern: pattern.clone(),
            source: e,
        })?;
    }

    for (i, override_cfg) in config.overrides.iter().enumerate() {
        if override_cfg.path.is_empty() {
            return Err(SlocGuardError::Config(format!(
                "override[{i}].path cannot be empty"
            )));
        }
    }

    for (name, rule) in &config.rules {
        if rule.extensions.is_empty() && rule.max_lines.is_none() {
            return Err(SlocGuardError::Config(format!(
                "rules.{name}: must specify at least extensions or max_lines"
            )));
        }
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
    let project_root = config_path
        .and_then(|p| p.parent())
        .map(std::path::Path::to_path_buf)
        .or_else(|| std::env::current_dir().ok());

    let loader = FileConfigLoader::with_options(cli.offline, project_root);
    config_path.map_or_else(|| loader.load(), |path| loader.load_from_path(path))
}

#[must_use]
pub(crate) fn format_config_text(config: &Config) -> String {
    use std::fmt::Write;

    let mut output = String::new();

    output.push_str("=== Effective Configuration ===\n\n");

    output.push_str("[default]\n");
    let _ = writeln!(output, "  max_lines = {}", config.default.max_lines);
    let _ = writeln!(output, "  extensions = {:?}", config.default.extensions);
    if !config.default.include_paths.is_empty() {
        let _ = writeln!(
            output,
            "  include_paths = {:?}",
            config.default.include_paths
        );
    }
    let _ = writeln!(output, "  skip_comments = {}", config.default.skip_comments);
    let _ = writeln!(output, "  skip_blank = {}", config.default.skip_blank);
    let _ = writeln!(
        output,
        "  warn_threshold = {}",
        config.default.warn_threshold
    );
    let _ = writeln!(output, "  strict = {}", config.default.strict);

    if !config.rules.is_empty() {
        output.push('\n');
        let mut rule_names: Vec<_> = config.rules.keys().collect();
        rule_names.sort();
        for name in rule_names {
            let rule = &config.rules[name];
            let _ = writeln!(output, "[rules.{name}]");
            if !rule.extensions.is_empty() {
                let _ = writeln!(output, "  extensions = {:?}", rule.extensions);
            }
            if let Some(max_lines) = rule.max_lines {
                let _ = writeln!(output, "  max_lines = {max_lines}");
            }
            if let Some(skip_comments) = rule.skip_comments {
                let _ = writeln!(output, "  skip_comments = {skip_comments}");
            }
            if let Some(skip_blank) = rule.skip_blank {
                let _ = writeln!(output, "  skip_blank = {skip_blank}");
            }
        }
    }

    if !config.exclude.patterns.is_empty() {
        output.push_str("\n[exclude]\n");
        output.push_str("  patterns = [\n");
        for pattern in &config.exclude.patterns {
            let _ = writeln!(output, "    \"{pattern}\",");
        }
        output.push_str("  ]\n");
    }

    if !config.overrides.is_empty() {
        output.push('\n');
        output.push_str("# Legacy V1 overrides (migrate to [[content.rules]]):\n");
        for override_cfg in &config.overrides {
            output.push_str("[[override]]  # DEPRECATED\n");
            let _ = writeln!(output, "  path = \"{}\"", override_cfg.path);
            let _ = writeln!(output, "  max_lines = {}", override_cfg.max_lines);
            if let Some(reason) = &override_cfg.reason {
                let _ = writeln!(output, "  reason = \"{reason}\"");
            }
        }
    }

    output
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
