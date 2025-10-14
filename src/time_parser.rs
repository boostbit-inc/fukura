use anyhow::{bail, Result};
use chrono::{Duration, Utc};
use regex::Regex;
use std::time::SystemTime;

/// Parse time expressions like "3m ago", "2h ago", "1h 30m ago"
pub fn parse_time_ago(input: &str) -> Result<SystemTime> {
    let input = input.trim().to_lowercase();

    // Remove "ago" suffix if present
    let time_part = input.strip_suffix(" ago").unwrap_or(&input);

    // Regex patterns for different time formats
    let re = Regex::new(r"^(?:(\d+)h)?\s*(?:(\d+)m)?\s*(?:(\d+)s)?$")?;

    if let Some(captures) = re.captures(time_part) {
        let hours = captures
            .get(1)
            .and_then(|m| m.as_str().parse::<u32>().ok())
            .unwrap_or(0);
        let minutes = captures
            .get(2)
            .and_then(|m| m.as_str().parse::<u32>().ok())
            .unwrap_or(0);
        let seconds = captures
            .get(3)
            .and_then(|m| m.as_str().parse::<u32>().ok())
            .unwrap_or(0);

        if hours == 0 && minutes == 0 && seconds == 0 {
            bail!("Invalid time format. Use formats like '3m', '2h', '1h 30m', etc.");
        }

        let total_duration = Duration::hours(hours as i64)
            + Duration::minutes(minutes as i64)
            + Duration::seconds(seconds as i64);

        let target_time = Utc::now() - total_duration;

        // Convert to SystemTime
        let system_time =
            SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(target_time.timestamp() as u64);

        Ok(system_time)
    } else {
        bail!("Invalid time format. Use formats like '3m ago', '2h ago', '1h 30m ago', etc.");
    }
}

/// Validate time against configuration limits
pub fn validate_time_ago(
    target_time: SystemTime,
    max_lookback_hours: u32,
    min_lookback_minutes: u32,
) -> Result<()> {
    let now = SystemTime::now();
    let duration = now.duration_since(target_time)?;

    let max_duration = std::time::Duration::from_secs(max_lookback_hours as u64 * 3600);
    let min_duration = std::time::Duration::from_secs(min_lookback_minutes as u64 * 60);

    if duration > max_duration {
        bail!(
            "Time too far back. Maximum allowed is {} hours ago.",
            max_lookback_hours
        );
    }

    if duration < min_duration {
        bail!(
            "Time too recent. Minimum allowed is {} minutes ago.",
            min_lookback_minutes
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration as StdDuration;

    #[test]
    fn test_parse_minutes_only() {
        let result = parse_time_ago("5m ago");
        assert!(result.is_ok());

        let parsed_time = result.unwrap();
        let now = SystemTime::now();
        let duration = now.duration_since(parsed_time).unwrap();

        // Should be approximately 5 minutes (300 seconds), allow 1 second tolerance
        assert!((duration.as_secs() as i64 - 300).abs() <= 1);
    }

    #[test]
    fn test_parse_hours_only() {
        let result = parse_time_ago("2h ago");
        assert!(result.is_ok());

        let parsed_time = result.unwrap();
        let now = SystemTime::now();
        let duration = now.duration_since(parsed_time).unwrap();

        // Should be approximately 2 hours (7200 seconds), allow 1 second tolerance
        assert!((duration.as_secs() as i64 - 7200).abs() <= 1);
    }

    #[test]
    fn test_parse_combined_hours_minutes() {
        let result = parse_time_ago("1h 30m ago");
        assert!(result.is_ok());

        let parsed_time = result.unwrap();
        let now = SystemTime::now();
        let duration = now.duration_since(parsed_time).unwrap();

        // Should be approximately 1.5 hours (5400 seconds), allow 1 second tolerance
        assert!((duration.as_secs() as i64 - 5400).abs() <= 1);
    }

    #[test]
    fn test_parse_with_seconds() {
        let result = parse_time_ago("1h 30m 45s ago");
        assert!(result.is_ok());

        let parsed_time = result.unwrap();
        let now = SystemTime::now();
        let duration = now.duration_since(parsed_time).unwrap();

        // Should be approximately 5445 seconds, allow 1 second tolerance
        assert!((duration.as_secs() as i64 - 5445).abs() <= 1);
    }

    #[test]
    fn test_parse_without_ago_suffix() {
        let result = parse_time_ago("3m");
        assert!(result.is_ok());

        let parsed_time = result.unwrap();
        let now = SystemTime::now();
        let duration = now.duration_since(parsed_time).unwrap();

        // Should be approximately 3 minutes (180 seconds), allow 1 second tolerance
        assert!((duration.as_secs() as i64 - 180).abs() <= 1);
    }

    #[test]
    fn test_parse_case_insensitive() {
        let test_cases = vec!["3M AGO", "2H AGO", "1h 30M ago", "45m AGO"];

        for case in test_cases {
            let result = parse_time_ago(case);
            assert!(result.is_ok(), "Failed to parse: {}", case);
        }
    }

    #[test]
    fn test_parse_with_extra_whitespace() {
        let result = parse_time_ago("  1h   30m  ago  ");
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_formats() {
        let invalid_cases = vec![
            "invalid",
            "3x ago",
            "ago",
            "",
            "3m 4x ago",
            "3.5h ago", // Decimal not supported
            "-1m ago",  // Negative not supported
        ];

        for case in invalid_cases {
            let result = parse_time_ago(case);
            assert!(result.is_err(), "Should have failed to parse: {}", case);
        }
    }

    #[test]
    fn test_empty_time_components() {
        let result = parse_time_ago("0m ago");
        assert!(result.is_err(), "Should reject zero time");

        let result = parse_time_ago("0h 0m 0s ago");
        assert!(result.is_err(), "Should reject all zero components");
    }

    #[test]
    fn test_validate_time_ago_within_limits() {
        let now = SystemTime::now();
        let two_hours_ago = now - StdDuration::from_secs(2 * 3600);

        // Should pass: 2 hours ago within 3 hour limit, above 1 minute minimum
        let result = validate_time_ago(two_hours_ago, 3, 1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_time_ago_too_far_back() {
        let now = SystemTime::now();
        let four_hours_ago = now - StdDuration::from_secs(4 * 3600);

        // Should fail: 4 hours ago exceeds 3 hour limit
        let result = validate_time_ago(four_hours_ago, 3, 1);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Maximum allowed is 3 hours"));
    }

    #[test]
    fn test_validate_time_ago_too_recent() {
        let now = SystemTime::now();
        let thirty_seconds_ago = now - StdDuration::from_secs(30);

        // Should fail: 30 seconds ago is less than 1 minute minimum
        let result = validate_time_ago(thirty_seconds_ago, 3, 1);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Minimum allowed is 1 minutes"));
    }

    #[test]
    fn test_validate_boundary_cases() {
        let now = SystemTime::now();

        // Test slightly within boundaries to account for test execution time
        let almost_three_hours_ago = now - StdDuration::from_secs(3 * 3600 - 5);
        let over_one_minute_ago = now - StdDuration::from_secs(65);

        let result1 = validate_time_ago(almost_three_hours_ago, 3, 1);
        let result2 = validate_time_ago(over_one_minute_ago, 3, 1);

        // Both should pass (within boundaries)
        assert!(result1.is_ok());
        assert!(result2.is_ok());

        // Test just outside boundaries
        let just_over_three_hours_ago = now - StdDuration::from_secs(3 * 3600 + 5);
        let just_under_one_minute_ago = now - StdDuration::from_secs(55);

        let result3 = validate_time_ago(just_over_three_hours_ago, 3, 1);
        let result4 = validate_time_ago(just_under_one_minute_ago, 3, 1);

        // Both should fail (outside boundaries)
        assert!(result3.is_err());
        assert!(result4.is_err());
    }

    #[test]
    fn test_large_time_values() {
        // Test parsing large but reasonable values
        let result = parse_time_ago("23h 59m ago");
        assert!(result.is_ok());

        let parsed_time = result.unwrap();
        let now = SystemTime::now();
        let duration = now.duration_since(parsed_time).unwrap();

        // Should be approximately 86340 seconds (23h 59m)
        let expected = 23 * 3600 + 59 * 60;
        assert!((duration.as_secs() as i64 - expected).abs() <= 1);
    }

    #[test]
    fn test_edge_case_formats() {
        // Test edge cases that should work
        let valid_edge_cases = vec![
            "1s ago",       // Just seconds
            "59m ago",      // Large minutes
            "23h ago",      // Large hours
            "1h 1m 1s ago", // All components with 1
        ];

        for case in valid_edge_cases {
            let result = parse_time_ago(case);
            assert!(result.is_ok(), "Should parse: {}", case);
        }
    }

    #[test]
    fn test_configuration_validation_custom_limits() {
        let now = SystemTime::now();

        // Test with custom limits: 6 hours max, 5 minutes min
        let five_hours_ago = now - StdDuration::from_secs(5 * 3600);
        let _three_minutes_ago = now - StdDuration::from_secs(3 * 60);
        let seven_hours_ago = now - StdDuration::from_secs(7 * 3600);
        let one_minute_ago = now - StdDuration::from_secs(60);

        // Should pass
        assert!(validate_time_ago(five_hours_ago, 6, 5).is_ok());

        // Should fail: too far
        assert!(validate_time_ago(seven_hours_ago, 6, 5).is_err());

        // Should fail: too recent
        assert!(validate_time_ago(one_minute_ago, 6, 5).is_err());

        // Should pass: exactly at minimum
        let five_minutes_ago = now - StdDuration::from_secs(5 * 60);
        assert!(validate_time_ago(five_minutes_ago, 6, 5).is_ok());
    }
}
