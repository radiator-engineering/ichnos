use chrono::{DateTime, Utc};
use ichnos_core::{Event, EventLog, Trace};
use ichnos_golden::{cases, golden};
use ichnos_privacy::{PrivacyOptions, anonymize_differential_privacy, trace_variant_query};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde_json::json;
use std::collections::BTreeMap;
fn input(sequences: &[Vec<String>]) -> EventLog {
    let mut log = EventLog::default();
    for (i, sequence) in sequences.iter().enumerate() {
        let mut trace = Trace::with_case_id(i.to_string());
        trace.attributes.insert("private", "source-id");
        for (j, a) in sequence.iter().enumerate() {
            let mut e = Event::new();
            e.insert("concept:name", a.as_str());
            e.insert(
                "time:timestamp",
                DateTime::<Utc>::from_timestamp(1577836800 + (100 * i + j) as i64, 0)
                    .unwrap()
                    .fixed_offset(),
            );
            e.insert("cost", (i % 11) as f64);
            e.insert("flag", i % 2 == 0);
            e.insert("category", if i % 2 == 0 { "red" } else { "blue" });
            trace.events.push(e);
        }
        log.traces.push(trace);
    }
    log
}
fn counts(sequences: impl IntoIterator<Item = Vec<String>>) -> Vec<serde_json::Value> {
    let mut map = BTreeMap::new();
    for seq in sequences {
        *map.entry(seq).or_insert(0usize) += 1;
    }
    map.into_iter().map(|(seq, n)| json!([seq, n])).collect()
}
#[test]
fn entry_point_real_projections_and_synthetic() {
    for id in cases("simulation").into_iter().filter(|s| {
        s.starts_with("privacy-")
            && !s.starts_with("privacy-mechanisms")
            && s != "privacy-epsilon-one-distribution"
            && s != "privacy-timestamp-sensitivity"
            && s != "privacy-behavioral-relations"
    }) {
        let g = golden("simulation", &id);
        let e = &g.expected;
        let seq: Vec<Vec<String>> = serde_json::from_value(e["input"].clone()).unwrap();
        let source = input(&seq);
        let snapshot = source.clone();
        let options = PrivacyOptions {
            epsilon: 10.,
            max_prefix_length: 3,
            pruning_count: 2,
            ..Default::default()
        };
        let query =
            trace_variant_query(&source, &options, &mut ChaCha8Rng::seed_from_u64(1729)).unwrap();
        assert_eq!(json!(counts(query)), e["query_variants"], "{id}");
        let output =
            anonymize_differential_privacy(&source, &options, &mut ChaCha8Rng::seed_from_u64(1729))
                .unwrap();
        assert_eq!(source, snapshot);
        assert_eq!(
            output,
            anonymize_differential_privacy(&source, &options, &mut ChaCha8Rng::seed_from_u64(1729))
                .unwrap()
        );
        let sequences = output.traces.iter().map(|t| {
            t.events
                .iter()
                .map(|e| e.get("concept:name").unwrap().as_str().unwrap().to_string())
                .collect()
        });
        assert_eq!(json!(counts(sequences)), e["output_variants"], "{id}");
        assert_eq!(
            output.traces.len(),
            e["invariants"]["traces"].as_u64().unwrap() as usize
        );
        for (i, t) in output.traces.iter().enumerate() {
            assert_eq!(t.case_id().unwrap().as_str(), Some(i.to_string().as_str()));
            assert!(t.attributes.get("private").is_none());
            assert!(t.len() <= 3);
            for event in &t.events {
                assert!((0. ..=10.).contains(&event.get("cost").unwrap().as_f64().unwrap()));
                assert!(event.get("flag").unwrap().as_bool().is_some());
                assert!(matches!(
                    event.get("category").unwrap().as_str(),
                    Some("red" | "blue")
                ));
            }
            assert!(
                t.events
                    .windows(2)
                    .all(|w| w[0].get("time:timestamp").unwrap().as_date()
                        <= w[1].get("time:timestamp").unwrap().as_date())
            );
        }
    }
}
#[test]
fn categorical_context_blocklist_and_resource_errors() {
    let mut source = input(&vec![vec!["a".into(), "b".into()]; 30]);
    for (i, t) in source.traces.iter_mut().enumerate() {
        for e in &mut t.events {
            e.insert("category", if i % 2 == 0 { "red" } else { "blue" });
            e.insert("identifier", i as i64);
            e.insert("number", (i % 11) as i64);
            e.insert("constant", 3.5);
        }
    }
    let options = PrivacyOptions {
        epsilon: 1.,
        pruning_count: 2,
        max_prefix_length: 3,
        blocklist: ["identifier".into()].into_iter().collect(),
        ..Default::default()
    };
    let log = anonymize_differential_privacy(&source, &options, &mut ChaCha8Rng::seed_from_u64(77))
        .unwrap();
    assert!(log.traces.iter().flat_map(|t| &t.events).all(|e| {
        e.get("number")
            .unwrap()
            .as_i64()
            .is_some_and(|v| (0..=10).contains(&v))
            && e.get("constant").unwrap().as_f64() == Some(3.5)
            && e.get("identifier").is_none()
            && matches!(e.get("category").unwrap().as_str(), Some("red" | "blue"))
    }));
    let bound = PrivacyOptions {
        max_matching_cells: 1,
        ..options.clone()
    };
    assert!(
        anonymize_differential_privacy(&source, &bound, &mut ChaCha8Rng::seed_from_u64(77))
            .is_err()
    );
    let invalid = PrivacyOptions {
        epsilon: 0.,
        ..options
    };
    assert!(trace_variant_query(&source, &invalid, &mut ChaCha8Rng::seed_from_u64(77)).is_err());
}

#[test]
fn privacy_entry_point_at_epsilon_one_has_noise_distribution() {
    let g = golden("simulation", "privacy-epsilon-one-distribution");
    let e = &g.expected;
    let n = e["seeds"].as_u64().unwrap() as usize;
    let source = input(
        &(0..50)
            .map(|i| {
                if i % 2 == 0 {
                    vec!["a".into(), "b".into(), "c".into()]
                } else {
                    vec!["a".into(), "c".into()]
                }
            })
            .collect::<Vec<_>>(),
    );
    let options = PrivacyOptions {
        epsilon: 1.,
        max_prefix_length: 3,
        pruning_count: 2,
        ..Default::default()
    };
    let mut totals = Vec::new();
    let mut means = [0.; 4];
    for seed in 0..n {
        let output = anonymize_differential_privacy(
            &source,
            &options,
            &mut ChaCha8Rng::seed_from_u64(seed as u64),
        )
        .unwrap();
        totals.push(output.traces.len() as f64);
        let events: Vec<_> = output.traces.iter().flat_map(|t| &t.events).collect();
        means[0] += events.len() as f64 / output.traces.len().max(1) as f64;
        for event in &events {
            means[1] += event.get("cost").unwrap().as_f64().unwrap() / events.len() as f64;
            means[2] +=
                f64::from(event.get("flag").unwrap().as_bool().unwrap()) / events.len() as f64;
            means[3] += f64::from(event.get("category").unwrap().as_str() == Some("red"))
                / events.len() as f64;
        }
    }
    let mean = totals.iter().sum::<f64>() / n as f64;
    let std = (totals.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64).sqrt();
    for (key, actual, tolerance) in [
        ("mean_traces", mean, 2.),
        ("std_traces", std, 0.4),
        ("mean_length", means[0] / n as f64, 0.08),
        ("cost_mean", means[1] / n as f64, 0.2),
        ("flag_rate", means[2] / n as f64, 0.06),
        ("category_red", means[3] / n as f64, 0.06),
    ] {
        let expected = e[key].as_f64().unwrap();
        println!("{key}: Rust {actual}, pm4py {expected}");
        assert!(
            (actual - expected).abs() <= tolerance,
            "{key}: {actual} vs {expected}"
        );
    }
    assert!(std > 1.5, "a noiseless trace query must fail this check");
}
