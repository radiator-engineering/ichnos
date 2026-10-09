use ichnos_core::{
    Event, EventLog, Trace,
    chrono::{TimeZone, Utc},
};
use ichnos_golden::{JsonCompare, assert_json_eq, fixture_path, golden};
use ichnos_perf::{
    CaseTimeOptions, insert_case_arrival_finish_rate, insert_case_service_waiting_time,
};
use serde_json::{Value, json};
fn cases(log: &EventLog, o: &CaseTimeOptions) -> Value {
    let mut rows = std::collections::BTreeMap::new();
    for t in &log.traces {
        if let Some(e) = t.events.first() {
            let id = t.case_id().unwrap();
            let id = id
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| id.as_i64().unwrap().to_string());
            rows.insert(
                id.clone(),
                json!([
                    id,
                    e.get(&o.service).unwrap().as_f64().unwrap(),
                    e.get(&o.sojourn).unwrap().as_f64().unwrap(),
                    e.get(&o.waiting).unwrap().as_f64().unwrap(),
                    e.get(&o.arrival).unwrap().as_f64().unwrap(),
                    e.get(&o.finish).unwrap().as_f64().unwrap()
                ]),
            );
        }
    }
    json!(rows.into_values().collect::<Vec<_>>())
}
#[test]
fn enrichment_matches_complete_real_logs() {
    let o = CaseTimeOptions::default();
    for name in ["running-example", "receipt", "roadtraffic100traces"] {
        let log =
            ichnos_io::xes::read_xes(fixture_path(format!("{name}.xes")), &Default::default())
                .unwrap();
        let before = log.clone();
        let service = insert_case_service_waiting_time(&log, &o).unwrap();
        let out = insert_case_arrival_finish_rate(&service, &o).unwrap();
        assert_json_eq(
            &cases(&out, &o),
            &golden("analysis_remaining", &format!("times-{name}")).expected["cases"],
            &JsonCompare::default(),
        );
        assert_eq!(log, before);
        assert_eq!(out.attributes, log.attributes);
        assert_eq!(out.extensions, log.extensions);
        for trace in &out.traces {
            for e in &trace.events {
                assert_eq!(e.get(&o.event_duration).unwrap().as_f64(), Some(0.0));
            }
        }
    }
}
#[test]
fn overlapping_negative_intervals_and_numeric_ties_match() {
    let g = golden("analysis_remaining", "times-intervals");
    let mut log = EventLog::default();
    for row in g.expected["events"].as_array().unwrap() {
        let mut t = Trace::with_case_id(row[0].as_i64().unwrap());
        let mut e = Event::new();
        e.insert(
            "time:start",
            Utc.timestamp_opt(row[1].as_i64().unwrap(), 0)
                .unwrap()
                .fixed_offset(),
        );
        e.insert(
            "time:timestamp",
            Utc.timestamp_opt(row[2].as_i64().unwrap(), 0)
                .unwrap()
                .fixed_offset(),
        );
        t.events.push(e);
        log.traces.push(t);
    }
    let o = CaseTimeOptions {
        start_timestamp: "time:start".into(),
        ..Default::default()
    };
    let out =
        insert_case_arrival_finish_rate(&insert_case_service_waiting_time(&log, &o).unwrap(), &o)
            .unwrap();
    assert_json_eq(
        &cases(&out, &o),
        &g.expected["cases"],
        &JsonCompare::default(),
    );
    let durations: Vec<_> = out
        .traces
        .iter()
        .map(|t| {
            t.events[0]
                .get(&o.event_duration)
                .unwrap()
                .as_f64()
                .unwrap()
        })
        .collect();
    assert_json_eq(
        &json!(durations),
        &g.expected["durations"],
        &JsonCompare::default(),
    );
    log.traces[0].events[0].attributes.remove("time:start");
    let before = log.clone();
    assert!(insert_case_service_waiting_time(&log, &o).is_err());
    assert_eq!(log, before);
    assert!(
        insert_case_arrival_finish_rate(&EventLog::default(), &o)
            .unwrap()
            .traces
            .is_empty()
    );
}
