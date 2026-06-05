// ATOM: relative-time formatter (e.g. "3h ago")
// SOURCE: https://docs.rs/chrono
#![allow(
    clippy::same_name_method,
    reason = "dioxus #[component] macro generates builder() that collides with typed-builder trait"
)]
use chrono::{DateTime, Utc};
use dioxus::prelude::*;

#[expect(
    clippy::integer_division,
    reason = "truncating integer division is intentional for time unit conversion"
)]
pub fn format_relative(now: DateTime<Utc>, then: DateTime<Utc>) -> String {
    let secs = now.signed_duration_since(then).num_seconds().max(0);
    if secs < 60 {
        return format!("{secs}s ago");
    }
    let mins = secs / 60;
    if mins < 60 {
        return format!("{mins}m ago");
    }
    let hrs = mins / 60;
    if hrs < 24 {
        return format!("{hrs}h ago");
    }
    let days = hrs / 24;
    if days < 30 {
        return format!("{days}d ago");
    }
    let months = days / 30;
    if months < 12 {
        return format!("{months}mo ago");
    }
    let years = months / 12;
    format!("{years}y ago")
}

#[component]
pub fn RelativeTime(timestamp: Option<chrono::DateTime<Utc>>) -> Element {
    let label = timestamp.map_or_else(|| String::from("—"), |t| format_relative(Utc::now(), t));
    rsx! { span { class: "rel-time", title: "{label}", "{label}" } }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn should_format_seconds_when_under_a_minute() {
        let now = Utc::now();
        let then = now - Duration::seconds(30);
        assert_eq!(format_relative(now, then), "30s ago");
    }

    #[test]
    fn should_format_hours_when_under_a_day() {
        let now = Utc::now();
        let then = now - Duration::hours(5);
        assert_eq!(format_relative(now, then), "5h ago");
    }

    #[test]
    fn should_format_days_when_over_a_day() {
        let now = Utc::now();
        let then = now - Duration::days(7);
        assert_eq!(format_relative(now, then), "7d ago");
    }

    #[test]
    fn should_clamp_negative_to_zero_when_clock_skew() {
        let now = Utc::now();
        let then = now + Duration::seconds(10);
        assert_eq!(format_relative(now, then), "0s ago");
    }
}
