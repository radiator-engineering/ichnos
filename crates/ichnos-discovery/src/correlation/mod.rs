//! Classic case-independent correlation mining, ported from pm4py's
//! `algo.discovery.correlation_mining.variants.classic`, using a transportation LP.

use crate::{Error, Result};
use good_lp::{Expression, ProblemVariables, Solution, SolverModel, constraint, microlp, variable};
use ichnos_core::{EventKeys, EventLog, Position};
use ichnos_model::{Dfg, Label};
use std::collections::BTreeMap;

/// Classic correlation options; split and trace-based variants are not implemented.
#[derive(Debug, Clone, Copy, Default)]
pub struct CorrelationOptions {
    /// Read `keys.start_timestamp` as starts rather than completion timestamps.
    pub use_start_timestamp: bool,
    /// Exact full bipartite timestamp matching instead of the minimum FIFO/reverse-LIFO mean.
    pub exact_time_matching: bool,
}

/// Statistics used to price a source/target edge, including diagonals.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CorrelationEdge {
    /// Strictly-preceding pair fraction. Completion lists retain start-sorted order.
    pub precede_succeed: f64,
    /// Estimated duration in seconds; zero for empty matches or diagonals.
    pub duration: f64,
    /// Duration / fraction / minimum activity count; zero values become 1e11.
    pub cost: f64,
}

/// Frequency and performance graphs estimated without using case identifiers.
#[derive(Debug, Clone)]
pub struct CorrelationResult {
    /// Estimated frequency graph. Boundaries follow the first/last flattened
    /// input events and their outgoing/incoming counts, as in the public wrapper.
    pub dfg: Dfg,
    /// Estimated duration for every positive-frequency edge.
    pub performance: BTreeMap<(Label, Label), f64>,
    /// Complete statistics matrix, keyed by ordered labels.
    pub statistics: BTreeMap<(Label, Label), CorrelationEdge>,
    /// Observed event counts, the LP's incoming and outgoing marginal constraints.
    pub activity_counts: BTreeMap<Label, u64>,
}

fn mean(pairs: &[(f64, f64)]) -> f64 {
    if pairs.is_empty() {
        0.0
    } else {
        pairs.iter().map(|(a, b)| b - a).sum::<f64>() / pairs.len() as f64
    }
}

fn greedy(left: &[f64], right: &[f64]) -> f64 {
    let mut pairs = Vec::new();
    let mut j = 0;
    for &a in left {
        while j < right.len() {
            let b = right[j];
            j += 1;
            if a < b {
                pairs.push((a, b));
                break;
            }
        }
    }
    let fifo = mean(&pairs);
    pairs.clear();
    let mut i = left.len();
    for &b in right.iter().rev() {
        while i > 0 {
            i -= 1;
            let a = left[i];
            if a < b {
                pairs.push((a, b));
                break;
            }
        }
    }
    fifo.min(mean(&pairs))
}

/// Discover classic correlation graphs. Events are stably sorted by start,
/// completion and flattened index. Traces and case IDs do not constrain the LP.
/// Empty input returns empty graphs. Tied LP optima may select different edges
/// from pm4py's configurable solver; uniform-cost matrices choose diagonal
/// edges. Uses microlp rather than reproducing numerical failures of a configured
/// pm4py backend. Marginals and minimum objective are the solver contract.
pub fn correlation_miner(
    log: &EventLog,
    keys: &EventKeys,
    options: &CorrelationOptions,
) -> Result<CorrelationResult> {
    let sequences = log.activity_sequences(keys)?;
    let mut events = Vec::new();
    let mut input_labels = Vec::new();
    for (ti, trace) in log.traces.iter().enumerate() {
        for (ei, event) in trace.events.iter().enumerate() {
            let position = Position::Event {
                trace: ti,
                event: ei,
            };
            let end = crate::batches::timestamp(event, &keys.timestamp, position)?;
            let start = if options.use_start_timestamp {
                crate::batches::timestamp(event, &keys.start_timestamp, position)?
            } else {
                end
            };
            let label = Label::from(sequences.activities.name(sequences.traces[ti][ei]));
            input_labels.push(label.clone());
            events.push((start, end, events.len(), label));
        }
    }
    events.sort_by(|a, b| {
        a.0.total_cmp(&b.0)
            .then(a.1.total_cmp(&b.1))
            .then(a.2.cmp(&b.2))
    });
    let mut grouped = BTreeMap::<Label, Vec<(f64, f64)>>::new();
    for (start, end, _, label) in events {
        grouped.entry(label).or_default().push((start, end));
    }
    let labels: Vec<_> = grouped.keys().cloned().collect();
    let n = labels.len();
    let mut result = CorrelationResult {
        dfg: Dfg::new(),
        performance: BTreeMap::new(),
        statistics: BTreeMap::new(),
        activity_counts: grouped
            .iter()
            .map(|(label, events)| (label.clone(), events.len() as u64))
            .collect(),
    };
    if n == 0 {
        return Ok(result);
    }
    let mut costs = Vec::new();
    for (i, a) in labels.iter().enumerate() {
        let left: Vec<_> = grouped[a].iter().map(|v| v.1).collect();
        for (j, b) in labels.iter().enumerate() {
            let right: Vec<_> = grouped[b].iter().map(|v| v.0).collect();
            let (ps, duration) = if i == j {
                (0.0, 0.0)
            } else {
                let mut z = 0;
                let mut count = 0usize;
                for &t in &left {
                    while z < right.len() && t >= right[z] {
                        z += 1;
                    }
                    count += right.len() - z;
                }
                let ps = count as f64 / (left.len() as f64 * right.len() as f64);
                let duration = if options.exact_time_matching {
                    mean(
                        &ichnos_stats::time::exact_match_minimum_average(&left, &right).map_err(
                            |e| match e {
                                ichnos_stats::Error::Core(e) => Error::Core(e),
                                e => Error::Stats(e),
                            },
                        )?,
                    )
                } else {
                    greedy(&left, &right)
                };
                (ps, duration)
            };
            let cost = if ps > 0.0 {
                duration / ps / left.len().min(right.len()) as f64
            } else {
                0.0
            };
            let cost = if cost == 0.0 { 1e11 } else { cost };
            result.statistics.insert(
                (a.clone(), b.clone()),
                CorrelationEdge {
                    precede_succeed: ps,
                    duration,
                    cost,
                },
            );
            costs.push(cost);
        }
    }
    // Every feasible transportation flow has the same objective here. Choose
    // the diagonal vertex explicitly instead of a backend-dependent permutation.
    if costs.iter().all(|cost| *cost == costs[0]) {
        for label in &labels {
            let edge = (label.clone(), label.clone());
            result
                .dfg
                .add_edge(label.clone(), label.clone(), result.activity_counts[label]);
            result.performance.insert(edge, 0.0);
        }
        let first = input_labels.first().unwrap();
        let last = input_labels.last().unwrap();
        result
            .dfg
            .start_activities
            .insert(first.clone(), result.activity_counts[first]);
        result
            .dfg
            .end_activities
            .insert(last.clone(), result.activity_counts[last]);
        return Ok(result);
    }
    let mut vars = ProblemVariables::new();
    let x = vars.add_vector(variable().min(0.0), n * n);
    let objective: Expression = x.iter().zip(&costs).map(|(&v, &c)| c * v).sum();
    let mut problem = vars.minimise(objective).using(microlp);
    for (i, label) in labels.iter().enumerate() {
        let out: Expression = (0..n).map(|j| x[i * n + j]).sum();
        let incoming: Expression = (0..n).map(|j| x[j * n + i]).sum();
        let count = result.activity_counts[label] as f64;
        problem = problem
            .with(constraint!(out == count))
            .with(constraint!(incoming == count));
    }
    let solution = problem
        .solve()
        .map_err(|e| Error::CorrelationSolver(e.to_string()))?;
    for i in 0..n {
        for j in 0..n {
            let value = solution.value(x[i * n + j]);
            let count = value.round_ties_even();
            if !value.is_finite() || value < -1e-6 || (value - count).abs() > 1e-6 {
                return Err(Error::CorrelationSolver(
                    "nonintegral transportation solution".into(),
                ));
            }
            if count > 0.0 {
                let edge = (labels[i].clone(), labels[j].clone());
                result
                    .dfg
                    .add_edge(edge.0.clone(), edge.1.clone(), count as u64);
                result
                    .performance
                    .insert(edge.clone(), result.statistics[&edge].duration);
            }
        }
    }
    let first = input_labels.first().unwrap();
    let last = input_labels.last().unwrap();
    result
        .dfg
        .start_activities
        .insert(first.clone(), result.activity_counts[first]);
    result
        .dfg
        .end_activities
        .insert(last.clone(), result.activity_counts[last]);
    Ok(result)
}
