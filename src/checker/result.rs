use std::path::{Path, PathBuf};

use crate::analyzer::SplitSuggestion;
use crate::counter::LineStats;

use super::structure::violation::ViolationCategory;

/// Result of checking a file against configured thresholds.
///
/// Each variant represents a distinct check outcome. The `suggestions` field is only
/// available on `Warning` and `Failed` variants, making it impossible to have suggestions
/// on passed or grandfathered results.
///
/// The `violation_category` field distinguishes between content (SLOC) and structure
/// violations, preserving the structured `ViolationType` for structure violations.
///
/// `stats` contains the effective line counts (used for limit checking), while
/// `raw_stats` contains the original counts before `skip_comments`/`skip_blank` adjustments.
#[derive(Debug, Clone)]
pub enum CheckResult {
    Passed {
        path: PathBuf,
        stats: LineStats,
        raw_stats: Option<LineStats>,
        limit: usize,
        override_reason: Option<String>,
        violation_category: Option<ViolationCategory>,
    },
    Warning {
        path: PathBuf,
        stats: LineStats,
        raw_stats: Option<LineStats>,
        limit: usize,
        override_reason: Option<String>,
        suggestions: Option<SplitSuggestion>,
        violation_category: Option<ViolationCategory>,
    },
    Failed {
        path: PathBuf,
        stats: LineStats,
        raw_stats: Option<LineStats>,
        limit: usize,
        override_reason: Option<String>,
        suggestions: Option<SplitSuggestion>,
        violation_category: Option<ViolationCategory>,
    },
    Grandfathered {
        path: PathBuf,
        stats: LineStats,
        raw_stats: Option<LineStats>,
        limit: usize,
        override_reason: Option<String>,
        violation_category: Option<ViolationCategory>,
    },
}

impl CheckResult {
    // Accessor methods

    #[must_use]
    pub fn path(&self) -> &Path {
        match self {
            Self::Passed { path, .. }
            | Self::Warning { path, .. }
            | Self::Failed { path, .. }
            | Self::Grandfathered { path, .. } => path,
        }
    }

    #[must_use]
    pub const fn stats(&self) -> &LineStats {
        match self {
            Self::Passed { stats, .. }
            | Self::Warning { stats, .. }
            | Self::Failed { stats, .. }
            | Self::Grandfathered { stats, .. } => stats,
        }
    }

    /// Returns the raw (original) statistics before `skip_comments`/`skip_blank` adjustments.
    ///
    /// Falls back to effective stats if raw stats are not available (e.g., structure violations).
    #[must_use]
    pub fn raw_stats(&self) -> &LineStats {
        match self {
            Self::Passed {
                raw_stats, stats, ..
            }
            | Self::Warning {
                raw_stats, stats, ..
            }
            | Self::Failed {
                raw_stats, stats, ..
            }
            | Self::Grandfathered {
                raw_stats, stats, ..
            } => raw_stats.as_ref().unwrap_or(stats),
        }
    }

    #[must_use]
    pub const fn limit(&self) -> usize {
        match self {
            Self::Passed { limit, .. }
            | Self::Warning { limit, .. }
            | Self::Failed { limit, .. }
            | Self::Grandfathered { limit, .. } => *limit,
        }
    }

    #[must_use]
    pub fn override_reason(&self) -> Option<&str> {
        match self {
            Self::Passed {
                override_reason, ..
            }
            | Self::Warning {
                override_reason, ..
            }
            | Self::Failed {
                override_reason, ..
            }
            | Self::Grandfathered {
                override_reason, ..
            } => override_reason.as_deref(),
        }
    }

    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Accessing option reference isn't const
    pub fn suggestions(&self) -> Option<&SplitSuggestion> {
        match self {
            Self::Warning { suggestions, .. } | Self::Failed { suggestions, .. } => {
                suggestions.as_ref()
            }
            Self::Passed { .. } | Self::Grandfathered { .. } => None,
        }
    }

    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // Accessing option reference isn't const
    pub fn violation_category(&self) -> Option<&ViolationCategory> {
        match self {
            Self::Passed {
                violation_category, ..
            }
            | Self::Warning {
                violation_category, ..
            }
            | Self::Failed {
                violation_category, ..
            }
            | Self::Grandfathered {
                violation_category, ..
            } => violation_category.as_ref(),
        }
    }

    // Predicate methods

    #[must_use]
    pub const fn is_passed(&self) -> bool {
        matches!(self, Self::Passed { .. })
    }

    #[must_use]
    pub const fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    #[must_use]
    pub const fn is_warning(&self) -> bool {
        matches!(self, Self::Warning { .. })
    }

    #[must_use]
    pub const fn is_grandfathered(&self) -> bool {
        matches!(self, Self::Grandfathered { .. })
    }

    // Transformation methods

    /// Convert a Failed result to Grandfathered (used for baseline comparison).
    /// Returns self unchanged if not Failed.
    #[must_use]
    pub fn into_grandfathered(self) -> Self {
        match self {
            Self::Failed {
                path,
                stats,
                raw_stats,
                limit,
                override_reason,
                violation_category,
                ..
            } => Self::Grandfathered {
                path,
                stats,
                raw_stats,
                limit,
                override_reason,
                violation_category,
            },
            other => other,
        }
    }

    /// Add split suggestions to a Warning or Failed result.
    /// Returns self unchanged if Passed or Grandfathered.
    #[must_use]
    pub fn with_suggestions(self, new_suggestions: SplitSuggestion) -> Self {
        match self {
            Self::Warning {
                path,
                stats,
                raw_stats,
                limit,
                override_reason,
                violation_category,
                ..
            } => Self::Warning {
                path,
                stats,
                raw_stats,
                limit,
                override_reason,
                suggestions: Some(new_suggestions),
                violation_category,
            },
            Self::Failed {
                path,
                stats,
                raw_stats,
                limit,
                override_reason,
                violation_category,
                ..
            } => Self::Failed {
                path,
                stats,
                raw_stats,
                limit,
                override_reason,
                suggestions: Some(new_suggestions),
                violation_category,
            },
            other => other,
        }
    }

    #[must_use]
    #[allow(clippy::cast_precision_loss)] // Precision loss is acceptable for usage percentage
    pub fn usage_percent(&self) -> f64 {
        let limit = self.limit();
        if limit == 0 {
            return 0.0;
        }
        (self.stats().sloc() as f64 / limit as f64) * 100.0
    }
}
