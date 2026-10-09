//! Proxy-trie online approximate alignments with bounded, decaying candidates.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use ichnos_conformance::alignments::{Aligner, AlignmentOptions, Heuristic, Move};
use ichnos_core::{AttributeValue, Event};
use ichnos_model::{Marking, PetriNet, TransitionId};

use crate::conformance::Input;
use crate::{Error, MissingEventPolicy, Result, StreamSink, StreamingConformanceOptions};

use ichnos_conformance::alignments::costs::{
    STD_LOG_MOVE_COST, STD_MODEL_MOVE_COST, STD_SILENT_MOVE_COST,
};

/// Settings for the IWS proxy-trie algorithm.
#[derive(Debug, Clone)]
pub struct StreamingAlignmentOptions {
    /// Event keys and incomplete-event handling. Defaults to Reject for IWS.
    pub events: StreamingConformanceOptions,
    /// Maximum number of visible trie edges inspected for a matching activity.
    pub look_ahead: usize,
    /// Initial candidate lifetime in events.
    pub decay_time: f64,
    /// Multiplier per accumulated visible deviation, in (0, 1].
    pub discount_factor: f64,
    /// Maximum surviving candidates per case.
    pub max_states: usize,
    /// A truthy value under this key completes a case after consuming its event.
    pub complete_case_attribute: String,
    /// Maximum distinct visible traces in an automatically generated proxy.
    pub proxy_traces: usize,
    /// Maximum model firings per automatically generated path.
    pub max_trace_length: usize,
    /// Maximum paths expanded while generating the proxy.
    pub max_expansions: usize,
}

impl Default for StreamingAlignmentOptions {
    fn default() -> Self {
        Self {
            events: StreamingConformanceOptions {
                missing: MissingEventPolicy::Reject,
                ..Default::default()
            },
            look_ahead: 3,
            decay_time: 10.0,
            discount_factor: 0.9,
            max_states: 20,
            complete_case_attribute: "@@complete".into(),
            proxy_traces: 100,
            max_trace_length: 100,
            max_expansions: 100_000,
        }
    }
}

/// A labeled alignment step. Transition IDs retain silent/duplicate identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamingAlignmentStep {
    /// Consumed activity; None for a model-only move.
    pub activity: Option<String>,
    /// Fired transition; None for a log-only move.
    pub transition: Option<TransitionId>,
}

/// Prefix or completed alignment against the finite proxy behavior.
///
/// Its cost is an upper bound for the corresponding alignment problem, not an
/// optimality guarantee. Runtime measurements are deliberately omitted.
#[derive(Debug, Clone, PartialEq)]
pub struct StreamingAlignmentResult {
    /// Moves in replay order.
    pub alignment: Vec<StreamingAlignmentStep>,
    /// Standard cost: log/visible model moves cost 10000, silent moves cost 1.
    pub cost: u64,
    /// Whether both projections and every model firing are valid.
    pub is_valid: bool,
    /// Whether a suffix reaching the final marking has been included.
    pub is_complete: bool,
    /// Number of surviving per-case candidates before completion.
    pub active_states: usize,
    /// Prefix trie node; completed results have None.
    pub trie_node: Option<usize>,
    /// Remaining candidate lifetime; completed results have None.
    pub decay: Option<f64>,
}

#[derive(Debug, Clone, Default)]
struct Node {
    label: Option<String>,
    segment: Vec<TransitionId>,
    children: Vec<usize>,
    trailing: Vec<Vec<TransitionId>>,
}

#[derive(Debug, Clone, Default)]
struct Suffix {
    cost: u64,
    next: Option<usize>,
    trailing: Vec<TransitionId>,
}

#[derive(Debug, Clone)]
struct State {
    node: usize,
    steps: Vec<StreamingAlignmentStep>,
    cost: u64,
    decay: f64,
}

/// IWS online approximate alignments over a finite trie of complete model runs.
///
/// [`Self::new`] uses bounded deterministic breadth-first model exploration.
/// [`Self::from_proxy_traces`] reuses the merged exact aligner to prepare a
/// proxy log, and [`Self::with_proxy_sequences`] accepts complete model runs.
/// Equal-cost choices keep proxy insertion order. A completed result continues
/// to shadow its case's subsequent prefixes, following native IWS semantics;
/// call [`Self::remove_case`] to release history and reuse the ID cleanly.
#[derive(Debug, Clone)]
pub struct StreamingAlignments {
    net: PetriNet,
    initial: Marking,
    final_marking: Marking,
    options: StreamingAlignmentOptions,
    input: Input,
    nodes: Vec<Node>,
    suffixes: Vec<Suffix>,
    cases: BTreeMap<String, Vec<State>>,
    completed: BTreeMap<String, StreamingAlignmentResult>,
    processed: usize,
}

impl StreamingAlignments {
    /// Create an IWS consumer with bounded deterministic proxy generation.
    /// Random simulation is replaced by breadth-first exploration ordered by
    /// transition name/ID; loops are bounded by max_trace_length/max_expansions.
    pub fn new(
        net: PetriNet,
        initial: Marking,
        final_marking: Marking,
        options: StreamingAlignmentOptions,
    ) -> Result<Self> {
        validate(&net, &initial, &final_marking, &options)?;
        let mut queue = VecDeque::from([(initial.clone(), Vec::new())]);
        let mut seen = BTreeSet::new();
        let mut sequences = Vec::new();
        for _ in 0..options.max_expansions {
            let Some((marking, path)) = queue.pop_front() else {
                break;
            };
            if marking == final_marking {
                let labels: Vec<_> = path
                    .iter()
                    .filter_map(|&t| net.transition(t).label.clone())
                    .collect();
                if seen.insert(labels) {
                    sequences.push(path);
                    if sequences.len() >= options.proxy_traces {
                        break;
                    }
                }
                continue;
            }
            if path.len() >= options.max_trace_length {
                continue;
            }
            let mut enabled = net.enabled_transitions(&marking);
            enabled.sort_by(|&a, &b| {
                net.transition(a)
                    .name
                    .cmp(&net.transition(b).name)
                    .then(a.index().cmp(&b.index()))
            });
            for t in enabled {
                // Bound queued paths too, not only popped paths.
                if queue.len() >= options.max_expansions {
                    break;
                }
                let next = net
                    .fire(t, &marking)
                    .expect("enabled transition checked above");
                let mut path = path.clone();
                path.push(t);
                queue.push_back((next, path));
            }
        }
        Self::with_proxy_sequences(net, initial, final_marking, sequences, options)
    }

    /// Build proxy runs by aligning activity traces with the merged exact
    /// aligner. Only the first run for each distinct visible trace is retained.
    pub fn from_proxy_traces<S: AsRef<str>>(
        net: PetriNet,
        initial: Marking,
        final_marking: Marking,
        traces: &[Vec<S>],
        options: StreamingAlignmentOptions,
    ) -> Result<Self> {
        validate(&net, &initial, &final_marking, &options)?;
        let aligner = Aligner::new(
            &net,
            &initial,
            &final_marking,
            AlignmentOptions::default().heuristic(Heuristic::None),
        )?;
        let mut sequences = Vec::new();
        for trace in traces {
            let aligned = aligner
                .align(trace)?
                .expect("no alignment time limit configured");
            sequences.push(aligned.moves.iter().filter_map(Move::transition).collect());
        }
        Self::with_proxy_sequences(net, initial, final_marking, sequences, options)
    }

    /// Build a proxy from supplied complete transition runs. Disabled,
    /// foreign, non-final runs and special arcs return typed errors.
    pub fn with_proxy_sequences(
        net: PetriNet,
        initial: Marking,
        final_marking: Marking,
        sequences: Vec<Vec<TransitionId>>,
        options: StreamingAlignmentOptions,
    ) -> Result<Self> {
        validate(&net, &initial, &final_marking, &options)?;
        let mut nodes = vec![Node::default()];
        let mut seen = BTreeSet::new();
        for sequence in sequences {
            let mut marking = initial.clone();
            for &t in &sequence {
                if !net.contains_transition(t) || !net.is_enabled(t, &marking) {
                    return Err(Error::InvalidConformanceOption(
                        "proxy run contains a foreign or disabled transition",
                    ));
                }
                marking = net
                    .fire(t, &marking)
                    .expect("enabled transition checked above");
            }
            if marking != final_marking {
                return Err(Error::InvalidConformanceOption(
                    "proxy run does not reach the final marking",
                ));
            }
            let labels: Vec<_> = sequence
                .iter()
                .filter_map(|&t| net.transition(t).label.clone())
                .collect();
            if !seen.insert(labels) {
                continue;
            }
            let mut node = 0;
            let mut segment = Vec::new();
            for t in sequence {
                segment.push(t);
                if let Some(label) = net.transition(t).label.as_ref() {
                    let child = nodes[node]
                        .children
                        .iter()
                        .copied()
                        .find(|&n| nodes[n].segment == segment);
                    node = match child {
                        Some(n) => n,
                        None => {
                            let n = nodes.len();
                            nodes.push(Node {
                                label: Some(label.to_string()),
                                segment: std::mem::take(&mut segment),
                                ..Default::default()
                            });
                            nodes[node].children.push(n);
                            n
                        }
                    };
                    segment.clear();
                }
            }
            if !nodes[node].trailing.contains(&segment) {
                nodes[node].trailing.push(segment);
            }
        }
        if nodes.iter().all(|n| n.trailing.is_empty()) {
            return Err(Error::InvalidConformanceOption(
                "proxy behavior contains no complete model trace",
            ));
        }
        // Children are allocated after parents. Compute suffixes backwards to
        // avoid recursion and quadratic suffix copies on long proxy traces;
        // insertion order breaks ties.
        let mut suffixes = vec![Suffix::default(); nodes.len()];
        for n in (0..nodes.len()).rev() {
            let mut best = nodes[n]
                .trailing
                .iter()
                .map(|s| Suffix {
                    cost: path_cost(&net, s),
                    next: None,
                    trailing: s.clone(),
                })
                .min_by_key(|s| s.cost);
            for &child in &nodes[n].children {
                let cost = path_cost(&net, &nodes[child].segment) + suffixes[child].cost;
                if best.as_ref().is_none_or(|b| cost < b.cost) {
                    best = Some(Suffix {
                        cost,
                        next: Some(child),
                        trailing: Vec::new(),
                    });
                }
            }
            suffixes[n] = best.expect("every inserted node leads to a complete proxy run");
        }
        Ok(Self {
            input: Input {
                options: options.events.clone(),
                ..Default::default()
            },
            net,
            initial,
            final_marking,
            options,
            nodes,
            suffixes,
            cases: BTreeMap::new(),
            completed: BTreeMap::new(),
            processed: 0,
        })
    }

    /// The immutable model against which transition IDs are interpreted.
    pub fn net(&self) -> &PetriNet {
        &self.net
    }

    /// Successfully consumed complete events across all cases.
    pub fn processed_events(&self) -> usize {
        self.processed
    }

    /// Incomplete events ignored under the optional Ignore policy.
    pub fn skipped_events(&self) -> usize {
        self.input.skipped
    }

    /// Current prefix results, overlaid by retained completed results.
    pub fn get(&self) -> BTreeMap<String, StreamingAlignmentResult> {
        let mut result: BTreeMap<_, _> = self
            .cases
            .iter()
            .map(|(c, states)| {
                let chosen = states
                    .iter()
                    .min_by_key(|s| s.cost)
                    .expect("each case retains a candidate");
                (c.clone(), self.result(chosen, states.len(), false))
            })
            .collect();
        result.extend(self.completed.clone());
        result
    }

    /// Complete a known case by appending the cheapest retained trie suffix.
    /// Returns None for an unknown case; retains active states and completion.
    pub fn finish(&mut self, case: &str) -> Option<StreamingAlignmentResult> {
        let states = self.cases.get(case)?;
        let chosen =
            states
                .iter()
                .map(|s| {
                    let mut s = s.clone();
                    s.cost += self.suffixes[s.node].cost;
                    let mut node = s.node;
                    while let Some(child) = self.suffixes[node].next {
                        s.steps.extend(self.nodes[child].segment.iter().map(|&t| {
                            StreamingAlignmentStep {
                                activity: None,
                                transition: Some(t),
                            }
                        }));
                        node = child;
                    }
                    s.steps
                        .extend(self.suffixes[node].trailing.iter().map(|&t| {
                            StreamingAlignmentStep {
                                activity: None,
                                transition: Some(t),
                            }
                        }));
                    s
                })
                .min_by_key(|s| s.cost)
                .expect("each case retains a candidate");
        let result = self.result(&chosen, states.len(), true);
        self.completed.insert(case.into(), result.clone());
        Some(result)
    }

    /// Release both active and completed history for a case. This explicit
    /// memory/restart lifecycle is an extension to native IWS.
    pub fn remove_case(&mut self, case: &str) -> bool {
        let active = self.cases.remove(case).is_some();
        self.completed.remove(case).is_some() || active
    }

    /// Consume one event. Direct matches reset decay; look-ahead deviations and
    /// log moves discount it. Missing-field rejection leaves case state intact.
    pub fn push(&mut self, event: &Event) -> Result<()> {
        let Some((case, activity)) = self.input.fields(event, &[])? else {
            return Ok(());
        };
        let initial = vec![State {
            node: 0,
            steps: Vec::new(),
            cost: 0,
            decay: self.options.decay_time,
        }];
        let states = self.cases.get(&case).unwrap_or(&initial);
        let mut generated = Vec::new();
        for s in states {
            let remaining = s.decay - 1.0;
            let mut log = s.clone();
            log.steps.push(StreamingAlignmentStep {
                activity: Some(activity.clone()),
                transition: None,
            });
            log.cost += STD_LOG_MOVE_COST;
            log.decay = self.discounted(remaining, log.cost);
            generated.push(log);
            let mut stack = vec![(s.node, Vec::new())];
            while let Some((node, path)) = stack.pop() {
                if path.len() >= self.options.look_ahead {
                    continue;
                }
                for &child in &self.nodes[node].children {
                    let mut path = path.clone();
                    path.push(child);
                    if self.nodes[child].label.as_deref() == Some(&activity) {
                        let mut next = s.clone();
                        next.node = child;
                        for (i, &n) in path.iter().enumerate() {
                            for (j, &t) in self.nodes[n].segment.iter().enumerate() {
                                let sync =
                                    i + 1 == path.len() && j + 1 == self.nodes[n].segment.len();
                                next.steps.push(StreamingAlignmentStep {
                                    activity: sync.then(|| activity.clone()),
                                    transition: Some(t),
                                });
                                if !sync {
                                    next.cost += transition_cost(&self.net, t);
                                }
                            }
                        }
                        next.decay = if path.len() == 1 {
                            self.options.decay_time
                        } else {
                            self.discounted(remaining, next.cost)
                        };
                        generated.push(next);
                    }
                    stack.push((child, path));
                }
            }
        }
        generated.sort_by(|a, b| {
            a.cost
                .cmp(&b.cost)
                .then_with(|| b.decay.total_cmp(&a.decay))
        });
        let mut seen = BTreeSet::new();
        let mut survivors: Vec<_> = generated
            .iter()
            .filter(|s| s.decay > 0.0 && seen.insert(s.node))
            .take(self.options.max_states)
            .cloned()
            .collect();
        if survivors.is_empty() {
            // Native fallback min(cost) preserves generation order, not the
            // decay-sorted order; equal-cost candidates share the same node
            // preference here because all exhausted decays are zero.
            let mut survivor = generated
                .into_iter()
                .min_by_key(|s| s.cost)
                .expect("log moves always generate a candidate");
            survivor.decay = 1.0;
            survivors.push(survivor);
        }
        self.cases.insert(case.clone(), survivors);
        self.processed += 1;
        if event
            .get(&self.options.complete_case_attribute)
            .is_some_and(truthy)
        {
            self.finish(&case);
        }
        Ok(())
    }

    fn discounted(&self, remaining: f64, cost: u64) -> f64 {
        remaining
            .min(
                self.options.decay_time
                    * self
                        .options
                        .discount_factor
                        .powf((cost / STD_LOG_MOVE_COST) as f64),
            )
            .max(0.0)
    }

    fn result(&self, s: &State, active_states: usize, complete: bool) -> StreamingAlignmentResult {
        let mut marking = self.initial.clone();
        let mut valid = true;
        for step in &s.steps {
            if let Some(t) = step.transition {
                if step
                    .activity
                    .as_ref()
                    .is_some_and(|a| self.net.transition(t).label.as_deref() != Some(a))
                {
                    valid = false;
                    break;
                }
                match self.net.fire(t, &marking) {
                    Ok(m) => marking = m,
                    Err(_) => {
                        valid = false;
                        break;
                    }
                }
            }
        }
        StreamingAlignmentResult {
            alignment: s.steps.clone(),
            cost: s.cost,
            is_valid: valid && (!complete || marking == self.final_marking),
            is_complete: complete,
            active_states,
            trie_node: (!complete).then_some(s.node),
            decay: (!complete).then_some(s.decay),
        }
    }
}

impl StreamSink for StreamingAlignments {
    fn push(&mut self, event: &Event) -> Result<()> {
        self.push(event)
    }
}

fn transition_cost(net: &PetriNet, t: TransitionId) -> u64 {
    if net.transition(t).is_silent() {
        STD_SILENT_MOVE_COST
    } else {
        STD_MODEL_MOVE_COST
    }
}

fn path_cost(net: &PetriNet, path: &[TransitionId]) -> u64 {
    path.iter().map(|&t| transition_cost(net, t)).sum()
}

fn validate(
    net: &PetriNet,
    initial: &Marking,
    final_marking: &Marking,
    o: &StreamingAlignmentOptions,
) -> Result<()> {
    if o.look_ahead == 0
        || o.max_states == 0
        || !o.decay_time.is_finite()
        || o.decay_time <= 0.0
        || !o.discount_factor.is_finite()
        || o.discount_factor <= 0.0
        || o.discount_factor > 1.0
        || o.proxy_traces == 0
        || o.max_trace_length == 0
        || o.max_expansions == 0
    {
        return Err(Error::InvalidConformanceOption(
            "look-ahead, decay, state/proxy limits must be positive and discount in (0,1]",
        ));
    }
    if net.has_special_arcs()
        || initial
            .iter()
            .chain(final_marking.iter())
            .any(|(p, _)| !net.contains_place(p))
    {
        return Err(Error::InvalidConformanceOption(
            "normal arcs and model-owned marking places required",
        ));
    }
    Ok(())
}

fn truthy(v: &AttributeValue) -> bool {
    match v.plain() {
        AttributeValue::Bool(v) => *v,
        AttributeValue::Int(v) => *v != 0,
        AttributeValue::Float(v) => *v != 0.0,
        AttributeValue::String(v) | AttributeValue::Id(v) => !v.is_empty(),
        AttributeValue::List(v) => !v.is_empty(),
        AttributeValue::Container(v) => !v.is_empty(),
        AttributeValue::Date(_) => true,
        AttributeValue::Meta(_) => unreachable!("plain strips metadata"),
    }
}
