//! `convert_log_to_time_intervals` against the pm4py goldens in
//! `fixtures/golden/convert`.

mod common;

use ichnos_golden::{JsonCompare, assert_json_eq, golden};
use ichnos_perf::{TimeIntervalOptions, convert_log_to_time_intervals};
use serde_json::{Value, json};

#[test]
fn time_intervals_match_pm4py() {
    for (case, log) in [
        ("time-intervals-running-example", "running-example"),
        ("time-intervals-receipt-couple", "receipt"),
        ("time-intervals-interval-log-couple", "interval_event_log"),
    ] {
        let g = golden("convert", case);
        let p = &g.meta().params;
        let log = common::load(log);
        let mut o = TimeIntervalOptions::default();
        if let Some(v) = p.get("start_timestamp_key") {
            o.start_timestamp_key = v.as_str().unwrap().to_owned();
        }
        if let Some(Value::Array(v)) = p.get("filter_activity_couple") {
            let s = |i: usize| v[i].as_str().unwrap().to_owned();
            o.filter_activity_couple = Some((s(0), s(1)));
        }
        let rows: Vec<Value> = convert_log_to_time_intervals(&log, &o)
            .unwrap()
            .into_iter()
            .map(|i| {
                let case = log.traces[i.trace].case_id().unwrap().to_string();
                json!([i.begin, i.end, case, i.source, i.target])
            })
            .collect();
        assert_json_eq(&json!(rows), &g.expected, &JsonCompare::default());
    }
}

#[test]
fn a_missing_timestamp_is_an_error() {
    let mut log = common::load("running-example");
    log.traces[0].events[0].attributes = Default::default();
    assert!(convert_log_to_time_intervals(&log, &TimeIntervalOptions::default()).is_err());
}
