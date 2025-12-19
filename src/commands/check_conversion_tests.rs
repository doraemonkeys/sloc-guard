use std::path::PathBuf;

use crate::checker::{StructureViolation, ViolationType};

#[test]
fn structure_violation_to_check_result_disallowed_file() {
    let violation = StructureViolation::disallowed_file(
        PathBuf::from("src/config.json"),
        "**/src".to_string(),
    );

    let result = super::structure_violation_to_check_result(&violation);

    match result {
        crate::checker::CheckResult::Failed {
            path,
            override_reason,
            ..
        } => {
            assert_eq!(path, PathBuf::from("src/config.json"));
            assert!(override_reason
                .unwrap()
                .contains("structure: disallowed file"));
        }
        _ => panic!("Expected Failed result"),
    }
}

#[test]
fn structure_violation_to_check_result_file_count_warning() {
    let violation = StructureViolation::warning(
        PathBuf::from("src"),
        ViolationType::FileCount,
        45,
        50,
        None,
    );

    let result = super::structure_violation_to_check_result(&violation);

    match result {
        crate::checker::CheckResult::Warning {
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
    let violation = StructureViolation::new(
        PathBuf::from("src"),
        ViolationType::DirCount,
        15,
        10,
        None,
    );

    let result = super::structure_violation_to_check_result(&violation);

    match result {
        crate::checker::CheckResult::Failed {
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

    let result = super::structure_violation_to_check_result(&violation);

    match result {
        crate::checker::CheckResult::Failed {
            override_reason, ..
        } => {
            assert!(override_reason.unwrap().contains("structure: depth"));
        }
        _ => panic!("Expected Failed result"),
    }
}

