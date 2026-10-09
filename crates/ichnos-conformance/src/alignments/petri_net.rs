//! Alignments of traces and logs against an accepting Petri net (pm4py's
//! `algo/conformance/alignments/petri_net`).

use std::thread;
use std::time::{Duration, Instant};

use ichnos_core::{EventKeys, EventLog};
use ichnos_model::{Marking, PetriNet};

use super::costs::{ModelCosts, STD_LOG_MOVE_COST};
use super::result::{AlignmentFitness, LogAlignment, Move, TraceAlignment};
use super::search::search;
use super::sync_product::{ModelPart, MoveSet, SyncProduct};
use crate::error::{Error, Result};

/// How the search estimates the cost still to go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Heuristic {
    /// A* with the marking-equation heuristic: pm4py's
    /// `VERSION_STATE_EQUATION_A_STAR`, its default when an LP solver is
    /// installed.
    #[default]
    StateEquation,
    /// No estimate: Dijkstra's algorithm, as pm4py's
    /// `VERSION_DIJKSTRA_NO_HEURISTICS`. pm4py's
    /// `VERSION_DIJKSTRA_LESS_MEMORY` gives other costs and fitness; ichnos
    /// does not reproduce it.
    None,
}

/// Options of an alignment run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlignmentOptions {
    /// The search heuristic. Every heuristic finds an optimal alignment;
    /// they differ in speed.
    pub heuristic: Heuristic,
    /// The cost of a log move on any event, when the trace has no per-event
    /// costs. pm4py's default is [`STD_LOG_MOVE_COST`].
    pub log_move_cost: u64,
    /// Give up on a trace after this long and report no alignment for it
    /// (pm4py's `max_align_time_trace`). `None` waits for the answer.
    pub trace_time_limit: Option<Duration>,
    /// Threads that align variants in parallel. 1 aligns them in order on
    /// the calling thread.
    pub threads: usize,
}

impl Default for AlignmentOptions {
    fn default() -> Self {
        Self {
            heuristic: Heuristic::default(),
            log_move_cost: STD_LOG_MOVE_COST,
            trace_time_limit: None,
            threads: 1,
        }
    }
}

impl AlignmentOptions {
    /// Sets the heuristic.
    pub fn heuristic(mut self, heuristic: Heuristic) -> Self {
        self.heuristic = heuristic;
        self
    }

    /// Sets the log move cost.
    pub fn log_move_cost(mut self, cost: u64) -> Self {
        self.log_move_cost = cost;
        self
    }

    /// Sets the time limit per trace.
    pub fn trace_time_limit(mut self, limit: Duration) -> Self {
        self.trace_time_limit = Some(limit);
        self
    }

    /// Sets the number of threads.
    pub fn threads(mut self, threads: usize) -> Self {
        self.threads = threads.max(1);
        self
    }
}

/// Aligns traces against one accepting Petri net.
///
/// Building an aligner prepares the net once and computes the cost of
/// aligning the empty trace (pm4py's "best worst cost"), which every trace
/// fitness needs. That search also checks that the final marking is
/// reachable: pm4py refuses nets that are not easy sound, and so does
/// [`Aligner::new`], with [`Error::FinalMarkingUnreachable`].
#[derive(Debug, Clone)]
pub struct Aligner {
    model: ModelPart,
    options: AlignmentOptions,
    best_worst_cost: u64,
}

impl Aligner {
    /// Prepares `net` with pm4py's standard costs.
    pub fn new(
        net: &PetriNet,
        initial_marking: &Marking,
        final_marking: &Marking,
        options: AlignmentOptions,
    ) -> Result<Self> {
        Self::with_costs(
            net,
            initial_marking,
            final_marking,
            &ModelCosts::standard(net),
            options,
        )
    }

    /// Prepares `net` with the given model and synchronous move costs.
    ///
    /// # Panics
    ///
    /// Panics if `costs` was made for a net with fewer transitions.
    pub fn with_costs(
        net: &PetriNet,
        initial_marking: &Marking,
        final_marking: &Marking,
        costs: &ModelCosts,
        options: AlignmentOptions,
    ) -> Result<Self> {
        assert!(costs.covers(net), "the costs were made for another net");
        let model = ModelPart::new(net, initial_marking, final_marking, costs)?;
        let empty = SyncProduct::new::<&str>(&model, &[], &[], MoveSet::All);
        let best_worst_cost = search(&empty, options.heuristic, None)?
            .ok_or(Error::FinalMarkingUnreachable)?
            .cost;
        Ok(Self {
            model,
            options,
            best_worst_cost,
        })
    }

    /// The cost of the cheapest alignment of the empty trace: the cheapest
    /// run of the model from the initial to the final marking.
    pub fn best_worst_cost(&self) -> u64 {
        self.best_worst_cost
    }

    /// The prepared model half of every synchronous product.
    pub(super) fn model(&self) -> &ModelPart {
        &self.model
    }

    /// The options this aligner was built with.
    pub fn options(&self) -> &AlignmentOptions {
        &self.options
    }

    /// Aligns a trace given as its activity labels, with the options' log
    /// move cost for every event. Returns `None` if the time limit passed.
    pub fn align<S: AsRef<str>>(&self, trace: &[S]) -> Result<Option<TraceAlignment>> {
        let costs = vec![self.options.log_move_cost; trace.len()];
        self.align_with_log_costs(trace, &costs)
    }

    /// Aligns a trace where a log move on event `i` costs `log_costs[i]`
    /// (pm4py's `trace_cost_function`).
    pub fn align_with_log_costs<S: AsRef<str>>(
        &self,
        trace: &[S],
        log_costs: &[u64],
    ) -> Result<Option<TraceAlignment>> {
        if log_costs.len() != trace.len() {
            return Err(Error::LogMoveCostCount {
                expected: trace.len(),
                actual: log_costs.len(),
            });
        }
        let deadline = self.options.trace_time_limit.map(|d| Instant::now() + d);
        let sp = SyncProduct::new(&self.model, trace, log_costs, MoveSet::All);
        let Some(found) = search(&sp, self.options.heuristic, deadline)? else {
            return Ok(None);
        };
        let best_worst_cost = log_costs.iter().sum::<u64>() + self.best_worst_cost;
        Ok(Some(TraceAlignment {
            moves: found
                .path
                .iter()
                .map(|&t| sp.moves[t as usize])
                .collect::<Vec<Move>>(),
            cost: found.cost,
            fitness: trace_fitness(found.cost, best_worst_cost),
            best_worst_cost,
            visited_states: found.stats.visited,
            queued_states: found.stats.queued,
            traversed_arcs: found.stats.traversed,
            lp_solved: found.stats.lp_solved,
        }))
    }

    /// Aligns every variant of `log` once (pm4py's `apply_log`).
    pub fn align_log(&self, log: &EventLog, keys: &EventKeys) -> Result<LogAlignment> {
        let variants = log.variants(keys)?;
        let traces: Vec<Vec<&str>> = variants
            .iter()
            .map(|v| variants.names(v).collect())
            .collect();
        let alignments = self.align_all(&traces)?;
        Ok(LogAlignment::new(variants, alignments))
    }

    /// Aligns each trace, on `options.threads` threads.
    pub fn align_all<S: AsRef<str> + Sync>(
        &self,
        traces: &[Vec<S>],
    ) -> Result<Vec<Option<TraceAlignment>>> {
        let threads = self.options.threads.clamp(1, traces.len().max(1));
        if threads == 1 {
            return traces.iter().map(|t| self.align(t)).collect();
        }
        let next = std::sync::atomic::AtomicUsize::new(0);
        let mut results: Vec<Option<Result<Option<TraceAlignment>>>> =
            (0..traces.len()).map(|_| None).collect();
        let chunks: Vec<Vec<(usize, Result<Option<TraceAlignment>>)>> = thread::scope(|s| {
            let workers: Vec<_> = (0..threads)
                .map(|_| {
                    s.spawn(|| {
                        let mut done = Vec::new();
                        loop {
                            let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                            if i >= traces.len() {
                                return done;
                            }
                            done.push((i, self.align(&traces[i])));
                        }
                    })
                })
                .collect();
            workers
                .into_iter()
                .map(|w| w.join().expect("alignment worker panicked"))
                .collect()
        });
        for (i, r) in chunks.into_iter().flatten() {
            results[i] = Some(r);
        }
        results
            .into_iter()
            .map(|r| r.expect("every trace aligned"))
            .collect()
    }
}

/// pm4py's trace fitness: costs are divided by the standard move cost with
/// integer division, so silent moves do not count.
fn trace_fitness(cost: u64, best_worst_cost: u64) -> f64 {
    let num = cost / STD_LOG_MOVE_COST;
    let den = best_worst_cost / STD_LOG_MOVE_COST;
    if den > 0 {
        1.0 - num as f64 / den as f64
    } else {
        0.0
    }
}

/// Aligns every trace of `log` against the accepting net (pm4py's
/// `conformance_diagnostics_alignments`).
pub fn align_log(
    log: &EventLog,
    net: &PetriNet,
    initial_marking: &Marking,
    final_marking: &Marking,
    keys: &EventKeys,
    options: AlignmentOptions,
) -> Result<LogAlignment> {
    Aligner::new(net, initial_marking, final_marking, options)?.align_log(log, keys)
}

/// Alignment-based fitness of `log` on the accepting net (pm4py's
/// `fitness_alignments`).
pub fn fitness_alignments(
    log: &EventLog,
    net: &PetriNet,
    initial_marking: &Marking,
    final_marking: &Marking,
    keys: &EventKeys,
    options: AlignmentOptions,
) -> Result<AlignmentFitness> {
    Ok(align_log(log, net, initial_marking, final_marking, keys, options)?.fitness())
}
