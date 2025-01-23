use std::time::Duration;

pub trait TimeFormat {
    fn format_time(ms: u64) -> String;
    fn format_duration(duration: Duration) -> String;
    fn parse_time_str(time_str: &str) -> Option<u64>;
}

pub struct TimeUtils;

impl TimeFormat for TimeUtils {
    /// Formats milliseconds into "MM:SS" format
    fn format_time(ms: u64) -> String {
        let total_seconds = ms / 1000;
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        format!("{:02}:{:02}", minutes, seconds)
    }

    /// Formats Duration into "MM:SS" format
    fn format_duration(duration: Duration) -> String {
        Self::format_time(duration.as_millis() as u64)
    }

    /// Parses a time string in "MM:SS" format into milliseconds
    fn parse_time_str(time_str: &str) -> Option<u64> {
        let parts: Vec<&str> = time_str.split(':').collect();
        if parts.len() != 2 {
            return None;
        }

        let minutes: u64 = parts[0].parse().ok()?;
        let seconds: u64 = parts[1].parse().ok()?;

        if seconds >= 60 {
            return None;
        }

        Some((minutes * 60 + seconds) * 1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_time() {
        assert_eq!(TimeUtils::format_time(0), "00:00");
        assert_eq!(TimeUtils::format_time(1000), "00:01");
        assert_eq!(TimeUtils::format_time(60000), "01:00");
        assert_eq!(TimeUtils::format_time(61000), "01:01");
        assert_eq!(TimeUtils::format_time(3599000), "59:59");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(TimeUtils::format_duration(Duration::from_secs(0)), "00:00");
        assert_eq!(TimeUtils::format_duration(Duration::from_secs(61)), "01:01");
        assert_eq!(TimeUtils::format_duration(Duration::from_secs(3599)), "59:59");
    }

    #[test]
    fn test_parse_time_str() {
        assert_eq!(TimeUtils::parse_time_str("00:00"), Some(0));
        assert_eq!(TimeUtils::parse_time_str("01:00"), Some(60000));
        assert_eq!(TimeUtils::parse_time_str("01:30"), Some(90000));
        assert_eq!(TimeUtils::parse_time_str("invalid"), None);
        assert_eq!(TimeUtils::parse_time_str("01:60"), None);
        assert_eq!(TimeUtils::parse_time_str("01:01:01"), None);
    }
}