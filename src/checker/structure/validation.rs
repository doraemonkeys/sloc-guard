//! Validation logic for structure configuration.

use crate::config::{StructureConfig, StructureRule, UNLIMITED};
use crate::error::{Result, SlocGuardError};

/// Validate that all limit values are >= -1.
pub(super) fn validate_limits(config: &StructureConfig) -> Result<()> {
    validate_global_limits(config)?;
    validate_rule_limits(&config.rules)?;
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

/// Validate sibling rule configuration.
/// Ensures sibling rules have valid structure.
#[allow(clippy::literal_string_with_formatting_args)] // {stem} is template syntax, not a format arg
pub(super) fn validate_sibling_rules(rules: &[StructureRule]) -> Result<()> {
    use crate::config::SiblingRule;

    for (i, rule) in rules.iter().enumerate() {
        for (j, sibling) in rule.siblings.iter().enumerate() {
            match sibling {
                SiblingRule::Directed {
                    match_pattern,
                    require,
                    ..
                } => {
                    if match_pattern.is_empty() {
                        return Err(SlocGuardError::Config(format!(
                            "Rule {} sibling {} has empty 'match' pattern.",
                            i + 1,
                            j + 1
                        )));
                    }
                    let patterns = require.as_patterns();
                    if patterns.is_empty() {
                        return Err(SlocGuardError::Config(format!(
                            "Rule {} sibling {} has empty 'require' pattern.",
                            i + 1,
                            j + 1
                        )));
                    }
                    for pattern in patterns {
                        if pattern.is_empty() {
                            return Err(SlocGuardError::Config(format!(
                                "Rule {} sibling {} has empty pattern in 'require' array.",
                                i + 1,
                                j + 1
                            )));
                        }
                        // Require patterns must contain {stem} placeholder for sibling derivation
                        if !pattern.contains("{stem}") {
                            return Err(SlocGuardError::Config(format!(
                                "Rule {} sibling {} 'require' pattern '{}' must contain {{stem}} placeholder.",
                                i + 1,
                                j + 1,
                                pattern
                            )));
                        }
                    }
                }
                SiblingRule::Group { group, .. } => {
                    if group.len() < 2 {
                        return Err(SlocGuardError::Config(format!(
                            "Rule {} sibling {} group must have at least 2 patterns.",
                            i + 1,
                            j + 1
                        )));
                    }
                    for (k, pattern) in group.iter().enumerate() {
                        if pattern.is_empty() {
                            return Err(SlocGuardError::Config(format!(
                                "Rule {} sibling {} has empty pattern at index {}.",
                                i + 1,
                                j + 1,
                                k
                            )));
                        }
                        // Group patterns must contain {stem} placeholder for stem extraction
                        if !pattern.contains("{stem}") {
                            return Err(SlocGuardError::Config(format!(
                                "Rule {} sibling {} pattern '{}' must contain {{stem}} placeholder.",
                                i + 1,
                                j + 1,
                                pattern
                            )));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// Validate mutual exclusion between allow and deny fields.
/// At each level (global or rule), only allow-mode OR deny-mode is permitted, not both.
pub(super) fn validate_allow_deny_mutual_exclusion(config: &StructureConfig) -> Result<()> {
    validate_global_allow_deny_exclusion(config)?;
    validate_rule_allow_deny_exclusion(&config.rules)?;
    Ok(())
}

fn validate_global_allow_deny_exclusion(config: &StructureConfig) -> Result<()> {
    let has_allow = !config.allow_files.is_empty()
        || !config.allow_dirs.is_empty()
        || !config.allow_extensions.is_empty();

    let has_deny = !config.deny_files.is_empty()
        || !config.deny_dirs.is_empty()
        || !config.deny_extensions.is_empty()
        || !config.deny_patterns.is_empty();

    if has_allow && has_deny {
        return Err(SlocGuardError::Config(
            "Global structure config cannot mix allow_* and deny_* fields. \
             Use either allowlist mode OR denylist mode, not both."
                .to_string(),
        ));
    }
    Ok(())
}

fn validate_rule_allow_deny_exclusion(rules: &[StructureRule]) -> Result<()> {
    for (i, rule) in rules.iter().enumerate() {
        let has_allow = !rule.allow_files.is_empty()
            || !rule.allow_dirs.is_empty()
            || !rule.allow_extensions.is_empty()
            || !rule.allow_patterns.is_empty();

        let has_deny = !rule.deny_files.is_empty()
            || !rule.deny_dirs.is_empty()
            || !rule.deny_extensions.is_empty()
            || !rule.deny_patterns.is_empty();

        if has_allow && has_deny {
            return Err(SlocGuardError::Config(format!(
                "Rule {} (scope '{}') cannot mix allow_* and deny_* fields. \
                 Use either allowlist mode OR denylist mode, not both.",
                i + 1,
                rule.scope
            )));
        }
    }
    Ok(())
}
