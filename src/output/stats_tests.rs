use std::path::PathBuf;

use crate::counter::LineStats;
use crate::output::stats::{
    FileStatistics, ProjectStatistics, StatsFormatter, StatsJsonFormatter, StatsTextFormatter,
};

#[test]
fn project_statistics_empty() {
    let stats = ProjectStatistics::new(vec![]);
    assert_eq!(stats.total_files, 0);
    assert_eq!(stats.total_lines, 0);
    assert_eq!(stats.total_code, 0);
    assert_eq!(stats.total_comment, 0);
    assert_eq!(stats.total_blank, 0);
}

#[test]
fn project_statistics_single_file() {
    let files = vec![FileStatistics {
        path: PathBuf::from("test.rs"),
        stats: LineStats {
            total: 100,
            code: 80,
            comment: 15,
            blank: 5,
        },
    }];

    let stats = ProjectStatistics::new(files);
    assert_eq!(stats.total_files, 1);
    assert_eq!(stats.total_lines, 100);
    assert_eq!(stats.total_code, 80);
    assert_eq!(stats.total_comment, 15);
    assert_eq!(stats.total_blank, 5);
}

#[test]
fn project_statistics_multiple_files() {
    let files = vec![
        FileStatistics {
            path: PathBuf::from("a.rs"),
            stats: LineStats {
                total: 100,
                code: 80,
                comment: 15,
                blank: 5,
            },
        },
        FileStatistics {
            path: PathBuf::from("b.rs"),
            stats: LineStats {
                total: 50,
                code: 40,
                comment: 5,
                blank: 5,
            },
        },
    ];

    let stats = ProjectStatistics::new(files);
    assert_eq!(stats.total_files, 2);
    assert_eq!(stats.total_lines, 150);
    assert_eq!(stats.total_code, 120);
    assert_eq!(stats.total_comment, 20);
    assert_eq!(stats.total_blank, 10);
}

#[test]
fn text_formatter_empty() {
    let stats = ProjectStatistics::new(vec![]);
    let output = StatsTextFormatter.format(&stats).unwrap();

    assert!(output.contains("Summary:"));
    assert!(output.contains("Files: 0"));
    assert!(output.contains("Total lines: 0"));
}

#[test]
fn text_formatter_with_files() {
    let files = vec![FileStatistics {
        path: PathBuf::from("test.rs"),
        stats: LineStats {
            total: 100,
            code: 80,
            comment: 15,
            blank: 5,
        },
    }];

    let stats = ProjectStatistics::new(files);
    let output = StatsTextFormatter.format(&stats).unwrap();

    assert!(output.contains("test.rs"));
    assert!(output.contains("100 lines"));
    assert!(output.contains("code=80"));
    assert!(output.contains("comment=15"));
    assert!(output.contains("blank=5"));
    assert!(output.contains("Files: 1"));
}

#[test]
fn json_formatter_empty() {
    let stats = ProjectStatistics::new(vec![]);
    let output = StatsJsonFormatter.format(&stats).unwrap();

    assert!(output.contains("\"total_files\": 0"));
    assert!(output.contains("\"files\": []"));
}

#[test]
fn json_formatter_with_files() {
    let files = vec![FileStatistics {
        path: PathBuf::from("test.rs"),
        stats: LineStats {
            total: 100,
            code: 80,
            comment: 15,
            blank: 5,
        },
    }];

    let stats = ProjectStatistics::new(files);
    let output = StatsJsonFormatter.format(&stats).unwrap();

    assert!(output.contains("\"total_files\": 1"));
    assert!(output.contains("\"total_lines\": 100"));
    assert!(output.contains("\"code\": 80"));
    assert!(output.contains("\"test.rs\""));
}

#[test]
fn json_formatter_valid_json() {
    let files = vec![FileStatistics {
        path: PathBuf::from("test.rs"),
        stats: LineStats {
            total: 100,
            code: 80,
            comment: 15,
            blank: 5,
        },
    }];

    let stats = ProjectStatistics::new(files);
    let output = StatsJsonFormatter.format(&stats).unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(parsed.get("summary").is_some());
    assert!(parsed.get("files").is_some());
}
