use std::path::PathBuf;

use serde::Serialize;

/// Counts of immediate children in a directory.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DirStats {
    pub file_count: usize,
    pub dir_count: usize,
    /// Depth relative to scan root (root = 0).
    pub depth: usize,
}

/// Type of structure violation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ViolationType {
    FileCount,
    DirCount,
    MaxDepth,
    /// File type not allowed by allowlist (`allow_extensions`/`allow_patterns`).
    DisallowedFile,
    /// Directory not allowed by allowlist (`allow_dirs`).
    DisallowedDirectory,
    /// File matches a deny pattern (`deny_extensions`/`deny_patterns`).
    DeniedFile {
        /// The pattern or extension that matched (e.g., ".exe" or "*.bak").
        pattern_or_extension: String,
    },
    /// Directory matches a directory-only deny pattern (patterns ending with `/`).
    DeniedDirectory {
        /// The pattern that matched (e.g., "`**/node_modules/`").
        pattern: String,
    },
    /// File name does not match required naming pattern (`file_naming_pattern`).
    NamingConvention {
        /// The regex pattern that the filename should have matched.
        expected_pattern: String,
    },
    /// Required sibling file is missing (directed `siblings` rule).
    MissingSibling {
        /// The sibling pattern template that was expected (e.g., "{stem}.test.tsx").
        expected_sibling_pattern: String,
    },
    /// Atomic group is incomplete (group `siblings` rule).
    /// If ANY file in the group exists, ALL must exist.
    GroupIncomplete {
        /// The group patterns that form the atomic set.
        group_patterns: Vec<String>,
        /// Which patterns from the group are missing.
        missing_patterns: Vec<String>,
    },
}

/// Category of violation for `CheckResult`.
///
/// Distinguishes between content (SLOC) violations and structure violations,
/// carrying the structured `ViolationType` for structure violations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "category", rename_all = "snake_case")]
pub enum ViolationCategory {
    /// Content violation (SLOC limit exceeded).
    Content,
    /// Structure violation with specific type.
    Structure {
        violation_type: ViolationType,
        /// Pattern of the rule that triggered this violation.
        #[serde(skip_serializing_if = "Option::is_none")]
        triggering_rule: Option<String>,
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

    /// Create a disallowed directory violation.
    #[must_use]
    pub const fn disallowed_directory(path: PathBuf, rule_pattern: String) -> Self {
        Self {
            path,
            violation_type: ViolationType::DisallowedDirectory,
            actual: 1,
            limit: 0,
            is_warning: false,
            override_reason: None,
            triggering_rule_pattern: Some(rule_pattern),
        }
    }

    /// Create a denied file violation.
    #[must_use]
    pub const fn denied_file(
        path: PathBuf,
        rule_pattern: String,
        pattern_or_extension: String,
    ) -> Self {
        Self {
            path,
            violation_type: ViolationType::DeniedFile {
                pattern_or_extension,
            },
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

    /// Create a missing sibling violation (directed rule).
    #[must_use]
    pub const fn missing_sibling(
        path: PathBuf,
        rule_pattern: String,
        expected_sibling_pattern: String,
    ) -> Self {
        Self {
            path,
            violation_type: ViolationType::MissingSibling {
                expected_sibling_pattern,
            },
            actual: 1,
            limit: 1,
            is_warning: false,
            override_reason: None,
            triggering_rule_pattern: Some(rule_pattern),
        }
    }

    /// Create a missing sibling violation as a warning.
    #[must_use]
    pub const fn missing_sibling_warning(
        path: PathBuf,
        rule_pattern: String,
        expected_sibling_pattern: String,
    ) -> Self {
        Self {
            path,
            violation_type: ViolationType::MissingSibling {
                expected_sibling_pattern,
            },
            actual: 1,
            limit: 1,
            is_warning: true,
            override_reason: None,
            triggering_rule_pattern: Some(rule_pattern),
        }
    }

    /// Create a group incomplete violation (atomic group rule).
    #[must_use]
    pub const fn group_incomplete(
        path: PathBuf,
        rule_pattern: String,
        group_patterns: Vec<String>,
        missing_patterns: Vec<String>,
    ) -> Self {
        Self {
            path,
            violation_type: ViolationType::GroupIncomplete {
                group_patterns,
                missing_patterns,
            },
            actual: 1,
            limit: 1,
            is_warning: false,
            override_reason: None,
            triggering_rule_pattern: Some(rule_pattern),
        }
    }

    /// Create a group incomplete violation as a warning.
    #[must_use]
    pub const fn group_incomplete_warning(
        path: PathBuf,
        rule_pattern: String,
        group_patterns: Vec<String>,
        missing_patterns: Vec<String>,
    ) -> Self {
        Self {
            path,
            violation_type: ViolationType::GroupIncomplete {
                group_patterns,
                missing_patterns,
            },
            actual: 1,
            limit: 1,
            is_warning: true,
            override_reason: None,
            triggering_rule_pattern: Some(rule_pattern),
        }
    }

    /// Create a denied directory violation (for patterns ending with `/`).
    #[must_use]
    pub const fn denied_directory(path: PathBuf, rule_pattern: String, pattern: String) -> Self {
        Self {
            path,
            violation_type: ViolationType::DeniedDirectory { pattern },
            actual: 1,
            limit: 0,
            is_warning: false,
            override_reason: None,
            triggering_rule_pattern: Some(rule_pattern),
        }
    }
}
