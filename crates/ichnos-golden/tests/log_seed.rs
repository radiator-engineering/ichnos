//! The seed `log` goldens load, match the `LogSummary` schema, and agree with themselves.

use ichnos_golden::log::LogSummary;
use ichnos_golden::{assert_map_eq, assert_multiset_eq, cases, golden};

const SEED_CASES: [(&str, &str); 4] = [
    ("receipt-xes", "fixtures/logs/receipt.xes"),
    (
        "roadtraffic100traces-xes",
        "fixtures/logs/roadtraffic100traces.xes",
    ),
    ("running-example-csv", "fixtures/logs/running-example.csv"),
    ("running-example-xes", "fixtures/logs/running-example.xes"),
];

#[test]
fn seed_cases_are_present() {
    let found = cases("log");
    for (case, _) in SEED_CASES {
        assert!(
            found.iter().any(|c| c == case),
            "log/{case} missing; found {found:?}"
        );
    }
}

#[test]
fn seed_cases_match_schema_and_are_consistent() {
    for (case, fixture) in SEED_CASES {
        let g = golden("log", case);
        let meta = g.meta();
        assert_eq!(meta.area, "log");
        assert_eq!(meta.case, case);
        assert_eq!(meta.pm4py_version, "2.7.23.8");
        assert!(
            meta.pm4py_commit.starts_with("24a3bf6"),
            "{}",
            meta.pm4py_commit
        );
        assert_eq!(meta.fixtures.get("log").map(String::as_str), Some(fixture));
        assert!(g.fixture("log").is_file());

        let s: LogSummary = g.expected_as();
        let total = |m: &std::collections::BTreeMap<String, u64>| m.values().sum::<u64>();

        assert!(s.n_cases > 0 && s.n_events >= s.n_cases, "{case}");
        assert_eq!(total(&s.activities), s.n_events, "{case}: activity counts");
        assert_eq!(
            total(&s.start_activities),
            s.n_cases,
            "{case}: start activities"
        );
        assert_eq!(
            total(&s.end_activities),
            s.n_cases,
            "{case}: end activities"
        );
        assert_eq!(
            s.variants.iter().map(|v| v.count).sum::<u64>(),
            s.n_cases,
            "{case}: variants"
        );
        let variant_events: u64 = s
            .variants
            .iter()
            .map(|v| v.count * v.activities.len() as u64)
            .sum();
        assert_eq!(variant_events, s.n_events, "{case}: variant lengths");
        // Each case of length n contributes n - 1 directly-follows pairs.
        assert_eq!(
            s.dfg.iter().map(|e| e.count).sum::<u64>(),
            s.n_events - s.n_cases,
            "{case}: dfg"
        );

        // The start and end activities, activity counts and DFG all follow from the variants.
        let mut starts = Vec::new();
        let mut ends = Vec::new();
        let mut events = Vec::new();
        let mut pairs = Vec::new();
        for v in &s.variants {
            for _ in 0..v.count {
                starts.push(v.activities[0].clone());
                ends.push(v.activities[v.activities.len() - 1].clone());
                events.extend(v.activities.iter().cloned());
                pairs.extend(
                    v.activities
                        .windows(2)
                        .map(|w| (w[0].clone(), w[1].clone())),
                );
            }
        }
        let expand = |m: &std::collections::BTreeMap<String, u64>| {
            m.iter()
                .flat_map(|(k, n)| std::iter::repeat_n(k.clone(), *n as usize))
                .collect::<Vec<_>>()
        };
        assert_multiset_eq(starts, expand(&s.start_activities));
        assert_multiset_eq(ends, expand(&s.end_activities));
        assert_multiset_eq(events, expand(&s.activities));
        let dfg = s.dfg_counts();
        assert_multiset_eq(
            pairs,
            dfg.iter()
                .flat_map(|(k, n)| std::iter::repeat_n(k.clone(), *n as usize)),
        );

        // Variants are listed once each, sorted.
        let keys: Vec<_> = s.variants.iter().map(|v| &v.activities).collect();
        assert!(
            keys.windows(2).all(|w| w[0] < w[1]),
            "{case}: variants not sorted and unique"
        );
        assert_eq!(s.variant_counts().len(), s.variants.len());
    }
}

#[test]
fn running_example_csv_and_xes_agree() {
    let xes: LogSummary = golden("log", "running-example-xes").expected_as();
    let csv: LogSummary = golden("log", "running-example-csv").expected_as();
    assert_eq!(xes.n_cases, 6);
    assert_eq!(xes.n_events, 42);
    assert_map_eq(csv.activities.clone(), xes.activities.clone());
    assert_eq!(csv, xes);
}
