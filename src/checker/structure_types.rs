use std::path::PathBuf;

/// Counts of immediate children in a directory.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DirStats {
    pub file_count: usize,
    pub dir_count: usize,
    /// Depth relative to scan root (root = 0).
    pub depth: usize,
}

/// Type of structure violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViolationType {
    FileCount,
    DirCount,
    MaxDepth,
    /// File type not allowed by allowlist (`allow_extensions`/`allow_patterns`).
    DisallowedFile,
    /// File name does not match required naming pattern (`file_naming_pattern`).
    NamingConvention {
        /// The regex pattern that the filename should have matched.
        expected_pattern: String,
    },
}

/// A structure limit violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureViolation {
    pub path: PathBuf,
    pub violation_type: ViolationType,
    pub actual: usize,
    pub limit: usize,
    /// True if this is a warning (threshold exceeded but under hard limit).
    pub is_warning: bool,
    /// Reason for override if one was applied.
    pub override_reason: Option<String>,
    /// Pattern of the rule that triggered this violation (for `DisallowedFile`).
    pub triggering_rule_pattern: Option<String>,
}

impl StructureViolation {
    #[must_use]
    pub const fn new(
        path: PathBuf,
        violation_type: ViolationType,
        actual: usize,
        limit: usize,
        override_reason: Option<String>,
    ) -> Self {
        Self {
            path,
            violation_type,
            actual,
            limit,
            is_warning: false,
            override_reason,
            triggering_rule_pattern: None,
        }
    }

    #[must_use]
    pub const fn warning(
        path: PathBuf,
        violation_type: ViolationType,
        actual: usize,
        limit: usize,
        override_reason: Option<String>,
    ) -> Self {
        Self {
            path,
            violation_type,
            actual,
            limit,
            is_warning: true,
            override_reason,
            triggering_rule_pattern: None,
        }
    }

    /// Create a disallowed file violation.
    #[must_use]
    pub const fn disallowed_file(path: PathBuf, rule_pattern: String) -> Self {
        Self {
            path,
            violation_type: ViolationType::DisallowedFile,
            actual: 1,
            limit: 0,
            is_warning: false,
            override_reason: None,
            triggering_rule_pattern: Some(rule_pattern),
        }
    }

    /// Create a naming convention violation.
    #[must_use]
    pub const fn naming_convention(
        path: PathBuf,
        rule_pattern: String,
        expected_naming_pattern: String,
    ) -> Self {
        Self {
            path,
            violation_type: ViolationType::NamingConvention {
                expected_pattern: expected_naming_pattern,
            },
            actual: 1,
            limit: 0,
            is_warning: false,
            override_reason: None,
            triggering_rule_pattern: Some(rule_pattern),
        }
    }
}
