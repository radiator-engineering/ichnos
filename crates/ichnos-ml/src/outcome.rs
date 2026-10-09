//! Case timing columns and the outcome-enriched table, ported from pm4py's
//! `extract_outcome_enriched_dataframe` and the `insert_case_*` helpers of
//! `util/pandas_utils.py`.

use std::collections::{BTreeMap, HashMap};

use ichnos_core::EventLog;

use crate::error::Result;
use crate::features::{FeatureOptions, FeatureTable, extract_features_dataframe};
use crate::util::{Table, event_seconds};

/// The timing columns pm4py adds to each event, from its case.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CaseTimes {
    /// `@@arrival_rate`: seconds between the case's first start and the
    /// previous case's, with cases ordered by first start; 0 for the first.
    pub arrival_rate: Vec<f64>,
    /// `@@finish_rate`: the same for the last completion.
    pub finish_rate: Vec<f64>,
    /// `@@diff_start_end`: the event's completion minus its start, in
    /// seconds.
    pub diff_start_end: Vec<f64>,
    /// `@@service_time`: the sum of `diff_start_end` over the case.
    pub service_time: Vec<f64>,
    /// `@@sojourn_time`: the case's last completion minus its first start.
    pub sojourn_time: Vec<f64>,
    /// `@@waiting_time`: sojourn time minus service time.
    pub waiting_time: Vec<f64>,
}

/// Computes [`CaseTimes`] for each row of `table`, whose cases are `cases`.
pub(crate) fn case_times(
    table: &Table,
    cases: &[String],
    timestamp_key: &str,
    start_timestamp_key: &str,
) -> Result<CaseTimes> {
    let mut start = Vec::with_capacity(cases.len());
    let mut end = Vec::with_capacity(cases.len());
    for (r, &(t, e)) in table.rows.iter().enumerate() {
        let event = table.event(r);
        start.push(event_seconds(event, start_timestamp_key, t, e)?);
        end.push(event_seconds(event, timestamp_key, t, e)?);
    }
    let mut first: BTreeMap<&str, f64> = BTreeMap::new();
    let mut last: BTreeMap<&str, f64> = BTreeMap::new();
    let mut service: HashMap<&str, f64> = HashMap::new();
    for (r, c) in cases.iter().enumerate() {
        let f = first.entry(c).or_insert(start[r]);
        *f = f.min(start[r]);
        let l = last.entry(c).or_insert(end[r]);
        *l = l.max(end[r]);
        *service.entry(c).or_insert(0.0) += end[r] - start[r];
    }
    let arrival = rates(&first);
    let finish = rates(&last);
    let mut out = CaseTimes::default();
    for (r, c) in cases.iter().enumerate() {
        let c = c.as_str();
        let sojourn = last[c] - first[c];
        out.arrival_rate.push(arrival[c]);
        out.finish_rate.push(finish[c]);
        out.diff_start_end.push(end[r] - start[r]);
        out.service_time.push(service[c]);
        out.sojourn_time.push(sojourn);
        out.waiting_time.push(sojourn - service[c]);
    }
    Ok(out)
}

/// Differences between consecutive times, cases ordered by time then ID.
fn rates<'a>(times: &BTreeMap<&'a str, f64>) -> HashMap<&'a str, f64> {
    let mut sorted: Vec<(&str, f64)> = times.iter().map(|(c, t)| (*c, *t)).collect();
    sorted.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(b.0)));
    let mut out = HashMap::new();
    for i in 0..sorted.len() {
        let rate = if i == 0 {
            0.0
        } else {
            sorted[i].1 - sorted[i - 1].1
        };
        out.insert(sorted[i].0, rate);
    }
    out
}

/// Options for [`extract_outcome_enriched_dataframe`], with pm4py's
/// defaults.
#[derive(Clone, Debug)]
pub struct OutcomeOptions {
    /// The activity attribute. Default `concept:name`.
    pub activity_key: String,
    /// The timestamp attribute. Default `time:timestamp`.
    pub timestamp_key: String,
    /// The start timestamp attribute. Default `time:timestamp`.
    pub start_timestamp_key: String,
    /// The case column. Default `case:concept:name`.
    pub case_id_key: String,
    /// The prefix of trace attributes in the flat table. Default `case:`.
    pub case_prefix: String,
}

impl Default for OutcomeOptions {
    fn default() -> Self {
        OutcomeOptions {
            activity_key: "concept:name".to_owned(),
            timestamp_key: "time:timestamp".to_owned(),
            start_timestamp_key: "time:timestamp".to_owned(),
            case_id_key: "case:concept:name".to_owned(),
            case_prefix: "case:".to_owned(),
        }
    }
}

/// The events of a log with the timing and feature columns of their case.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OutcomeEnriched {
    /// `(trace, event)` positions of the rows, in log order.
    pub events: Vec<(usize, usize)>,
    /// The case of each row.
    pub case_ids: Vec<String>,
    /// The timing columns of each row.
    pub times: CaseTimes,
    /// The case features of [`extract_features_dataframe`] with its
    /// automatic selection.
    pub features: FeatureTable,
    /// For each row, its case's row in `features`.
    pub feature_rows: Vec<usize>,
}

/// Each event with its case's timing columns and case features, as pm4py's
/// `extract_outcome_enriched_dataframe` computes them.
///
/// pm4py merges the features into the event table, so a feature named like
/// an event column gets the suffix `_y` and the column `_x`. Here the event
/// attributes stay in the log and the features stay in their own table,
/// under their own names.
pub fn extract_outcome_enriched_dataframe(
    log: &EventLog,
    options: &OutcomeOptions,
) -> Result<OutcomeEnriched> {
    let features = extract_features_dataframe(
        log,
        &FeatureOptions {
            activity_key: options.activity_key.clone(),
            timestamp_key: options.timestamp_key.clone(),
            case_id_key: options.case_id_key.clone(),
            case_prefix: options.case_prefix.clone(),
            ..FeatureOptions::default()
        },
    )?;
    let table = Table::new(log, &options.case_prefix);
    let case_ids = table.cases(&options.case_id_key)?;
    let times = case_times(
        &table,
        &case_ids,
        &options.timestamp_key,
        &options.start_timestamp_key,
    )?;
    let position: HashMap<&str, usize> = features
        .case_ids
        .iter()
        .enumerate()
        .map(|(i, c)| (c.as_str(), i))
        .collect();
    let feature_rows = case_ids.iter().map(|c| position[c.as_str()]).collect();
    Ok(OutcomeEnriched {
        events: table.rows.clone(),
        case_ids,
        times,
        features,
        feature_rows,
    })
}
