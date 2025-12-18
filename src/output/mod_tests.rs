use std::path::PathBuf;

use super::*;
use crate::checker::CheckStatus;
use crate::counter::LineStats;

fn sample_results() -> Vec<CheckResult> {
    vec![
        CheckResult {
            path: PathBuf::from("src/main.rs"),
            status: CheckStatus::Passed,
            stats: LineStats {
                total: 110,
                code: 100,
                comment: 5,
                blank: 5, ignored: 0,
            },
            limit: 500,
            override_reason: None,
            suggestions: None,
        },
        CheckResult {
            path: PathBuf::from("src/lib.rs"),
            status: CheckStatus::Failed,
            stats: LineStats {
                total: 600,
                code: 550,
                comment: 30,
                blank: 20, ignored: 0,
            },
            limit: 500,
            override_reason: None,
            suggestions: None,
        },
    ]
}

#[test]
fn output_format_from_str() {
    assert_eq!("text".parse::<OutputFormat>().unwrap(), OutputFormat::Text);
    assert_eq!("json".parse::<OutputFormat>().unwrap(), OutputFormat::Json);
    assert_eq!(
        "sarif".parse::<OutputFormat>().unwrap(),
        OutputFormat::Sarif
    );
    assert_eq!(
        "markdown".parse::<OutputFormat>().unwrap(),
        OutputFormat::Markdown
    );
    assert_eq!("md".parse::<OutputFormat>().unwrap(), OutputFormat::Markdown);
}

#[test]
fn output_format_unknown() {
    assert!("unknown".parse::<OutputFormat>().is_err());
}

#[test]
fn text_formatter_produces_output() {
    let formatter = TextFormatter::new(ColorMode::Never);
    let results = sample_results();
    let output = formatter.format(&results).unwrap();

    assert!(output.contains("src/lib.rs"));
    assert!(output.contains("Summary"));
}

#[test]
fn json_formatter_produces_valid_json() {
    let formatter = JsonFormatter::new();
    let results = sample_results();
    let output = formatter.format(&results).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(parsed.is_object());
}
