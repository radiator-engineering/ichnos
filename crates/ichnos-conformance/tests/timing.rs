//! Wall-clock timing of the alignment search on `receipt` against its
//! inductive-miner net. Ignored by default; run in release mode:
//!
//! ```text
//! cargo test --release -p ichnos-conformance --test timing -- --ignored --nocapture
//! ```

mod common;

use std::time::Instant;

use ichnos_conformance::alignments::{
    Aligner, AlignmentOptions, DEFAULT_DISCOUNT_EXPONENT, Heuristic, precision_alignments,
};
use ichnos_core::EventKeys;
use ichnos_golden::golden;

use common::{build_net, load_csv_log};

#[test]
#[ignore = "timing, not a check; run in release mode"]
fn receipt_against_inductive_net() {
    let keys = EventKeys::default();
    let g = golden("conformance", "alignments-receipt-im");
    let log = load_csv_log(&g.fixture("log"));
    let (net, im, fm) = build_net(g.expected_at("/model"));
    for (heuristic, threads) in [
        (Heuristic::StateEquation, 1),
        (Heuristic::None, 1),
        (Heuristic::StateEquation, 8),
        (Heuristic::None, 8),
    ] {
        let start = Instant::now();
        let options = AlignmentOptions::default()
            .heuristic(heuristic)
            .threads(threads);
        let aligned = Aligner::new(&net, &im, &fm, options)
            .expect("easy sound")
            .align_log(&log, &keys)
            .expect("aligned");
        let elapsed = start.elapsed();
        let visited: usize = aligned
            .alignments
            .iter()
            .flatten()
            .map(|a| a.visited_states)
            .sum();
        let lps: usize = aligned
            .alignments
            .iter()
            .flatten()
            .map(|a| a.lp_solved)
            .sum();
        println!(
            "{heuristic:?}, {threads} thread(s): {:.3} s for {} traces, {} variants; {visited} states visited, {lps} LPs",
            elapsed.as_secs_f64(),
            aligned.trace_count(),
            aligned.variants.len(),
        );
    }
    let start = Instant::now();
    let discounted = Aligner::new(&net, &im, &fm, AlignmentOptions::default())
        .expect("easy sound")
        .align_log_discounted(&log, &keys, DEFAULT_DISCOUNT_EXPONENT)
        .expect("aligned");
    println!(
        "discounted: {:.3} s for {} variants",
        start.elapsed().as_secs_f64(),
        discounted.variants.len(),
    );
    let start = Instant::now();
    let precision = precision_alignments(&log, &net, &im, &fm, &keys).expect("precision");
    println!(
        "precision {precision}: {:.3} s",
        start.elapsed().as_secs_f64()
    );
}
