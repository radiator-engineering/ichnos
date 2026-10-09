//! Performance spectra, ported from pm4py's
//! `algo/discovery/performance_spectrum/variants/log.py` and
//! `visualization/performance_spectrum/variants/neato.py`.

use std::fmt::Write as _;
use std::sync::OnceLock;

use chrono::{DateTime, Utc};
use ichnos_core::EventLog;

use crate::style::py_datetime;

/// A performance spectrum: for each run of the activities, in order and
/// back to back within a case, the timestamps of its events in seconds
/// since the Unix epoch.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PerformanceSpectrum {
    /// The activities, in order.
    pub activities: Vec<String>,
    /// One timestamp list per run, sorted by its first timestamp.
    pub points: Vec<Vec<f64>>,
}

/// Options for [`performance_spectrum`], with pm4py's defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerformanceSpectrumOptions {
    /// The activity attribute. Default `concept:name`.
    pub activity_key: String,
    /// The timestamp attribute. Default `time:timestamp`.
    pub timestamp_key: String,
    /// Keep at most about this many runs. Default 10000.
    pub sample_size: usize,
}

impl Default for PerformanceSpectrumOptions {
    fn default() -> Self {
        PerformanceSpectrumOptions {
            activity_key: "concept:name".to_owned(),
            timestamp_key: "time:timestamp".to_owned(),
            sample_size: 10_000,
        }
    }
}

/// The performance spectrum of `log` over `activities`, as pm4py's
/// `log` variant finds it.
///
/// Each trace keeps only the events of the activities, sorted by time.
/// Every window of consecutive events whose activities are exactly
/// `activities` gives one run. With more runs than the sample size, an
/// evenly spaced sample is kept, with the first and last run.
///
/// pm4py fails on an event without an activity or timestamp; this leaves
/// the event out. pm4py also regroups the filtered events by case id, and
/// its wrapper `save_vis_performance_spectrum` takes a random sample from a
/// data frame; this keeps the traces of `log` and samples as the `log`
/// variant does.
pub fn performance_spectrum(
    log: &EventLog,
    activities: &[String],
    options: &PerformanceSpectrumOptions,
) -> PerformanceSpectrum {
    let k = activities.len();
    let mut points: Vec<Vec<f64>> = Vec::new();
    for trace in &log.traces {
        let mut events: Vec<(&str, DateTime<chrono::FixedOffset>)> = trace
            .events
            .iter()
            .filter_map(|e| {
                let activity = e.get(&options.activity_key)?.as_str()?;
                let time = e.get(&options.timestamp_key)?.as_date()?;
                activities
                    .iter()
                    .any(|a| a == activity)
                    .then_some((activity, time))
            })
            .collect();
        events.sort_by_key(|(_, t)| *t);
        if k == 0 || events.len() < k {
            continue;
        }
        for window in events.windows(k) {
            if window.iter().zip(activities).all(|((a, _), b)| *a == b) {
                points.push(window.iter().map(|(_, t)| seconds(t)).collect());
            }
        }
    }
    points.sort_by(|a, b| a[0].total_cmp(&b[0]));
    let m = options.sample_size;
    if points.len() > m && m > 0 {
        points = pick_chosen_points(m, &points);
    }
    PerformanceSpectrum {
        activities: activities.to_vec(),
        points,
    }
}

/// Python's `datetime.timestamp()`: microseconds over 10⁶, rounded once.
fn seconds<Tz: chrono::TimeZone>(t: &DateTime<Tz>) -> f64 {
    t.timestamp_micros() as f64 / 1e6
}

/// pm4py's `pick_chosen_points_list(m, lst)` with the extremes.
fn pick_chosen_points(m: usize, list: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = list.len();
    let mut indexes: Vec<usize> = (0..m).map(|i| i * n / m + n / (2 * m)).collect();
    if !indexes.contains(&0) {
        indexes.insert(0, 0);
    }
    if !indexes.contains(&(n - 1)) {
        indexes.push(n - 1);
    }
    indexes.into_iter().map(|i| list[i].clone()).collect()
}

/// Options for [`performance_spectrum_dot`], with pm4py's defaults.
#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceSpectrumDotOptions {
    /// The vertical space between activities. Default 3.0.
    pub act_divider_space: f64,
    /// The space between the last activity and the dates. Default 1.0.
    pub date_divider_space: f64,
    /// The length of the time axis. Default 10.0.
    pub overall_length_x: f64,
    /// The number of intervals between the dates written under the axis.
    /// Default 2.
    pub n_div_dates: u32,
    /// Draw only this share of the runs, the longest first. Default 1.0.
    pub perc_paths: f64,
    /// The scale of the layout. Default 100.
    pub layout_ext_multiplier: f64,
    /// A title above the drawing. `None` or an empty title draws none.
    pub graph_title: Option<String>,
}

impl Default for PerformanceSpectrumDotOptions {
    fn default() -> Self {
        PerformanceSpectrumDotOptions {
            act_divider_space: 3.0,
            date_divider_space: 1.0,
            overall_length_x: 10.0,
            n_div_dates: 2,
            perc_paths: 1.0,
            layout_ext_multiplier: 100.0,
            graph_title: None,
        }
    }
}

/// The DOT text of a performance spectrum, as pm4py's
/// `save_vis_performance_spectrum` writes it before running `neato -n1`
/// on it.
///
/// Each activity is a horizontal line, the first at the top, and time runs
/// from left to right. Each run is a polyline across the lines; a segment
/// is grey for a short step and turns red for a long one, on a scale from
/// the shortest step to the longest. Dates are written under the last
/// line. The node positions are fixed, so render the text with
/// `neato -n1`, as
/// [`write_performance_spectrum`](crate::write_performance_spectrum) does.
///
/// pm4py writes the dates in the machine's local time zone; this writes
/// them in UTC. pm4py fails on a spectrum without runs, or whose runs all
/// start and end at once, or whose steps all take as long; this draws the
/// activities alone, puts every point at the left, or colours every step
/// grey.
pub fn performance_spectrum_dot(
    spectrum: &PerformanceSpectrum,
    options: &PerformanceSpectrumDotOptions,
) -> String {
    let mult = options.layout_ext_multiplier;
    let length = options.overall_length_x;
    let n_acts = spectrum.activities.len();
    let row = |i: usize| options.act_divider_space * (n_acts as f64 - i as f64 - 1.0);
    let mut lines: Vec<String> = vec!["graph G {".to_owned()];
    if let Some(title) = options.graph_title.as_deref().filter(|t| !t.is_empty()) {
        lines.push(format!(
            "label=<<FONT POINT-SIZE=\"20\">{title}</FONT>>;\nlabelloc=\"top\";\n"
        ));
    }
    let points = &spectrum.points;
    let min_x = points
        .iter()
        .filter_map(|p| p.first().copied())
        .fold(f64::INFINITY, f64::min);
    let max_x = points
        .iter()
        .filter_map(|p| p.last().copied())
        .fold(f64::NEG_INFINITY, f64::max);
    let diffs = || {
        points
            .iter()
            .flat_map(|p| p.windows(2).map(|w| w[1] - w[0]))
    };
    let min_diff = diffs().fold(f64::INFINITY, f64::min);
    let max_diff = diffs().fold(f64::NEG_INFINITY, f64::max);
    let span = |v: f64, lo: f64, hi: f64| if hi > lo { (v - lo) / (hi - lo) } else { 0.0 };

    let mut drawn: Vec<&Vec<f64>> = points.iter().collect();
    let duration = |p: &Vec<f64>| match (p.first(), p.last()) {
        (Some(a), Some(b)) => b - a,
        _ => 0.0,
    };
    drawn.sort_by(|a, b| duration(b).total_cmp(&duration(a)));
    drawn.truncate((options.perc_paths * drawn.len() as f64).ceil() as usize);
    let mut next = 0;
    let mut id = || {
        next += 1;
        format!("n{next}e")
    };
    for polyline in drawn {
        let mut ids = Vec::with_capacity(polyline.len());
        for (i, &p) in polyline.iter().enumerate() {
            let pid = id();
            lines.push(format!(
                "{pid} [label=\"\", pos=\"{:.10},{:.10}!\", shape=none, width=\"0px\", height=\"0px\"];",
                span(p, min_x, max_x) * length * mult,
                row(i) * mult
            ));
            ids.push(pid);
        }
        for (i, pair) in ids.windows(2).enumerate() {
            let diff = polyline[i + 1] - polyline[i];
            lines.push(format!(
                "{} -- {} [ color=\"{}\" ];",
                pair[0],
                pair[1],
                line_color(span(diff, min_diff, max_diff))
            ));
        }
    }
    for (i, act) in spectrum.activities.iter().enumerate() {
        let a_id = id();
        lines.push(format!(
            "{a_id} [label=\"{act}\", pos=\"{:.10},{:.10}!\", shape=none, width=\"0px\", height=\"0px\"];",
            length * mult,
            row(i) * mult
        ));
        let s_id = id();
        lines.push(format!(
            "{s_id} [label=\"\", pos=\"0,{:.10}!\", shape=none, width=\"0px\", height=\"0px\"];",
            row(i) * mult
        ));
        lines.push(format!("{s_id} -- {a_id} [ color=\"black\" ];"));
        if i + 1 == n_acts && !points.is_empty() {
            let n_div = f64::from(options.n_div_dates);
            for j in 0..=options.n_div_dates {
                let j = f64::from(j);
                let pos = j * length / n_div;
                let tst = min_x + j / n_div * (max_x - min_x);
                lines.push(format!(
                    "{} [label=\"{}\", pos=\"{:.10},{:.10}!\", shape=none, width=\"0px\", height=\"0px\"];",
                    id(),
                    utc_label(tst),
                    pos * mult,
                    (row(i) - options.date_divider_space) * mult
                ));
            }
        }
    }
    lines.push("}".to_owned());
    lines.join("\n")
}

/// Python's `str(datetime.fromtimestamp(t, timezone.utc))`, with
/// microseconds rounded half to even as `fromtimestamp` does.
fn utc_label(t: f64) -> String {
    let mut whole = t.trunc();
    let mut micros = ((t - whole) * 1e6).round_ties_even();
    if micros >= 1e6 {
        micros -= 1e6;
        whole += 1.0;
    } else if micros < 0.0 {
        micros += 1e6;
        whole -= 1.0;
    }
    let dt =
        DateTime::<Utc>::from_timestamp(whole as i64, micros as u32 * 1000).unwrap_or_default();
    py_datetime(&dt.fixed_offset())
}

/// The nodes and colours of pm4py's colour map for spectrum lines:
/// deepskyblue, skyblue, lightcyan, lightgray, gray, lightgray, mistyrose,
/// salmon and tomato.
const NODES: [f64; 9] = [0.0, 0.01, 0.25, 0.4, 0.45, 0.55, 0.75, 0.99, 1.0];
const COLORS: [[u8; 3]; 9] = [
    [0x00, 0xBF, 0xFF],
    [0x87, 0xCE, 0xEB],
    [0xE0, 0xFF, 0xFF],
    [0xD3, 0xD3, 0xD3],
    [0x80, 0x80, 0x80],
    [0xD3, 0xD3, 0xD3],
    [0xFF, 0xE4, 0xE1],
    [0xFA, 0x80, 0x72],
    [0xFF, 0x63, 0x47],
];
/// Matplotlib's default number of colours in a colour map.
const N: usize = 256;

/// pm4py's `give_color_to_line`: the colour of a step whose length is at
/// `dir` between the shortest (0) and the longest (1), from matplotlib's
/// 256-entry lookup table for the colour map.
pub(crate) fn line_color(dir: f64) -> String {
    let dir = 0.5 + 0.5 * dir;
    let mut index = dir * N as f64;
    if index == N as f64 {
        index = (N - 1) as f64;
    }
    let index = (index.max(0.0) as usize).min(N - 1);
    static TABLES: OnceLock<[Vec<f64>; 3]> = OnceLock::new();
    let tables = TABLES.get_or_init(|| [lookup_table(0), lookup_table(1), lookup_table(2)]);
    let mut out = String::from("#");
    for table in tables {
        let c = table[index];
        let v = (c * 255.0).ceil() as u32;
        let _ = write!(out, "{v:02X}");
    }
    out
}

/// Matplotlib's `_create_lookup_table(N, data)` for one channel of the
/// colour map, where both sides of each node have the same value.
fn lookup_table(channel: usize) -> Vec<f64> {
    let y: Vec<f64> = COLORS
        .iter()
        .map(|c| f64::from(c[channel]) / 255.0)
        .collect();
    let x: Vec<f64> = NODES.iter().map(|n| n * (N - 1) as f64).collect();
    // `np.linspace(0, 1, N)`: i * (1 / (N - 1)), with the last exactly 1.
    let step = 1.0 / (N - 1) as f64;
    let xind: Vec<f64> = (0..N)
        .map(|i| {
            let v = if i == N - 1 { 1.0 } else { i as f64 * step };
            (N - 1) as f64 * v
        })
        .collect();
    let mut lut = Vec::with_capacity(N);
    lut.push(y[0]);
    for &xi in &xind[1..N - 1] {
        // `np.searchsorted(x, xi)`, side left.
        let ind = x.partition_point(|&v| v < xi);
        let distance = (xi - x[ind - 1]) / (x[ind] - x[ind - 1]);
        lut.push(distance * (y[ind] - y[ind - 1]) + y[ind - 1]);
    }
    lut.push(y[y.len() - 1]);
    lut.into_iter().map(|v| v.clamp(0.0, 1.0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_colours_match_matplotlib() {
        // pm4py's give_color_to_line under matplotlib 3.11.
        for (dir, color) in [
            (0.0, "#ACACAC"),
            (0.001, "#ACACAC"),
            (0.0039, "#ACACAC"),
            (0.004, "#ACACAC"),
            (0.1, "#D3D3D3"),
            (0.25, "#E5DAD9"),
            (1.0 / 3.0, "#EDDDDC"),
            (0.5, "#FFE3E0"),
            (0.731, "#FDB4AC"),
            (0.9, "#FB9084"),
            (0.99, "#FE6F58"),
            (0.999, "#FF6347"),
            (1.0, "#FF6347"),
        ] {
            assert_eq!(line_color(dir), color, "{dir}");
        }
    }

    #[test]
    fn dates_round_microseconds_half_to_even() {
        assert_eq!(utc_label(1_293_840_000.0), "2011-01-01 00:00:00+00:00");
        assert_eq!(
            utc_label(1_293_840_000.25),
            "2011-01-01 00:00:00.250000+00:00"
        );
    }

    #[test]
    fn samples_keep_the_extremes() {
        let list: Vec<Vec<f64>> = (0..10).map(|i| vec![f64::from(i)]).collect();
        let picked: Vec<f64> = pick_chosen_points(3, &list).iter().map(|p| p[0]).collect();
        // [i * 10 // 3 + 10 // 6] = [1, 4, 7], then 0 and 9.
        assert_eq!(picked, [0.0, 1.0, 4.0, 7.0, 9.0]);
    }
}
