//! Approximate alignments against an accepting Petri net (pm4py's
//! `approx_tandem_repeats`, `approx_sliding_window` and
//! `approx_fixed_horizon` variants of `algo/conformance/alignments/petri_net`).
//!
//! These methods trade optimality for speed on long traces. Each returns a
//! valid alignment, whose cost is an upper bound on the optimal cost.
//! [`Approximation`] picks the method:
//!
//! - [`Approximation::TandemRepeats`] cuts runs of three or more copies of
//!   a block to two copies, aligns the shorter trace exactly and puts the
//!   copies back.
//! - [`Approximation::SlidingWindow`] aligns the trace window by window and
//!   keeps a few of the best partial alignments between windows.
//! - [`Approximation::FixedHorizon`] commits a few moves at a time, chosen
//!   by their cost plus the marking-equation estimate of the rest.
//!
//! The searches run on the model itself, with pm4py's tie-breaking: equal
//! choices go by transition label, then name. The results match pm4py move
//! for move on nets whose transition names are unique.
//!
//! ```
//! use ichnos_conformance::alignments::{
//!     ApproximateAligner, ApproximateOptions, Approximation, SlidingWindow,
//! };
//! use ichnos_model::{Marking, PetriNet};
//!
//! // source -> a -> p -> b -> sink
//! let mut net = PetriNet::new("n");
//! let source = net.add_place("source");
//! let p = net.add_place("p");
//! let sink = net.add_place("sink");
//! let a = net.add_transition("a", Some("a"));
//! let b = net.add_transition("b", Some("b"));
//! net.add_input_arc(source, a).unwrap();
//! net.add_output_arc(a, p).unwrap();
//! net.add_input_arc(p, b).unwrap();
//! net.add_output_arc(b, sink).unwrap();
//! let im = Marking::from([(source, 1)]);
//! let fm = Marking::from([(sink, 1)]);
//!
//! let options = ApproximateOptions::new(Approximation::SlidingWindow(SlidingWindow {
//!     window_size: 1,
//!     ..SlidingWindow::default()
//! }));
//! let aligner = ApproximateAligner::new(&net, &im, &fm, options).unwrap();
//! let result = aligner.align(&["a", "c", "b"]).unwrap().expect("no limits hit");
//! assert_eq!(result.alignment.cost, 10_000);
//! assert!(result.is_valid);
//! ```

mod horizon;
mod net;
mod search;
mod sliding;
mod subset;
mod tandem;

use std::time::{Duration, Instant};

use ichnos_core::{EventKeys, EventLog};
use ichnos_model::{Marking, PetriNet};

use super::costs::{ModelCosts, STD_LOG_MOVE_COST};
use super::result::{AlignmentFitness, LogAlignment, Move, TraceAlignment};
use super::sync_product::ModelPart;
use crate::error::{Error, Result};
use net::{Net, Step};
use search::Stats;
pub use subset::{
    DeviationCounts, SubsetAlignment, SubsetOptions, SubsetSelection, SubsetSize, SubsetSummary,
    SubsetTraceAlignment, align_log_subset,
};

/// Settings of [`Approximation::SlidingWindow`] (pm4py's
/// `approx_sliding_window`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlidingWindow {
    /// Events per window. pm4py's default is 20.
    pub window_size: usize,
    /// Partial alignments kept between windows, each ending in a different
    /// marking. pm4py's default is 5.
    pub max_candidates: usize,
    /// Model moves allowed after the last event of a window that is not
    /// the last window. `None` takes pm4py's default: the number of
    /// transitions, at least 1.
    pub max_post_model_moves: Option<usize>,
}

impl Default for SlidingWindow {
    fn default() -> Self {
        Self {
            window_size: 20,
            max_candidates: 5,
            max_post_model_moves: None,
        }
    }
}

/// Settings of [`Approximation::FixedHorizon`] (pm4py's
/// `approx_fixed_horizon`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedHorizon {
    /// The most moves a round looks ahead. pm4py's default is 4.
    pub horizon: usize,
    /// The fewest events a committed prefix must move on, unless it ends
    /// in the final marking. pm4py's default is 1.
    pub min_progress: usize,
    /// The horizon grows up to this value. `None` takes pm4py's default:
    /// `max(20, horizon)`.
    pub max_horizon: Option<usize>,
    /// The most states one round takes off its queue. pm4py's default is
    /// 20000.
    pub max_prefix_states: usize,
    /// The most rounds before the method falls back to the exact search.
    /// `None` takes pm4py's default: `max(20, 2 * (events + transitions))`.
    pub max_iterations: Option<usize>,
}

impl Default for FixedHorizon {
    fn default() -> Self {
        Self {
            horizon: 4,
            min_progress: 1,
            max_horizon: None,
            max_prefix_states: 20_000,
            max_iterations: None,
        }
    }
}

/// The approximation method.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Approximation {
    /// Tandem-repeat reduction (pm4py's `approx_tandem_repeats`).
    TandemRepeats,
    /// Sliding windows (pm4py's `approx_sliding_window`).
    SlidingWindow(SlidingWindow),
    /// Fixed horizon (pm4py's `approx_fixed_horizon`).
    FixedHorizon(FixedHorizon),
}

/// Options of an approximate alignment run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApproximateOptions {
    /// The method.
    pub method: Approximation,
    /// The cost of a log move on any event, when the trace has no per-event
    /// costs. pm4py's default is [`STD_LOG_MOVE_COST`].
    pub log_move_cost: u64,
    /// The most states one search takes off its queue (pm4py's
    /// `max_expansions`, default 100000). A search that hits the limit
    /// finds nothing.
    pub max_expansions: usize,
    /// Give up on a trace after this long and report no alignment for it
    /// (pm4py's `max_align_time_trace`). `None` waits for the answer.
    pub trace_time_limit: Option<Duration>,
}

impl ApproximateOptions {
    /// The method with pm4py's defaults for everything else.
    pub fn new(method: Approximation) -> Self {
        Self {
            method,
            log_move_cost: STD_LOG_MOVE_COST,
            max_expansions: 100_000,
            trace_time_limit: None,
        }
    }
}

/// Why [`Approximation::FixedHorizon`] fell back to the exact search
/// (pm4py's `fallback_reason`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixedHorizonFallback {
    /// The time limit passed.
    Timeout,
    /// No prefix qualified, even at the largest horizon.
    NoPrefixSolution,
    /// The best prefix was empty.
    PrefixMadeNoProgress,
    /// The rounds ran out.
    MaximumIterations,
    /// The rounds ended without reaching the final marking.
    IncompleteProductPath,
}

impl FixedHorizonFallback {
    /// pm4py's name for the reason.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Timeout => "timeout",
            Self::NoPrefixSolution => "no_prefix_solution",
            Self::PrefixMadeNoProgress => "prefix_made_no_progress",
            Self::MaximumIterations => "maximum_iterations",
            Self::IncompleteProductPath => "incomplete_product_path",
        }
    }
}

/// What the method did, beyond the alignment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApproximationReport {
    /// [`Approximation::TandemRepeats`].
    TandemRepeats {
        /// The length of the reduced trace.
        reduced_trace_length: usize,
        /// The runs that were cut.
        tandem_repeats: usize,
        /// The events cut from the trace.
        removed_events: usize,
        /// Copies put back as a replay of the model's loop instead of as
        /// log moves.
        model_loop_expansions: usize,
    },
    /// [`Approximation::SlidingWindow`].
    SlidingWindow {
        /// The number of windows.
        window_count: usize,
        /// Partial alignments kept after each window.
        retained_candidates: Vec<usize>,
        /// Whether the windows found nothing and the exact search ran.
        fallback_used: bool,
    },
    /// [`Approximation::FixedHorizon`].
    FixedHorizon {
        /// The horizon of each committed prefix.
        committed_horizons: Vec<usize>,
        /// Why the exact search ran, if it did.
        fallback: Option<FixedHorizonFallback>,
    },
}

/// An approximate alignment of one trace.
#[derive(Debug, Clone, PartialEq)]
pub struct ApproximateAlignment {
    /// The alignment. Its cost uses the aligner's costs; its fitness and
    /// best worst cost follow pm4py, with the best worst cost from the same
    /// method on the empty trace.
    pub alignment: TraceAlignment,
    /// The cost of the moves under pm4py's standard costs.
    pub standard_cost: u64,
    /// Whether the moves replay the trace and reach the final marking
    /// (pm4py's `is_valid`). It is `false` only on nets where the method's
    /// own checks are not enough, such as nets with duplicate transition
    /// names.
    pub is_valid: bool,
    /// What the method did.
    pub report: ApproximationReport,
}

/// Aligns traces approximately against one accepting Petri net.
#[derive(Debug, Clone)]
pub struct ApproximateAligner {
    net: Net,
    model: ModelPart,
    names: Vec<String>,
    options: ApproximateOptions,
    best_worst_cost: u64,
}

impl ApproximateAligner {
    /// Prepares `net` with pm4py's standard costs.
    pub fn new(
        net: &PetriNet,
        initial_marking: &Marking,
        final_marking: &Marking,
        options: ApproximateOptions,
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
    /// Fails with [`Error::InvalidApproximation`] on settings pm4py also
    /// rejects, and with [`Error::FinalMarkingUnreachable`] when the method
    /// finds no alignment for the empty trace.
    ///
    /// # Panics
    ///
    /// Panics if `costs` was made for a net with fewer transitions.
    pub fn with_costs(
        net: &PetriNet,
        initial_marking: &Marking,
        final_marking: &Marking,
        costs: &ModelCosts,
        options: ApproximateOptions,
    ) -> Result<Self> {
        assert!(costs.covers(net), "the costs were made for another net");
        validate(&options.method)?;
        let mut names = vec![String::new(); net.transition_index_bound()];
        for (id, t) in net.transitions() {
            names[id.index()] = t.name.clone();
        }
        let mut aligner = Self {
            net: Net::new(net, initial_marking, final_marking, costs)?,
            model: ModelPart::new(net, initial_marking, final_marking, costs)?,
            names,
            options,
            best_worst_cost: 0,
        };
        // pm4py's `get_best_worst_cost`: the same method on the empty trace.
        aligner.best_worst_cost = aligner
            .run(&[], &[], None)?
            .ok_or(Error::FinalMarkingUnreachable)?
            .cost;
        Ok(aligner)
    }

    /// The cost the method gives the empty trace.
    pub fn best_worst_cost(&self) -> u64 {
        self.best_worst_cost
    }

    /// The options this aligner was built with.
    pub fn options(&self) -> &ApproximateOptions {
        &self.options
    }

    /// Aligns a trace given as its activity labels, with the options' log
    /// move cost for every event. Returns `None` if a limit was hit before
    /// any alignment was found.
    pub fn align<S: AsRef<str>>(&self, trace: &[S]) -> Result<Option<ApproximateAlignment>> {
        let costs = vec![self.options.log_move_cost; trace.len()];
        self.align_with_log_costs(trace, &costs)
    }

    /// Aligns a trace where a log move on event `i` costs `log_costs[i]`.
    pub fn align_with_log_costs<S: AsRef<str>>(
        &self,
        trace: &[S],
        log_costs: &[u64],
    ) -> Result<Option<ApproximateAlignment>> {
        if log_costs.len() != trace.len() {
            return Err(Error::LogMoveCostCount {
                expected: trace.len(),
                actual: log_costs.len(),
            });
        }
        let labels: Vec<&str> = trace.iter().map(AsRef::as_ref).collect();
        let deadline = self.options.trace_time_limit.map(|d| Instant::now() + d);
        let Some(run) = self.run(&labels, log_costs, deadline)? else {
            return Ok(None);
        };
        let best_worst_cost = log_costs.iter().sum::<u64>() + self.best_worst_cost;
        let mut event = 0;
        let moves = run
            .steps
            .iter()
            .map(|s| {
                let transition = s.transition.map(|t| self.net.transitions[t as usize].id);
                match (s.log, transition) {
                    (Some(_), Some(transition)) => {
                        event += 1;
                        Move::Sync {
                            event: event - 1,
                            transition,
                        }
                    }
                    (Some(_), None) => {
                        event += 1;
                        Move::Log { event: event - 1 }
                    }
                    (None, Some(transition)) => Move::Model { transition },
                    (None, None) => unreachable!("a step moves on the log or the model"),
                }
            })
            .collect();
        Ok(Some(ApproximateAlignment {
            alignment: TraceAlignment {
                moves,
                cost: run.cost,
                fitness: trace_fitness(run.cost, best_worst_cost),
                best_worst_cost,
                visited_states: run.stats.visited,
                queued_states: run.stats.queued,
                traversed_arcs: run.stats.traversed,
                lp_solved: run.lp_solved,
            },
            standard_cost: self.net.standard_cost(&run.steps),
            is_valid: self.net.validate(&labels, &run.steps),
            report: run.report,
        }))
    }

    /// Aligns every variant of `log` once.
    pub fn align_log(
        &self,
        log: &EventLog,
        keys: &EventKeys,
    ) -> Result<LogAlignment<ApproximateAlignment>> {
        let variants = log.variants(keys)?;
        let alignments = variants
            .iter()
            .map(|v| {
                let trace: Vec<&str> = variants.names(v).collect();
                self.align(&trace)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(LogAlignment::new(variants, alignments))
    }

    fn run<'l>(
        &self,
        labels: &[&'l str],
        log_costs: &[u64],
        deadline: Option<Instant>,
    ) -> Result<Option<Run<'l>>> {
        let max_expansions = self.options.max_expansions;
        Ok(match &self.options.method {
            Approximation::TandemRepeats => {
                tandem::align(&self.net, labels, log_costs, max_expansions, deadline).map(|o| Run {
                    cost: o.steps.iter().map(|s| s.cost).sum(),
                    steps: o.steps,
                    stats: o.stats,
                    lp_solved: 0,
                    report: ApproximationReport::TandemRepeats {
                        reduced_trace_length: o.reduced_trace_length,
                        tandem_repeats: o.tandem_repeats,
                        removed_events: o.removed_events,
                        model_loop_expansions: o.model_loop_expansions,
                    },
                })
            }
            Approximation::SlidingWindow(w) => {
                let settings = sliding::Settings {
                    window_size: w.window_size,
                    max_candidates: w.max_candidates,
                    max_post_model_moves: w
                        .max_post_model_moves
                        .unwrap_or(self.net.transition_count().max(1)),
                    max_expansions,
                };
                sliding::align(&self.net, labels, log_costs, &settings, deadline).map(|o| Run {
                    steps: o.steps,
                    cost: o.cost,
                    stats: o.stats,
                    lp_solved: 0,
                    report: ApproximationReport::SlidingWindow {
                        window_count: o.window_count,
                        retained_candidates: o.retained_candidates,
                        fallback_used: o.fallback_used,
                    },
                })
            }
            Approximation::FixedHorizon(h) => {
                let settings = horizon::Settings {
                    horizon: h.horizon,
                    min_progress: h.min_progress,
                    max_horizon: h.max_horizon.unwrap_or(h.horizon.max(20)),
                    max_prefix_states: h.max_prefix_states,
                    max_iterations: h
                        .max_iterations
                        .unwrap_or((2 * (labels.len() + self.net.transition_count())).max(20)),
                    max_expansions,
                };
                let names: Vec<&str> = self.names.iter().map(String::as_str).collect();
                horizon::align(
                    &self.net,
                    &self.model,
                    &names,
                    labels,
                    log_costs,
                    &settings,
                    deadline,
                )?
                .map(|o| Run {
                    steps: o.steps,
                    cost: o.cost,
                    stats: o.stats,
                    lp_solved: o.lp_solved,
                    report: ApproximationReport::FixedHorizon {
                        committed_horizons: o.committed_horizons,
                        fallback: o.fallback,
                    },
                })
            }
        })
    }
}

/// The outcome of one method run.
struct Run<'l> {
    steps: Vec<Step<'l>>,
    cost: u64,
    stats: Stats,
    lp_solved: usize,
    report: ApproximationReport,
}

/// Rejects the settings pm4py rejects.
fn validate(method: &Approximation) -> Result<()> {
    let bad = |what: &'static str| Err(Error::InvalidApproximation(what));
    match method {
        Approximation::TandemRepeats => Ok(()),
        Approximation::SlidingWindow(w) => {
            if w.window_size < 1 {
                return bad("window_size must be at least 1");
            }
            if w.max_candidates < 1 {
                return bad("max_candidates must be at least 1");
            }
            Ok(())
        }
        Approximation::FixedHorizon(h) => {
            if h.horizon < 1 || h.min_progress > h.horizon {
                return bad("require 1 <= horizon and min_progress <= horizon");
            }
            if h.max_horizon.is_some_and(|m| m < h.horizon) {
                return bad("max_horizon must not be smaller than horizon");
            }
            Ok(())
        }
    }
}

/// pm4py's trace fitness: costs are divided by the standard move cost with
/// integer division.
fn trace_fitness(cost: u64, best_worst_cost: u64) -> f64 {
    let num = cost / STD_LOG_MOVE_COST;
    let den = best_worst_cost / STD_LOG_MOVE_COST;
    if den > 0 {
        1.0 - num as f64 / den as f64
    } else {
        0.0
    }
}

impl LogAlignment<ApproximateAlignment> {
    /// The log-level fitness of these alignments, computed as for exact
    /// alignments (pm4py's `replay_fitness.alignment_based.evaluate`).
    pub fn fitness(&self) -> AlignmentFitness {
        AlignmentFitness::evaluate(self.traces().map(|a| a.map(|a| &a.alignment)))
    }
}
