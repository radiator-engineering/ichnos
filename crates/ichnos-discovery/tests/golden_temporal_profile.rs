//! Compares temporal profile discovery with pm4py's golden output in
//! `fixtures/golden/discovery/temporal-profile-*.json`.
//!
//! Each golden holds pm4py's log and dataframe variants, each with elapsed
//! time and with business hours. ichnos has one implementation, so its
//! profile must match all four records of its time measure.

// This test uses only the log loader of the shared helpers.
#[allow(dead_code)]
mod common;

use common::load_csv_log;
use ichnos_core::EventKeys;
use ichnos_discovery::{TemporalProfile, TemporalProfileOptions, discover_temporal_profile};
use ichnos_golden::{Tolerance, cases, compare_close, golden};
use ichnos_stats::time::BusinessHours;
use serde_json::Value;

/// Times are seconds, so allow a microsecond besides the relative error.
const SECONDS: Tolerance = Tolerance {
    abs: 1e-6,
    rel: 1e-6,
};

fn check(profile: &TemporalProfile, expected: &Value, what: &str) {
    let rows = expected.as_array().expect("profile rows");
    let pairs: Vec<(&str, &str)> = profile
        .keys()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    let expected_pairs: Vec<(&str, &str)> = rows
        .iter()
        .map(|r| (r[0].as_str().expect("from"), r[1].as_str().expect("to")))
        .collect();
    assert_eq!(pairs, expected_pairs, "{what}: activity pairs");
    for (((a, b), (mean, stdev)), row) in profile.iter().zip(rows) {
        for (name, actual, expected) in [("mean", mean, &row[2]), ("stdev", stdev, &row[3])] {
            let expected = expected.as_f64().expect("number");
            if let Err(m) = compare_close(*actual, expected, SECONDS) {
                panic!("{what}: {name} of ({a}, {b}): {m}");
            }
        }
    }
}

#[test]
fn temporal_profile_matches_pm4py() {
    let ids: Vec<String> = cases("discovery")
        .into_iter()
        .filter(|id| id.starts_with("temporal-profile-"))
        .collect();
    assert_eq!(
        ids.len(),
        4,
        "expected 4 temporal profile goldens, found {ids:?}"
    );
    let keys = EventKeys::default();
    for id in ids {
        let g = golden("discovery", &id);
        let log = load_csv_log(&g.fixture("log"));
        let use_start_timestamp = g.meta["params"].get("start_timestamp_key").is_some();
        for (measure, business_hours) in [
            ("elapsed", None),
            ("business", Some(BusinessHours::default())),
        ] {
            let options = TemporalProfileOptions {
                use_start_timestamp,
                business_hours,
            };
            let profile = discover_temporal_profile(&log, &keys, &options).expect("profile");
            for variant in ["log", "dataframe"] {
                let field = format!("{variant}_{measure}");
                check(&profile, &g.expected[&field], &format!("{id} {field}"));
            }
        }
    }
}
