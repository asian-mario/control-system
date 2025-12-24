use chrono::{DateTime, Local, Utc};

/// Format a timestamp as relative time (e.g., "5 minutes ago")
pub fn format_relative(timestamp: DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(timestamp);

    if duration.num_seconds() < 0 {
        return "in the future".to_string();
    }

    let seconds = duration.num_seconds();
    let minutes = duration.num_minutes();
    let hours = duration.num_hours();
    let days = duration.num_days();
    let weeks = days / 7;
    let months = days / 30;
    let years = days / 365;

    if seconds < 60 {
        if seconds <= 1 {
            "just now".to_string()
        } else {
            format!("{}s ago", seconds)
        }
    } else if minutes < 60 {
        if minutes == 1 {
            "1 min ago".to_string()
        } else {
            format!("{} mins ago", minutes)
        }
    } else if hours < 24 {
        if hours == 1 {
            "1 hour ago".to_string()
        } else {
            format!("{} hours ago", hours)
        }
    } else if days < 7 {
        if days == 1 {
            "yesterday".to_string()
        } else {
            format!("{} days ago", days)
        }
    } else if weeks < 4 {
        if weeks == 1 {
            "1 week ago".to_string()
        } else {
            format!("{} weeks ago", weeks)
        }
    } else if months < 12 {
        if months == 1 {
            "1 month ago".to_string()
        } else {
            format!("{} months ago", months)
        }
    } else if years == 1 {
        "1 year ago".to_string()
    } else {
        format!("{} years ago", years)
    }
}

/// Get current local time formatted for display
pub fn format_current_time() -> String {
    Local::now().format("%H:%M:%S").to_string()
}

/// Get current local date formatted for display
pub fn format_current_date() -> String {
    Local::now().format("%A, %B %d, %Y").to_string()
}

/// Get a compact date/time string
pub fn format_datetime_compact(timestamp: DateTime<Utc>) -> String {
    timestamp
        .with_timezone(&Local)
        .format("%m/%d %H:%M")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_format_relative() {
        let now = Utc::now();
        
        assert_eq!(format_relative(now), "just now");
        assert_eq!(format_relative(now - Duration::seconds(30)), "30s ago");
        assert_eq!(format_relative(now - Duration::minutes(5)), "5 mins ago");
        assert_eq!(format_relative(now - Duration::hours(2)), "2 hours ago");
        assert_eq!(format_relative(now - Duration::days(1)), "yesterday");
        assert_eq!(format_relative(now - Duration::days(3)), "3 days ago");
    }
}
