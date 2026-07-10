mod parser;
mod split;

pub use parser::{FunctionInfo, FunctionParser, get_parser};
pub use split::{SplitAnalyzer, SplitChunk, SplitSuggestion};

use crate::checker::CheckResult;
use crate::counter::LineStats;
use crate::language::LanguageRegistry;
use crate::project::ProjectPaths;

/// Generate split suggestions for failed or warning results.
pub fn generate_split_suggestions(results: &mut [CheckResult], registry: &LanguageRegistry) {
    generate_split_suggestions_with_project_paths(results, registry, &ProjectPaths::unrooted());
}

/// Generate split suggestions while resolving logical result paths back to physical files.
pub fn generate_split_suggestions_with_project_paths(
    results: &mut [CheckResult],
    registry: &LanguageRegistry,
    project_paths: &ProjectPaths,
) {
    let analyzer = SplitAnalyzer::default();

    for result in results.iter_mut() {
        if !result.is_failed() && !result.is_warning() {
            continue;
        }

        let Some(ext) = result.path().extension().and_then(|e| e.to_str()) else {
            continue;
        };

        let Some(language) = registry.get_by_extension(ext) else {
            continue;
        };

        let physical_path = project_paths.physical(result.path());
        let Ok(content) = std::fs::read_to_string(physical_path) else {
            continue;
        };

        if let Some(suggestion) =
            analyzer.analyze(result.path(), &content, &language.name, result.limit())
            && suggestion.has_suggestions()
        {
            // Replace the result with its version containing suggestions
            let owned = std::mem::replace(
                result,
                CheckResult::Passed {
                    path: std::path::PathBuf::new(),
                    stats: LineStats::default(),
                    raw_stats: None,
                    limit: 0,
                    override_reason: None,
                    violation_category: None,
                },
            );
            *result = owned.with_suggestions(suggestion);
        }
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
