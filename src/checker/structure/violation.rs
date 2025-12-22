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
    /// Required sibling file is missing (`require_sibling`).
    MissingSibling {
        /// The sibling pattern template that was expected (e.g., "{stem}.test.tsx").
        expected_sibling_pattern: String,
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

    /// Create a missing sibling violation.
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
