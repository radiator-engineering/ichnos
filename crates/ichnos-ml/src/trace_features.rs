//! Case features of an `EventLog`, ported from the `EventLog` branch of
//! pm4py's `trace_based` encoding
//! (`algo/transformation/trace_encodings/variants/trace_based.py`).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use ichnos_core::{Event, EventLog, Trace};

use crate::error::{Error, Result};
use crate::features::FeatureTable;
use crate::util::{event_seconds, event_str};

/// Extra timing features of [`trace_features`]. All are off by default, as
/// in pm4py.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TraceExtras {
    /// `@@caseDuration`.
    pub case_duration: bool,
    /// `startToFirstOcc@@<act>` and `firstOccToEnd@@<act>`.
    pub times_from_first_occurrence: bool,
    /// `startToLastOcc@@<act>` and `lastOccToEnd@@<act>`.
    pub times_from_last_occurrence: bool,
    /// `directPathPerformanceLastOcc@@<a>##<b>`.
    pub direct_paths_times_last_occ: bool,
    /// `indirectPathPerformanceLastOcc@@<a>##<b>`.
    pub indirect_paths_times_last_occ: bool,
    /// `@@work_in_progress`.
    pub work_in_progress: bool,
    /// `resource_workload@@<resource>`.
    pub resource_workload: bool,
    /// `firstIndexAct@@<act>` and `lastIndexAct@@<act>`.
    pub first_last_activity_index: bool,
    /// `@@max_concurrent_activities_general`.
    pub max_concurrent_events: bool,
    /// `@@max_concurrent_activities_like_<act>`.
    pub max_concurrent_events_per_activity: bool,
}

impl TraceExtras {
    /// Every extra feature, as pm4py's `enable_all_extra_features`.
    pub fn all() -> Self {
        TraceExtras {
            case_duration: true,
            times_from_first_occurrence: true,
            times_from_last_occurrence: true,
            direct_paths_times_last_occ: true,
            indirect_paths_times_last_occ: true,
            work_in_progress: true,
            resource_workload: true,
            first_last_activity_index: true,
            max_concurrent_events: true,
            max_concurrent_events_per_activity: true,
        }
    }
}

/// Options for [`trace_features`], with pm4py's defaults.
#[derive(Clone, Debug)]
pub struct TraceFeatureOptions {
    /// String trace attributes: one feature `trace:<attr>@<value>` per value.
    pub str_tr_attr: Vec<String>,
    /// String event attributes: one feature `event:<attr>@<value>` per value.
    pub str_ev_attr: Vec<String>,
    /// Numeric trace attributes: the feature `trace:<attr>`.
    pub num_tr_attr: Vec<String>,
    /// Numeric event attributes: the feature `event:<attr>`, the last value
    /// in the trace.
    pub num_ev_attr: Vec<String>,
    /// Event attributes whose directly-following pairs are features
    /// `succession:<attr>@<a>#<b>`.
    pub str_evsucc_attr: Vec<String>,
    /// Fixed feature names for the attribute features, as from an earlier
    /// call. Values not among them are dropped.
    pub feature_names: Option<Vec<String>>,
    /// The trace attribute that names the case. Default `concept:name`.
    pub case_id_key: String,
    /// The activity attribute. Default `concept:name`.
    pub activity_key: String,
    /// The timestamp attribute. Default `time:timestamp`.
    pub timestamp_key: String,
    /// The start timestamp attribute. Default `time:timestamp`.
    pub start_timestamp_key: String,
    /// The resource attribute. Default `org:resource`.
    pub resource_key: String,
    /// Widens each case interval for work in progress and resource
    /// workload. Default 0.000001 seconds.
    pub epsilon: f64,
    /// The value of a timing feature that does not apply to a case. `None`
    /// keeps pm4py's defaults: 0, and -1 for activity indexes.
    pub default_not_present: Option<f64>,
    /// The extra timing features.
    pub extras: TraceExtras,
}

impl Default for TraceFeatureOptions {
    fn default() -> Self {
        TraceFeatureOptions {
            str_tr_attr: Vec::new(),
            str_ev_attr: Vec::new(),
            num_tr_attr: Vec::new(),
            num_ev_attr: Vec::new(),
            str_evsucc_attr: Vec::new(),
            feature_names: None,
            case_id_key: "concept:name".to_owned(),
            activity_key: "concept:name".to_owned(),
            timestamp_key: "time:timestamp".to_owned(),
            start_timestamp_key: "time:timestamp".to_owned(),
            resource_key: "org:resource".to_owned(),
            epsilon: 0.000001,
            default_not_present: None,
            extras: TraceExtras::default(),
        }
    }
}

/// One row of features per trace, as pm4py's `extract_features_dataframe`
/// computes them for an `EventLog`.
///
/// Rows follow the traces of the log; `case_ids` holds the `case_id_key`
/// trace attribute (pm4py's `add_case_identifier_column`). Attribute
/// features come first, then the enabled extras in pm4py's order. pm4py
/// orders the per-activity concurrency features by Python's set order; this
/// sorts them.
pub fn trace_features(log: &EventLog, options: &TraceFeatureOptions) -> Result<FeatureTable> {
    let mut case_ids = Vec::with_capacity(log.traces.len());
    for (t, trace) in log.traces.iter().enumerate() {
        let id = trace.attributes.get(&options.case_id_key).ok_or_else(|| {
            Error::MissingTraceAttribute {
                trace: t,
                key: options.case_id_key.clone(),
            }
        })?;
        case_ids.push(id.to_string());
    }
    let (mut names, mut rows) = representation(log, options)?;
    let x = options.extras;
    let parts: [(bool, Part); 10] = [
        (x.case_duration, case_duration),
        (x.times_from_first_occurrence, times_from_first_occurrence),
        (x.times_from_last_occurrence, times_from_last_occurrence),
        (x.direct_paths_times_last_occ, direct_paths),
        (x.indirect_paths_times_last_occ, indirect_paths),
        (x.work_in_progress, work_in_progress),
        (x.resource_workload, resource_workload),
        (x.first_last_activity_index, first_last_activity_index),
        (x.max_concurrent_events, max_concurrent_events),
        (
            x.max_concurrent_events_per_activity,
            max_concurrent_events_per_activity,
        ),
    ];
    let ctx = Ctx::new(log, options)?;
    for (enabled, part) in parts {
        if enabled {
            let (n, r) = part(&ctx)?;
            names.extend(n);
            for (row, extra) in rows.iter_mut().zip(r) {
                row.extend(extra);
            }
        }
    }
    Ok(FeatureTable {
        case_ids,
        names,
        rows,
    })
}

type Part = fn(&Ctx) -> Result<(Vec<String>, Vec<Vec<f64>>)>;

/// Activities and times of each trace, read once.
struct Ctx<'a> {
    log: &'a EventLog,
    options: &'a TraceFeatureOptions,
    activities: Vec<Vec<String>>,
    starts: Vec<Vec<f64>>,
    ends: Vec<Vec<f64>>,
}

impl<'a> Ctx<'a> {
    fn new(log: &'a EventLog, options: &'a TraceFeatureOptions) -> Result<Self> {
        let x = options.extras;
        let need_times = x.case_duration
            || x.times_from_first_occurrence
            || x.times_from_last_occurrence
            || x.direct_paths_times_last_occ
            || x.indirect_paths_times_last_occ
            || x.work_in_progress
            || x.resource_workload
            || x.max_concurrent_events
            || x.max_concurrent_events_per_activity;
        let need_acts = x.times_from_first_occurrence
            || x.times_from_last_occurrence
            || x.direct_paths_times_last_occ
            || x.indirect_paths_times_last_occ
            || x.first_last_activity_index
            || x.max_concurrent_events_per_activity;
        let mut ctx = Ctx {
            log,
            options,
            activities: Vec::new(),
            starts: Vec::new(),
            ends: Vec::new(),
        };
        for (t, trace) in log.traces.iter().enumerate() {
            let mut acts = Vec::new();
            let mut starts = Vec::new();
            let mut ends = Vec::new();
            for (e, event) in trace.events.iter().enumerate() {
                if need_acts {
                    acts.push(event_str(event, &options.activity_key, t, e)?);
                }
                if need_times {
                    starts.push(event_seconds(event, &options.start_timestamp_key, t, e)?);
                    ends.push(event_seconds(event, &options.timestamp_key, t, e)?);
                }
            }
            ctx.activities.push(acts);
            ctx.starts.push(starts);
            ctx.ends.push(ends);
        }
        Ok(ctx)
    }

    fn missing(&self) -> f64 {
        self.options.default_not_present.unwrap_or(0.0)
    }

    fn sorted_activities(&self) -> Vec<String> {
        let set: BTreeSet<&String> = self.activities.iter().flatten().collect();
        set.into_iter().cloned().collect()
    }
}

fn representation(log: &EventLog, o: &TraceFeatureOptions) -> Result<(Vec<String>, Vec<Vec<f64>>)> {
    let names: Vec<String> = match &o.feature_names {
        Some(n) => n.clone(),
        None => {
            let mut names = Vec::new();
            for a in &o.str_tr_attr {
                let set: BTreeSet<String> = log.traces.iter().map(|t| trace_rep(t, a)).collect();
                names.extend(set);
            }
            for a in &o.str_ev_attr {
                let set: BTreeSet<String> =
                    log.traces.iter().flat_map(|t| event_reps(t, a)).collect();
                names.extend(set);
            }
            names.extend(o.num_tr_attr.iter().map(|a| format!("trace:{a}")));
            names.extend(o.num_ev_attr.iter().map(|a| format!("event:{a}")));
            for a in &o.str_evsucc_attr {
                let set: BTreeSet<String> = log
                    .traces
                    .iter()
                    .flat_map(|t| succession_reps(t, a))
                    .collect();
                names.extend(set);
            }
            names
        }
    };
    let index: HashMap<&str, usize> = names
        .iter()
        .enumerate()
        .map(|(i, n)| (n.as_str(), i))
        .collect();
    let mut rows = Vec::with_capacity(log.traces.len());
    for (t, trace) in log.traces.iter().enumerate() {
        let mut row = vec![0.0; names.len()];
        let mut mark = |rep: &str| {
            if let Some(&k) = index.get(rep) {
                row[k] = 1.0;
            }
        };
        for a in &o.str_tr_attr {
            mark(&trace_rep(trace, a));
        }
        for a in &o.str_ev_attr {
            for rep in event_reps(trace, a) {
                mark(&rep);
            }
        }
        for a in &o.num_tr_attr {
            if let Some(&k) = index.get(format!("trace:{a}").as_str()) {
                let v = trace
                    .attributes
                    .get(a)
                    .ok_or_else(|| Error::MissingTraceAttribute {
                        trace: t,
                        key: a.clone(),
                    })?;
                row[k] = v.as_f64().ok_or_else(|| Error::NotNumeric(a.clone()))?;
            }
        }
        for a in &o.num_ev_attr {
            if let Some(&k) = index.get(format!("event:{a}").as_str()) {
                let mut last = None;
                for event in &trace.events {
                    if let Some(v) = event.get(a) {
                        last = Some(v.as_f64().ok_or_else(|| Error::NotNumeric(a.clone()))?);
                    }
                }
                row[k] = last.ok_or_else(|| Error::MissingEventAttribute {
                    trace: t,
                    key: a.clone(),
                })?;
            }
        }
        for a in &o.str_evsucc_attr {
            for rep in succession_reps(trace, a) {
                if let Some(&k) = index.get(rep.as_str()) {
                    row[k] = 1.0;
                }
            }
        }
        rows.push(row);
    }
    Ok((names, rows))
}

fn trace_rep(trace: &Trace, attr: &str) -> String {
    match trace.attributes.get(attr) {
        Some(v) => format!("trace:{attr}@{v}"),
        None => format!("trace:{attr}@UNDEFINED"),
    }
}

fn event_reps(trace: &Trace, attr: &str) -> BTreeSet<String> {
    let mut set: BTreeSet<String> = trace
        .events
        .iter()
        .filter_map(|e| e.get(attr))
        .map(|v| format!("event:{attr}@{v}"))
        .collect();
    if set.is_empty() {
        set.insert(format!("event:{attr}@UNDEFINED"));
    }
    set
}

fn succession_reps(trace: &Trace, attr: &str) -> BTreeSet<String> {
    let mut set: BTreeSet<String> = trace
        .events
        .windows(2)
        .filter_map(|w| {
            Some(format!(
                "succession:{attr}@{}#{}",
                w[0].get(attr)?,
                w[1].get(attr)?
            ))
        })
        .collect();
    if set.is_empty() {
        set.insert(format!("succession:{attr}@UNDEFINED"));
    }
    set
}

fn case_duration(c: &Ctx) -> Result<(Vec<String>, Vec<Vec<f64>>)> {
    let rows = (0..c.log.traces.len())
        .map(|t| match (c.starts[t].first(), c.ends[t].last()) {
            (Some(s), Some(e)) => vec![e - s],
            _ => vec![0.0],
        })
        .collect();
    Ok((vec!["@@caseDuration".to_owned()], rows))
}

fn occurrence_times(c: &Ctx, last: bool) -> Result<(Vec<String>, Vec<Vec<f64>>)> {
    let acts = c.sorted_activities();
    let (a, b) = if last {
        ("startToLastOcc@@", "lastOccToEnd@@")
    } else {
        ("startToFirstOcc@@", "firstOccToEnd@@")
    };
    let names = acts
        .iter()
        .flat_map(|x| [format!("{a}{x}"), format!("{b}{x}")])
        .collect();
    let mut rows = Vec::new();
    for t in 0..c.log.traces.len() {
        let mut occ: HashMap<&str, usize> = HashMap::new();
        for (i, act) in c.activities[t].iter().enumerate() {
            if last {
                occ.insert(act, i);
            } else {
                occ.entry(act).or_insert(i);
            }
        }
        let mut row = Vec::new();
        for act in &acts {
            match occ.get(act.as_str()) {
                Some(&i) => {
                    let n = c.starts[t].len();
                    row.push(c.starts[t][i] - c.ends[t][0]);
                    row.push(c.starts[t][n - 1] - c.ends[t][i]);
                }
                None => row.extend([c.missing(), c.missing()]),
            }
        }
        rows.push(row);
    }
    Ok((names, rows))
}

fn times_from_first_occurrence(c: &Ctx) -> Result<(Vec<String>, Vec<Vec<f64>>)> {
    occurrence_times(c, false)
}

fn times_from_last_occurrence(c: &Ctx) -> Result<(Vec<String>, Vec<Vec<f64>>)> {
    occurrence_times(c, true)
}

fn paths(c: &Ctx, indirect: bool) -> Result<(Vec<String>, Vec<Vec<f64>>)> {
    let gap = if indirect { 2 } else { 1 };
    let pairs = |t: usize| {
        let n = c.activities[t].len();
        (0..n.saturating_sub(1)).flat_map(move |i| {
            let js = if indirect {
                (i + gap)..n
            } else {
                (i + 1)..(i + 2).min(n)
            };
            js.map(move |j| (i, j))
        })
    };
    let mut all: BTreeSet<(&str, &str)> = BTreeSet::new();
    for t in 0..c.log.traces.len() {
        for (i, j) in pairs(t) {
            all.insert((&c.activities[t][i], &c.activities[t][j]));
        }
    }
    let prefix = if indirect {
        "indirectPathPerformanceLastOcc@@"
    } else {
        "directPathPerformanceLastOcc@@"
    };
    let names = all
        .iter()
        .map(|(a, b)| format!("{prefix}{a}##{b}"))
        .collect();
    let mut rows = Vec::new();
    for t in 0..c.log.traces.len() {
        let mut perf: HashMap<(&str, &str), f64> = HashMap::new();
        for (i, j) in pairs(t) {
            let tc = c.ends[t][i];
            let ts = c.starts[t][j];
            if ts > tc {
                perf.insert((&c.activities[t][i], &c.activities[t][j]), ts - tc);
            }
        }
        rows.push(
            all.iter()
                .map(|p| perf.get(p).copied().unwrap_or(c.missing()))
                .collect(),
        );
    }
    Ok((names, rows))
}

fn direct_paths(c: &Ctx) -> Result<(Vec<String>, Vec<Vec<f64>>)> {
    paths(c, false)
}

fn indirect_paths(c: &Ctx) -> Result<(Vec<String>, Vec<Vec<f64>>)> {
    paths(c, true)
}

/// The interval of a non-empty trace, widened by epsilon.
fn interval(c: &Ctx, t: usize) -> Option<(f64, f64)> {
    let eps = c.options.epsilon;
    Some((*c.starts[t].first()? - eps, *c.ends[t].last()? + eps))
}

/// Intervals of a set (pm4py's `IntervalTree` drops duplicates) that
/// overlap `[st, ct)`.
fn overlaps(set: &HashSet<(u64, u64)>, st: f64, ct: f64) -> usize {
    set.iter()
        .filter(|(b, e)| f64::from_bits(*b) < ct && f64::from_bits(*e) > st)
        .count()
}

fn work_in_progress(c: &Ctx) -> Result<(Vec<String>, Vec<Vec<f64>>)> {
    let set: HashSet<(u64, u64)> = (0..c.log.traces.len())
        .filter_map(|t| interval(c, t))
        .map(|(s, e)| (s.to_bits(), e.to_bits()))
        .collect();
    let rows = (0..c.log.traces.len())
        .map(|t| match interval(c, t) {
            Some((s, e)) => vec![overlaps(&set, s, e) as f64],
            None => vec![c.missing()],
        })
        .collect();
    Ok((vec!["@@work_in_progress".to_owned()], rows))
}

fn resources(trace: &Trace, key: &str) -> BTreeSet<String> {
    trace
        .events
        .iter()
        .filter_map(|e: &Event| e.get(key))
        .map(ToString::to_string)
        .collect()
}

fn resource_workload(c: &Ctx) -> Result<(Vec<String>, Vec<Vec<f64>>)> {
    let key = &c.options.resource_key;
    let mut trees: BTreeMap<String, HashSet<(u64, u64)>> = BTreeMap::new();
    for (t, trace) in c.log.traces.iter().enumerate() {
        if let Some((s, e)) = interval(c, t) {
            for r in resources(trace, key) {
                trees
                    .entry(r)
                    .or_default()
                    .insert((s.to_bits(), e.to_bits()));
            }
        }
    }
    let names = trees
        .keys()
        .map(|r| format!("resource_workload@@{r}"))
        .collect();
    let mut rows = Vec::new();
    for (t, trace) in c.log.traces.iter().enumerate() {
        let own = resources(trace, key);
        let iv = interval(c, t);
        rows.push(
            trees
                .iter()
                .map(|(r, set)| match iv {
                    Some((s, e)) if own.contains(r) => overlaps(set, s, e) as f64,
                    _ => c.missing(),
                })
                .collect(),
        );
    }
    Ok((names, rows))
}

fn first_last_activity_index(c: &Ctx) -> Result<(Vec<String>, Vec<Vec<f64>>)> {
    let missing = c.options.default_not_present.unwrap_or(-1.0);
    let acts = c.sorted_activities();
    let names = acts
        .iter()
        .flat_map(|x| [format!("firstIndexAct@@{x}"), format!("lastIndexAct@@{x}")])
        .collect();
    let mut rows = Vec::new();
    for t in 0..c.log.traces.len() {
        let mut first: HashMap<&str, usize> = HashMap::new();
        let mut last: HashMap<&str, usize> = HashMap::new();
        for (i, act) in c.activities[t].iter().enumerate() {
            last.insert(act, i);
            first.entry(act).or_insert(i);
        }
        let mut row = Vec::new();
        for act in &acts {
            match first.get(act.as_str()) {
                Some(&f) => row.extend([f as f64, last[act.as_str()] as f64]),
                None => row.extend([missing, missing]),
            }
        }
        rows.push(row);
    }
    Ok((names, rows))
}

fn max_concurrent_events(c: &Ctx) -> Result<(Vec<String>, Vec<Vec<f64>>)> {
    let mut rows = Vec::new();
    for t in 0..c.log.traces.len() {
        let n = c.starts[t].len();
        let mut max = 0usize;
        for i in 0..n.saturating_sub(1) {
            let ct = c.ends[t][i];
            let conc = (i + 1..n).take_while(|&j| c.starts[t][j] <= ct).count();
            max = max.max(conc);
        }
        rows.push(vec![max as f64]);
    }
    Ok((vec!["@@max_concurrent_activities_general".to_owned()], rows))
}

fn max_concurrent_events_per_activity(c: &Ctx) -> Result<(Vec<String>, Vec<Vec<f64>>)> {
    let acts = c.sorted_activities();
    let names = acts
        .iter()
        .map(|x| format!("@@max_concurrent_activities_like_{x}"))
        .collect();
    let mut rows = Vec::new();
    for t in 0..c.log.traces.len() {
        let n = c.starts[t].len();
        let mut max: HashMap<&str, usize> = HashMap::new();
        for i in 0..n.saturating_sub(1) {
            let act = c.activities[t][i].as_str();
            let ct = c.ends[t][i];
            let conc = (i + 1..n)
                .take_while(|&j| c.starts[t][j] <= ct)
                .filter(|&j| c.activities[t][j] == act)
                .count();
            let m = max.entry(act).or_insert(0);
            *m = (*m).max(conc);
        }
        rows.push(
            acts.iter()
                .map(|a| max.get(a.as_str()).copied().unwrap_or(0) as f64)
                .collect(),
        );
    }
    Ok((names, rows))
}
