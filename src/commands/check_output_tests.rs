use std::path::PathBuf;

use crate::checker::{CheckResult, StructureViolation, ViolationType};
use crate::output::{ColorMode, OutputFormat};

use super::{format_output, structure_violation_to_check_result};

#[test]
fn format_output_text() {
    let results: Vec<CheckResult> = vec![];
    let output = format_output(OutputFormat::Text, &results, ColorMode::Never, 0, false).unwrap();
    assert!(output.contains("Summary"));
}

#[test]
fn format_output_json() {
    let results: Vec<CheckResult> = vec![];
    let output = format_output(OutputFormat::Json, &results, ColorMode::Never, 0, false).unwrap();
    assert!(output.contains("summary"));
}

#[test]
fn format_output_sarif_works() {
    let results: Vec<CheckResult> = vec![];
    let result = format_output(OutputFormat::Sarif, &results, ColorMode::Never, 0, false);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("$schema"));
    assert!(output.contains("2.1.0"));
}

#[test]
fn format_output_markdown_works() {
    let results: Vec<CheckResult> = vec![];
    let result = format_output(OutputFormat::Markdown, &results, ColorMode::Never, 0, false);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("## SLOC Guard Results"));
    assert!(output.contains("| Total Files | 0 |"));
}

// Structure violation conversion tests (moved from check_conversion_tests.rs)

#[test]
fn structure_violation_to_check_result_disallowed_file() {
    let violation =
        StructureViolation::disallowed_file(PathBuf::from("src/config.json"), "**/src".to_string());

    let result = structure_violation_to_check_result(&violation);

    match result {
        CheckResult::Failed {
            path,
            override_reason,
            ..
        } => {
            assert_eq!(path, PathBuf::from("src/config.json"));
            assert!(
                override_reason
                    .unwrap()
                    .contains("structure: disallowed file")
            );
        }
        _ => panic!("Expected Failed result"),
    }
}

#[test]
fn structure_violation_to_check_result_file_count_warning() {
    let violation =
        StructureViolation::warning(PathBuf::from("src"), ViolationType::FileCount, 45, 50, None);

    let result = structure_violation_to_check_result(&violation);

    match result {
        CheckResult::Warning {
            path,
            override_reason,
            limit,
            ..
        } => {
            assert_eq!(path, PathBuf::from("src"));
            assert!(override_reason.unwrap().contains("structure: files"));
            assert_eq!(limit, 50);
        }
        _ => panic!("Expected Warning result"),
    }
}

#[test]
fn structure_violation_to_check_result_dir_count() {
    let violation =
        StructureViolation::new(PathBuf::from("src"), ViolationType::DirCount, 15, 10, None);

    let result = structure_violation_to_check_result(&violation);

    match result {
        CheckResult::Failed {
            override_reason, ..
        } => {
            assert!(override_reason.unwrap().contains("structure: subdirs"));
        }
        _ => panic!("Expected Failed result"),
    }
}

#[test]
fn structure_violation_to_check_result_max_depth() {
    let violation = StructureViolation::new(
        PathBuf::from("src/a/b/c/d"),
        ViolationType::MaxDepth,
        5,
        3,
        None,
    );

    let result = structure_violation_to_check_result(&violation);

    match result {
        CheckResult::Failed {
            override_reason, ..
        } => {
            assert!(override_reason.unwrap().contains("structure: depth"));
        }
        _ => panic!("Expected Failed result"),
    }
}

#[test]
fn structure_violation_to_check_result_naming_convention() {
    let violation = StructureViolation::naming_convention(
        PathBuf::from("src/userCard.tsx"),
        "**/src".to_string(),
        "^[A-Z][a-zA-Z0-9]*\\.tsx$".to_string(),
    );

    let result = structure_violation_to_check_result(&violation);

    match result {
        CheckResult::Failed {
            path,
            override_reason,
            ..
        } => {
            assert_eq!(path, PathBuf::from("src/userCard.tsx"));
            let reason = override_reason.unwrap();
            assert!(reason.contains("structure: naming convention violation"));
            assert!(reason.contains("^[A-Z][a-zA-Z0-9]*\\.tsx$"));
            assert!(reason.contains("**/src"));
        }
        _ => panic!("Expected Failed result"),
    }
}
