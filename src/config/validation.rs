//! Configuration semantic validation.
//!
//! Validates that configuration values are semantically correct after parsing.
//! This includes range checks, pattern validation, and cross-field constraints.

use crate::config::Config;
use crate::stats::parse_duration;
use crate::{Result, SlocGuardError};

/// Valid section names for `stats.report.exclude`.
const VALID_REPORT_SECTIONS: &[&str] = &["summary", "files", "breakdown", "trend"];

/// Valid values for `stats.report.breakdown_by`.
const VALID_BREAKDOWN_BY: &[&str] = &["lang", "language", "dir", "directory"];

/// Validates semantic correctness of a configuration.
///
/// # Errors
/// Returns an error if `warn_threshold` is out of range, glob patterns are invalid,
/// `warn_at >= max_lines`, or rules are misconfigured.
pub fn validate_config_semantics(config: &Config) -> Result<()> {
    validate_content_section(config)?;
    validate_glob_patterns(config)?;
    validate_stats_section(config)?;
    validate_structure_section(config)?;
    Ok(())
}

fn validate_content_section(config: &Config) -> Result<()> {
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
    Ok(())
}

fn validate_glob_patterns(config: &Config) -> Result<()> {
    for pattern in &config.scanner.exclude {
        globset::Glob::new(pattern).map_err(|e| SlocGuardError::InvalidPattern {
            pattern: pattern.clone(),
            source: e,
        })?;
    }
    for pattern in &config.content.exclude {
        globset::Glob::new(pattern).map_err(|e| SlocGuardError::InvalidPattern {
            pattern: pattern.clone(),
            source: e,
        })?;
    }
    Ok(())
}

fn validate_stats_section(config: &Config) -> Result<()> {
    for section in &config.stats.report.exclude {
        let normalized = section.to_lowercase();
        if !VALID_REPORT_SECTIONS.contains(&normalized.as_str()) {
            return Err(SlocGuardError::Config(format!(
                "stats.report.exclude contains invalid section '{section}'. Valid values: {}",
                VALID_REPORT_SECTIONS.join(", ")
            )));
        }
    }

    if let Some(breakdown_by) = &config.stats.report.breakdown_by {
        let normalized = breakdown_by.to_lowercase();
        if !VALID_BREAKDOWN_BY.contains(&normalized.as_str()) {
            return Err(SlocGuardError::Config(format!(
                "stats.report.breakdown_by has invalid value '{breakdown_by}'. Valid values: lang, dir"
            )));
        }
    }

    if let Some(trend_since) = &config.stats.report.trend_since {
        parse_duration(trend_since).map_err(|_| {
            SlocGuardError::Config(format!(
                "stats.report.trend_since has invalid duration format '{trend_since}'. Expected format: <number><unit> (e.g., 7d, 1w, 12h)"
            ))
        })?;
    }
    Ok(())
}

fn validate_structure_section(config: &Config) -> Result<()> {
    validate_structure_global(config)?;
    validate_structure_rules(config)?;
    Ok(())
}

fn validate_structure_global(config: &Config) -> Result<()> {
    // Validate structure warn_* thresholds (0.0-1.0 range)
    if let Some(warn_threshold) = config.structure.warn_threshold
        && !(0.0..=1.0).contains(&warn_threshold)
    {
        return Err(SlocGuardError::Config(format!(
            "structure.warn_threshold must be between 0.0 and 1.0, got {warn_threshold}"
        )));
    }
    if let Some(warn_files_threshold) = config.structure.warn_files_threshold
        && !(0.0..=1.0).contains(&warn_files_threshold)
    {
        return Err(SlocGuardError::Config(format!(
            "structure.warn_files_threshold must be between 0.0 and 1.0, got {warn_files_threshold}"
        )));
    }
    if let Some(warn_dirs_threshold) = config.structure.warn_dirs_threshold
        && !(0.0..=1.0).contains(&warn_dirs_threshold)
    {
        return Err(SlocGuardError::Config(format!(
            "structure.warn_dirs_threshold must be between 0.0 and 1.0, got {warn_dirs_threshold}"
        )));
    }

    // Validate structure warn_*_at values (must be non-negative)
    if let Some(warn_files_at) = config.structure.warn_files_at
        && warn_files_at < 0
    {
        return Err(SlocGuardError::Config(format!(
            "structure.warn_files_at must be non-negative, got {warn_files_at}"
        )));
    }
    if let Some(warn_dirs_at) = config.structure.warn_dirs_at
        && warn_dirs_at < 0
    {
        return Err(SlocGuardError::Config(format!(
            "structure.warn_dirs_at must be non-negative, got {warn_dirs_at}"
        )));
    }

    // Validate structure warn_*_at < max_* (when both are set and max is not unlimited)
    if let (Some(warn_files_at), Some(max_files)) =
        (config.structure.warn_files_at, config.structure.max_files)
        && max_files >= 0
        && warn_files_at >= max_files
    {
        return Err(SlocGuardError::Config(format!(
            "structure.warn_files_at ({warn_files_at}) must be less than structure.max_files ({max_files})"
        )));
    }
    if let (Some(warn_dirs_at), Some(max_dirs)) =
        (config.structure.warn_dirs_at, config.structure.max_dirs)
        && max_dirs >= 0
        && warn_dirs_at >= max_dirs
    {
        return Err(SlocGuardError::Config(format!(
            "structure.warn_dirs_at ({warn_dirs_at}) must be less than structure.max_dirs ({max_dirs})"
        )));
    }
    Ok(())
}

fn validate_structure_rules(config: &Config) -> Result<()> {
    for (i, rule) in config.structure.rules.iter().enumerate() {
        if let Some(warn_threshold) = rule.warn_threshold
            && !(0.0..=1.0).contains(&warn_threshold)
        {
            return Err(SlocGuardError::Config(format!(
                "structure.rules[{i}].warn_threshold must be between 0.0 and 1.0, got {warn_threshold}"
            )));
        }
        if let Some(warn_files_threshold) = rule.warn_files_threshold
            && !(0.0..=1.0).contains(&warn_files_threshold)
        {
            return Err(SlocGuardError::Config(format!(
                "structure.rules[{i}].warn_files_threshold must be between 0.0 and 1.0, got {warn_files_threshold}"
            )));
        }
        if let Some(warn_dirs_threshold) = rule.warn_dirs_threshold
            && !(0.0..=1.0).contains(&warn_dirs_threshold)
        {
            return Err(SlocGuardError::Config(format!(
                "structure.rules[{i}].warn_dirs_threshold must be between 0.0 and 1.0, got {warn_dirs_threshold}"
            )));
        }
        if let Some(warn_files_at) = rule.warn_files_at
            && warn_files_at < 0
        {
            return Err(SlocGuardError::Config(format!(
                "structure.rules[{i}].warn_files_at must be non-negative, got {warn_files_at}"
            )));
        }
        if let Some(warn_dirs_at) = rule.warn_dirs_at
            && warn_dirs_at < 0
        {
            return Err(SlocGuardError::Config(format!(
                "structure.rules[{i}].warn_dirs_at must be non-negative, got {warn_dirs_at}"
            )));
        }
        if let (Some(warn_files_at), Some(max_files)) = (rule.warn_files_at, rule.max_files)
            && max_files >= 0
            && warn_files_at >= max_files
        {
            return Err(SlocGuardError::Config(format!(
                "structure.rules[{i}].warn_files_at ({warn_files_at}) must be less than structure.rules[{i}].max_files ({max_files})"
            )));
        }
        if let (Some(warn_dirs_at), Some(max_dirs)) = (rule.warn_dirs_at, rule.max_dirs)
            && max_dirs >= 0
            && warn_dirs_at >= max_dirs
        {
            return Err(SlocGuardError::Config(format!(
                "structure.rules[{i}].warn_dirs_at ({warn_dirs_at}) must be less than structure.rules[{i}].max_dirs ({max_dirs})"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_config_passes_validation() {
        let config = Config::default();
        assert!(validate_config_semantics(&config).is_ok());
    }

    #[test]
    fn test_invalid_warn_threshold_rejected() {
        let mut config = Config::default();
        config.content.warn_threshold = 1.5;
        let result = validate_config_semantics(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("warn_threshold"));
    }

    #[test]
    fn test_warn_at_greater_than_max_lines_rejected() {
        let mut config = Config::default();
        config.content.max_lines = 100;
        config.content.warn_at = Some(150);
        let result = validate_config_semantics(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("warn_at"));
    }
}
