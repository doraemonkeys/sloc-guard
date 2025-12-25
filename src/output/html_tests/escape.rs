use std::path::PathBuf;

use crate::checker::CheckResult;
use crate::counter::LineStats;
use crate::output::html::html_escape;
use crate::output::{HtmlFormatter, OutputFormatter};

#[test]
fn html_escape_special_characters() {
    assert_eq!(html_escape("<script>"), "&lt;script&gt;");
    assert_eq!(html_escape("a & b"), "a &amp; b");
    assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
    assert_eq!(html_escape("it's"), "it&#39;s");
}

#[test]
fn escapes_file_paths() {
    let results = vec![CheckResult::Failed {
        path: PathBuf::from("src/<script>.rs"),
        stats: LineStats {
            total: 600,
            code: 600,
            comment: 0,
            blank: 0,
            ignored: 0,
        },
        raw_stats: None,
        limit: 500,
        override_reason: None,
        suggestions: None,
        violation_category: None,
    }];

    let formatter = HtmlFormatter::new();
    let output = formatter.format(&results).unwrap();

    // File path should be escaped
    assert!(output.contains("&lt;script&gt;"));
    // The raw file path should not appear in the table (only the escaped version)
    assert!(!output.contains("src/<script>.rs"));
}
