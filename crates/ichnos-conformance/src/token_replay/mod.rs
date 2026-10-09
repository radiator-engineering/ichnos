//! Token-based replay (pm4py's `algo/conformance/tokenreplay`, variant
//! `token_replay`).
//!
//! Token replay fires the transition of each event of a trace on an
//! accepting Petri net, starting from the initial marking. When a
//! transition is not enabled, the replay first fires silent transitions on
//! shortest paths towards its input places. If that is not enough it adds
//! the *missing* tokens and fires anyway. At the end it tries to reach the
//! final marking through silent transitions. Tokens left over are
//! *remaining*. A trace's fitness is
//!
//! ```text
//! 0.5 * (1 - missing / consumed) + 0.5 * (1 - remaining / produced)
//! ```
//!
//! where the initial marking counts as produced and the final marking as
//! consumed.
//!
//! ```
//! use ichnos_conformance::token_replay::{TokenReplayOptions, TokenReplayer};
//! use ichnos_model::{Marking, PetriNet};
//!
//! // source -> a -> sink
//! let mut net = PetriNet::new("n");
//! let source = net.add_place("source");
//! let sink = net.add_place("sink");
//! let a = net.add_transition("a", Some("a"));
//! net.add_input_arc(source, a).unwrap();
//! net.add_output_arc(a, sink).unwrap();
//! let im = Marking::from([(source, 1)]);
//! let fm = Marking::from([(sink, 1)]);
//!
//! let replayer = TokenReplayer::new(&net, &im, &fm, TokenReplayOptions::default()).unwrap();
//! let fit = replayer.replay(&["a"]).unwrap();
//! assert!(fit.is_fit);
//! assert_eq!(fit.fitness, 1.0);
//!
//! // Firing `a` twice needs one token more than `source` holds, and leaves
//! // one token too many in `sink`.
//! let twice = replayer.replay(&["a", "a"]).unwrap();
//! assert!(!twice.is_fit);
//! assert_eq!((twice.missing_tokens, twice.consumed_tokens), (1, 3));
//! assert_eq!((twice.remaining_tokens, twice.produced_tokens), (1, 3));
//! assert!((twice.fitness - 2.0 / 3.0).abs() < 1e-12);
//! ```
//!
//! [`replay_log`] and [`fitness_token_based_replay`] are pm4py's
//! `conformance_diagnostics_token_based_replay` and
//! `fitness_token_based_replay`.
//!
//! # Matching pm4py
//!
//! Token replay is a heuristic. Which silent transitions it fires depends on
//! the order in which pm4py walks Python sets of places, transitions and
//! arcs, and that order follows object addresses. pm4py sorts by node name
//! in most places; where it does not, ichnos takes nodes in name order too.
//! On nets with unique names this reproduces pm4py's replay on every golden
//! case, transition for transition.
//!
//! Arcs of every kind count as ordinary arcs, as in pm4py's classic
//! semantics. pm4py's post-fix and marking-to-activity caches are off by
//! default and are not ported; they only speed up the replay.

mod net;
mod replay;

use std::collections::{BTreeSet, HashMap};

use ichnos_core::{EventKeys, EventLog, Variants};
use ichnos_model::{Marking, PetriNet, TransitionId};

use self::net::{ReplayNet, Tokens};
use crate::error::{Error, Result};

/// Options of a token replay. The defaults are pm4py's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenReplayOptions {
    /// A trace fits only when no tokens remain (pm4py's
    /// `consider_remaining_in_fitness`, default `true`). Otherwise missing
    /// tokens alone decide.
    pub consider_remaining_in_fitness: bool,
    /// After the last event, fire silent transitions to reach the final
    /// marking (pm4py's `try_to_reach_final_marking_through_hidden`,
    /// default `true`).
    pub try_to_reach_final_marking_through_hidden: bool,
    /// Stop the replay at the first transition that is not enabled, and
    /// count one missing token for it (pm4py's `stop_immediately_unfit`,
    /// default `false`).
    pub stop_immediately_unfit: bool,
    /// Fire silent transitions to enable the transition of an event (pm4py's
    /// `walk_through_hidden_trans`, default `true`).
    pub walk_through_hidden_transitions: bool,
    /// Search all orders of the silent transitions on the shortest paths,
    /// breadth first, for a marking that enables the event's transition
    /// (pm4py's `exhaustive_invisible_exploration`, default `false`). The
    /// default walks the paths once, in order.
    pub exhaustive_invisible_exploration: bool,
    /// After a transition fires with missing tokens, empty the places that
    /// share an S-component with a place that just got its first token, and
    /// count one remaining token for each (pm4py's `cleaning_token_flood`,
    /// default `false`). S-components are only found for nets with one
    /// initial and one final place.
    pub cleaning_token_flood: bool,
    /// A trace with an activity that no transition has does not fit (pm4py's
    /// `consider_activities_not_in_model_in_fitness`, default `false`).
    pub consider_activities_not_in_model_in_fitness: bool,
}

impl Default for TokenReplayOptions {
    fn default() -> Self {
        Self {
            consider_remaining_in_fitness: true,
            try_to_reach_final_marking_through_hidden: true,
            stop_immediately_unfit: false,
            walk_through_hidden_transitions: true,
            exhaustive_invisible_exploration: false,
            cleaning_token_flood: false,
            consider_activities_not_in_model_in_fitness: false,
        }
    }
}

impl TokenReplayOptions {
    /// Sets [`consider_remaining_in_fitness`](Self::consider_remaining_in_fitness).
    pub fn consider_remaining_in_fitness(mut self, on: bool) -> Self {
        self.consider_remaining_in_fitness = on;
        self
    }

    /// Sets [`try_to_reach_final_marking_through_hidden`](Self::try_to_reach_final_marking_through_hidden).
    pub fn try_to_reach_final_marking_through_hidden(mut self, on: bool) -> Self {
        self.try_to_reach_final_marking_through_hidden = on;
        self
    }

    /// Sets [`stop_immediately_unfit`](Self::stop_immediately_unfit).
    pub fn stop_immediately_unfit(mut self, on: bool) -> Self {
        self.stop_immediately_unfit = on;
        self
    }

    /// Sets [`walk_through_hidden_transitions`](Self::walk_through_hidden_transitions).
    pub fn walk_through_hidden_transitions(mut self, on: bool) -> Self {
        self.walk_through_hidden_transitions = on;
        self
    }

    /// Sets [`exhaustive_invisible_exploration`](Self::exhaustive_invisible_exploration).
    pub fn exhaustive_invisible_exploration(mut self, on: bool) -> Self {
        self.exhaustive_invisible_exploration = on;
        self
    }

    /// Sets [`cleaning_token_flood`](Self::cleaning_token_flood).
    pub fn cleaning_token_flood(mut self, on: bool) -> Self {
        self.cleaning_token_flood = on;
        self
    }

    /// Sets [`consider_activities_not_in_model_in_fitness`](Self::consider_activities_not_in_model_in_fitness).
    pub fn consider_activities_not_in_model_in_fitness(mut self, on: bool) -> Self {
        self.consider_activities_not_in_model_in_fitness = on;
        self
    }
}

/// The token replay of one trace, with pm4py's diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct TraceReplay {
    /// pm4py's `trace_is_fit`: no tokens were missing during the replay
    /// and, with [`TokenReplayOptions::consider_remaining_in_fitness`], none
    /// remain. Tokens missing from the final marking do not count here.
    pub is_fit: bool,
    /// pm4py's `trace_fitness`, from the token counts below; 1 when nothing
    /// was consumed or produced.
    pub fitness: f64,
    /// The transitions fired, in order, silent ones included.
    pub activated_transitions: Vec<TransitionId>,
    /// The transitions that fired with missing tokens, in order.
    pub transitions_with_problems: Vec<TransitionId>,
    /// The marking at the end of the replay.
    pub reached_marking: Marking,
    /// Visible transitions enabled in the reached marking, directly or
    /// after silent transitions, as pm4py's
    /// `get_visible_transitions_eventually_enabled_by_marking` finds them.
    pub enabled_transitions_in_marking: BTreeSet<TransitionId>,
    /// Tokens added to fire transitions, plus tokens the reached marking
    /// lacks of the final marking.
    pub missing_tokens: u64,
    /// Tokens consumed by firing, plus the tokens of the final marking.
    pub consumed_tokens: u64,
    /// Tokens in the reached marking beyond the final marking, plus the
    /// tokens cleaned from a token flood.
    pub remaining_tokens: u64,
    /// Tokens produced by firing, plus the tokens of the initial marking.
    pub produced_tokens: u64,
}

/// Log fitness from token replay, as `pm4py.fitness_token_based_replay`
/// reports it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TokenReplayFitness {
    /// Percentage (0 to 100) of the traces that fit.
    pub percentage_of_fitting_traces: f64,
    /// Mean trace fitness.
    pub average_trace_fitness: f64,
    /// `0.5 * (1 - missing / consumed) + 0.5 * (1 - remaining / produced)`
    /// over the token counts summed over all traces.
    pub log_fitness: f64,
}

impl TokenReplayFitness {
    /// pm4py's `replay_fitness.token_replay.evaluate`. All three values are
    /// 0 when there are no traces or no tokens were consumed or produced.
    pub fn evaluate<'a>(replays: impl IntoIterator<Item = &'a TraceReplay>) -> Self {
        let mut traces = 0usize;
        let mut fit = 0usize;
        let mut sum_fitness = 0.0;
        let (mut m, mut c, mut r, mut p) = (0u64, 0u64, 0u64, 0u64);
        for t in replays {
            traces += 1;
            fit += usize::from(t.is_fit);
            sum_fitness += t.fitness;
            m += t.missing_tokens;
            c += t.consumed_tokens;
            r += t.remaining_tokens;
            p += t.produced_tokens;
        }
        if traces == 0 || c == 0 || p == 0 {
            return Self {
                percentage_of_fitting_traces: 0.0,
                average_trace_fitness: 0.0,
                log_fitness: 0.0,
            };
        }
        Self {
            percentage_of_fitting_traces: 100.0 * fit as f64 / traces as f64,
            average_trace_fitness: sum_fitness / traces as f64,
            log_fitness: 0.5 * (1.0 - m as f64 / c as f64) + 0.5 * (1.0 - r as f64 / p as f64),
        }
    }
}

/// The token replay of every trace of a log, computed once per variant.
#[derive(Debug, Clone)]
pub struct LogReplay {
    /// The variants of the log.
    pub variants: Variants,
    /// One replay per variant, in the order of `variants.variants`.
    pub replays: Vec<TraceReplay>,
    trace_variant: Vec<usize>,
}

impl LogReplay {
    fn new(variants: Variants, replays: Vec<TraceReplay>) -> Self {
        let traces = variants.iter().map(|v| v.count()).sum();
        let mut trace_variant = vec![0; traces];
        for (i, v) in variants.iter().enumerate() {
            for &t in &v.traces {
                trace_variant[t] = i;
            }
        }
        Self {
            variants,
            replays,
            trace_variant,
        }
    }

    /// The number of traces in the log.
    pub fn trace_count(&self) -> usize {
        self.trace_variant.len()
    }

    /// The replay of trace `i`.
    ///
    /// # Panics
    ///
    /// Panics if `i` is not a trace index of the log.
    pub fn trace(&self, i: usize) -> &TraceReplay {
        &self.replays[self.trace_variant[i]]
    }

    /// The replay of every trace, in log order, as pm4py's
    /// `conformance_diagnostics_token_based_replay` lists them.
    pub fn traces(&self) -> impl Iterator<Item = &TraceReplay> + '_ {
        self.trace_variant.iter().map(|&v| &self.replays[v])
    }

    /// The log-level fitness of these replays (pm4py's
    /// `replay_fitness.token_replay.evaluate`).
    pub fn fitness(&self) -> TokenReplayFitness {
        TokenReplayFitness::evaluate(self.traces())
    }
}

/// An accepting Petri net prepared for token replay.
///
/// Preparing finds the shortest paths of silent transitions between places
/// (pm4py's `get_places_shortest_path_by_hidden`) and, when token-flood
/// cleaning is on, the S-components. Both are reused for every trace.
#[derive(Debug, Clone)]
pub struct TokenReplayer {
    net: ReplayNet,
    options: TokenReplayOptions,
    initial: Tokens,
    final_tokens: Tokens,
    /// The places of the final marking, in name order.
    final_places: Vec<usize>,
    /// pm4py's `trans_map`: each label to the last of its transitions in
    /// name order.
    by_label: HashMap<String, TransitionId>,
    /// Each label to its transitions, in name order.
    labelled: HashMap<String, Vec<TransitionId>>,
    s_components: Vec<BTreeSet<usize>>,
}

impl TokenReplayer {
    /// Prepares `net` with its initial and final markings.
    ///
    /// Fails with [`Error::UnknownPlace`] if a marking puts tokens on a
    /// place that is not in the net.
    pub fn new(
        net: &PetriNet,
        initial_marking: &Marking,
        final_marking: &Marking,
        options: TokenReplayOptions,
    ) -> Result<Self> {
        for (p, _) in initial_marking.iter().chain(final_marking.iter()) {
            if !net.contains_place(p) {
                return Err(Error::UnknownPlace(p));
            }
        }
        let replay_net = ReplayNet::new(net);
        let initial = replay_net.tokens(initial_marking);
        let final_tokens = replay_net.tokens(final_marking);
        let final_places = replay_net.marked_by_name(&final_tokens).collect();
        let mut by_label = HashMap::new();
        let mut labelled: HashMap<String, Vec<TransitionId>> = HashMap::new();
        for &t in &replay_net.transitions_by_name {
            if let Some(label) = &net.transition(t).label {
                by_label.insert(label.to_string(), t);
                labelled.entry(label.to_string()).or_default().push(t);
            }
        }
        let s_components = if options.cleaning_token_flood {
            replay_net.s_components(net, initial_marking, final_marking)
        } else {
            Vec::new()
        };
        Ok(Self {
            net: replay_net,
            options,
            initial,
            final_tokens,
            final_places,
            by_label,
            labelled,
            s_components,
        })
    }

    /// The options this replayer was prepared with.
    pub fn options(&self) -> &TokenReplayOptions {
        &self.options
    }

    /// Replays one trace, given as its activity names.
    ///
    /// Fails with [`Error::Reachability`] when the silent transitions
    /// enabled in the reached marking lead to more markings than
    /// [`ichnos_model::petri::ReachabilityOptions`] allows by default.
    pub fn replay<S: AsRef<str>>(&self, trace: &[S]) -> Result<TraceReplay> {
        self.replay_trace(trace)
    }

    /// Replays every variant of `log` once.
    pub fn replay_log(&self, log: &EventLog, keys: &EventKeys) -> Result<LogReplay> {
        let variants = log.variants(keys)?;
        let replays = variants
            .iter()
            .map(|v| {
                let names: Vec<&str> = variants.names(v).collect();
                self.replay(&names)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(LogReplay::new(variants, replays))
    }
}

/// Token replay of every trace of `log` (pm4py's
/// `conformance_diagnostics_token_based_replay`).
pub fn replay_log(
    log: &EventLog,
    net: &PetriNet,
    initial_marking: &Marking,
    final_marking: &Marking,
    keys: &EventKeys,
    options: TokenReplayOptions,
) -> Result<LogReplay> {
    TokenReplayer::new(net, initial_marking, final_marking, options)?.replay_log(log, keys)
}

/// Token-based fitness of `log` on the accepting net (pm4py's
/// `fitness_token_based_replay`), with pm4py's default options.
pub fn fitness_token_based_replay(
    log: &EventLog,
    net: &PetriNet,
    initial_marking: &Marking,
    final_marking: &Marking,
    keys: &EventKeys,
) -> Result<TokenReplayFitness> {
    let options = TokenReplayOptions::default();
    Ok(replay_log(log, net, initial_marking, final_marking, keys, options)?.fitness())
}

#[cfg(test)]
mod tests;
