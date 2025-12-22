//! Rule building logic for structure checker.

use globset::Glob;

use crate::config::StructureRule;
use crate::error::{Result, SlocGuardError};

use super::compiled_rules::{CompiledSiblingRule, CompiledStructureRule};

/// Build compiled structure rules from config rules.
pub(super) fn build_rules(rules: &[StructureRule]) -> Result<Vec<CompiledStructureRule>> {
    rules
        .iter()
        .map(|rule| {
            let glob = Glob::new(&rule.scope).map_err(|e| SlocGuardError::InvalidPattern {
                pattern: rule.scope.clone(),
                source: e,
            })?;

            // Calculate base_depth: count path components before first glob metacharacter
            let base_depth = calculate_base_depth(&rule.scope);

            Ok(CompiledStructureRule {
                scope: rule.scope.clone(),
                matcher: glob.compile_matcher(),
                max_files: rule.max_files,
                max_dirs: rule.max_dirs,
                max_depth: rule.max_depth,
                relative_depth: rule.relative_depth,
                base_depth,
                warn_threshold: rule.warn_threshold,
                warn_files_at: rule.warn_files_at,
                warn_dirs_at: rule.warn_dirs_at,
                warn_files_threshold: rule.warn_files_threshold,
                warn_dirs_threshold: rule.warn_dirs_threshold,
            })
        })
        .collect()
}

/// Build sibling rules from config rules that have `require_sibling` set.
pub(super) fn build_sibling_rules(rules: &[StructureRule]) -> Result<Vec<CompiledSiblingRule>> {
    rules
        .iter()
        .filter(|rule| rule.require_sibling.is_some() && rule.file_pattern.is_some())
        .map(|rule| {
            let dir_glob = Glob::new(&rule.scope).map_err(|e| SlocGuardError::InvalidPattern {
                pattern: rule.scope.clone(),
                source: e,
            })?;

            let file_pattern = rule.file_pattern.as_ref().unwrap();
            let file_glob =
                Glob::new(file_pattern).map_err(|e| SlocGuardError::InvalidPattern {
                    pattern: file_pattern.clone(),
                    source: e,
                })?;

            Ok(CompiledSiblingRule {
                dir_scope: rule.scope.clone(),
                dir_matcher: dir_glob.compile_matcher(),
                file_matcher: file_glob.compile_matcher(),
                sibling_template: rule.require_sibling.clone().unwrap(),
            })
        })
        .collect()
}

/// Calculate the depth of the pattern's base directory.
/// This is the number of path components before the first glob metacharacter.
/// Examples:
/// - "src/features/**" → 2 (src, features)
/// - "src/*/utils" → 1 (src)
/// - "**/*.rs" → 0
/// - "exact/path" → 2
pub(super) fn calculate_base_depth(pattern: &str) -> usize {
    let mut depth = 0;
    for component in pattern.split(['/', '\\']) {
        if component.is_empty() {
            continue;
        }
        // Check if this component contains any glob metacharacters
        if component.contains(['*', '?', '[', '{']) {
            break;
        }
        depth += 1;
    }
    depth
}
