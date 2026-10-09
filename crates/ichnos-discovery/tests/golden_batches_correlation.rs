use ichnos_core::{AttributeValue, Event, EventKeys, EventLog, Trace, chrono::DateTime};
use ichnos_discovery::*;
use ichnos_golden::{Golden, assert_json_eq, fixture_path, golden};
use serde_json::{Value, json};

fn load(g: &Golden) -> (EventLog, EventKeys) {
    let params = &g.meta().params;
    let keys = EventKeys::default()
        .with_activity(params["activity_key"].as_str().unwrap_or("concept:name"));
    let mut log = if let Some(rows) = params["traces"].as_array() {
        let mut log = EventLog::default();
        for row in rows {
            let mut trace = Trace::new();
            trace
                .attributes
                .insert("concept:name", row[0].as_str().unwrap());
            for ev in row[1].as_array().unwrap() {
                let mut event = Event::new();
                event
                    .attributes
                    .insert(keys.activity.as_str(), ev[0].as_str().unwrap());
                event
                    .attributes
                    .insert(keys.resource.as_str(), ev[1].as_str().unwrap());
                for (key, value) in [(&keys.start_timestamp, &ev[2]), (&keys.timestamp, &ev[3])] {
                    event.attributes.insert(
                        key.as_str(),
                        AttributeValue::Date(
                            DateTime::parse_from_rfc3339(value.as_str().unwrap()).unwrap(),
                        ),
                    );
                }
                trace.events.push(event);
            }
            log.traces.push(trace);
        }
        log
    } else {
        let path = fixture_path(
            g.meta().fixtures["log"]
                .strip_prefix("fixtures/logs/")
                .unwrap(),
        );
        if path.extension().unwrap() == "xes" {
            ichnos_io::read_xes(path, &Default::default()).unwrap()
        } else {
            ichnos_io::read_csv(path, &Default::default()).unwrap()
        }
    };
    if let Some(resource) = params["fill_missing_resource"].as_str() {
        for event in log.traces.iter_mut().flat_map(|trace| &mut trace.events) {
            if event.get(&keys.resource).is_none() {
                event.attributes.insert(keys.resource.as_str(), resource);
            }
        }
    }
    (log, keys)
}
fn batches_json(groups: &[BatchGroup]) -> Value {
    json!(groups.iter().map(|group| {
        let batches: std::collections::BTreeMap<_,_> = group.batches.iter().map(|(kind,batches)| {
            let mut batches: Vec<_> = batches.iter().map(|batch| json!([batch.start,batch.end,batch.events.iter().map(|e| json!([e.start,e.end,e.case])).collect::<Vec<_>>()])).collect();
            batches.sort_by(|a,b| a[0].as_f64().unwrap().total_cmp(&b[0].as_f64().unwrap()).then(a[1].as_f64().unwrap().total_cmp(&b[1].as_f64().unwrap())).then(a[2].to_string().cmp(&b[2].to_string())));
            (kind.as_str(),batches)
        }).collect();
        json!({"activity":group.activity,"resource":group.resource,"count":group.count(),"batches":batches})
    }).collect::<Vec<_>>())
}
fn check(name: &str) {
    let g = golden("discovery", &format!("batches-correlation-{name}"));
    let expected: Value = g.expected_as();
    let (log, keys) = load(&g);
    let before = log.clone();
    let interval = g.meta().params["interval"].as_bool().unwrap_or(false);
    for row in expected["batches"].as_array().unwrap() {
        let options = BatchOptions {
            merge_distance: row["distance"].as_f64().unwrap(),
            min_batch_size: row["size"].as_u64().unwrap() as usize,
            use_start_timestamp: interval,
            ..Default::default()
        };
        let batches = discover_batches(&log, &keys, &options).unwrap();
        let mut model = batches_json(&batches);
        if row["model"].is_object() {
            use sha2::{Digest, Sha256};
            let mut samples = Vec::new();
            let mut batch_count = 0usize;
            for group in model.as_array_mut().unwrap() {
                batch_count += group["count"].as_u64().unwrap() as usize;
                let activity = group["activity"].clone();
                let resource = group["resource"].clone();
                for (kind, batches) in group["batches"].as_object_mut().unwrap() {
                    for batch in batches.as_array_mut().unwrap() {
                        for i in 0..2 {
                            batch[i] =
                                json!((batch[i].as_f64().unwrap() * 1e6).round_ties_even() as i64);
                        }
                        let start = batch[0].clone();
                        let end = batch[1].clone();
                        for event in batch[2].as_array_mut().unwrap() {
                            for i in 0..2 {
                                event[i] = json!(
                                    (event[i].as_f64().unwrap() * 1e6).round_ties_even() as i64
                                );
                            }
                            samples.push(json!([
                                activity, resource, kind, start, end, event[0], event[1], event[2]
                            ]));
                        }
                    }
                }
            }
            // Fixture labels and category strings have the same lexicographic
            // UTF-8 order as the reference; timestamps follow numerically.
            samples.sort_by(|a, b| {
                (0..8)
                    .find_map(|i| {
                        let ordering = if let (Some(a), Some(b)) = (a[i].as_i64(), b[i].as_i64()) {
                            a.cmp(&b)
                        } else {
                            a[i].as_str().unwrap().cmp(b[i].as_str().unwrap())
                        };
                        (ordering != std::cmp::Ordering::Equal).then_some(ordering)
                    })
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            assert_eq!(
                json!(model.as_array().unwrap().len()),
                row["model"]["groups"]
            );
            assert_eq!(json!(batch_count), row["model"]["batches"]);
            assert_eq!(json!(samples.len()), row["model"]["events"]);
            assert_eq!(
                format!("{:x}", Sha256::digest(serde_json::to_vec(&model).unwrap())),
                row["model"]["sha256"].as_str().unwrap()
            );
            assert_eq!(
                json!(&samples[..samples.len().min(3)]),
                row["model"]["samples"]
            );
        } else {
            assert_json_eq(
                &model,
                &row["model"],
                &ichnos_golden::JsonCompare::default()
                    .tolerance(ichnos_golden::Tolerance::absolute(1e-6)),
            );
        }
    }
    for row in expected["correlation"].as_array().unwrap() {
        let result = correlation_miner(
            &log,
            &keys,
            &CorrelationOptions {
                use_start_timestamp: interval,
                exact_time_matching: row["exact"].as_bool().unwrap(),
            },
        )
        .unwrap();
        let frequency = json!(
            result
                .dfg
                .graph
                .iter()
                .map(|((a, b), n)| json!([a, b, n]))
                .collect::<Vec<_>>()
        );
        assert_json_eq(&frequency, &row["frequency"], &Default::default());
        assert_json_eq(
            &json!(
                result
                    .performance
                    .iter()
                    .map(|((a, b), n)| json!([a, b, n]))
                    .collect::<Vec<_>>()
            ),
            &row["performance"],
            &Default::default(),
        );
        assert_eq!(json!(result.dfg.start_activities), row["starts"]);
        assert_eq!(json!(result.dfg.end_activities), row["ends"]);
        assert_eq!(json!(result.activity_counts), row["counts"]);
        assert_json_eq(
            &json!(
                result
                    .statistics
                    .iter()
                    .map(|((a, b), s)| json!([a, b, s.precede_succeed, s.duration, s.cost]))
                    .collect::<Vec<_>>()
            ),
            &row["statistics"],
            &Default::default(),
        );
        let objective = result
            .dfg
            .graph
            .iter()
            .map(|(edge, &n)| n as f64 * result.statistics[edge].cost)
            .sum::<f64>();
        ichnos_golden::assert_close(
            objective,
            row["objective"].as_f64().unwrap(),
            ichnos_golden::Tolerance::METRIC,
        );
        for (activity, count) in &result.activity_counts {
            assert_eq!(
                result
                    .dfg
                    .graph
                    .iter()
                    .filter(|((a, _), _)| a == activity)
                    .map(|(_, n)| n)
                    .sum::<u64>(),
                *count
            );
            assert_eq!(
                result
                    .dfg
                    .graph
                    .iter()
                    .filter(|((_, b), _)| b == activity)
                    .map(|(_, n)| n)
                    .sum::<u64>(),
                *count
            );
        }
    }
    if expected["correlation"].as_array().unwrap().is_empty() {
        assert!(
            correlation_miner(&log, &keys, &Default::default())
                .unwrap()
                .dfg
                .graph
                .is_empty()
        );
    }
    assert_eq!(log, before);
}
macro_rules! cases { ($($test:ident => $name:literal),*) => { $(#[test] fn $test() { check($name); })* }; }
cases!(running_example=>"running-example-xes",receipt=>"receipt-xes",roadtraffic=>"roadtraffic100traces-xes",even=>"interleavings-receipt_even-csv",odd=>"interleavings-receipt_odd-csv",empty=>"empty",empty_traces=>"empty-traces",five_types=>"five-types",duplicates=>"duplicates",heap_order=>"heap-order",intervals=>"intervals",custom_key=>"custom-key");

#[test]
fn typed_errors() {
    let log = EventLog::default();
    let keys = EventKeys::default();
    for distance in [f64::NAN, f64::INFINITY, -1.0] {
        assert!(matches!(
            discover_batches(
                &log,
                &keys,
                &BatchOptions {
                    merge_distance: distance,
                    ..Default::default()
                }
            ),
            Err(Error::InvalidOption(_))
        ));
    }
    assert!(
        discover_batches(
            &log,
            &keys,
            &BatchOptions {
                min_batch_size: 0,
                ..Default::default()
            }
        )
        .is_err()
    );
    let mut trace = Trace::new();
    let mut event = Event::new();
    event.attributes.insert("concept:name", "a");
    trace.events.push(event);
    let log = EventLog {
        traces: vec![trace],
        ..Default::default()
    };
    assert!(matches!(
        discover_batches(&log, &keys, &Default::default()),
        Err(Error::BatchCase { trace: 0, .. })
    ));
    assert!(matches!(
        correlation_miner(&log, &keys, &Default::default()),
        Err(Error::Core(ichnos_core::Error::MissingAttribute {
            position: ichnos_core::Position::Event { trace: 0, event: 0 },
            ..
        }))
    ));
}
