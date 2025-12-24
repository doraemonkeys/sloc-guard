//! Date expiration validation for rules.
//!
//! Provides simple YYYY-MM-DD date parsing and expiration checking.

use super::Config;

/// The type of rule that has expired.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpiredRuleType {
    Content,
    Structure,
}

impl std::fmt::Display for ExpiredRuleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Content => write!(f, "content"),
            Self::Structure => write!(f, "structure"),
        }
    }
}

/// Description of an expired rule for warning output.
#[derive(Debug, Clone)]
pub struct ExpiredRule {
    /// Rule type: content or structure
    pub rule_type: ExpiredRuleType,
    /// Index of the rule in the config array
    pub index: usize,
    /// Pattern or scope of the rule
    pub pattern: String,
    /// The expiration date
    pub expires: String,
    /// Optional reason field from the rule
    pub reason: Option<String>,
}

/// Collect all expired rules from the configuration using today's date.
///
/// Returns a vector of `ExpiredRule` descriptions for any rules with
/// `expires` dates that are in the past.
#[must_use]
pub fn collect_expired_rules(config: &Config) -> Vec<ExpiredRule> {
    collect_expired_rules_with_date(config, ParsedDate::today())
}

/// Collect all expired rules from the configuration using a specified date.
///
/// This function enables dependency injection for testing by accepting
/// the reference date as a parameter.
#[must_use]
pub fn collect_expired_rules_with_date(config: &Config, today: ParsedDate) -> Vec<ExpiredRule> {
    let mut expired = Vec::new();

    // Check content rules
    for (i, rule) in config.content.rules.iter().enumerate() {
        if let Some(ref expires) = rule.expires
            && is_expired_at(expires, today).unwrap_or(false)
        {
            expired.push(ExpiredRule {
                rule_type: ExpiredRuleType::Content,
                index: i,
                pattern: rule.pattern.clone(),
                expires: expires.clone(),
                reason: rule.reason.clone(),
            });
        }
    }

    // Check structure rules
    for (i, rule) in config.structure.rules.iter().enumerate() {
        if let Some(ref expires) = rule.expires
            && is_expired_at(expires, today).unwrap_or(false)
        {
            expired.push(ExpiredRule {
                rule_type: ExpiredRuleType::Structure,
                index: i,
                pattern: rule.scope.clone(),
                expires: expires.clone(),
                reason: rule.reason.clone(),
            });
        }
    }

    expired
}

/// A parsed date with year, month, and day components.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ParsedDate {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

impl ParsedDate {
    /// Parse a date string in YYYY-MM-DD format.
    ///
    /// # Errors
    /// Returns an error message if the format is invalid.
    pub fn parse(date_str: &str) -> Result<Self, String> {
        let parts: Vec<&str> = date_str.split('-').collect();
        if parts.len() != 3 {
            return Err(format!(
                "Invalid date format: '{date_str}'. Expected YYYY-MM-DD"
            ));
        }

        let year: u16 = parts[0]
            .parse()
            .map_err(|_| format!("Invalid year in date: '{date_str}'. Expected YYYY-MM-DD"))?;

        let month: u8 = parts[1]
            .parse()
            .map_err(|_| format!("Invalid month in date: '{date_str}'. Expected YYYY-MM-DD"))?;

        let day: u8 = parts[2]
            .parse()
            .map_err(|_| format!("Invalid day in date: '{date_str}'. Expected YYYY-MM-DD"))?;

        // Basic validation
        if !(1..=12).contains(&month) {
            return Err(format!(
                "Invalid month {month} in date: '{date_str}'. Month must be 1-12"
            ));
        }

        if !(1..=31).contains(&day) {
            return Err(format!(
                "Invalid day {day} in date: '{date_str}'. Day must be 1-31"
            ));
        }

        Ok(Self { year, month, day })
    }

    /// Get today's date.
    #[must_use]
    pub fn today() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};

        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let secs = duration.as_secs();

        // Simple calculation: days since epoch
        let days = secs / 86400;

        // Convert days since 1970-01-01 to year/month/day
        // Using a simplified algorithm
        let (year, month, day) = days_to_ymd(days);

        // Values are guaranteed to fit: year <= 9999, month 1-12, day 1-31
        #[allow(clippy::cast_possible_truncation)]
        Self {
            year: year as u16,
            month: month as u8,
            day: day as u8,
        }
    }
}

/// Convert days since Unix epoch (1970-01-01) to (year, month, day).
const fn days_to_ymd(days: u64) -> (u32, u32, u32) {
    // Algorithm based on Howard Hinnant's date algorithms
    // http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    // Algorithm guarantees: y fits in u32 for dates until year ~5.8 million,
    // m is 1-12, d is 1-31
    #[allow(clippy::cast_possible_truncation)]
    (y as u32, m as u32, d as u32)
}

/// Check if a date string has expired relative to a given date.
fn is_expired_at(date_str: &str, today: ParsedDate) -> Result<bool, String> {
    let expires = ParsedDate::parse(date_str)?;
    Ok(expires < today)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_date() {
        let date = ParsedDate::parse("2025-12-31").unwrap();
        assert_eq!(date.year, 2025);
        assert_eq!(date.month, 12);
        assert_eq!(date.day, 31);
    }

    #[test]
    fn test_parse_invalid_format() {
        assert!(ParsedDate::parse("2025/12/31").is_err());
        assert!(ParsedDate::parse("2025-12").is_err());
        assert!(ParsedDate::parse("not-a-date").is_err());
    }

    #[test]
    fn test_parse_invalid_month() {
        assert!(ParsedDate::parse("2025-13-01").is_err());
        assert!(ParsedDate::parse("2025-00-01").is_err());
    }

    #[test]
    fn test_parse_invalid_day() {
        assert!(ParsedDate::parse("2025-01-32").is_err());
        assert!(ParsedDate::parse("2025-01-00").is_err());
    }

    #[test]
    fn test_date_comparison() {
        let earlier = ParsedDate::parse("2024-01-01").unwrap();
        let later = ParsedDate::parse("2025-12-31").unwrap();
        assert!(earlier < later);
    }

    #[test]
    fn test_is_expired_at_past_date() {
        let today = ParsedDate::parse("2025-06-15").unwrap();
        assert!(is_expired_at("2025-01-01", today).unwrap());
    }

    #[test]
    fn test_is_expired_at_future_date() {
        let today = ParsedDate::parse("2025-06-15").unwrap();
        assert!(!is_expired_at("2025-12-31", today).unwrap());
    }

    #[test]
    fn test_is_expired_at_same_date() {
        let today = ParsedDate::parse("2025-06-15").unwrap();
        assert!(!is_expired_at("2025-06-15", today).unwrap());
    }

    #[test]
    fn test_today_returns_valid_date() {
        let today = ParsedDate::today();
        assert!(today.year >= 2024);
        assert!((1..=12).contains(&today.month));
        assert!((1..=31).contains(&today.day));
    }

    #[test]
    fn test_collect_expired_rules_with_date() {
        use crate::config::{Config, ContentRule, StructureRule};

        let mut config = Config::default();
        config.content.rules = vec![
            ContentRule {
                pattern: "src/old/**".to_string(),
                max_lines: 500,
                expires: Some("2025-01-01".to_string()),
                reason: Some("Legacy code".to_string()),
                warn_threshold: None,
                warn_at: None,
                skip_comments: None,
                skip_blank: None,
            },
            ContentRule {
                pattern: "src/new/**".to_string(),
                max_lines: 500,
                expires: Some("2025-12-31".to_string()),
                reason: None,
                warn_threshold: None,
                warn_at: None,
                skip_comments: None,
                skip_blank: None,
            },
        ];
        config.structure.rules = vec![StructureRule {
            scope: "tests/".to_string(),
            expires: Some("2024-06-01".to_string()),
            max_files: None,
            max_dirs: None,
            max_depth: None,
            relative_depth: false,
            warn_threshold: None,
            warn_files_at: None,
            warn_dirs_at: None,
            warn_files_threshold: None,
            warn_dirs_threshold: None,
            allow_extensions: vec![],
            allow_patterns: vec![],
            allow_files: vec![],
            allow_dirs: vec![],
            deny_extensions: vec![],
            deny_patterns: vec![],
            deny_files: vec![],
            deny_dirs: vec![],
            file_naming_pattern: None,
            file_pattern: None,
            require_sibling: None,
            reason: None,
        }];

        let today = ParsedDate::parse("2025-06-15").unwrap();
        let expired = collect_expired_rules_with_date(&config, today);

        assert_eq!(expired.len(), 2);
        assert_eq!(expired[0].rule_type, ExpiredRuleType::Content);
        assert_eq!(expired[0].pattern, "src/old/**");
        assert_eq!(expired[0].reason, Some("Legacy code".to_string()));
        assert_eq!(expired[1].rule_type, ExpiredRuleType::Structure);
        assert_eq!(expired[1].pattern, "tests/");
    }
}
