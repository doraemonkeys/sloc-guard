mod parser;
mod split;
mod types;

pub use parser::{FunctionParser, get_parser};
pub use split::SplitAnalyzer;
pub use types::{FunctionInfo, SplitChunk, SplitSuggestion};

use crate::checker::CheckResult;
use crate::language::LanguageRegistry;

/// Generate split suggestions for failed or warning results.
pub fn generate_split_suggestions(results: &mut [CheckResult], registry: &LanguageRegistry) {
    let analyzer = SplitAnalyzer::default();

    for result in results.iter_mut() {
        if !result.is_failed() && !result.is_warning() {
            continue;
        }

        let Some(ext) = result.path.extension().and_then(|e| e.to_str()) else {
            continue;
        };

        let Some(language) = registry.get_by_extension(ext) else {
            continue;
        };

        let Ok(content) = std::fs::read_to_string(&result.path) else {
            continue;
        };

        if let Some(suggestion) =
            analyzer.analyze(&result.path, &content, &language.name, result.limit)
            && suggestion.has_suggestions()
        {
            result.set_suggestions(suggestion);
        }
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
