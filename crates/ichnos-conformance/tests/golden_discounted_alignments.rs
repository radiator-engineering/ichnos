//! Discounted alignments against pm4py's `discounted_a_star` variant
//! (`fixtures/golden/conformance/alignments-discounted-*`).
//!
//! pm4py breaks ties by hash order, so its cost for a trace can change from
//! run to run. The golden lists the costs pm4py gives under several seeded
//! hash orders; ichnos's cost must be one of them.

mod common;

use std::collections::HashMap;

use ichnos_conformance::alignments::{Aligner, AlignmentOptions, DEFAULT_DISCOUNT_EXPONENT, Move};
use ichnos_core::EventKeys;
use ichnos_golden::{cases, golden};

use common::{build_net, load_csv_log};

#[test]
fn discounted_alignments_match_pm4py() {
    let keys = EventKeys::default();
    let ids: Vec<String> = cases("conformance")
        .into_iter()
        .filter(|id| id.starts_with("alignments-discounted-"))
        .collect();
    assert_eq!(ids.len(), 3, "discounted goldens: {ids:?}");
    for id in ids {
        let g = golden("conformance", &id);
        let log = load_csv_log(&g.fixture("log"));
        let (net, im, fm) = build_net(g.expected_at("/model"));
        let aligner = Aligner::new(&net, &im, &fm, AlignmentOptions::default()).expect("net");
        let result = aligner
            .align_log_discounted(&log, &keys, DEFAULT_DISCOUNT_EXPONENT)
            .expect("aligned");
        let index: HashMap<String, usize> = log
            .traces
            .iter()
            .enumerate()
            .map(|(i, t)| (t.case_id().expect("case id").to_string(), i))
            .collect();
        let expected = g.expected_at("/traces").as_array().expect("traces");
        assert_eq!(result.trace_count(), expected.len(), "{id}: trace count");
        for e in expected {
            let case_id = e["case_id"].as_str().expect("case id");
            let i = index[case_id];
            let a = result.trace(i).expect("aligned");
            let choices: Vec<f64> = e["cost_choices"]
                .as_array()
                .expect("choices")
                .iter()
                .map(|c| c.as_f64().expect("cost"))
                .collect();
            assert!(
                choices.contains(&a.cost),
                "{id} case {case_id}: cost {} not in {choices:?}",
                a.cost
            );
            let len = log.traces[i].events.len();
            let events: Vec<usize> = a.moves.iter().filter_map(Move::event).collect();
            assert_eq!(events, (0..len).collect::<Vec<_>>(), "{id} case {case_id}");
            let mut cost = 0.0;
            for (l, m) in a.moves.iter().enumerate() {
                if !matches!(m, Move::Sync { .. }) {
                    cost += DEFAULT_DISCOUNT_EXPONENT.powf(-(l as f64));
                }
            }
            assert_eq!(cost, a.cost, "{id} case {case_id}: move costs");
        }
    }
}
