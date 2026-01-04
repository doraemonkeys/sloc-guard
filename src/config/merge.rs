//! TOML value merging with inheritance support.
//!
//! Handles merging parent and child config values, including:
//! - Recursive table merging (child values take precedence)
//! - Array appending (parent + child)
//! - `$reset` marker support for clearing parent arrays

use crate::error::{Result, SlocGuardError};

/// The reset marker used to clear parent arrays during merge.
pub const RESET_MARKER: &str = "$reset";

/// Merge two TOML values. Child values take precedence.
/// Tables are merged recursively. Arrays are appended (parent + child),
/// unless child array starts with `$reset` marker which clears the parent.
pub fn merge_toml_values(base: toml::Value, child: toml::Value) -> toml::Value {
    match (base, child) {
        (toml::Value::Table(mut base_table), toml::Value::Table(child_table)) => {
            for (key, child_val) in child_table {
                match base_table.remove(&key) {
                    Some(base_val) => {
                        base_table.insert(key, merge_toml_values(base_val, child_val));
                    }
                    None => {
                        base_table.insert(key, child_val);
                    }
                }
            }
            toml::Value::Table(base_table)
        }
        (toml::Value::Array(base_arr), toml::Value::Array(child_arr)) => {
            merge_arrays(base_arr, child_arr)
        }
        (_, child) => child,
    }
}

/// Merge two arrays. Appends by default, but if child starts with `$reset`,
/// clears the parent and uses remaining child elements.
///
/// Note: The reset marker is intentionally handled in two places:
/// 1. Here during parent-child merge (skips marker, discards parent)
/// 2. In `strip_reset_markers()` for standalone configs without extends
pub fn merge_arrays(base: Vec<toml::Value>, mut child: Vec<toml::Value>) -> toml::Value {
    if has_reset_marker(&child) {
        // Reset: remove the marker, discard parent, use remaining child elements
        child.remove(0);
        toml::Value::Array(child)
    } else {
        // Append: parent + child
        let mut merged = base;
        merged.extend(child);
        toml::Value::Array(merged)
    }
}

/// Check if an array starts with a reset marker.
/// - For string arrays: first element is "$reset"
/// - For table arrays (rules): first element has `pattern = "$reset"` or `scope = "$reset"`
pub fn has_reset_marker(arr: &[toml::Value]) -> bool {
    arr.first().is_some_and(is_reset_element)
}

/// Check if a value is a reset marker element.
pub fn is_reset_element(value: &toml::Value) -> bool {
    match value {
        toml::Value::String(s) => s == RESET_MARKER,
        toml::Value::Table(table) => {
            // For content.rules: check pattern field
            // For structure.rules: check scope field
            table
                .get("pattern")
                .or_else(|| table.get("scope"))
                .and_then(toml::Value::as_str)
                .is_some_and(|s| s == RESET_MARKER)
        }
        _ => false,
    }
}

/// Strip remaining reset markers from the merged config value.
///
/// This handles the case where a config has `$reset` but no parent extends.
/// Note: `merge_arrays()` also removes the marker during parent-child merge;
/// this function catches markers in standalone configs without extends chain.
pub fn strip_reset_markers(value: &mut toml::Value) {
    match value {
        toml::Value::Table(table) => {
            for (_, val) in table.iter_mut() {
                strip_reset_markers(val);
            }
        }
        toml::Value::Array(arr) => {
            // Remove reset marker if it's the first element
            if arr.first().is_some_and(is_reset_element) {
                arr.remove(0);
            }
            // Recursively strip from nested values
            for val in arr {
                strip_reset_markers(val);
            }
        }
        _ => {}
    }
}

/// Check if a TOML value contains any reset markers anywhere in its structure.
///
/// Used to determine whether we can skip the serialize-then-parse path
/// for configs without reset markers (preserving precise line numbers).
///
/// Detects markers at ANY array position (including invalid non-first positions)
/// so that `validate_reset_positions` can catch misplaced markers.
pub fn has_any_reset_markers(value: &toml::Value) -> bool {
    match value {
        toml::Value::Table(table) => table.values().any(has_any_reset_markers),
        toml::Value::Array(arr) => {
            // Check if ANY element is a reset marker (at any position), or recursively contains one
            arr.iter()
                .any(|v| is_reset_element(v) || has_any_reset_markers(v))
        }
        _ => false,
    }
}

/// Validate that `$reset` markers are only in first position of arrays.
/// Returns an error if `$reset` is found in any position other than first.
pub fn validate_reset_positions(value: &toml::Value, path: &str) -> Result<()> {
    match value {
        toml::Value::Table(table) => {
            for (key, val) in table {
                let child_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                validate_reset_positions(val, &child_path)?;
            }
        }
        toml::Value::Array(arr) => {
            for (i, val) in arr.iter().enumerate() {
                if i > 0 && is_reset_element(val) {
                    return Err(SlocGuardError::Config(format!(
                        "'{RESET_MARKER}' must be the first element in array '{path}', found at position {i}"
                    )));
                }
                // Recursively validate nested values
                validate_reset_positions(val, path)?;
            }
        }
        _ => {}
    }
    Ok(())
}
