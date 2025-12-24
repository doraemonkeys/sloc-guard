//! Duration parsing for `--since` flag.
//!
//! Supports human-readable duration strings: `7d`, `30d`, `1w`, `12h`, `5m`, `300s`.

use crate::{Result, SlocGuardError};

/// Seconds per time unit.
const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = 3600;
const SECONDS_PER_DAY: u64 = 86400;
const SECONDS_PER_WEEK: u64 = 604_800;

/// Parse a duration string into seconds.
///
/// Supported formats:
/// - `30s` - 30 seconds
/// - `5m` - 5 minutes
/// - `12h` - 12 hours
/// - `7d` - 7 days
/// - `1w` - 1 week
///
/// # Errors
/// Returns an error if the format is invalid.
///
/// # Examples
/// ```ignore
/// assert_eq!(parse_duration("7d").unwrap(), 7 * 86400);
/// assert_eq!(parse_duration("1w").unwrap(), 604800);
/// assert_eq!(parse_duration("12h").unwrap(), 12 * 3600);
/// ```
pub fn parse_duration(input: &str) -> Result<u64> {
    let input = input.trim();
    if input.is_empty() {
        return Err(SlocGuardError::Config(
            "Duration cannot be empty. Expected format: <number><unit> (e.g., 7d, 1w, 12h)"
                .to_string(),
        ));
    }

    // Find where the numeric part ends and the unit begins
    let unit_start = input
        .find(|c: char| !c.is_ascii_digit())
        .ok_or_else(|| {
            SlocGuardError::Config(format!(
                "Invalid duration format: '{input}'. Missing unit. Expected format: <number><unit> (e.g., 7d, 1w, 12h)"
            ))
        })?;

    if unit_start == 0 {
        return Err(SlocGuardError::Config(format!(
            "Invalid duration format: '{input}'. Missing number. Expected format: <number><unit> (e.g., 7d, 1w, 12h)"
        )));
    }

    let (num_str, unit) = input.split_at(unit_start);

    let value: u64 = num_str.parse().map_err(|_| {
        SlocGuardError::Config(format!(
            "Invalid duration number: '{num_str}'. Expected a positive integer"
        ))
    })?;

    if value == 0 {
        return Err(SlocGuardError::Config(
            "Duration must be greater than zero".to_string(),
        ));
    }

    let multiplier = match unit.to_lowercase().as_str() {
        "s" | "sec" | "secs" | "second" | "seconds" => 1,
        "m" | "min" | "mins" | "minute" | "minutes" => SECONDS_PER_MINUTE,
        "h" | "hr" | "hrs" | "hour" | "hours" => SECONDS_PER_HOUR,
        "d" | "day" | "days" => SECONDS_PER_DAY,
        "w" | "wk" | "wks" | "week" | "weeks" => SECONDS_PER_WEEK,
        _ => {
            return Err(SlocGuardError::Config(format!(
                "Invalid duration unit: '{unit}'. Supported units: s, m, h, d, w"
            )));
        }
    };

    Ok(value * multiplier)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_seconds() {
        assert_eq!(parse_duration("30s").unwrap(), 30);
        assert_eq!(parse_duration("1sec").unwrap(), 1);
        assert_eq!(parse_duration("60seconds").unwrap(), 60);
    }

    #[test]
    fn test_parse_minutes() {
        assert_eq!(parse_duration("5m").unwrap(), 5 * 60);
        assert_eq!(parse_duration("1min").unwrap(), 60);
        assert_eq!(parse_duration("30minutes").unwrap(), 30 * 60);
    }

    #[test]
    fn test_parse_hours() {
        assert_eq!(parse_duration("12h").unwrap(), 12 * 3600);
        assert_eq!(parse_duration("1hr").unwrap(), 3600);
        assert_eq!(parse_duration("24hours").unwrap(), 24 * 3600);
    }

    #[test]
    fn test_parse_days() {
        assert_eq!(parse_duration("7d").unwrap(), 7 * 86400);
        assert_eq!(parse_duration("30days").unwrap(), 30 * 86400);
        assert_eq!(parse_duration("1day").unwrap(), 86400);
    }

    #[test]
    fn test_parse_weeks() {
        assert_eq!(parse_duration("1w").unwrap(), 604_800);
        assert_eq!(parse_duration("2weeks").unwrap(), 2 * 604_800);
        assert_eq!(parse_duration("4wk").unwrap(), 4 * 604_800);
    }

    #[test]
    fn test_case_insensitive() {
        assert_eq!(parse_duration("7D").unwrap(), 7 * 86400);
        assert_eq!(parse_duration("1W").unwrap(), 604_800);
        assert_eq!(parse_duration("12H").unwrap(), 12 * 3600);
    }

    #[test]
    fn test_whitespace_trimmed() {
        assert_eq!(parse_duration("  7d  ").unwrap(), 7 * 86400);
    }

    #[test]
    fn test_empty_string_error() {
        let err = parse_duration("").unwrap_err();
        assert!(err.message().contains("cannot be empty"));
    }

    #[test]
    fn test_missing_unit_error() {
        let err = parse_duration("30").unwrap_err();
        assert!(err.message().contains("Missing unit"));
    }

    #[test]
    fn test_missing_number_error() {
        let err = parse_duration("d").unwrap_err();
        assert!(err.message().contains("Missing number"));
    }

    #[test]
    fn test_invalid_unit_error() {
        let err = parse_duration("30x").unwrap_err();
        assert!(err.message().contains("Invalid duration unit"));
    }

    #[test]
    fn test_zero_duration_error() {
        let err = parse_duration("0d").unwrap_err();
        assert!(err.message().contains("greater than zero"));
    }

    #[test]
    fn test_invalid_number_error() {
        let err = parse_duration("abc").unwrap_err();
        assert!(err.message().contains("Missing number"));
    }
}
