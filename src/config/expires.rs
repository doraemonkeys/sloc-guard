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
#[path = "expires_tests.rs"]
mod tests;
