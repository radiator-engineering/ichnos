use ichnos_core::{Event, EventLog, Trace};
use ichnos_golden::{JsonCompare, assert_json_eq, fixture_path, golden};
use ichnos_ml::profiles::{self, Clusterer, KMeans, ProfileOptions};
use serde_json::json;
use sha2::{Digest, Sha256};
#[test]
fn complete_real_log_profiles_and_explicit_lloyd_match() {
    for name in ["running-example", "receipt", "roadtraffic100traces"] {
        let g = golden("analysis_remaining", &format!("clusters-{name}"));
        let e = &g.expected;
        let log =
            ichnos_io::xes::read_xes(fixture_path(format!("{name}.xes")), &Default::default())
                .unwrap();
        let options = ProfileOptions::activities("concept:name");
        let data = profiles::trace_profiles(&log, &options).unwrap();
        assert_eq!(json!(data.names), e["names"]);
        assert_eq!(data.rows.len(), e["rows"].as_u64().unwrap() as usize);
        let bytes: Vec<u8> = data
            .rows
            .iter()
            .flatten()
            .map(|x| u8::from(*x == 1.0))
            .collect();
        assert_eq!(
            format!("{:x}", Sha256::digest(bytes)),
            e["profile_sha256"].as_str().unwrap()
        );
        let centers: Vec<Vec<f64>> = serde_json::from_value(e["centers"].clone()).unwrap();
        let clusterer = KMeans {
            initial_centers: Some(centers),
            ..Default::default()
        };
        assert_eq!(
            clusterer.fit_predict(&data.rows).unwrap(),
            KMeans::default().fit_predict(&data.rows).unwrap()
        );
        let logs = profiles::cluster_log(&log, &options, &clusterer).unwrap();
        let groups: Vec<Vec<_>> = logs
            .iter()
            .map(|l| {
                l.traces
                    .iter()
                    .map(|t| t.case_id().unwrap().as_str().unwrap().to_string())
                    .collect()
            })
            .collect();
        assert_eq!(json!(groups), e["groups"], "{name}");
        assert_eq!(
            json!(profiles::activity_labels(&log, "concept:name").unwrap()),
            golden("analysis_remaining", &format!("times-{name}")).expected["labels"]
        );
    }
}
#[test]
fn numeric_and_string_profiles_match() {
    let g = golden("analysis_remaining", "profiles-numeric");
    let e = &g.expected;
    let mut log = EventLog::default();
    for (i, row) in e["input"].as_array().unwrap().iter().enumerate() {
        let mut t = Trace::with_case_id(i.to_string());
        t.attributes.insert("group", row["group"].as_str().unwrap());
        t.attributes.insert("score", row["score"].as_i64().unwrap());
        for p in row["events"].as_array().unwrap() {
            let mut event = Event::new();
            event.insert("concept:name", p[0].as_str().unwrap());
            event.insert("cost", p[1].as_i64().unwrap());
            t.events.push(event);
        }
        log.traces.push(t);
    }
    let options = ProfileOptions {
        string_trace: vec!["group".into()],
        numeric_trace: vec!["score".into()],
        numeric_event: vec!["cost".into()],
        ..ProfileOptions::activities("concept:name")
    };
    let data = profiles::trace_profiles(&log, &options).unwrap();
    assert_eq!(json!(data.names), e["names"]);
    assert_json_eq(&json!(data.rows), &e["data"], &JsonCompare::default());
    let clusterer = KMeans {
        initial_centers: Some(vec![data.rows[0].clone(), data.rows[2].clone()]),
        ..Default::default()
    };
    let groups: Vec<Vec<_>> = profiles::cluster_log(&log, &options, &clusterer)
        .unwrap()
        .iter()
        .map(|l| {
            l.traces
                .iter()
                .map(|t| t.case_id().unwrap().as_str().unwrap().to_string())
                .collect()
        })
        .collect();
    assert_eq!(json!(groups), e["groups"]);
    let inferred = ProfileOptions::infer(&log, "concept:name");
    assert!(!inferred.string_trace.contains(&"concept:name".into()));
    assert!(inferred.numeric_event.contains(&"cost".into()));
    let inferred_data = profiles::trace_profiles(&log, &inferred).unwrap();
    assert_eq!(json!(inferred_data.names), e["inferred_names"]);
    assert_json_eq(
        &json!(inferred_data.rows),
        &e["inferred_data"],
        &JsonCompare::default(),
    );
    assert_eq!(
        clusterer.fit_predict(&data.rows).unwrap(),
        KMeans::default().fit_predict(&data.rows).unwrap()
    );
}
#[test]
fn invalid_inputs_and_empty_clusters_are_checked() {
    let k = KMeans::default();
    assert!(k.fit_predict(&[]).is_err());
    assert!(k.fit_predict(&[vec![f64::NAN], vec![1.0]]).is_err());
    assert!(k.fit_predict(&[vec![1.0], vec![1.0, 2.0]]).is_err());
    assert_eq!(
        k.fit_predict(&[vec![1.0], vec![1.0], vec![1.0]]).unwrap(),
        vec![0, 0, 0]
    );
    let k = KMeans {
        initial_centers: Some(vec![vec![0.0], vec![0.0]]),
        ..Default::default()
    };
    assert!(
        k.fit_predict(&[vec![0.0], vec![0.0], vec![10.0]])
            .unwrap()
            .contains(&1)
    );
    let log = EventLog::from_trace_strings(["A,B", "B,A"], ",", &Default::default());
    let before = log.clone();
    let o = ProfileOptions {
        numeric_trace: vec!["missing".into()],
        ..Default::default()
    };
    assert!(profiles::trace_profiles(&log, &o).is_err());
    assert_eq!(log, before);
}

#[test]
fn custom_assignments_keep_empty_indices_and_metadata() {
    struct Fixed(Vec<usize>);
    impl Clusterer for Fixed {
        fn fit_predict(&self, _: &[Vec<f64>]) -> Result<Vec<usize>, profiles::Error> {
            Ok(self.0.clone())
        }
    }
    let mut log = EventLog::from_trace_strings(["A", "B", "C"], ",", &Default::default());
    log.attributes.insert("source", "example");
    let options = ProfileOptions::activities("concept:name");
    let clusters = profiles::cluster_log(&log, &options, &Fixed(vec![0, 2, 2])).unwrap();
    assert_eq!(clusters.len(), 3);
    assert!(clusters[1].traces.is_empty());
    assert_eq!(clusters[2].traces, log.traces[1..]);
    assert_eq!(clusters[0].attributes, log.attributes);
    assert!(profiles::cluster_log(&log, &options, &Fixed(vec![0])).is_err());
    assert!(profiles::cluster_log(&log, &options, &Fixed(vec![0, 9, 2])).is_err());
    assert!(
        KMeans::default()
            .fit_predict(&[vec![f64::MAX], vec![-f64::MAX]])
            .is_err()
    );
}
