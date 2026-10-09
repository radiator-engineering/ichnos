//! Timestamps as pm4py's OCEL readers parse them (`strpfromiso.apply`): a
//! final `Z` becomes `+00:00`, Python 3.12's `datetime.fromisoformat` parses
//! the text, and the result is converted to UTC, with a timestamp without an
//! offset taken as UTC.
//!
//! This follows CPython's C implementation of `fromisoformat`, including
//! ISO week dates, the basic forms without separators, any one character
//! between date and time, `,` before the fraction, and offsets of the form
//! `+HH`, `+HHMM` or `+HH:MM[:SS[.ffffff]]`. Like Python, it keeps
//! microseconds and drops finer digits.

use chrono::{DateTime, Datelike, Duration, FixedOffset, NaiveDate, NaiveTime, Weekday};

/// Parses a timestamp and converts it to UTC.
pub(crate) fn parse(text: &str) -> Option<DateTime<FixedOffset>> {
    let fixed;
    let text = match text.strip_suffix('Z') {
        Some(rest) => {
            fixed = format!("{rest}+00:00");
            &fixed
        }
        None => text,
    };
    let (local, offset) = fromisoformat(text)?;
    let utc = local.checked_sub_signed(offset.unwrap_or_default())?;
    Some(utc.and_utc().fixed_offset())
}

/// Python's `datetime.fromisoformat`: the local time and, when given, the
/// UTC offset.
fn fromisoformat(text: &str) -> Option<(chrono::NaiveDateTime, Option<Duration>)> {
    if text.chars().count() < 7 {
        return None;
    }
    let s = text.as_bytes();
    let sep = separator(s)?;
    let date = date(s.get(..sep)?)?;
    let (time, offset) = match text.get(sep..).and_then(|t| t.chars().next()) {
        // The separator may be any one character.
        Some(c) => time(&s[sep + c.len_utf8()..])?,
        None if sep == s.len() => (NaiveTime::MIN, None),
        None => return None,
    };
    Some((date.and_time(time), offset))
}

/// Where the date ends (CPython's `_find_isoformat_datetime_separator`).
fn separator(s: &[u8]) -> Option<usize> {
    let len = s.len();
    if len == 7 {
        return Some(7);
    }
    if s[4] == b'-' {
        if s[5] != b'W' {
            return Some(10);
        }
        if len > 8 && s[8] == b'-' {
            if len == 9 {
                return None;
            }
            if len > 10 && s[10].is_ascii_digit() {
                return Some(8);
            }
            return Some(10);
        }
        return Some(8);
    }
    if s[4] == b'W' {
        let digits = s[7..].iter().take_while(|c| c.is_ascii_digit()).count();
        let idx = 7 + digits;
        if idx < 9 {
            return Some(idx);
        }
        return Some(if idx % 2 == 0 { 7 } else { 8 });
    }
    Some(8)
}

/// Reads `n` ASCII digits at `*pos`.
fn digits(s: &[u8], pos: &mut usize, n: usize) -> Option<u32> {
    let part = s.get(*pos..*pos + n)?;
    if !part.iter().all(u8::is_ascii_digit) {
        return None;
    }
    *pos += n;
    Some(part.iter().fold(0, |v, c| v * 10 + u32::from(c - b'0')))
}

/// `YYYY-MM-DD`, `YYYYMMDD`, `YYYY-Www[-D]` or `YYYYWww[D]`.
fn date(s: &[u8]) -> Option<NaiveDate> {
    let mut pos = 0;
    let year = digits(s, &mut pos, 4)?;
    if year == 0 {
        return None;
    }
    let dash = s.get(pos) == Some(&b'-');
    pos += usize::from(dash);
    if s.get(pos) == Some(&b'W') {
        pos += 1;
        let week = digits(s, &mut pos, 2)?;
        let day = if pos < s.len() {
            if dash && s.get(pos) != Some(&b'-') {
                return None;
            }
            pos += usize::from(dash);
            digits(s, &mut pos, 1)?
        } else {
            1
        };
        let day = match day {
            1 => Weekday::Mon,
            2 => Weekday::Tue,
            3 => Weekday::Wed,
            4 => Weekday::Thu,
            5 => Weekday::Fri,
            6 => Weekday::Sat,
            7 => Weekday::Sun,
            _ => return None,
        };
        let date = NaiveDate::from_isoywd_opt(year as i32, week, day)?;
        return (1..=9999).contains(&date.year()).then_some(date);
    }
    let month = digits(s, &mut pos, 2)?;
    if dash && s.get(pos) != Some(&b'-') {
        return None;
    }
    pos += usize::from(dash);
    let day = digits(s, &mut pos, 2)?;
    NaiveDate::from_ymd_opt(year as i32, month, day)
}

/// The time and offset after the separator (CPython's
/// `parse_isoformat_time`).
fn time(s: &[u8]) -> Option<(NaiveTime, Option<Duration>)> {
    let tz = s
        .iter()
        .position(|c| matches!(c, b'Z' | b'+' | b'-'))
        .unwrap_or(s.len());
    let (h, m, sec, us, more) = hh_mm_ss_ff(s, 0, tz)?;
    let time = NaiveTime::from_hms_micro_opt(h, m, sec, us)?;
    if tz == s.len() {
        return (!more).then_some((time, None));
    }
    if s[tz] == b'Z' {
        return (tz + 1 == s.len()).then_some((time, Some(Duration::zero())));
    }
    let (oh, om, os, ous, more) = hh_mm_ss_ff(s, tz + 1, s.len())?;
    if more {
        return None;
    }
    let offset = Duration::hours(oh.into())
        + Duration::minutes(om.into())
        + Duration::seconds(os.into())
        + Duration::microseconds(ous.into());
    if offset >= Duration::days(1) {
        return None;
    }
    let offset = if s[tz] == b'-' { -offset } else { offset };
    Some((time, Some(offset)))
}

/// `HH[:?MM[:?SS[{.,}ffffff]]]` between `start` and `end` (CPython's
/// `parse_hh_mm_ss_ff`). The last value says whether text other than the
/// end of the string follows.
fn hh_mm_ss_ff(s: &[u8], start: usize, end: usize) -> Option<(u32, u32, u32, u32, bool)> {
    // C reads a NUL past the end of the string.
    let at = |i: usize| s.get(i).copied().unwrap_or(0);
    let mut values = [0u32; 3];
    let mut p = start;
    let mut colon = true;
    for (i, value) in values.iter_mut().enumerate() {
        *value = digits(s, &mut p, 2)?;
        let c = at(p);
        p += 1;
        if i == 0 {
            colon = c == b':';
        }
        if p >= end {
            return Some((values[0], values[1], values[2], 0, c != 0));
        } else if colon && c == b':' {
            continue;
        } else if c == b'.' || c == b',' {
            break;
        } else if !colon {
            p -= 1;
        } else {
            return None;
        }
    }
    let remains = end - p;
    let n = remains.min(6);
    let mut us = digits(s, &mut p, n)?;
    us *= 10u32.pow((6 - n) as u32);
    while at(p).is_ascii_digit() {
        p += 1;
    }
    Some((values[0], values[1], values[2], us, at(p) != 0))
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use ichnos_golden::golden;

    use super::parse;

    /// pm4py's `strpfromiso.apply` on each string, as recorded in the golden.
    #[test]
    fn follows_pm4py() {
        let g = golden("ocel", "timestamps");
        let rows = g.expected["timestamps"].as_array().unwrap();
        assert!(rows.len() > 40);
        for row in rows {
            let text = row[0].as_str().unwrap();
            let want = row[1]
                .as_str()
                .map(|t| DateTime::parse_from_rfc3339(t).unwrap());
            assert_eq!(parse(text), want, "{text:?}");
        }
    }
}
