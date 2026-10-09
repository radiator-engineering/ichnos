//! Subset selection and edit distance (pm4py's `approx_subset` variant of
//! `algo/conformance/alignments/edit_distance`, after Fani Sani, van Zelst
//! and van der Aalst, 2020).
//!
//! A few variants of the log are aligned exactly. Their model runs become
//! the representatives. Every other variant is aligned against the
//! representative whose visible activities are closest in edit distance
//! (insertions and deletions only), and the representative's run is kept
//! whole, silent transitions included, so the alignment stays valid.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use ichnos_core::{EventKeys, EventLog};
use ichnos_model::{Label, Marking, PetriNet};

use super::super::costs::{
    ModelCosts, STD_LOG_MOVE_COST, STD_MODEL_MOVE_COST, STD_SILENT_MOVE_COST,
};
use super::super::result::{LogAlignment, Move, TraceAlignment};
use super::net::{Net, Step};
use super::search::{Query, Stats, search};
use crate::error::{Error, Result};

/// How many variants to align exactly.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SubsetSize {
    /// This many variants, at least 1 and at most the number of variants.
    Count(usize),
    /// This share of the variants, rounded up: a number in `(0, 1]`. pm4py's
    /// default is 0.1.
    Fraction(f64),
}

/// How to pick the variants to align exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubsetSelection {
    /// The most frequent variants; ties go by activity sequence (pm4py's
    /// `frequency`, its default).
    Frequency,
    /// k-medoids over the edit distance, weighted by frequency, starting
    /// from the most frequent variants (pm4py's `k_medoids`).
    KMedoids {
        /// The most rounds of reassignment. pm4py's default is 10.
        max_iterations: usize,
    },
    /// The given variants, as activity sequences, in this order. Sequences
    /// that are not variants of the log are aligned too and still serve as
    /// representatives. The subset size is ignored.
    Variants(Vec<Vec<String>>),
}

/// Options of [`align_log_subset`].
#[derive(Debug, Clone, PartialEq)]
pub struct SubsetOptions {
    /// How to pick the representatives.
    pub selection: SubsetSelection,
    /// How many representatives to pick.
    pub size: SubsetSize,
    /// The most states one exact search takes off its queue (pm4py's
    /// `max_expansions`, default 100000).
    pub max_expansions: usize,
    /// A time limit for all exact searches together (pm4py's
    /// `max_align_time_trace`). `None` waits for the answer.
    pub time_limit: Option<Duration>,
}

impl Default for SubsetOptions {
    fn default() -> Self {
        Self {
            selection: SubsetSelection::Frequency,
            size: SubsetSize::Fraction(0.1),
            max_expansions: 100_000,
            time_limit: None,
        }
    }
}

/// Move counts per activity (pm4py's `deviation_counts`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviationCounts {
    /// Model moves on visible transitions, by label.
    pub insertions: BTreeMap<Label, usize>,
    /// Log moves, by activity.
    pub deletions: BTreeMap<Label, usize>,
    /// Synchronous moves, by activity.
    pub synchronous: BTreeMap<Label, usize>,
}

impl DeviationCounts {
    fn add(&mut self, other: &DeviationCounts) {
        for (mine, theirs) in [
            (&mut self.insertions, &other.insertions),
            (&mut self.deletions, &other.deletions),
            (&mut self.synchronous, &other.synchronous),
        ] {
            for (k, v) in theirs {
                *mine.entry(k.clone()).or_default() += v;
            }
        }
    }
}

/// The subset alignment of one variant.
#[derive(Debug, Clone, PartialEq)]
pub struct SubsetTraceAlignment {
    /// The alignment, with standard costs. Its `fitness` is the lower bound
    /// on the fitness, as pm4py reports it, and its `best_worst_cost` is
    /// `10000 * (events + visible transitions of the shortest model run)`.
    /// The search counters are those of the exact search, or 0 for a
    /// variant aligned by edit distance.
    pub alignment: TraceAlignment,
    /// A lower bound on the optimal cost.
    pub lower_bound_cost: u64,
    /// An upper bound on the fitness.
    pub fitness_upper_bound: f64,
    /// Whether the bounds hold: the shortest model run was found exactly.
    pub fitness_bounds_guaranteed: bool,
    /// Whether this variant was aligned exactly.
    pub selected_exact: bool,
    /// The visible activities of the representative's model run.
    pub representative: Vec<Label>,
    /// Move counts per activity.
    pub deviations: DeviationCounts,
    /// Whether the moves replay the trace and reach the final marking.
    pub is_valid: bool,
}

impl SubsetTraceAlignment {
    /// The midpoint of the fitness bounds (pm4py's `approximated_fitness`).
    pub fn approximated_fitness(&self) -> f64 {
        (self.alignment.fitness + self.fitness_upper_bound) / 2.0
    }
}

/// The subset alignment of a log.
#[derive(Debug, Clone)]
pub struct SubsetAlignment {
    /// One alignment per variant.
    pub log: LogAlignment<SubsetTraceAlignment>,
    /// The number of representatives (pm4py's `subset_size`).
    pub representatives: usize,
}

/// pm4py's `apply_with_summary`: means over the traces of the log.
#[derive(Debug, Clone, PartialEq)]
pub struct SubsetSummary {
    /// The mean of the approximated fitness.
    pub log_fitness: f64,
    /// The mean of the lower bounds on the fitness.
    pub fitness_lower_bound: f64,
    /// The mean of the upper bounds on the fitness.
    pub fitness_upper_bound: f64,
    /// Move counts over all traces.
    pub deviations: DeviationCounts,
}

impl SubsetAlignment {
    /// Fitness bounds and move counts over all traces. An empty log has
    /// fitness 1.
    pub fn summary(&self) -> SubsetSummary {
        let n = self.log.trace_count();
        let mut deviations = DeviationCounts::default();
        if n == 0 {
            return SubsetSummary {
                log_fitness: 1.0,
                fitness_lower_bound: 1.0,
                fitness_upper_bound: 1.0,
                deviations,
            };
        }
        let (mut fit, mut lower, mut upper) = (0.0, 0.0, 0.0);
        for a in self.log.traces().flatten() {
            fit += a.approximated_fitness();
            lower += a.alignment.fitness;
            upper += a.fitness_upper_bound;
            deviations.add(&a.deviations);
        }
        let n = n as f64;
        SubsetSummary {
            log_fitness: fit / n,
            fitness_lower_bound: lower / n,
            fitness_upper_bound: upper / n,
            deviations,
        }
    }
}

struct Representative<'l> {
    visible: Vec<&'l str>,
    transitions: Vec<u32>,
    source: Option<usize>,
    steps: Vec<Step<'l>>,
    stats: Stats,
}

/// Aligns every variant of `log` against the accepting net by subset
/// selection and edit distance, with pm4py's standard costs.
///
/// Fails with [`Error::NoRepresentative`] when no picked variant can be
/// aligned within the limits.
pub fn align_log_subset(
    log: &EventLog,
    net: &PetriNet,
    initial_marking: &Marking,
    final_marking: &Marking,
    keys: &EventKeys,
    options: &SubsetOptions,
) -> Result<SubsetAlignment> {
    let variants = log.variants(keys)?;
    let owned: Vec<Vec<String>> = variants
        .iter()
        .map(|v| variants.names(v).map(str::to_owned).collect())
        .collect();
    let traces: Vec<Vec<&str>> = owned
        .iter()
        .map(|t| t.iter().map(String::as_str).collect())
        .collect();
    if traces.is_empty() {
        return Ok(SubsetAlignment {
            log: LogAlignment::new(variants, Vec::new()),
            representatives: 0,
        });
    }
    let frequencies: Vec<usize> = variants.iter().map(|v| v.count()).collect();
    let size = match options.size {
        SubsetSize::Count(n) => n,
        SubsetSize::Fraction(f) => {
            if !(f > 0.0 && f <= 1.0) {
                return Err(Error::InvalidApproximation(
                    "the subset fraction must be in (0, 1]",
                ));
            }
            (traces.len() as f64 * f).ceil() as usize
        }
    }
    .clamp(1, traces.len());
    let model = Net::new(
        net,
        initial_marking,
        final_marking,
        &ModelCosts::standard(net),
    )?;
    let deadline = options.time_limit.map(|d| Instant::now() + d);

    let extra: Vec<Vec<&str>>;
    let selected: Vec<(Option<usize>, &[&str])> = match &options.selection {
        SubsetSelection::Frequency => by_frequency(&traces, &frequencies, size)
            .into_iter()
            .map(|i| (Some(i), traces[i].as_slice()))
            .collect(),
        SubsetSelection::KMedoids { max_iterations } => {
            k_medoids(&traces, &frequencies, size, *max_iterations)
                .into_iter()
                .map(|i| (Some(i), traces[i].as_slice()))
                .collect()
        }
        SubsetSelection::Variants(chosen) => {
            extra = chosen
                .iter()
                .map(|v| v.iter().map(String::as_str).collect())
                .collect();
            extra
                .iter()
                .map(|v| (traces.iter().position(|t| t == v), v.as_slice()))
                .collect()
        }
    };

    let mut representatives: Vec<Representative<'_>> = Vec::new();
    for (source, trace) in selected {
        if deadline.is_some_and(|d| Instant::now() >= d) {
            break;
        }
        let costs = vec![STD_LOG_MOVE_COST; trace.len()];
        let (found, stats) = search(
            &model,
            &exact_query(&model, trace, &costs, options, deadline),
            None,
        );
        if let Some(f) = found.into_iter().next() {
            let transitions: Vec<u32> = f.steps.iter().filter_map(|s| s.transition).collect();
            representatives.push(Representative {
                visible: transitions.iter().filter_map(|&t| model.label(t)).collect(),
                transitions,
                source,
                steps: f.steps,
                stats,
            });
        }
    }
    if representatives.is_empty() {
        return Err(Error::NoRepresentative);
    }

    let (shortest, _) = search(
        &model,
        &exact_query(&model, &[], &[], options, deadline),
        None,
    );
    let guaranteed = !shortest.is_empty();
    let shortest_visible = match shortest.first() {
        Some(f) => f
            .steps
            .iter()
            .filter(|s| s.transition.is_some_and(|t| model.label(t).is_some()))
            .count(),
        None => representatives
            .iter()
            .map(|r| r.visible.len())
            .min()
            .expect("representatives is not empty"),
    };

    let alignments = traces
        .iter()
        .enumerate()
        .map(|(v, trace)| {
            let exact = representatives.iter().find(|r| r.source == Some(v));
            let (rep, steps) = match exact {
                Some(r) => (r, r.steps.clone()),
                None => {
                    let r = representatives
                        .iter()
                        .min_by_key(|r| edit_distance(trace, &r.visible))
                        .expect("representatives is not empty");
                    let ops = edit_operations(trace, &r.visible);
                    (r, materialize(&model, trace, &r.transitions, &ops))
                }
            };
            Some(finish(
                &model,
                trace,
                rep,
                steps,
                exact.is_some(),
                shortest_visible,
                guaranteed,
            ))
        })
        .collect();
    Ok(SubsetAlignment {
        log: LogAlignment::new(variants, alignments),
        representatives: representatives.len(),
    })
}

fn exact_query<'a, 'l>(
    model: &'a Net,
    trace: &'a [&'l str],
    costs: &'a [u64],
    options: &SubsetOptions,
    deadline: Option<Instant>,
) -> Query<'a, 'l> {
    Query {
        labels: trace,
        log_costs: costs,
        offset: 0,
        start: &model.initial,
        to_final: true,
        max_results: 1,
        max_expansions: options.max_expansions,
        max_post_model_moves: 0,
        deadline,
    }
}

fn finish(
    model: &Net,
    trace: &[&str],
    rep: &Representative<'_>,
    steps: Vec<Step<'_>>,
    exact: bool,
    shortest_visible: usize,
    guaranteed: bool,
) -> SubsetTraceAlignment {
    let standard_cost = model.standard_cost(&steps);
    let denominator = trace.len() + shortest_visible;
    let upper_moves = standard_cost / STD_LOG_MOVE_COST;
    let lower_moves = if exact {
        upper_moves
    } else if guaranteed {
        shortest_visible.saturating_sub(trace.len()) as u64
    } else {
        0
    };
    let fitness_lower = if denominator > 0 {
        (1.0 - upper_moves as f64 / denominator as f64).max(0.0)
    } else if upper_moves == 0 {
        1.0
    } else {
        0.0
    };
    let fitness_upper = if denominator > 0 {
        (1.0 - lower_moves as f64 / denominator as f64).max(0.0)
    } else {
        1.0
    };
    let mut deviations = DeviationCounts::default();
    let mut event = 0;
    let moves = steps
        .iter()
        .map(|s| {
            let transition = s.transition.map(|t| model.transitions[t as usize].id);
            match (s.log, transition) {
                (Some(l), Some(transition)) => {
                    *deviations.synchronous.entry(Label::from(l)).or_default() += 1;
                    event += 1;
                    Move::Sync {
                        event: event - 1,
                        transition,
                    }
                }
                (Some(l), None) => {
                    *deviations.deletions.entry(Label::from(l)).or_default() += 1;
                    event += 1;
                    Move::Log { event: event - 1 }
                }
                (None, Some(transition)) => {
                    if let Some(l) = s.transition.and_then(|t| model.label(t)) {
                        *deviations.insertions.entry(Label::from(l)).or_default() += 1;
                    }
                    Move::Model { transition }
                }
                (None, None) => unreachable!("a step moves on the log or the model"),
            }
        })
        .collect();
    let stats = if exact { rep.stats } else { Stats::default() };
    SubsetTraceAlignment {
        alignment: TraceAlignment {
            moves,
            cost: standard_cost,
            fitness: fitness_lower,
            best_worst_cost: denominator as u64 * STD_LOG_MOVE_COST,
            visited_states: stats.visited,
            queued_states: stats.queued,
            traversed_arcs: stats.traversed,
            lp_solved: 0,
        },
        lower_bound_cost: if exact {
            standard_cost
        } else {
            lower_moves * STD_LOG_MOVE_COST
        },
        fitness_upper_bound: fitness_upper,
        fitness_bounds_guaranteed: guaranteed,
        selected_exact: exact,
        representative: rep.visible.iter().map(|&l| Label::from(l)).collect(),
        deviations,
        is_valid: model.validate(trace, &steps),
    }
}

/// The `size` most frequent variants; ties go by activity sequence.
fn by_frequency(traces: &[Vec<&str>], frequencies: &[usize], size: usize) -> Vec<usize> {
    let mut order: Vec<usize> = (0..traces.len()).collect();
    order.sort_by(|&a, &b| {
        frequencies[b]
            .cmp(&frequencies[a])
            .then_with(|| traces[a].cmp(&traces[b]))
    });
    order.truncate(size);
    order
}

/// pm4py's `_k_medoids`.
fn k_medoids(
    traces: &[Vec<&str>],
    frequencies: &[usize],
    size: usize,
    max_iterations: usize,
) -> Vec<usize> {
    let mut medoids = by_frequency(traces, frequencies, size);
    let mut cache: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    let mut distance = |a: usize, b: usize| -> usize {
        let key = (a.min(b), a.max(b));
        *cache
            .entry(key)
            .or_insert_with(|| edit_distance(&traces[a], &traces[b]))
    };
    for _ in 0..max_iterations {
        // pm4py keys the clusters by medoid in a dict, so a repeated medoid
        // keeps one cluster.
        let mut clusters: Vec<(usize, Vec<usize>)> = Vec::new();
        for &m in &medoids {
            if !clusters.iter().any(|c| c.0 == m) {
                clusters.push((m, Vec::new()));
            }
        }
        for v in 0..traces.len() {
            let mut best = medoids[0];
            let mut best_d = distance(v, best);
            for &m in &medoids[1..] {
                let d = distance(v, m);
                if d < best_d {
                    best = m;
                    best_d = d;
                }
            }
            clusters
                .iter_mut()
                .find(|c| c.0 == best)
                .expect("every medoid has a cluster")
                .1
                .push(v);
        }
        let mut next = Vec::with_capacity(clusters.len());
        for (m, cluster) in &clusters {
            if cluster.is_empty() {
                next.push(*m);
                continue;
            }
            let mut best = cluster[0];
            let mut best_cost = usize::MAX;
            for &c in cluster {
                let cost: usize = cluster
                    .iter()
                    .map(|&v| frequencies[v] * distance(c, v))
                    .sum();
                if cost < best_cost {
                    best = c;
                    best_cost = cost;
                }
            }
            next.push(best);
        }
        if next == medoids {
            break;
        }
        medoids = next;
    }
    medoids
}

/// Edit distance with insertions and deletions of cost 1 and substitutions
/// of cost 2.
fn edit_distance(left: &[&str], right: &[&str]) -> usize {
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    for (i, l) in left.iter().enumerate() {
        let mut current = vec![i + 1];
        for (j, r) in right.iter().enumerate() {
            let diagonal = if l == r { previous[j] } else { previous[j] + 2 };
            current.push((previous[j + 1] + 1).min(current[j] + 1).min(diagonal));
        }
        previous = current;
    }
    previous[right.len()]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    /// Event `i` with visible step `j`.
    Sync(usize, usize),
    /// Event `i` alone.
    Log(usize),
    /// Visible step `j` alone.
    Model(usize),
}

/// pm4py's `_edit_operations`: an optimal edit script, read back from the
/// end, preferring synchronous moves, then log moves.
fn edit_operations(left: &[&str], right: &[&str]) -> Vec<Op> {
    let (rows, cols) = (left.len() + 1, right.len() + 1);
    let mut c = vec![vec![0usize; cols]; rows];
    for (i, row) in c.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in c[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..rows {
        for j in 1..cols {
            let diagonal = if left[i - 1] == right[j - 1] {
                c[i - 1][j - 1]
            } else {
                c[i - 1][j - 1] + 2
            };
            c[i][j] = (c[i - 1][j] + 1).min(c[i][j - 1] + 1).min(diagonal);
        }
    }
    let mut ops = Vec::new();
    let (mut i, mut j) = (left.len(), right.len());
    while i > 0 || j > 0 {
        if i > 0 && j > 0 && left[i - 1] == right[j - 1] && c[i][j] == c[i - 1][j - 1] {
            ops.push(Op::Sync(i - 1, j - 1));
            i -= 1;
            j -= 1;
        } else if i > 0 && c[i][j] == c[i - 1][j] + 1 {
            ops.push(Op::Log(i - 1));
            i -= 1;
        } else {
            ops.push(Op::Model(j - 1));
            j -= 1;
        }
    }
    ops.reverse();
    ops
}

/// pm4py's `_materialize_alignment`: the representative's run, with each
/// visible transition paired with its event or made a model move, and the
/// unmatched events as log moves.
fn materialize<'l>(model: &Net, trace: &[&'l str], run: &[u32], ops: &[Op]) -> Vec<Step<'l>> {
    let visible: Vec<usize> = run
        .iter()
        .enumerate()
        .filter(|(_, t)| model.label(**t).is_some())
        .map(|(i, _)| i)
        .collect();
    let model_move = |t: u32, cost: u64| Step {
        log: None,
        log_index: None,
        transition: Some(t),
        cost,
    };
    let mut steps = Vec::new();
    let mut cursor = 0;
    for &op in ops {
        let (event, j) = match op {
            Op::Sync(i, j) => (Some(i), j),
            Op::Model(j) => (None, j),
            Op::Log(i) => {
                steps.push(Step {
                    log: Some(trace[i]),
                    log_index: Some(i),
                    transition: None,
                    cost: STD_LOG_MOVE_COST,
                });
                continue;
            }
        };
        while cursor < visible[j] {
            steps.push(model_move(run[cursor], STD_SILENT_MOVE_COST));
            cursor += 1;
        }
        let t = run[cursor];
        cursor += 1;
        steps.push(match event {
            Some(i) => Step {
                log: Some(trace[i]),
                log_index: Some(i),
                transition: Some(t),
                cost: 0,
            },
            None => model_move(t, STD_MODEL_MOVE_COST),
        });
    }
    for &t in &run[cursor..] {
        let cost = if model.label(t).is_none() {
            STD_SILENT_MOVE_COST
        } else {
            STD_MODEL_MOVE_COST
        };
        steps.push(model_move(t, cost));
    }
    steps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_distance_counts_insertions_and_deletions() {
        assert_eq!(edit_distance(&["a", "b", "c"], &["a", "c"]), 1);
        assert_eq!(edit_distance(&["a", "b"], &["a", "c"]), 2);
        assert_eq!(edit_distance(&[], &["a"]), 1);
    }

    #[test]
    fn edit_operations_prefer_sync_then_log() {
        let ops = edit_operations(&["a", "b"], &["a", "c"]);
        assert_eq!(ops, [Op::Sync(0, 0), Op::Model(1), Op::Log(1)]);
    }
}
