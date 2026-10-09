//! Tree top-bottom, Petri-net basic and DFG probability-ordered playout.
use chrono::{DateTime, Utc};
use ichnos_core::{Event, EventLog, Trace};
use ichnos_model::{
    Label,
    dfg::Dfg,
    petri::{AcceptingPetriNet, TransitionId},
    process_tree::{Operator, ProcessTree},
};
use rand::{Rng, RngExt, seq::SliceRandom};
use std::collections::{BTreeMap, VecDeque};

/// A model accepted by the playout entry point.
#[derive(Debug, Clone, Copy)]
pub enum Model<'a> {
    /// Top-bottom tree execution.
    Tree(&'a ProcessTree),
    /// Prefix-safe DECLARE generation (without final obligation checks).
    Declare(&'a ichnos_discovery::declare::DeclareModel),
    /// Basic or explicitly weighted transition execution.
    PetriNet(&'a AcceptingPetriNet),
    /// Probability-ordered variant enumeration.
    Dfg(&'a Dfg),
}
/// Invalid models/options or execution limits.
#[derive(Debug, thiserror::Error)]
pub enum SimulationError {
    /// An option violates its allowed range.
    #[error("invalid simulation options: {0}")]
    Options(&'static str),
    /// The tree is malformed or contains an operator unsupported by top-bottom.
    #[error("invalid tree: {0}")]
    Tree(String),
    /// A bound was reached; no partial log is returned.
    #[error("simulation exceeded {0}")]
    Limit(&'static str),
    /// The graph has no positive start weight or has a reachable dead end.
    #[error("invalid DFG: {0}")]
    Dfg(&'static str),
}
/// Tree and Petri-net options. Randomness is supplied by the caller.
#[derive(Debug, Clone)]
pub struct PlayOutOptions {
    /// Number of traces (default 1000).
    pub traces: usize,
    /// Minimum random target length for DECLARE (default 3).
    pub declare_min_length: usize,
    /// Maximum random target length for DECLARE (default 15).
    pub declare_max_length: usize,
    /// Petri-net visible trace limit (default 1000).
    pub max_trace_length: usize,
    /// Bound all tree nodes and Petri-net firings, including silent ones.
    pub max_steps: usize,
    /// Bound attempts when only final-marking traces are accepted.
    pub max_attempts: usize,
    /// Keep only traces ending at the final marking.
    pub require_final: bool,
    /// Accept a marking that covers the final marking.
    pub final_marking_leq: bool,
    /// Optional transition weights. Missing weights are 1; the final stop has weight 1.
    pub transition_weights: BTreeMap<TransitionId, f64>,
    /// DFG enumeration options, independent of random trace count.
    pub dfg: DfgOptions,
}
impl Default for PlayOutOptions {
    fn default() -> Self {
        Self {
            traces: 1000,
            declare_min_length: 3,
            declare_max_length: 15,
            max_trace_length: 1000,
            max_steps: 100_000,
            max_attempts: 100_000,
            require_final: false,
            final_marking_leq: false,
            transition_weights: BTreeMap::new(),
            dfg: DfgOptions::default(),
        }
    }
}
/// Dispatches to top-bottom tree, basic/weighted Petri-net or classic DFG playout.
pub fn play_out<R: Rng + ?Sized>(
    model: Model<'_>,
    options: &PlayOutOptions,
    rng: &mut R,
) -> Result<EventLog, SimulationError> {
    if options.max_steps == 0
        || options
            .transition_weights
            .values()
            .any(|w| !w.is_finite() || *w < 0.)
    {
        return Err(SimulationError::Options(
            "positive step bound and finite nonnegative weights required",
        ));
    }
    match model {
        Model::Declare(model) => crate::declare::play_out_declare(model, options, rng),
        Model::Dfg(dfg) => play_out_dfg(dfg, &options.dfg),
        Model::Tree(tree) => {
            tree.validate()
                .map_err(|e| SimulationError::Tree(e.to_string()))?;
            let mut log = EventLog::default();
            for _ in 0..options.traces {
                let mut steps = options.max_steps;
                let sequence = execute(tree, rng, &mut steps)?;
                // Top-bottom has neither case IDs nor timestamps.
                let mut trace = Trace::new();
                for label in sequence.into_iter().flatten() {
                    let mut event = Event::new();
                    event.insert("concept:name", label.as_str());
                    trace.events.push(event);
                }
                log.traces.push(trace);
            }
            Ok(log)
        }
        Model::PetriNet(net) => petri(net, options, rng),
    }
}
fn execute<R: Rng + ?Sized>(
    tree: &ProcessTree,
    rng: &mut R,
    steps: &mut usize,
) -> Result<Vec<Option<Label>>, SimulationError> {
    if *steps == 0 {
        return Err(SimulationError::Limit("tree execution steps"));
    }
    *steps -= 1;
    let ProcessTree::Node(op, children) = tree else {
        return Ok(vec![tree.label().cloned()]);
    };
    let mut result = Vec::new();
    match op {
        Operator::Or => {
            return Err(SimulationError::Tree(
                "OR is unsupported by pm4py top-bottom playout".into(),
            ));
        }
        Operator::Xor => {
            return execute(&children[rng.random_range(0..children.len())], rng, steps);
        }
        Operator::Loop => loop {
            result.extend(execute(&children[0], rng, steps)?);
            if rng.random::<f64>() > 0.5 {
                break;
            }
            result.extend(execute(&children[1], rng, steps)?);
        },
        Operator::Sequence => {
            for child in children {
                result.extend(execute(child, rng, steps)?);
            }
        }
        Operator::Interleaving => {
            let mut order: Vec<_> = (0..children.len()).collect();
            order.shuffle(rng);
            for i in order {
                result.extend(execute(&children[i], rng, steps)?);
            }
        }
        Operator::Parallel => {
            let mut queues = Vec::new();
            let mut choices = Vec::new();
            for (i, child) in children.iter().enumerate() {
                let seq = execute(child, rng, steps)?;
                choices.extend(std::iter::repeat_n(i, seq.len()));
                queues.push(VecDeque::from(seq));
            }
            choices.shuffle(rng);
            for i in choices {
                result.push(queues[i].pop_front().expect("one choice per leaf"));
            }
        }
    }
    Ok(result)
}
fn petri<R: Rng + ?Sized>(
    net: &AcceptingPetriNet,
    o: &PlayOutOptions,
    rng: &mut R,
) -> Result<EventLog, SimulationError> {
    let mut log = EventLog::default();
    let mut timestamp = 10_000_000i64;
    for _ in 0..o.max_attempts {
        if log.traces.len() == o.traces {
            return Ok(log);
        }
        let mut marking = net.initial_marking.clone();
        let mut labels = Vec::new();
        let mut ended = false;
        for _ in 0..o.max_steps {
            if labels.len() >= o.max_trace_length {
                ended = true;
                break;
            }
            let enabled = net.net.enabled_transitions(&marking);
            if enabled.is_empty() {
                ended = true;
                break;
            }
            let stop = net.final_marking.is_covered_by(&marking)
                && (o.final_marking_leq || marking == net.final_marking);
            let total = enabled
                .iter()
                .map(|t| o.transition_weights.get(t).copied().unwrap_or(1.))
                .sum::<f64>()
                + f64::from(stop);
            if !total.is_finite() || total <= 0. {
                return Err(SimulationError::Options(
                    "enabled transitions have zero total weight",
                ));
            }
            let mut draw = rng.random::<f64>() * total;
            let mut selected = None;
            for t in enabled {
                draw -= o.transition_weights.get(&t).copied().unwrap_or(1.);
                if draw < 0. {
                    selected = Some(t);
                    break;
                }
            }
            let Some(t) = selected else {
                ended = true;
                break;
            };
            if let Some(label) = &net.net.transition(t).label {
                labels.push(label.clone());
            }
            marking = net
                .net
                .fire(t, &marking)
                .expect("selected enabled transition");
        }
        if !ended {
            return Err(SimulationError::Limit("Petri-net firing steps"));
        }
        let final_reached = if o.final_marking_leq {
            net.final_marking.is_covered_by(&marking)
        } else {
            marking == net.final_marking
        };
        if !o.require_final || final_reached {
            log.traces
                .push(trace_from_labels(labels, log.traces.len(), &mut timestamp));
        }
    }
    if log.traces.len() == o.traces {
        Ok(log)
    } else {
        Err(SimulationError::Limit("Petri-net attempts"))
    }
}
fn trace_from_labels(labels: Vec<Label>, id: usize, timestamp: &mut i64) -> Trace {
    let mut trace = Trace::with_case_id(id.to_string());
    for label in labels {
        let mut e = Event::new();
        e.insert("concept:name", label.as_str());
        e.insert(
            "time:timestamp",
            DateTime::<Utc>::from_timestamp(*timestamp, 0)
                .expect("bounded synthetic timestamp")
                .fixed_offset(),
        );
        *timestamp += 1;
        trace.events.push(e);
    }
    trace
}
/// Classic DFG enumeration parameters.
#[derive(Debug, Clone)]
pub struct DfgOptions {
    /// Maximum number of distinct variants (default 3000).
    pub max_variants: usize,
    /// Stop after accumulated probability exceeds this value (default 1).
    pub min_weighted_probability: f64,
    /// Maximum occurrences of each activity in a variant (default 2).
    pub max_occurrences: usize,
    /// Stop when starts, ends and edges have all been covered.
    pub stop_when_complete: bool,
    /// Accept only variants that cover a new start, end or edge.
    pub only_new_edges: bool,
    /// Minimum ceil(probability * max_variants), following pm4py's early stop.
    pub min_variant_occurrences: usize,
    /// Resource bound for partial paths.
    pub max_steps: usize,
}
impl Default for DfgOptions {
    fn default() -> Self {
        Self {
            max_variants: 3000,
            min_weighted_probability: 1.,
            max_occurrences: 2,
            stop_when_complete: false,
            only_new_edges: false,
            min_variant_occurrences: 1,
            max_steps: 1_000_000,
        }
    }
}
/// Enumerates likely DFG variants. Each returned trace carries its probability;
/// probabilities determine ordering, not how many copies are emitted.
pub fn play_out_dfg(dfg: &Dfg, o: &DfgOptions) -> Result<EventLog, SimulationError> {
    use std::collections::BTreeSet;
    if !o.min_weighted_probability.is_finite()
        || o.min_weighted_probability < 0.
        || o.max_occurrences == 0
    {
        return Err(SimulationError::Options(
            "invalid DFG probability or occurrence bound",
        ));
    }
    let start_total = dfg
        .start_activities
        .values()
        .map(|&n| n as f64)
        .sum::<f64>();
    if start_total == 0. {
        return Err(SimulationError::Dfg("no positive start weights"));
    }
    let mut pending: Vec<(f64, Vec<Label>)> = dfg
        .start_activities
        .iter()
        .filter(|(_, n)| **n > 0)
        .map(|(a, n)| (-(*n as f64 / start_total).ln(), vec![a.clone()]))
        .collect();
    let mut variants = Vec::new();
    let mut probability = 0.;
    let mut max_occ = 0;
    let mut starts = BTreeSet::new();
    let mut ends = BTreeSet::new();
    let mut edges = BTreeSet::new();
    let mut steps = 0;
    'outer: while !pending.is_empty() {
        if steps >= o.max_steps {
            return Err(SimulationError::Limit("DFG partial paths"));
        }
        steps += 1;
        pending.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
        let (cost, path) = pending.pop().expect("nonempty");
        let last = path.last().expect("start path");
        let outgoing = dfg.outgoing(last);
        let end = dfg.end_activities.get(last).copied().unwrap_or(0);
        let total = outgoing.values().map(|&n| n as f64).sum::<f64>() + end as f64;
        if total == 0. {
            return Err(SimulationError::Dfg(
                "reachable activity has no outgoing/end weight",
            ));
        }
        for (next, n) in outgoing {
            if n > 0 && path.iter().filter(|a| *a == next).count() < o.max_occurrences {
                let mut ext = path.clone();
                ext.push(next.clone());
                pending.push((cost - (n as f64 / total).ln(), ext));
            }
        }
        if end == 0 {
            continue;
        }
        if variants.len() >= o.max_variants || probability > o.min_weighted_probability {
            break;
        }
        let p = (-cost + (end as f64 / total).ln()).exp();
        probability += p;
        let new = !starts.contains(&path[0])
            || !ends.contains(last)
            || path
                .windows(2)
                .any(|w| !edges.contains(&(w[0].clone(), w[1].clone())));
        if o.only_new_edges && !new {
            continue;
        }
        starts.insert(path[0].clone());
        ends.insert(last.clone());
        edges.extend(path.windows(2).map(|w| (w[0].clone(), w[1].clone())));
        let occ = (p * o.max_variants as f64).ceil() as usize;
        max_occ = max_occ.max(occ);
        if occ < o.min_variant_occurrences && o.min_variant_occurrences <= max_occ {
            break;
        }
        variants.push((p, path));
        if o.stop_when_complete
            && dfg.start_activities.keys().all(|a| starts.contains(a))
            && dfg.end_activities.keys().all(|a| ends.contains(a))
            && dfg.graph.keys().all(|e| edges.contains(e))
        {
            break 'outer;
        }
    }
    variants.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    let mut log = EventLog::default();
    let mut timestamp = 10_000_000;
    for (p, path) in variants {
        let mut trace = trace_from_labels(path, log.traces.len(), &mut timestamp);
        trace.attributes.insert("probability", p);
        log.traces.push(trace);
    }
    Ok(log)
}
