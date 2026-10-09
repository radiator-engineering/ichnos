//! Timestamps as pm4py's OCEL readers parse them: Python's
//! `datetime.fromisoformat`, then converted to UTC, with a timestamp without
//! an offset taken as UTC.

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, Utc};

const NAIVE: &[&str] = &[
    "%Y-%m-%dT%H:%M:%S%.f",
    "%Y-%m-%d %H:%M:%S%.f",
    "%Y-%m-%dT%H:%M",
    "%Y-%m-%d %H:%M",
];

const AWARE: &[&str] = &[
    "%Y-%m-%dT%H:%M:%S%.f%:z",
    "%Y-%m-%d %H:%M:%S%.f%:z",
    "%Y-%m-%dT%H:%M:%S%.f%z",
    "%Y-%m-%d %H:%M:%S%.f%z",
    "%Y-%m-%dT%H:%M%:z",
    "%Y-%m-%d %H:%M%:z",
];

/// Parses an ISO 8601 timestamp and converts it to UTC.
pub(crate) fn parse(text: &str) -> Option<DateTime<FixedOffset>> {
    let text = text.trim();
    let text = match text.strip_suffix('Z').or_else(|| text.strip_suffix('z')) {
        Some(rest) => format!("{rest}+00:00"),
        None => text.to_owned(),
    };
    let utc = |d: DateTime<Utc>| d.fixed_offset();
    for f in AWARE {
        if let Ok(d) = DateTime::parse_from_str(&text, f) {
            return Some(utc(d.to_utc()));
        }
    }
    for f in NAIVE {
        if let Ok(d) = NaiveDateTime::parse_from_str(&text, f) {
            return Some(utc(d.and_utc()));
        }
    }
    NaiveDate::parse_from_str(&text, "%Y-%m-%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|d| utc(d.and_utc()))
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn forms() {
        let want = "2022-01-09T14:00:00+00:00";
        for text in [
            "2022-01-09T14:00:00",
            "2022-01-09T14:00:00Z",
            "2022-01-09T14:00:00.000Z",
            "2022-01-09 14:00:00",
            "2022-01-09T15:00:00+01:00",
            "2022-01-09T14:00",
        ] {
            assert_eq!(parse(text).unwrap().to_rfc3339(), want, "{text}");
        }
        assert_eq!(
            parse("2022-01-09").unwrap().to_rfc3339(),
            "2022-01-09T00:00:00+00:00"
        );
        assert!(parse("yesterday").is_none());
    }
}
