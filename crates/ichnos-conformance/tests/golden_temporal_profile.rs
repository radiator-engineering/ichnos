//! Temporal profile conformance against pm4py
//! (`fixtures/golden/conformance/temporal-profile-*`).
//!
//! Each golden checks a log against profiles from pm4py's dataframe variant
//! of `discover_temporal_profile`, under several settings. ichnos must report
//! the deviations of pm4py's dataframe variant: as many as pm4py for every
//! trace, and the same ones for the traces whose rows the golden records. pm4py's log variant reports
//! the same ones plus `log_extra`; the test checks that each of those comes
//! from float noise on a pair with a standard deviation of 0.

mod common;

use common::load_csv_log;
use ichnos_conformance::temporal_profile::{
    TemporalConformanceOptions, TemporalDeviation, TemporalProfile, TemporalProfileOptions,
    conformance_temporal_profile,
};
use ichnos_core::EventKeys;
use ichnos_golden::{Tolerance, cases, compare_close, golden};
use ichnos_stats::time::BusinessHours;
use serde_json::Value;

/// Times are seconds, so allow a microsecond besides the relative error.
const SECONDS: Tolerance = Tolerance {
    abs: 1e-6,
    rel: 1e-6,
};

/// pm4py's `sys.maxsize`, its zeta for a standard deviation of 0.
const MAXSIZE: u64 = i64::MAX as u64;

fn zeta(v: &Value) -> f64 {
    if v.as_u64() == Some(MAXSIZE) {
        f64::INFINITY
    } else {
        v.as_f64().expect("zeta")
    }
}

fn profile(rows: &Value) -> TemporalProfile {
    rows.as_array()
        .expect("profile")
        .iter()
        .map(|r| {
            let text = |i: usize| r[i].as_str().expect("activity").to_owned();
            let number = |i: usize| r[i].as_f64().expect("number");
            ((text(0), text(1)), (number(2), number(3)))
        })
        .collect()
}

fn check_trace(actual: &[TemporalDeviation], expected: &Value, what: &str) {
    let mut actual: Vec<&TemporalDeviation> = actual.iter().collect();
    actual.sort_by(|a, b| {
        (&a.from, &a.to)
            .cmp(&(&b.from, &b.to))
            .then(a.seconds.total_cmp(&b.seconds))
    });
    let expected = expected.as_array().expect("deviations");
    assert_eq!(actual.len(), expected.len(), "{what}: deviation count");
    for (a, e) in actual.iter().zip(expected) {
        let pair = (e[0].as_str().expect("from"), e[1].as_str().expect("to"));
        assert_eq!((a.from.as_str(), a.to.as_str()), pair, "{what}: pair");
        if let Err(m) = compare_close(a.seconds, e[2].as_f64().expect("seconds"), SECONDS) {
            panic!("{what}: seconds of {pair:?}: {m}");
        }
        let expected_zeta = zeta(&e[3]);
        if expected_zeta.is_infinite() {
            assert!(a.zeta.is_infinite(), "{what}: zeta of {pair:?}");
        } else if let Err(m) = compare_close(a.zeta, expected_zeta, SECONDS) {
            panic!("{what}: zeta of {pair:?}: {m}");
        }
    }
}

#[test]
fn temporal_profile_conformance_matches_pm4py() {
    let ids: Vec<String> = cases("conformance")
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
        let g = golden("conformance", &id);
        let log = load_csv_log(&g.fixture("log"));
        let use_start_timestamp = g.meta["params"].get("start_timestamp_key").is_some();
        let settings = g.expected.as_object().expect("settings");
        for (name, e) in settings {
            let what = format!("{id} {name}");
            let options = TemporalConformanceOptions {
                zeta: e["zeta"].as_f64().expect("zeta"),
                time: TemporalProfileOptions {
                    use_start_timestamp,
                    business_hours: e["business_hours"]
                        .as_bool()
                        .expect("business_hours")
                        .then(BusinessHours::default),
                },
            };
            let profile = profile(&e["profile"]);
            let result =
                conformance_temporal_profile(&log, &keys, &profile, &options).expect("conformance");
            let counts = e["dataframe"]["counts"].as_array().expect("counts");
            assert_eq!(result.len(), counts.len(), "{what}: traces");
            for (t, (actual, count)) in result.iter().zip(counts).enumerate() {
                assert_eq!(
                    actual.len() as u64,
                    count.as_u64().expect("count"),
                    "{what}: trace {t}: deviation count"
                );
            }
            let rows = e["dataframe"]["rows"].as_array().expect("rows");
            for (t, (actual, expected)) in result.iter().zip(rows).enumerate() {
                check_trace(actual, expected, &format!("{what}: trace {t}"));
            }

            // The log variant's extra deviations: a pair with a standard
            // deviation of 0 whose float time misses the mean by noise.
            for extra in e["log_extra"].as_array().expect("log_extra") {
                let pair = (
                    extra[1].as_str().expect("from").to_owned(),
                    extra[2].as_str().expect("to").to_owned(),
                );
                let (mean, stdev) = profile[&pair];
                let seconds = extra[3].as_f64().expect("seconds");
                assert!(
                    stdev == 0.0 && zeta(&extra[4]).is_infinite(),
                    "{what}: log extra {extra}"
                );
                assert!(
                    compare_close(seconds, mean, SECONDS).is_ok(),
                    "{what}: log extra {extra} is not float noise"
                );
            }
        }
    }
}
