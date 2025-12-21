//! Validation logic for structure configuration.

use crate::config::{StructureConfig, StructureRule, UNLIMITED};
use crate::error::{Result, SlocGuardError};

/// Validate that all limit values are >= -1.
pub(super) fn validate_limits(config: &StructureConfig) -> Result<()> {
    validate_global_limits(config)?;
    validate_rule_limits(&config.rules)?;
    validate_override_limits(&config.overrides)?;
    Ok(())
}

fn validate_global_limits(config: &StructureConfig) -> Result<()> {
    if let Some(limit) = config.max_files
        && limit < UNLIMITED
    {
        return Err(SlocGuardError::Config(format!(
            "Invalid max_files value: {limit}. Use -1 for unlimited, 0 for prohibited, or a positive number."
        )));
    }
    if let Some(limit) = config.max_dirs
        && limit < UNLIMITED
    {
        return Err(SlocGuardError::Config(format!(
            "Invalid max_dirs value: {limit}. Use -1 for unlimited, 0 for prohibited, or a positive number."
        )));
    }
    if let Some(limit) = config.max_depth
        && limit < UNLIMITED
    {
        return Err(SlocGuardError::Config(format!(
            "Invalid max_depth value: {limit}. Use -1 for unlimited, or a positive number."
        )));
    }
    Ok(())
}

fn validate_rule_limits(rules: &[StructureRule]) -> Result<()> {
    for (i, rule) in rules.iter().enumerate() {
        if let Some(limit) = rule.max_files
            && limit < UNLIMITED
        {
            return Err(SlocGuardError::Config(format!(
                "Invalid max_files value in rule {}: {limit}. Use -1 for unlimited, 0 for prohibited, or a positive number.",
                i + 1
            )));
        }
        if let Some(limit) = rule.max_dirs
            && limit < UNLIMITED
        {
            return Err(SlocGuardError::Config(format!(
                "Invalid max_dirs value in rule {}: {limit}. Use -1 for unlimited, 0 for prohibited, or a positive number.",
                i + 1
            )));
        }
        if let Some(limit) = rule.max_depth
            && limit < UNLIMITED
        {
            return Err(SlocGuardError::Config(format!(
                "Invalid max_depth value in rule {}: {limit}. Use -1 for unlimited, or a positive number.",
                i + 1
            )));
        }
    }
    Ok(())
}

fn validate_override_limits(overrides: &[crate::config::StructureOverride]) -> Result<()> {
    for (i, ovr) in overrides.iter().enumerate() {
        if let Some(limit) = ovr.max_files
            && limit < UNLIMITED
        {
            return Err(SlocGuardError::Config(format!(
                "Invalid max_files value in override {}: {limit}. Use -1 for unlimited, 0 for prohibited, or a positive number.",
                i + 1
            )));
        }
        if let Some(limit) = ovr.max_dirs
            && limit < UNLIMITED
        {
            return Err(SlocGuardError::Config(format!(
                "Invalid max_dirs value in override {}: {limit}. Use -1 for unlimited, 0 for prohibited, or a positive number.",
                i + 1
            )));
        }
        if let Some(limit) = ovr.max_depth
            && limit < UNLIMITED
        {
            return Err(SlocGuardError::Config(format!(
                "Invalid max_depth value in override {}: {limit}. Use -1 for unlimited, or a positive number.",
                i + 1
            )));
        }
        // Require at least one limit to be set
        if ovr.max_files.is_none() && ovr.max_dirs.is_none() && ovr.max_depth.is_none() {
            return Err(SlocGuardError::Config(format!(
                "Override {} for path '{}' must specify at least one of max_files, max_dirs, or max_depth.",
                i + 1,
                ovr.path
            )));
        }
    }
    Ok(())
}

/// Validate sibling rule configuration.
/// `require_sibling` requires `file_pattern` to be set.
pub(super) fn validate_sibling_rules(rules: &[StructureRule]) -> Result<()> {
    for (i, rule) in rules.iter().enumerate() {
        if rule.require_sibling.is_some() && rule.file_pattern.is_none() {
            return Err(SlocGuardError::Config(format!(
                "Rule {} has 'require_sibling' but no 'file_pattern'. \
                 Both must be set together to specify which files need siblings.",
                i + 1
            )));
        }
    }
    Ok(())
}
