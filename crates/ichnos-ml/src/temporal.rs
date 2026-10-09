//! Features per time bin, ported from pm4py's
//! `extract_temporal_features_dataframe`
//! (`algo/transformation/trace_encodings/variants/temporal.py`).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use chrono::{DateTime, Datelike, Duration, FixedOffset, Months, NaiveDate, NaiveDateTime, Utc};
use ichnos_core::EventLog;

use crate::error::{Error, Result};
use crate::outcome::case_times;
use crate::util::{Table, event_date, mean};

/// The bin width, as a pandas frequency string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrouperFreq {
    /// `<n>h`: bins of `n` hours from midnight of the first day, labelled
    /// by their start.
    Hours(u32),
    /// `<n>D`: bins of `n` days from the first day, labelled by their start.
    Days(u32),
    /// `W`: weeks from Monday to Sunday, labelled by the Sunday.
    Week,
    /// `MS`: months, labelled by their first day.
    MonthStart,
    /// `ME`: months, labelled by their last day.
    MonthEnd,
    /// `YS`: years, labelled by 1 January.
    YearStart,
    /// `YE`: years, labelled by 31 December.
    YearEnd,
}

impl GrouperFreq {
    /// The label of the bin holding `t`.
    fn label(self, t: NaiveDateTime, origin: NaiveDate) -> NaiveDateTime {
        let d = t.date();
        let midnight = |d: NaiveDate| d.and_hms_opt(0, 0, 0).expect("midnight exists");
        match self {
            GrouperFreq::Hours(n) | GrouperFreq::Days(n) => {
                let step = self.step(n);
                let origin = midnight(origin);
                let k = (t - origin).num_microseconds().unwrap_or(i64::MAX)
                    / step.num_microseconds().unwrap_or(1);
                origin + step * i32::try_from(k).unwrap_or(i32::MAX)
            }
            GrouperFreq::Week => {
                midnight(d + Duration::days(6 - i64::from(d.weekday().num_days_from_monday())))
            }
            GrouperFreq::MonthStart => midnight(d.with_day(1).expect("day 1 exists")),
            GrouperFreq::MonthEnd => midnight(month_end(d)),
            GrouperFreq::YearStart => {
                midnight(NaiveDate::from_ymd_opt(d.year(), 1, 1).expect("1 January exists"))
            }
            GrouperFreq::YearEnd => {
                midnight(NaiveDate::from_ymd_opt(d.year(), 12, 31).expect("31 December exists"))
            }
        }
    }

    fn step(self, n: u32) -> Duration {
        match self {
            GrouperFreq::Hours(_) => Duration::hours(i64::from(n)),
            _ => Duration::days(i64::from(n)),
        }
    }

    /// The label after `label`.
    fn next(self, label: NaiveDateTime) -> NaiveDateTime {
        let d = label.date();
        let midnight = |d: NaiveDate| d.and_hms_opt(0, 0, 0).expect("midnight exists");
        match self {
            GrouperFreq::Hours(n) | GrouperFreq::Days(n) => label + self.step(n),
            GrouperFreq::Week => label + Duration::days(7),
            GrouperFreq::MonthStart => midnight(d + Months::new(1)),
            GrouperFreq::MonthEnd => midnight(month_end(
                d.with_day(1).expect("day 1 exists") + Months::new(1),
            )),
            GrouperFreq::YearStart => midnight(d + Months::new(12)),
            GrouperFreq::YearEnd => midnight(d + Months::new(12)),
        }
    }
}

fn month_end(d: NaiveDate) -> NaiveDate {
    let first = d.with_day(1).expect("day 1 exists");
    first + Months::new(1) - Duration::days(1)
}

/// Options for [`extract_temporal_features_dataframe`], with pm4py's
/// defaults.
#[derive(Clone, Debug)]
pub struct TemporalOptions {
    /// The bin width. Default [`GrouperFreq::Week`].
    pub grouper_freq: GrouperFreq,
    /// The activity attribute. Default `concept:name`.
    pub activity_key: String,
    /// The timestamp attribute. Default `time:timestamp`.
    pub timestamp_key: String,
    /// The timestamp that places an event in a bin. Default `time:timestamp`.
    pub start_timestamp_key: String,
    /// The case column. Default `case:concept:name`.
    pub case_id_key: String,
    /// The resource attribute. Default `org:resource`.
    pub resource_key: String,
    /// The prefix of trace attributes in the flat table. Default `case:`.
    pub case_prefix: String,
}

impl Default for TemporalOptions {
    fn default() -> Self {
        TemporalOptions {
            grouper_freq: GrouperFreq::Week,
            activity_key: "concept:name".to_owned(),
            timestamp_key: "time:timestamp".to_owned(),
            start_timestamp_key: "time:timestamp".to_owned(),
            case_id_key: "case:concept:name".to_owned(),
            resource_key: "org:resource".to_owned(),
            case_prefix: "case:".to_owned(),
        }
    }
}

/// The features of one time bin.
#[derive(Clone, Debug, PartialEq)]
pub struct TemporalRow {
    /// The bin's label, in UTC.
    pub timestamp: DateTime<FixedOffset>,
    /// Distinct resources of the bin's events.
    pub unique_resources: usize,
    /// Distinct activities of the bin's events.
    pub unique_activities: usize,
    /// Events in the bin.
    pub num_events: usize,
    /// Mean `@@arrival_rate` of the bin's cases.
    pub average_arrival_rate: f64,
    /// Mean `@@finish_rate` of the bin's cases.
    pub average_finish_rate: f64,
    /// Mean `@@waiting_time` of the bin's cases.
    pub average_waiting_time: f64,
    /// Mean `@@sojourn_time` of the bin's cases.
    pub average_sojourn_time: f64,
    /// Mean `@@service_time` of the bin's cases.
    pub average_service_time: f64,
    /// Over the bin's cases, events minus distinct activities.
    pub total_number_of_reworked_activities: usize,
    /// Mean number of cases per resource.
    pub avg_cases_per_resource: f64,
    /// Mean number of events per case.
    pub avg_events_per_case: f64,
    /// Cases with an event in the bin.
    pub number_of_cases: usize,
    /// Mean number of resources per case.
    pub avg_resources_per_case: f64,
}

/// Features per time bin, as pm4py's `extract_temporal_features_dataframe`
/// computes them.
///
/// Events go into bins by their start timestamp, in UTC. Bins run from the
/// first event's to the last event's, and empty bins give zeros. The
/// case timing columns come from the completion timestamp alone, as in
/// pm4py, so the service time is 0 and the waiting time equals the
/// sojourn time. Means over a bin's cases take each case once. An empty
/// log gives no bins; pm4py fails on it.
pub fn extract_temporal_features_dataframe(
    log: &EventLog,
    options: &TemporalOptions,
) -> Result<Vec<TemporalRow>> {
    let freq = options.grouper_freq;
    if matches!(freq, GrouperFreq::Hours(0) | GrouperFreq::Days(0)) {
        return Err(Error::InvalidOption("grouper_freq must be positive"));
    }
    let table = Table::new(log, &options.case_prefix);
    if table.rows.is_empty() {
        return Ok(Vec::new());
    }
    let cases = table.cases(&options.case_id_key)?;
    let times = case_times(
        &table,
        &cases,
        &options.timestamp_key,
        &options.timestamp_key,
    )?;
    let mut keys = Vec::with_capacity(table.rows.len());
    for (r, &(t, e)) in table.rows.iter().enumerate() {
        let d = event_date(table.event(r), &options.start_timestamp_key, t, e)?;
        keys.push(d.with_timezone(&Utc).naive_utc());
    }
    let origin = keys.iter().min().expect("rows exist").date();
    let mut bins: BTreeMap<NaiveDateTime, Vec<usize>> = BTreeMap::new();
    for (r, k) in keys.iter().enumerate() {
        bins.entry(freq.label(*k, origin)).or_default().push(r);
    }
    let first = *bins.keys().next().expect("rows exist");
    let last = *bins.keys().next_back().expect("rows exist");

    let text = |r: usize, key: &str| table.value(r, key).map(ToString::to_string);
    let mut out = Vec::new();
    let mut label = first;
    let empty = Vec::new();
    while label <= last {
        let rows = bins.get(&label).unwrap_or(&empty);
        let mut per_case: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
        for &r in rows {
            per_case.entry(&cases[r]).or_default().push(r);
        }
        let case_mean = |col: &[f64]| {
            let v: Vec<f64> = per_case.values().map(|rs| col[rs[0]]).collect();
            zero_nan(mean(&v))
        };
        let resources: BTreeSet<String> = rows
            .iter()
            .filter_map(|&r| text(r, &options.resource_key))
            .collect();
        let activities: BTreeSet<String> = rows
            .iter()
            .filter_map(|&r| text(r, &options.activity_key))
            .collect();
        let mut rework = 0;
        let mut events_per_case = Vec::new();
        let mut resources_per_case = Vec::new();
        let mut cases_of_resource: HashMap<String, BTreeSet<&str>> = HashMap::new();
        for (c, rs) in &per_case {
            let acts: BTreeSet<String> = rs
                .iter()
                .filter_map(|&r| text(r, &options.activity_key))
                .collect();
            rework += rs.len() - acts.len();
            events_per_case.push(rs.len() as f64);
            let res: BTreeSet<String> = rs
                .iter()
                .filter_map(|&r| text(r, &options.resource_key))
                .collect();
            resources_per_case.push(res.len() as f64);
            for x in res {
                cases_of_resource.entry(x).or_default().insert(c);
            }
        }
        let mut per_resource: Vec<(&String, f64)> = cases_of_resource
            .iter()
            .map(|(r, c)| (r, c.len() as f64))
            .collect();
        per_resource.sort_by(|a, b| a.0.cmp(b.0));
        let per_resource: Vec<f64> = per_resource.into_iter().map(|x| x.1).collect();
        out.push(TemporalRow {
            timestamp: label.and_utc().fixed_offset(),
            unique_resources: resources.len(),
            unique_activities: activities.len(),
            num_events: rows.len(),
            average_arrival_rate: case_mean(&times.arrival_rate),
            average_finish_rate: case_mean(&times.finish_rate),
            average_waiting_time: case_mean(&times.waiting_time),
            average_sojourn_time: case_mean(&times.sojourn_time),
            average_service_time: case_mean(&times.service_time),
            total_number_of_reworked_activities: rework,
            avg_cases_per_resource: zero_nan(mean(&per_resource)),
            avg_events_per_case: zero_nan(mean(&events_per_case)),
            number_of_cases: per_case.len(),
            avg_resources_per_case: zero_nan(mean(&resources_per_case)),
        });
        label = freq.next(label);
    }
    Ok(out)
}

/// pandas' `fillna(0)`.
fn zero_nan(v: f64) -> f64 {
    if v.is_nan() { 0.0 } else { v }
}
