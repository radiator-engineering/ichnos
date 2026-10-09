//! Sliding-window alignment (pm4py's `approx_sliding_window`, after
//! Bogdanov, Cohen and Gal, 2024).
//!
//! The trace is cut into windows. Each window is aligned from each retained
//! end marking of the previous window, and may end in any marking. The best
//! extensions with distinct end markings are kept, ranked by their cost plus
//! the log moves forced on later events whose activity the model can no
//! longer reach. The last window must end in the final marking.

use std::time::Instant;

use rustc_hash::FxHashMap;

use super::super::marking::Packed;
use super::net::{Net, Step};
use super::search::{Query, Stats, search};

/// The result of the method.
pub(super) struct Outcome<'l> {
    pub(super) steps: Vec<Step<'l>>,
    pub(super) cost: u64,
    pub(super) stats: Stats,
    pub(super) window_count: usize,
    pub(super) retained_candidates: Vec<usize>,
    pub(super) fallback_used: bool,
}

#[derive(Clone)]
struct Candidate<'l> {
    marking: Vec<Packed>,
    steps: Vec<Step<'l>>,
    cost: u64,
    score: u64,
}

pub(super) struct Settings {
    pub(super) window_size: usize,
    pub(super) max_candidates: usize,
    pub(super) max_post_model_moves: usize,
    pub(super) max_expansions: usize,
}

pub(super) fn align<'l>(
    net: &Net,
    labels: &[&'l str],
    log_costs: &[u64],
    s: &Settings,
    deadline: Option<Instant>,
) -> Option<Outcome<'l>> {
    let n = labels.len();
    let mut candidates = vec![Candidate {
        marking: net.initial.clone(),
        steps: Vec::new(),
        cost: 0,
        score: 0,
    }];
    let mut retained = Vec::new();
    let mut stats = Stats::default();
    let mut windows: Vec<(usize, usize)> = (0..n)
        .step_by(s.window_size)
        .map(|start| (start, (start + s.window_size).min(n)))
        .collect();
    if windows.is_empty() {
        windows.push((0, 0));
    }

    for (w, &(start, end)) in windows.iter().enumerate() {
        let is_last = w == windows.len() - 1;
        let mut extensions: Vec<Candidate<'l>> = Vec::new();
        let remaining = &labels[end..];
        let remaining_costs = &log_costs[end..];
        let mut cache: FxHashMap<Vec<Packed>, u64> = FxHashMap::default();
        let mut future = |m: &[Packed]| -> u64 {
            if let Some(&c) = cache.get(m) {
                return c;
            }
            let reachable = net.reachable_labels(m);
            let c = remaining
                .iter()
                .zip(remaining_costs)
                .filter(|(l, _)| !reachable.contains(*l))
                .map(|(_, c)| c)
                .sum();
            cache.insert(m.to_vec(), c);
            c
        };
        for c in &candidates {
            if deadline.is_some_and(|d| Instant::now() >= d) {
                break;
            }
            let q = Query {
                labels: &labels[start..end],
                log_costs: &log_costs[start..end],
                offset: start,
                start: &c.marking,
                to_final: is_last,
                max_results: if is_last { 1 } else { s.max_candidates },
                max_expansions: s.max_expansions,
                max_post_model_moves: if is_last { 0 } else { s.max_post_model_moves },
                deadline,
            };
            let (results, st) = search(net, &q, (!is_last).then_some(&mut future as _));
            if !results.is_empty() {
                stats += st;
            }
            for r in results {
                let mut steps = c.steps.clone();
                steps.extend(r.steps);
                extensions.push(Candidate {
                    marking: r.marking,
                    steps,
                    cost: c.cost + r.cost,
                    score: c.cost + r.cost + r.lower_bound,
                });
            }
        }
        if extensions.is_empty() {
            candidates.clear();
            break;
        }
        if is_last {
            // `sorted(..., key=cost)[:1]`: the first of the cheapest.
            let best = extensions
                .iter()
                .enumerate()
                .min_by_key(|(i, e)| (e.cost, *i))
                .map(|(i, _)| i)
                .expect("extensions is not empty");
            candidates = vec![extensions.swap_remove(best)];
        } else {
            extensions.sort_by_key(|e| (e.score, e.cost));
            let mut kept: Vec<Candidate<'l>> = Vec::new();
            for e in extensions {
                if !kept.iter().any(|k| k.marking == e.marking) {
                    kept.push(e);
                }
            }
            kept.truncate(s.max_candidates);
            candidates = kept;
        }
        retained.push(candidates.len());
    }

    let mut fallback_used = false;
    if candidates.is_empty() {
        fallback_used = true;
        let q = Query {
            labels,
            log_costs,
            offset: 0,
            start: &net.initial,
            to_final: true,
            max_results: 1,
            max_expansions: s.max_expansions,
            max_post_model_moves: 0,
            deadline,
        };
        let (mut results, st) = search(net, &q, None);
        if results.is_empty() {
            return None;
        }
        let r = results.swap_remove(0);
        stats += st;
        candidates.push(Candidate {
            marking: r.marking,
            steps: r.steps,
            cost: r.cost,
            score: r.cost,
        });
    }
    // `min(candidates, key=cost)`: the first of the cheapest.
    let chosen = candidates
        .into_iter()
        .enumerate()
        .min_by_key(|(i, c)| (c.cost, *i))
        .map(|(_, c)| c)
        .expect("candidates is not empty");
    Some(Outcome {
        steps: chosen.steps,
        cost: chosen.cost,
        stats,
        window_count: windows.len(),
        retained_candidates: retained,
        fallback_used,
    })
}
