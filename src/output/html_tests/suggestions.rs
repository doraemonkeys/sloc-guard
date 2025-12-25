use std::path::PathBuf;

use crate::analyzer::{SplitChunk, SplitSuggestion};
use crate::output::OutputFormatter;

use super::make_failed_result;
use crate::output::HtmlFormatter;

#[test]
fn shows_split_suggestions_when_enabled() {
    let result = make_failed_result("src/big_file.rs", 600, 500);
    let suggestion =
        SplitSuggestion::new(PathBuf::from("src/big_file.rs"), 600, 500).with_chunks(vec![
            SplitChunk {
                suggested_name: "big_file_part1".to_string(),
                functions: vec!["func1".to_string(), "func2".to_string()],
                start_line: 1,
                end_line: 300,
                line_count: 300,
            },
            SplitChunk {
                suggested_name: "big_file_part2".to_string(),
                functions: vec!["func3".to_string()],
                start_line: 301,
                end_line: 600,
                line_count: 300,
            },
        ]);
    let result = result.with_suggestions(suggestion);

    let formatter = HtmlFormatter::new().with_suggestions(true);
    let output = formatter.format(&[result]).unwrap();

    assert!(output.contains(r#"class="suggestions""#));
    assert!(output.contains("Split suggestions:"));
    assert!(output.contains("big_file_part1.*"));
    assert!(output.contains("big_file_part2.*"));
    assert!(output.contains("func1, func2"));
}

#[test]
fn hides_suggestions_when_disabled() {
    let result = make_failed_result("src/big_file.rs", 600, 500);
    let suggestion =
        SplitSuggestion::new(PathBuf::from("src/big_file.rs"), 600, 500).with_chunks(vec![
            SplitChunk {
                suggested_name: "big_file_part1".to_string(),
                functions: vec!["func1".to_string()],
                start_line: 1,
                end_line: 300,
                line_count: 300,
            },
        ]);
    let result = result.with_suggestions(suggestion);

    let formatter = HtmlFormatter::new().with_suggestions(false);
    let output = formatter.format(&[result]).unwrap();

    assert!(!output.contains(r#"class="suggestions""#));
}

#[test]
fn split_suggestions_with_empty_functions() {
    let result = make_failed_result("src/big_file.rs", 600, 500);
    let suggestion =
        SplitSuggestion::new(PathBuf::from("src/big_file.rs"), 600, 500).with_chunks(vec![
            SplitChunk {
                suggested_name: "big_file_part1".to_string(),
                functions: vec![], // Empty functions list
                start_line: 1,
                end_line: 300,
                line_count: 300,
            },
        ]);
    let result = result.with_suggestions(suggestion);

    let formatter = HtmlFormatter::new().with_suggestions(true);
    let output = formatter.format(&[result]).unwrap();

    assert!(output.contains("Split suggestions:"));
    assert!(output.contains("big_file_part1.*"));
    // Empty functions should not produce parentheses
    assert!(!output.contains("big_file_part1.* (~300 lines) ("));
}
