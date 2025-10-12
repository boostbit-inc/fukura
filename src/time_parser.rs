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
        let hours = captures.get(1)
            .and_then(|m| m.as_str().parse::<u32>().ok())
            .unwrap_or(0);
        let minutes = captures.get(2)
            .and_then(|m| m.as_str().parse::<u32>().ok())
            .unwrap_or(0);
        let seconds = captures.get(3)
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
        let system_time = SystemTime::UNIX_EPOCH + 
            std::time::Duration::from_secs(target_time.timestamp() as u64);
        
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
    
    #[test]
    fn test_parse_minutes() {
        let result = parse_time_ago("5m ago");
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_parse_hours() {
        let result = parse_time_ago("2h ago");
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_parse_combined() {
        let result = parse_time_ago("1h 30m ago");
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_parse_without_ago() {
        let result = parse_time_ago("3m");
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_invalid_format() {
        let result = parse_time_ago("invalid");
        assert!(result.is_err());
    }
    
    #[test]
    fn test_empty_time() {
        let result = parse_time_ago("0m ago");
        assert!(result.is_err());
    }
}
