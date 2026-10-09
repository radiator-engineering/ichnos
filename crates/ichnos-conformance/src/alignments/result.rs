//! Alignment results.

use ichnos_core::Variants;
use ichnos_model::{PetriNet, TransitionId};

/// One move of an alignment.
///
/// pm4py writes a move as a pair of labels, with `>>` for "no move":
/// `(a, a)`, `(a, >>)` or `(>>, a)`. A move here names the event by its
/// index in the trace and the transition by its id, so silent transitions
/// and duplicate labels stay distinct. [`Move::labels`] gives pm4py's pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Move {
    /// The event and a transition with the same label happen together.
    Sync {
        /// Index of the event in the trace.
        event: usize,
        /// The transition that fires.
        transition: TransitionId,
    },
    /// The event happens and the model does not move.
    Log {
        /// Index of the event in the trace.
        event: usize,
    },
    /// The model fires a transition that no event matches.
    Model {
        /// The transition that fires.
        transition: TransitionId,
    },
}

impl Move {
    /// The index of the event in the trace, unless this is a model move.
    pub fn event(&self) -> Option<usize> {
        match *self {
            Move::Sync { event, .. } | Move::Log { event } => Some(event),
            Move::Model { .. } => None,
        }
    }

    /// The transition that fires, unless this is a log move.
    pub fn transition(&self) -> Option<TransitionId> {
        match *self {
            Move::Sync { transition, .. } | Move::Model { transition } => Some(transition),
            Move::Log { .. } => None,
        }
    }

    /// pm4py's label pair for this move. The trace side is the event's
    /// activity, or `None` for a model move (pm4py's `>>`). The model side
    /// is `None` for a log move (`>>`), and `Some(None)` for a silent
    /// transition (pm4py's `None` label).
    pub fn labels<'a, S: AsRef<str>>(
        &self,
        trace: &'a [S],
        net: &'a PetriNet,
    ) -> (Option<&'a str>, Option<Option<&'a str>>) {
        let log = self.event().map(|e| trace[e].as_ref());
        let model = self
            .transition()
            .map(|t| net.transition(t).label.as_deref());
        (log, model)
    }
}

/// The optimal alignment of one trace, with pm4py's diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct TraceAlignment {
    /// The moves, in order.
    pub moves: Vec<Move>,
    /// The total cost of the moves.
    pub cost: u64,
    /// pm4py's trace fitness: `1 - (cost / 10000) / (bwc / 10000)`, with
    /// integer division, or 0 when `bwc / 10000` is 0.
    pub fitness: f64,
    /// pm4py's `bwc`: the cost of moving on the log for every event plus
    /// the cost of the cheapest alignment of the empty trace.
    pub best_worst_cost: u64,
    /// Markings taken off the queue and expanded.
    pub visited_states: usize,
    /// States put on the queue.
    pub queued_states: usize,
    /// Moves tried from expanded markings.
    pub traversed_arcs: usize,
    /// Linear programs solved (zero without the state-equation heuristic).
    pub lp_solved: usize,
}

impl TraceAlignment {
    /// Whether the trace fits: pm4py counts a trace as fit when its fitness
    /// is exactly 1.
    pub fn is_fit(&self) -> bool {
        self.fitness == 1.0
    }
}

/// The alignments of every trace of a log, computed once per variant.
#[derive(Debug, Clone)]
pub struct LogAlignment {
    /// The variants of the log.
    pub variants: Variants,
    /// One alignment per variant, in the order of `variants.variants`.
    /// `None` when the search hit its time limit.
    pub alignments: Vec<Option<TraceAlignment>>,
    trace_variant: Vec<usize>,
}

impl LogAlignment {
    pub(crate) fn new(variants: Variants, alignments: Vec<Option<TraceAlignment>>) -> Self {
        let traces = variants.iter().map(|v| v.count()).sum();
        let mut trace_variant = vec![0; traces];
        for (i, v) in variants.iter().enumerate() {
            for &t in &v.traces {
                trace_variant[t] = i;
            }
        }
        Self {
            variants,
            alignments,
            trace_variant,
        }
    }

    /// The number of traces in the log.
    pub fn trace_count(&self) -> usize {
        self.trace_variant.len()
    }

    /// The alignment of trace `i`, or `None` if its search hit the time
    /// limit.
    ///
    /// # Panics
    ///
    /// Panics if `i` is not a trace index of the log.
    pub fn trace(&self, i: usize) -> Option<&TraceAlignment> {
        self.alignments[self.trace_variant[i]].as_ref()
    }

    /// The alignment of every trace, in log order, as pm4py's
    /// `conformance_diagnostics_alignments` lists them.
    pub fn traces(&self) -> impl Iterator<Item = Option<&TraceAlignment>> + '_ {
        self.trace_variant
            .iter()
            .map(|&v| self.alignments[v].as_ref())
    }

    /// The log-level fitness of these alignments (pm4py's
    /// `replay_fitness.alignment_based.evaluate`).
    pub fn fitness(&self) -> AlignmentFitness {
        AlignmentFitness::evaluate(self.traces())
    }
}

/// Log fitness from alignments, as `pm4py.fitness_alignments` reports it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlignmentFitness {
    /// Percentage (0 to 100) of the aligned traces that fit.
    pub percentage_of_fitting_traces: f64,
    /// Mean trace fitness over the aligned traces.
    pub average_trace_fitness: f64,
    /// `1 - sum(cost) / sum(bwc)` over the aligned traces.
    pub log_fitness: f64,
}

impl AlignmentFitness {
    /// Evaluates trace alignments; `None` entries (timed out) are skipped,
    /// as in pm4py.
    pub fn evaluate<'a>(alignments: impl IntoIterator<Item = Option<&'a TraceAlignment>>) -> Self {
        let mut traces = 0usize;
        let mut fit = 0usize;
        let mut sum_fitness = 0.0;
        let mut sum_cost = 0.0;
        let mut sum_bwc = 0.0;
        for a in alignments.into_iter().flatten() {
            traces += 1;
            if a.is_fit() {
                fit += 1;
            }
            sum_fitness += a.fitness;
            sum_cost += a.cost as f64;
            sum_bwc += a.best_worst_cost as f64;
        }
        if traces == 0 {
            return Self {
                percentage_of_fitting_traces: 0.0,
                average_trace_fitness: 0.0,
                log_fitness: 0.0,
            };
        }
        let log_fitness = if sum_bwc > 0.0 {
            1.0 - sum_cost / sum_bwc
        } else {
            1.0
        };
        Self {
            percentage_of_fitting_traces: 100.0 * fit as f64 / traces as f64,
            average_trace_fitness: sum_fitness / traces as f64,
            log_fitness,
        }
    }
}
