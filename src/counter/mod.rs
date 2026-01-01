mod comment;
mod sloc;

pub use comment::CommentDetector;
pub use sloc::{CountResult, LineStats, SlocCounter};

#[cfg(test)]
mod test_fixtures;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::LanguageRegistry;

    #[test]
    fn counter_integration_with_language() {
        let registry = LanguageRegistry::default();
        let rust_lang = registry.get_by_extension("rs").unwrap();
        let counter = SlocCounter::new(&rust_lang.comment_syntax);

        let source = "fn main() {\n    // comment\n    println!(\"hello\");\n}\n";
        let result = counter.count(source);

        let CountResult::Stats(stats) = result else {
            panic!("Expected Stats, got IgnoredFile");
        };
        assert_eq!(stats.total, 4);
    }
}
