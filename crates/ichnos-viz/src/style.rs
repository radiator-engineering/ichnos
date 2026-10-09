//! Number formats, colours and pen widths from pm4py's `util/vis_utils.py`,
//! with Python's arithmetic and string forms.

use ichnos_discovery::dfg::BusinessHours;

const SECONDS_PER_DAY: f64 = 86400.0;

/// Python's `repr` of a float: the shortest form that reads back the same,
/// always with a `.` or an exponent.
pub(crate) fn py_float(x: f64) -> String {
    if x.is_nan() {
        return "nan".to_owned();
    }
    if x.is_infinite() {
        return if x > 0.0 { "inf" } else { "-inf" }.to_owned();
    }
    let a = x.abs();
    if a != 0.0 && !(1e-4..1e16).contains(&a) {
        let s = format!("{x:e}");
        let (mantissa, exp) = s.split_once('e').expect("`{:e}` has an exponent");
        let exp: i32 = exp.parse().expect("`{:e}` has an integer exponent");
        let sign = if exp < 0 { '-' } else { '+' };
        return format!("{mantissa}e{sign}{:02}", exp.abs());
    }
    let s = x.to_string();
    if s.contains('.') { s } else { s + ".0" }
}

/// Python's `a // b` on floats.
fn py_floordiv(a: f64, b: f64) -> f64 {
    let m = a % b;
    let mut div = (a - m) / b;
    if m != 0.0 && ((b < 0.0) != (m < 0.0)) {
        div -= 1.0;
    }
    if div == 0.0 {
        return 0.0_f64.copysign(a / b);
    }
    let floor = div.floor();
    if div - floor > 0.5 {
        floor + 1.0
    } else {
        floor
    }
}

/// Python's `int(x)` on a float: truncation towards zero.
fn py_int(x: f64) -> i64 {
    x.trunc() as i64
}

/// pm4py's `human_readable_stat`: a duration in seconds as its largest
/// whole unit, for example `3D` or `25m`.
///
/// A day has `day_seconds` seconds, a month 30 days and a year 360. Below a
/// second the value is in `ms`, then `ns`.
pub(crate) fn human_readable_stat(seconds: f64, day_seconds: f64) -> String {
    let c = py_int(seconds);
    let cf = c as f64;
    let years = py_int(py_floordiv(cf, day_seconds * 360.0));
    let months = py_int(py_floordiv(cf, day_seconds * 30.0));
    let days = py_int(py_floordiv(cf, day_seconds));
    let hours = c.div_euclid(3600);
    let minutes = c.div_euclid(60).rem_euclid(60);
    let secs = c.rem_euclid(60);
    if years > 0 {
        format!("{years}Y")
    } else if months > 0 {
        format!("{months}MO")
    } else if days > 0 {
        format!("{days}D")
    } else if hours > 0 {
        format!("{hours}h")
    } else if minutes > 0 {
        format!("{minutes}m")
    } else if secs > 0 {
        format!("{secs}s")
    } else {
        let ms = py_int(seconds * 1000.0);
        if ms > 0 {
            format!("{ms}ms")
        } else {
            format!("{}ns", py_int(seconds * 1e9))
        }
    }
}

/// pm4py's `get_business_day_seconds`: the mean length of a working day,
/// over the weekdays that have working time. Overlapping slots are joined
/// first. A schedule without working time gives a calendar day.
pub(crate) fn business_day_seconds(hours: &BusinessHours) -> f64 {
    let mut slots = hours.slots.clone();
    slots.sort_unstable();
    let mut unified: Vec<(u32, u32)> = Vec::new();
    for (begin, end) in slots {
        match unified.last_mut() {
            Some(last) if i64::from(last.1) >= i64::from(begin) - 1 => {
                last.1 = last.1.max(end);
            }
            _ => unified.push((begin, end)),
        }
    }
    let day = 86400_u64;
    let mut daily = [0_u64; 7];
    let mut working = [false; 7];
    for (start, end) in unified {
        for (weekday, total) in daily.iter_mut().enumerate() {
            let day_start = weekday as u64 * day;
            let from = u64::from(start).max(day_start);
            let to = u64::from(end).min(day_start + day);
            if to > from {
                *total += to - from;
                working[weekday] = true;
            }
        }
    }
    let totals: Vec<u64> = (0..7).filter(|&d| working[d]).map(|d| daily[d]).collect();
    if totals.is_empty() {
        return SECONDS_PER_DAY;
    }
    totals.iter().sum::<u64>() as f64 / totals.len() as f64
}

/// The day length for durations: the business day when a schedule is given,
/// else a calendar day.
pub(crate) fn day_seconds(hours: Option<&BusinessHours>) -> f64 {
    hours.map_or(SECONDS_PER_DAY, business_day_seconds)
}

const MIN_PENWIDTH: f64 = 1.0;
const MAX_PENWIDTH: f64 = 2.6;

/// pm4py's `get_arc_penwidth`: 1.0 to 2.6, scaled between `min` and `max`.
pub(crate) fn arc_penwidth(value: f64, min: f64, max: f64) -> f64 {
    MIN_PENWIDTH + (MAX_PENWIDTH - MIN_PENWIDTH) * (value - min) / (max - min + 0.00001)
}

/// The shade pm4py's frequency and service-time colours share: 255 for the
/// minimum down to 155 for the maximum, as upper-case hex.
fn shade(value: f64, min: f64, max: f64) -> String {
    let base = py_int(255.0 - 100.0 * (value - min) / (max - min + 0.00001));
    format!("{base:X}")
}

/// pm4py's `get_trans_freq_color`: white to blue.
pub(crate) fn frequency_color(value: f64, min: f64, max: f64) -> String {
    let s = shade(value, min, max);
    format!("#{s}{s}FF")
}

/// pm4py's `get_activities_color_serv_time` for one activity: white to red.
pub(crate) fn service_time_color(value: f64, min: f64, max: f64) -> String {
    let s = shade(value, min, max);
    format!("#FF{s}{s}")
}

/// pm4py's `value_to_color`: blue to red, as lower-case hex.
pub(crate) fn value_to_color(value: f64, min: f64, max: f64) -> String {
    let t = (value - min) / (max - min + 0.000001);
    let channel = |from: f64, to: f64| py_int(from + (to - from) * t);
    format!(
        "#{:02x}{:02x}{:02x}",
        channel(0.0, 255.0),
        channel(0.0, 0.0),
        channel(255.0, 0.0)
    )
}

/// pm4py's `get_min_max_value`, which starts from 9999999999 and -1.
pub(crate) fn min_max(values: impl IntoIterator<Item = f64>) -> (f64, f64) {
    values
        .into_iter()
        .fold((9_999_999_999.0, -1.0), |(lo, hi), v| {
            (if v < lo { v } else { lo }, if v > hi { v } else { hi })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floats_print_as_python_repr() {
        assert_eq!(py_float(1.0), "1.0");
        assert_eq!(py_float(1.31999936000128), "1.31999936000128");
        assert_eq!(py_float(-0.0), "-0.0");
        assert_eq!(py_float(1e-5), "1e-05");
        assert_eq!(py_float(1.5e16), "1.5e+16");
        assert_eq!(py_float(1e15), "1000000000000000.0");
        assert_eq!(py_float(0.0001), "0.0001");
    }

    #[test]
    fn durations_read_as_pm4py_writes_them() {
        assert_eq!(human_readable_stat(181_960.0, 86400.0), "2D");
        assert_eq!(human_readable_stat(36_000.0, 86400.0), "10h");
        assert_eq!(human_readable_stat(36_000.0, 36_000.0), "1D");
        assert_eq!(human_readable_stat(90.5, 86400.0), "1m");
        assert_eq!(human_readable_stat(0.25, 86400.0), "250ms");
        assert_eq!(human_readable_stat(0.0, 86400.0), "0ns");
        assert_eq!(human_readable_stat(40_000_000.0, 86400.0), "1Y");
        // Python floors negative values: -5 // 60 % 60 is 59.
        assert_eq!(human_readable_stat(-5.5, 86400.0), "59m");
        assert_eq!(human_readable_stat(-0.5, 86400.0), "-500000000ns");
    }

    #[test]
    fn business_days_average_the_working_weekdays() {
        assert_eq!(business_day_seconds(&BusinessHours::default()), 36_000.0);
        let empty = BusinessHours {
            slots: Vec::new(),
            ..BusinessHours::default()
        };
        assert_eq!(business_day_seconds(&empty), 86400.0);
        // Monday 8 to 12 and 11 to 13 join; Tuesday has 2 hours.
        let hours = BusinessHours {
            slots: vec![
                (8 * 3600, 12 * 3600),
                (11 * 3600, 13 * 3600),
                (86400 + 3600, 86400 + 3 * 3600),
            ],
            ..BusinessHours::default()
        };
        assert_eq!(
            business_day_seconds(&hours),
            (5.0 * 3600.0 + 2.0 * 3600.0) / 2.0
        );
    }

    #[test]
    fn colours_match_pm4py() {
        assert_eq!(frequency_color(6.0, 3.0, 9.0), "#CDCDFF");
        assert_eq!(frequency_color(3.0, 3.0, 9.0), "#FFFFFF");
        assert_eq!(value_to_color(0.0, 0.0, 1.0), "#0000ff");
        assert_eq!(value_to_color(1.0, 0.0, 1.0), "#fe0000");
        assert_eq!(service_time_color(0.0, 0.0, 10.0), "#FFFFFF");
    }
}
