//! Decomposed alignments (pm4py's `recompos_maximal` variant of
//! `algo/conformance/alignments/decomposed`).
//!
//! The net is cut into its maximal decomposition: components of places
//! joined by silent transitions, which share only visible transitions. Each
//! component aligns the trace projected on its activities. When two
//! components disagree on an activity they share (one moves on its event
//! synchronously, the other as a log move), they are merged and the merged
//! component aligns again. The component alignments are then chained into
//! one.
//!
//! pm4py's recomposition is a heuristic. The result can repeat moves and
//! need not be a run of the net; its cost is the cost of every move it
//! lists. ichnos follows pm4py step for step. pm4py orders components and
//! breaks search ties by object hashes, so its result can change between
//! runs; ichnos uses the net's order (see `docs/parity.md`).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::time::{Duration, Instant};

use ichnos_core::{EventKeys, EventLog};
use ichnos_model::petri::{ArcEnds, ArcKind};
use ichnos_model::{AcceptingPetriNet, Marking, PetriNet, PlaceId, TransitionId};

use super::costs::{
    ModelCosts, STD_LOG_MOVE_COST, STD_MODEL_MOVE_COST, STD_SILENT_MOVE_COST, STD_SYNC_MOVE_COST,
};
use super::petri_net::Heuristic;
use super::result::{LogAlignment, Move};
use super::search::search;
use super::sync_product::{ModelPart, MoveSet, SyncProduct};
use crate::error::{Error, Result};

/// Options of [`DecomposedAligner`].
///
/// The aligner always uses pm4py's standard costs: 10000 for a log or model
/// move on a visible transition, 1 for a silent move, 0 for a synchronous
/// move. pm4py's `model_cost_function` and `sync_cost_function` parameters
/// are not ported, because `recompos_maximal` raises `KeyError` when either
/// is set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecomposedOptions {
    /// Give up on a trace after this long and report no alignment for it
    /// (pm4py's `max_align_time_trace`). `None` waits for the answer.
    pub trace_time_limit: Option<Duration>,
    /// Give up on a trace when one merge involves more components than
    /// this (pm4py's `thresh_border_agreement`, default 100000000).
    pub max_border_disagreements: usize,
}

impl Default for DecomposedOptions {
    fn default() -> Self {
        Self {
            trace_time_limit: None,
            max_border_disagreements: 100_000_000,
        }
    }
}

/// The decomposed alignment of one trace.
#[derive(Debug, Clone, PartialEq)]
pub struct DecomposedAlignment {
    /// The recomposed moves, with event indices of the trace and
    /// transitions of the net. They can repeat, and need not form a run.
    pub moves: Vec<Move>,
    /// The cost of the moves under pm4py's standard costs.
    pub cost: u64,
    /// `1 - (cost / 10000) / (bwc / 10000)`, with integer division of the
    /// cost.
    pub fitness: f64,
    /// pm4py's `bwc`: `10000 * (events + c)`, where `c` is the cost of the
    /// cheapest model run divided by 10000 with integer division.
    pub best_worst_cost: u64,
}

impl DecomposedAlignment {
    /// Whether the trace fits: its fitness is exactly 1.
    pub fn is_fit(&self) -> bool {
        self.fitness == 1.0
    }
}

/// A part of the net: places and the transitions around them.
#[derive(Debug, Clone)]
struct Component {
    places: BTreeSet<PlaceId>,
    transitions: BTreeSet<TransitionId>,
    /// The visible labels, sorted, once per transition (pm4py's
    /// `lvis_labels`).
    labels: Vec<String>,
}

/// A component prepared for alignment.
#[derive(Debug)]
struct Prepared {
    model: ModelPart,
    /// The net transition of each component transition, by index.
    transitions: Vec<TransitionId>,
}

/// Aligns traces against the maximal decomposition of one accepting net.
#[derive(Debug, Clone)]
pub struct DecomposedAligner {
    net: PetriNet,
    initial_marking: Marking,
    final_marking: Marking,
    components: Vec<Component>,
    /// The cost of the cheapest model run, divided by 10000.
    best_worst: u64,
    options: DecomposedOptions,
}

/// One move of a component alignment. pm4py identifies a move by the
/// names of its product transition: the event by its index in the
/// component's projection of the trace, the model side by the transition.
#[derive(Debug, Clone, Copy)]
struct DMove {
    /// `(index in the projection, index in the trace)`.
    event: Option<(usize, usize)>,
    transition: Option<TransitionId>,
}

/// A component by its places and transitions.
type PartKey = (Vec<PlaceId>, Vec<TransitionId>);
type MoveKey<'a> = (Option<(usize, &'a str)>, Option<TransitionId>);
type LabelPair<'a> = (Option<&'a str>, Option<Option<&'a str>>);

impl DecomposedAligner {
    /// Decomposes `net`.
    pub fn new(
        net: &PetriNet,
        initial_marking: &Marking,
        final_marking: &Marking,
        options: DecomposedOptions,
    ) -> Result<Self> {
        if net.has_special_arcs() {
            return Err(Error::SpecialArcs);
        }
        for (p, _) in initial_marking.iter().chain(final_marking.iter()) {
            if !net.place_ids().any(|q| q == p) {
                return Err(Error::UnknownPlace(p));
            }
        }
        // pm4py's `get_best_worst_cost`: the cheapest run, or 0 without one.
        let model = ModelPart::new(
            net,
            initial_marking,
            final_marking,
            &ModelCosts::standard(net),
        )?;
        let empty = SyncProduct::new::<&str>(&model, &[], &[], MoveSet::All);
        let best_worst = search(&empty, Heuristic::StateEquation, None)?
            .map_or(0, |f| f.cost / STD_LOG_MOVE_COST);
        Ok(Self {
            components: decompose(net, initial_marking, final_marking),
            net: net.clone(),
            initial_marking: initial_marking.clone(),
            final_marking: final_marking.clone(),
            best_worst,
            options,
        })
    }

    /// The number of components of the maximal decomposition.
    pub fn component_count(&self) -> usize {
        self.components.len()
    }

    /// Aligns a trace given as its activity labels. Returns `None` if the
    /// time limit passed or a merge exceeded the border-disagreement limit.
    pub fn align<S: AsRef<str>>(&self, trace: &[S]) -> Result<Option<DecomposedAlignment>> {
        let trace: Vec<&str> = trace.iter().map(AsRef::as_ref).collect();
        let deadline = self.options.trace_time_limit.map(|d| Instant::now() + d);
        let mut prepared: HashMap<PartKey, Prepared> = HashMap::new();
        let mut cons = self.components.clone();
        let mut acache = activity_cache(&cons);
        let mut results: Vec<Option<Vec<DMove>>> = Vec::new();
        let mut alres: Vec<Option<BTreeMap<&str, Vec<u8>>>> = Vec::new();
        let mut max_alres = 0u8;
        let activities: BTreeSet<&str> = trace.iter().copied().collect();
        let timed_out = || deadline.is_some_and(|d| Instant::now() > d);

        let mut i = 0;
        while i < cons.len() {
            if timed_out() {
                return Ok(None);
            }
            let labels: BTreeSet<&str> = cons[i].labels.iter().map(String::as_str).collect();
            let relevant: BTreeSet<&str> = activities.intersection(&labels).copied().collect();
            let proj: Vec<usize> = (0..trace.len())
                .filter(|&e| relevant.contains(trace[e]))
                .collect();
            if proj.is_empty() {
                results.push(None);
                alres.push(None);
                i += 1;
                continue;
            }
            let moves = self.align_component(&cons[i], &mut prepared, &trace, &proj, deadline)?;
            let res = moves.as_ref().map(|m| agreement(&trace, m));
            if let Some(r) = &res {
                for v in r.values() {
                    max_alres = max_alres.max(v.iter().copied().max().unwrap_or(0));
                }
            }
            results.push(moves);
            alres.push(res);
            if timed_out() {
                return Ok(None);
            }

            if max_alres > 0 {
                let mut merge: BTreeSet<usize> = BTreeSet::new();
                for act in &relevant {
                    let owners = acache.get(*act).map_or(&[][..], Vec::as_slice);
                    for &ind in owners {
                        if ind >= i {
                            break;
                        }
                        let agree = match (&alres[ind], &alres[i]) {
                            (Some(a), Some(b)) if !a.is_empty() && !b.is_empty() => {
                                matches!((a.get(act), b.get(act)), (Some(x), Some(y)) if x == y)
                            }
                            _ => false,
                        };
                        if !agree {
                            merge.extend(owners.iter().copied());
                        }
                    }
                }
                if !merge.is_empty() {
                    if merge.len() > self.options.max_border_disagreements {
                        return Ok(None);
                    }
                    let mut merged = Component {
                        places: BTreeSet::new(),
                        transitions: BTreeSet::new(),
                        labels: Vec::new(),
                    };
                    for &z in merge.iter().rev() {
                        merged.places.extend(&cons[z].places);
                        merged.transitions.extend(&cons[z].transitions);
                    }
                    merged.labels = visible_labels(&self.net, &merged.transitions);
                    cons.push(merged);
                    for &z in merge.iter().rev() {
                        if z < i {
                            i -= 1;
                        }
                        if z <= i {
                            results.remove(z);
                            alres.remove(z);
                        }
                        cons.remove(z);
                    }
                    acache = activity_cache(&cons);
                    continue;
                }
            }
            i += 1;
        }
        if timed_out() {
            return Ok(None);
        }

        let order = recompose(&cons, &results, &trace, &self.net, &self.initial_marking);
        let mut cost = 0;
        let moves: Vec<Move> = order
            .iter()
            .map(|m| {
                cost += self.move_cost(m);
                match (m.event, m.transition) {
                    (Some((_, event)), Some(transition)) => Move::Sync { event, transition },
                    (Some((_, event)), None) => Move::Log { event },
                    (None, Some(transition)) => Move::Model { transition },
                    (None, None) => unreachable!("a move moves on the log or the model"),
                }
            })
            .collect();
        let best_worst_cost = (self.best_worst + trace.len() as u64) * STD_LOG_MOVE_COST;
        let den = self.best_worst + trace.len() as u64;
        let fitness = if den > 0 {
            1.0 - (cost / STD_LOG_MOVE_COST) as f64 / den as f64
        } else {
            0.0
        };
        Ok(Some(DecomposedAlignment {
            moves,
            cost,
            fitness,
            best_worst_cost,
        }))
    }

    /// Aligns every variant of `log` once.
    pub fn align_log(
        &self,
        log: &EventLog,
        keys: &EventKeys,
    ) -> Result<LogAlignment<DecomposedAlignment>> {
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

    /// pm4py's standard cost of a move.
    fn move_cost(&self, m: &DMove) -> u64 {
        match (m.event, m.transition) {
            (Some(_), Some(_)) => STD_SYNC_MOVE_COST,
            (Some(_), None) => STD_LOG_MOVE_COST,
            (None, Some(t)) if self.net.transition(t).is_silent() => STD_SILENT_MOVE_COST,
            _ => STD_MODEL_MOVE_COST,
        }
    }

    /// Aligns the projection `proj` of the trace on component `c` with
    /// state-equation A*.
    fn align_component(
        &self,
        c: &Component,
        prepared: &mut HashMap<PartKey, Prepared>,
        trace: &[&str],
        proj: &[usize],
        deadline: Option<Instant>,
    ) -> Result<Option<Vec<DMove>>> {
        let key: PartKey = (
            c.places.iter().copied().collect(),
            c.transitions.iter().copied().collect(),
        );
        if !prepared.contains_key(&key) {
            prepared.insert(key.clone(), self.prepare(c)?);
        }
        let p = &prepared[&key];
        let labels: Vec<&str> = proj.iter().map(|&e| trace[e]).collect();
        let costs = vec![STD_LOG_MOVE_COST; labels.len()];
        let sp = SyncProduct::new(&p.model, &labels, &costs, MoveSet::All);
        let Some(found) = search(&sp, Heuristic::StateEquation, deadline)? else {
            return Ok(None);
        };
        Ok(Some(
            found
                .path
                .iter()
                .map(|&m| match sp.moves[m as usize] {
                    Move::Sync { event, transition } => DMove {
                        event: Some((event, proj[event])),
                        transition: Some(p.transitions[transition.index()]),
                    },
                    Move::Log { event } => DMove {
                        event: Some((event, proj[event])),
                        transition: None,
                    },
                    Move::Model { transition } => DMove {
                        event: None,
                        transition: Some(p.transitions[transition.index()]),
                    },
                })
                .collect(),
        ))
    }

    /// The component as a net of its own, with the markings restricted to
    /// its places.
    fn prepare(&self, c: &Component) -> Result<Prepared> {
        let mut sub = PetriNet::new("");
        let mut place = BTreeMap::new();
        let mut im = Marking::new();
        let mut fm = Marking::new();
        for &p in &c.places {
            let q = sub.add_place(self.net.place(p).name.as_str());
            place.insert(p, q);
            if self.initial_marking.get(p) > 0 {
                im.set(q, self.initial_marking.get(p));
            }
            if self.final_marking.get(p) > 0 {
                fm.set(q, self.final_marking.get(p));
            }
        }
        let mut transition = BTreeMap::new();
        let mut transitions = Vec::new();
        for &t in &c.transitions {
            let tr = self.net.transition(t);
            let u = sub.add_transition(tr.name.as_str(), tr.label.as_ref().map(|l| l.as_str()));
            transition.insert(t, u);
            transitions.push(t);
        }
        for (_, arc) in self.net.arcs() {
            let (p, t) = match arc.ends {
                ArcEnds::PlaceToTransition(p, t) | ArcEnds::TransitionToPlace(t, p) => (p, t),
            };
            let (Some(&q), Some(&u)) = (place.get(&p), transition.get(&t)) else {
                continue;
            };
            let ends = match arc.ends {
                ArcEnds::PlaceToTransition(..) => ArcEnds::PlaceToTransition(q, u),
                ArcEnds::TransitionToPlace(..) => ArcEnds::TransitionToPlace(u, q),
            };
            sub.add_arc(ends, arc.weight, ArcKind::Normal)
                .expect("the component holds both ends");
        }
        Ok(Prepared {
            model: ModelPart::new(&sub, &im, &fm, &ModelCosts::standard(&sub))?,
            transitions,
        })
    }
}

/// pm4py's `get_alres`: per activity, in move order, 0 for a synchronous
/// move and 1 for a log move.
fn agreement<'a>(trace: &[&'a str], moves: &[DMove]) -> BTreeMap<&'a str, Vec<u8>> {
    let mut out: BTreeMap<&str, Vec<u8>> = BTreeMap::new();
    for m in moves {
        if let Some((_, e)) = m.event {
            out.entry(trace[e])
                .or_default()
                .push(u8::from(m.transition.is_none()));
        }
    }
    out
}

/// pm4py's `get_acache`: for each label, the components that have it, once
/// per transition with that label.
fn activity_cache(cons: &[Component]) -> HashMap<String, Vec<usize>> {
    let mut out: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, c) in cons.iter().enumerate() {
        for l in &c.labels {
            out.entry(l.clone()).or_default().push(i);
        }
    }
    out
}

fn visible_labels(net: &PetriNet, transitions: &BTreeSet<TransitionId>) -> Vec<String> {
    let mut labels: Vec<String> = transitions
        .iter()
        .filter_map(|&t| {
            net.transition(t)
                .label
                .as_ref()
                .map(|l| l.as_str().to_owned())
        })
        .collect();
    labels.sort();
    labels
}

/// pm4py's `decompose`: the maximal decomposition of
/// [`AcceptingPetriNet::maximal_decomposition_parts`].
fn decompose(net: &PetriNet, initial_marking: &Marking, final_marking: &Marking) -> Vec<Component> {
    AcceptingPetriNet::new(net.clone(), initial_marking.clone(), final_marking.clone())
        .maximal_decomposition_parts()
        .into_iter()
        .map(|part| {
            let transitions = part.transitions.into_iter().collect();
            Component {
                places: part.places.into_iter().collect(),
                labels: visible_labels(net, &transitions),
                transitions,
            }
        })
        .collect()
}

/// pm4py's `recompose_alignment`: chains the component alignments, from
/// the components with initial tokens along "last move of one equals first
/// move of the next" edges, then the rest.
fn recompose<'m>(
    cons: &[Component],
    results: &'m [Option<Vec<DMove>>],
    trace: &[&str],
    net: &PetriNet,
    initial_marking: &Marking,
) -> Vec<&'m DMove> {
    let pair = |m: &DMove| -> LabelPair<'_> {
        (
            m.event.map(|(_, e)| trace[e]),
            m.transition.map(|t| net.transition(t).label.as_deref()),
        )
    };
    let key = |m: &DMove| -> MoveKey<'_> { (m.event.map(|(p, e)| (p, trace[e])), m.transition) };
    let valid: Vec<usize> = (0..results.len())
        .filter(|&i| results[i].is_some())
        .collect();
    let moves = |i: usize| results[i].as_deref().expect("a valid node");
    // Successors in pm4py's edge order: by source, then by target.
    let mut successors: Vec<Vec<usize>> = vec![Vec::new(); results.len()];
    for &i in &valid {
        for &j in &valid {
            let (a, b) = (moves(i), moves(j));
            if i != j && !a.is_empty() && !b.is_empty() && pair(&a[a.len() - 1]) == pair(&b[0]) {
                successors[i].push(j);
            }
        }
    }
    let edge = |i: usize, j: usize| successors[i].contains(&j);

    let starts: Vec<usize> = (0..cons.len())
        .filter(|&i| cons[i].places.iter().any(|&p| initial_marking.get(p) > 0))
        .collect();
    // pm4py does not mark nodes in its first round, so a cycle reachable
    // from a start would loop forever. ichnos visits each node once then.
    let once = has_cycle(&starts, &successors);
    let mut visited = vec![false; results.len().max(cons.len())];
    let mut out: Vec<&DMove> = Vec::new();
    let mut count = 0;
    let mut queue: std::collections::VecDeque<usize> = starts.into_iter().collect();
    while let Some(curr) = queue.pop_front() {
        if once && visited[curr] {
            continue;
        }
        queue.extend(successors.get(curr).into_iter().flatten().copied());
        let skip = usize::from(count > 0);
        if let Some(Some(m)) = results.get(curr) {
            out.extend(m.iter().skip(skip));
        }
        visited[curr] = true;
        count += 1;
    }

    let mut rest: Vec<usize> = valid.iter().copied().filter(|&i| !visited[i]).collect();
    // pm4py's `order_nodes_second_round`: swap neighbours joined by an
    // edge against the order only. ichnos stops after n passes, where a
    // relation with cycles could make pm4py's loop run forever.
    for _ in 0..=rest.len() {
        let mut swapped = false;
        for k in 0..rest.len().saturating_sub(1) {
            let (a, b) = (rest[k], rest[k + 1]);
            if a != b && edge(b, a) && !edge(a, b) {
                rest.swap(k, k + 1);
                swapped = true;
            }
        }
        if !swapped {
            break;
        }
    }
    let mut added: Vec<MoveKey<'_>> = Vec::new();
    let mut queue: std::collections::VecDeque<usize> = rest.into_iter().collect();
    while let Some(curr) = queue.pop_front() {
        if !visited[curr] {
            queue.extend(successors[curr].iter().copied());
            let skip = usize::from(count > 0);
            for m in moves(curr).iter().skip(skip) {
                let k = key(m);
                if !added.contains(&k) {
                    out.push(m);
                    added.push(k);
                }
            }
            visited[curr] = true;
        }
        count += 1;
    }
    out
}

/// Whether a cycle is reachable from `starts`.
fn has_cycle(starts: &[usize], successors: &[Vec<usize>]) -> bool {
    // 0: unseen, 1: on the stack, 2: done.
    let mut state = vec![0u8; successors.len()];
    fn visit(n: usize, successors: &[Vec<usize>], state: &mut [u8]) -> bool {
        if n >= state.len() {
            return false;
        }
        match state[n] {
            1 => return true,
            2 => return false,
            _ => {}
        }
        state[n] = 1;
        for &s in &successors[n] {
            if visit(s, successors, state) {
                return true;
            }
        }
        state[n] = 2;
        false
    }
    starts.iter().any(|&s| visit(s, successors, &mut state))
}
