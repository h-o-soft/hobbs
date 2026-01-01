//! Date/time utilities for HOBBS.

use chrono::{DateTime, NaiveDateTime, Utc};
use chrono_tz::Tz;

/// Format a datetime string (stored as UTC) to the specified timezone.
///
/// # Arguments
///
/// * `datetime_str` - DateTime string in RFC3339 or SQLite format
/// * `timezone` - Timezone name (e.g., "Asia/Tokyo", "UTC")
/// * `format` - Output format string (e.g., "%Y/%m/%d %H:%M")
///
/// # Returns
///
/// Formatted datetime string, or the original string if parsing fails.
pub fn format_datetime(datetime_str: &str, timezone: &str, format: &str) -> String {
    // Parse timezone
    let tz: Tz = match timezone.parse() {
        Ok(tz) => tz,
        Err(_) => return datetime_str.to_string(),
    };

    // Try to parse as RFC3339 first
    if let Ok(dt) = DateTime::parse_from_rfc3339(datetime_str) {
        let utc_dt = dt.with_timezone(&Utc);
        let local_dt = utc_dt.with_timezone(&tz);
        return local_dt.format(format).to_string();
    }

    // Try SQLite datetime format (YYYY-MM-DD HH:MM:SS)
    if let Ok(naive) = NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%d %H:%M:%S") {
        let utc_dt = naive.and_utc();
        let local_dt = utc_dt.with_timezone(&tz);
        return local_dt.format(format).to_string();
    }

    // Return original if parsing fails
    datetime_str.to_string()
}

/// Format a datetime string with default format.
pub fn format_datetime_default(datetime_str: &str, timezone: &str) -> String {
    format_datetime(datetime_str, timezone, "%Y/%m/%d %H:%M")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_datetime_rfc3339() {
        // UTC time
        let dt = "2024-01-15T10:30:00+00:00";
        let result = format_datetime(dt, "Asia/Tokyo", "%Y/%m/%d %H:%M");
        assert_eq!(result, "2024/01/15 19:30"); // UTC+9
    }

    #[test]
    fn test_format_datetime_sqlite() {
        // SQLite format (assumed UTC)
        let dt = "2024-01-15 10:30:00";
        let result = format_datetime(dt, "Asia/Tokyo", "%Y/%m/%d %H:%M");
        assert_eq!(result, "2024/01/15 19:30"); // UTC+9
    }

    #[test]
    fn test_format_datetime_utc() {
        let dt = "2024-01-15 10:30:00";
        let result = format_datetime(dt, "UTC", "%Y/%m/%d %H:%M");
        assert_eq!(result, "2024/01/15 10:30");
    }

    #[test]
    fn test_format_datetime_invalid_timezone() {
        let dt = "2024-01-15 10:30:00";
        let result = format_datetime(dt, "Invalid/Zone", "%Y/%m/%d %H:%M");
        assert_eq!(result, dt); // Returns original
    }

    #[test]
    fn test_format_datetime_invalid_datetime() {
        let dt = "not a date";
        let result = format_datetime(dt, "Asia/Tokyo", "%Y/%m/%d %H:%M");
        assert_eq!(result, dt); // Returns original
    }

    #[test]
    fn test_format_datetime_default() {
        let dt = "2024-01-15 10:30:00";
        let result = format_datetime_default(dt, "Asia/Tokyo");
        assert_eq!(result, "2024/01/15 19:30");
    }
}
