//! Config source parsing with per-source schema enforcement.
//!
//! Every configuration source — local file, remote URL, or built-in preset — is
//! parsed through [`parse_source`] before entering the merge pipeline. The full
//! [`Config`] schema (including unknown-field rejection) is enforced against each
//! source individually: once values are merged across an extends chain, a schema
//! violation can no longer be attributed to the file that introduced it.

use crate::error::{ConfigSource, Result, SlocGuardError};

use super::merge::{has_any_reset_markers, strip_reset_markers, validate_reset_positions};
use super::model::{CONFIG_VERSION, Config};

/// A single config source parsed into both representations the loader needs.
#[derive(Debug, Clone)]
pub struct ParsedSource {
    /// Raw TOML value with `$reset` markers intact — input to the merge pipeline.
    pub value: toml::Value,
    /// Schema-checked config with `$reset` markers stripped — the effective
    /// config when no extends chain is involved.
    pub config: Config,
}

/// Parse one config source, enforcing the full `Config` schema against it.
///
/// Errors carry `origin` plus line/column resolved from this source's own text
/// (approximate when the source uses `$reset` markers, which force a re-render).
pub fn parse_source(content: &str, origin: &ConfigSource) -> Result<ParsedSource> {
    let value: toml::Value = toml::from_str(content)
        .map_err(|e| SlocGuardError::syntax_from_toml(&e, content, Some(origin.clone())))?;

    let config = if has_any_reset_markers(&value) {
        // `$reset` is merge-layer syntax the schema doesn't know; peel it off a copy.
        config_from_value(value.clone(), origin)?
    } else {
        config_from_str(content, origin)?
    };

    Ok(ParsedSource { value, config })
}

/// Finalize a merged extends chain into a version-validated config.
///
/// `origin` names the entry config file. Per-source schema checks in
/// [`parse_source`] have already run on every chain member, so a failure here
/// indicates a merge artifact rather than a typo in one file; reported line
/// numbers refer to the merged rendering, not any source file — schema errors
/// are marked "(in merged config ...)" so users are not sent to a wrong
/// location in a real file.
pub fn finalize_merged_config(value: toml::Value, origin: &ConfigSource) -> Result<Config> {
    let config = config_from_value(value, origin).map_err(mark_as_merged)?;
    validate_version(&config)?;
    Ok(config)
}

/// Tag schema errors on the merged value: the named origin is the entry file,
/// but line/column resolve against the invisible merged rendering.
fn mark_as_merged(err: SlocGuardError) -> SlocGuardError {
    match err {
        SlocGuardError::Syntax {
            origin,
            line,
            column,
            message,
        } => SlocGuardError::Syntax {
            origin,
            line,
            column,
            message: format!(
                "{message} (in merged config; line/column refer to the merged extends result, not a source file)"
            ),
        },
        other => other,
    }
}

/// Validate the config schema version. `None` means "use defaults" and is accepted.
///
/// Deliberately not part of [`parse_source`]: within an extends chain only the
/// merged result's version is authoritative, so per-source parsing must not
/// reject a base config that predates the version field.
pub fn validate_version(config: &Config) -> Result<()> {
    match &config.version {
        None => Ok(()),
        Some(v) if v == CONFIG_VERSION => Ok(()),
        Some(v) => Err(SlocGuardError::Config(format!(
            "Unsupported config version '{v}'. Only version '{CONFIG_VERSION}' is supported. \
             Please update your configuration to the V2 format."
        ))),
    }
}

/// Deserialize a `Config` from a value that may still carry `$reset` markers.
///
/// The value is re-rendered to TOML text so error spans have a document to
/// resolve against; those line numbers describe the rendering, not the
/// original source.
fn config_from_value(mut value: toml::Value, origin: &ConfigSource) -> Result<Config> {
    validate_reset_positions(&value, "", Some(origin))?;
    strip_reset_markers(&mut value);
    let rendered = toml::to_string(&value).map_err(|e| SlocGuardError::Config(e.to_string()))?;
    config_from_str(&rendered, origin)
}

fn config_from_str(content: &str, origin: &ConfigSource) -> Result<Config> {
    toml::from_str(content)
        .map_err(|e| SlocGuardError::syntax_from_toml(&e, content, Some(origin.clone())))
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod tests;
