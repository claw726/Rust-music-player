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
    use std::u64;

    use super::*;

    #[test]
    fn test_time_format_edge_cases() {
        assert_eq!(TimeUtils::format_time(0), "00:00");

        // Calcualte the expected string for MAX_FLT milliseconds
        let max_ms = u64::MAX;
        let total_seconds = max_ms / 1000;
        let minutes= total_seconds / 60;
        let seconds = total_seconds % 60;
        let expected = format!("{}:{:02}", minutes, seconds);
        assert_eq!(TimeUtils::format_time(u64::MAX), expected);
    }

    #[test]
    fn test_duration_format() {
        let cases = vec![
            (Duration::from_secs(0), "00:00"),
            (Duration::from_secs(61), "01:01"),
            (Duration::from_secs(3600), "60:00"),
        ];

        for (duration, expected) in cases {
            assert_eq!(TimeUtils::format_duration(duration), expected);
        }
    }

    #[test]
    fn test_time_str_parsing() {
        let cases = vec![
            ("00:00", Some(0)),
            ("01:30", Some(90000)),
            ("59:59", Some(3599000)),
            ("60:00", Some(3600000)),
            ("invalid", None),
            ("99:99", None),
        ];

        for (input, expected) in cases {
            assert_eq!(
                TimeUtils::parse_time_str(input), 
                expected,
                "Failed for input: {}", 
                input
            );
        }
    }
}