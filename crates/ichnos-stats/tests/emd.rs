use ichnos_golden::{Tolerance, assert_close, golden};
use ichnos_stats::emd::{StochasticLanguage, compute_emd, normalized_trace_distance};
use serde_json::Value;
fn language(v: &Value) -> StochasticLanguage {
    v.as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p[0].as_array()
                    .unwrap()
                    .iter()
                    .map(|s| s.as_str().unwrap().into())
                    .collect(),
                p[1].as_f64().unwrap(),
            )
        })
        .collect()
}
fn check(v: &Value) {
    let a = language(&v["a"]);
    let b = language(&v["b"]);
    let expected = v["distance"].as_f64().unwrap();
    assert_close(compute_emd(&a, &b).unwrap(), expected, Tolerance::METRIC);
    assert_close(compute_emd(&b, &a).unwrap(), expected, Tolerance::METRIC);
    assert_close(compute_emd(&a, &a).unwrap(), 0.0, Tolerance::METRIC);
}
#[test]
fn transport_matches_oracle() {
    for c in golden("analysis_remaining", "emd")
        .expected
        .as_array()
        .unwrap()
    {
        check(c);
    }
    for name in ["running-example", "receipt", "roadtraffic100traces"] {
        check(&golden("analysis_remaining", &format!("emd-{name}")).expected);
    }
}
#[test]
fn transport_validates_mass_and_empty_traces() {
    let empty = StochasticLanguage::new();
    assert_eq!(compute_emd(&empty, &empty).unwrap(), 0.0);
    let unit = StochasticLanguage::from([(vec![], 1.0)]);
    assert_eq!(compute_emd(&unit, &unit).unwrap(), 0.0);
    assert_eq!(normalized_trace_distance(&[], &[]), 0.0);
    assert!(compute_emd(&unit, &empty).is_err());
    for mass in [-1.0, f64::NAN, f64::INFINITY] {
        assert!(compute_emd(&StochasticLanguage::from([(vec![], mass)]), &unit).is_err());
    }
}
