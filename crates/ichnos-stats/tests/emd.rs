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
        let g = golden("analysis_remaining", &format!("emd-{name}"));
        let count = match name {
            "running-example" => 6,
            "receipt" => 116,
            "roadtraffic100traces" => 10,
            _ => unreachable!(),
        };
        assert_eq!(g.expected["a"].as_array().unwrap().len(), count);
        assert_eq!(g.expected["b"].as_array().unwrap().len(), count);
        check(&g.expected);
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

#[test]
fn mass_tolerance_matches_numpy_and_rescales_second_total() {
    let first = StochasticLanguage::from([(vec!["a".into()], 0.5), (vec!["b".into()], 0.5)]);
    let second =
        StochasticLanguage::from([(vec!["a".into()], 0.5), (vec!["c".into()], 0.500000001)]);
    assert_close(
        compute_emd(&first, &second).unwrap(),
        0.500000001 / 1.000000001,
        Tolerance::METRIC,
    );
    let near = StochasticLanguage::from([(vec!["a".into()], 1. + 0.000009)]);
    let unit = StochasticLanguage::from([(vec!["a".into()], 1.)]);
    assert_eq!(compute_emd(&unit, &near).unwrap(), 0.);
    let far = StochasticLanguage::from([(vec!["a".into()], 1. + 0.00002)]);
    assert!(compute_emd(&unit, &far).is_err());
    let tiny = StochasticLanguage::from([(vec!["a".into()], 1e-9)]);
    assert_eq!(compute_emd(&tiny, &StochasticLanguage::new()).unwrap(), 0.);
}
