//! Workflow net to POWL, ported from pm4py's
//! `objects/conversion/wf_net/variants/to_powl.py` (Kourani, Park and van der
//! Aalst, "Translating Workflow Nets into the Partially Ordered Workflow
//! Language", 2025).
//!
//! The conversion splits the net recursively. Each step looks for a choice,
//! a loop or a partial order over the transitions, cuts the net into one
//! subnet per part and converts each subnet. pm4py works on its net objects
//! and follows set order; here the net is a small graph of numbered nodes
//! and every step follows node order, so the result is deterministic.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::petri::{PetriNet, PlaceId};
use crate::powl::StrictPartialOrder;
use crate::{Label, Powl};

/// Why a net could not be converted to POWL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum WfNetToPowlError {
    /// The net has inhibitor or reset arcs. pm4py fails on such nets with a
    /// `TypeError`.
    #[error("the Petri net has inhibitor or reset arcs")]
    SpecialArcs,
    /// The net is not a workflow net (see [`PetriNet::is_workflow_net`]).
    #[error("the Petri net is not a workflow net")]
    NotWorkflowNet,
    /// A part of a partial order has several start (or end) places that do
    /// not connect to the same transitions of the part.
    #[error("a part of the partial order has no unique local start or end place")]
    NoUniqueLocalStartOrEnd,
    /// The order between the parts of a partial order has a cycle.
    #[error("the order between the parts of the net has a cycle")]
    CyclicOrder,
    /// No choice, loop or partial order splits a part of the net.
    #[error("no POWL structure splits a part of the net")]
    NoStructure,
}

type Result<T> = std::result::Result<T, WfNetToPowlError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Node {
    Place(u32),
    Transition(u32),
}

use Node::{Place as P, Transition as T};

/// A net under conversion. Places and transitions are numbers; a subnet
/// keeps the numbers of the nodes it copies.
#[derive(Debug, Clone, Default)]
struct Net {
    places: BTreeSet<u32>,
    transitions: BTreeSet<u32>,
    succ: BTreeMap<Node, BTreeSet<Node>>,
    pred: BTreeMap<Node, BTreeSet<Node>>,
}

impl Net {
    fn add_arc(&mut self, source: Node, target: Node) {
        self.succ.entry(source).or_default().insert(target);
        self.pred.entry(target).or_default().insert(source);
    }

    fn remove_arc(&mut self, source: Node, target: Node) {
        if let Some(s) = self.succ.get_mut(&source) {
            s.remove(&target);
        }
        if let Some(p) = self.pred.get_mut(&target) {
            p.remove(&source);
        }
    }

    fn post(&self, node: Node) -> BTreeSet<Node> {
        self.succ.get(&node).cloned().unwrap_or_default()
    }

    fn pre(&self, node: Node) -> BTreeSet<Node> {
        self.pred.get(&node).cloned().unwrap_or_default()
    }

    fn arc_count(&self) -> usize {
        self.succ.values().map(BTreeSet::len).sum()
    }

    fn arcs(&self) -> Vec<(Node, Node)> {
        self.succ
            .iter()
            .flat_map(|(&s, ts)| ts.iter().map(move |&t| (s, t)))
            .collect()
    }

    fn remove_node(&mut self, node: Node) {
        for t in self.post(node) {
            self.remove_arc(node, t);
        }
        for s in self.pre(node) {
            self.remove_arc(s, node);
        }
        match node {
            P(p) => self.places.remove(&p),
            T(t) => self.transitions.remove(&t),
        };
    }

    /// The transitions among `nodes`.
    fn transitions_of(nodes: &BTreeSet<Node>) -> BTreeSet<u32> {
        nodes
            .iter()
            .filter_map(|n| match n {
                T(t) => Some(*t),
                P(_) => None,
            })
            .collect()
    }

    /// pm4py's `locally_identical`: the two places have the same inputs and
    /// outputs among `transitions`.
    fn locally_identical(&self, p1: u32, p2: u32, transitions: &BTreeSet<u32>) -> bool {
        let within = |nodes: BTreeSet<Node>| -> BTreeSet<u32> {
            Self::transitions_of(&nodes)
                .intersection(transitions)
                .copied()
                .collect()
        };
        within(self.pre(P(p1))) == within(self.pre(P(p2)))
            && within(self.post(P(p1))) == within(self.post(P(p2)))
    }

    /// pm4py's `get_simplified_reachability_graph`: for each transition, the
    /// nodes reachable from it, itself included.
    fn reachability(&self) -> BTreeMap<u32, BTreeSet<Node>> {
        self.transitions
            .iter()
            .map(|&t| {
                let mut seen = BTreeSet::new();
                let mut queue = VecDeque::from([T(t)]);
                while let Some(node) = queue.pop_front() {
                    if seen.insert(node) {
                        queue.extend(self.post(node));
                    }
                }
                (t, seen)
            })
            .collect()
    }

    /// pm4py's `get_reachable_transitions_from_place_to_another`: the
    /// transitions reachable from `from` without passing through `to`.
    fn transitions_between(&self, from: u32, to: u32) -> BTreeSet<u32> {
        let mut seen = BTreeSet::new();
        let mut queue = VecDeque::from([P(from)]);
        while let Some(node) = queue.pop_front() {
            if seen.insert(node) && node != P(to) {
                queue.extend(self.post(node));
            }
        }
        Self::transitions_of(&seen)
    }
}

/// pm4py's `__combine_parts`: merges every part that meets `group` into one
/// part, placed last.
fn combine(group: &BTreeSet<u32>, partition: Vec<BTreeSet<u32>>) -> Vec<BTreeSet<u32>> {
    let mut combined = BTreeSet::new();
    let mut out = Vec::with_capacity(partition.len());
    for part in partition {
        if part.is_disjoint(group) {
            out.push(part);
        } else {
            combined.extend(part);
        }
    }
    if !combined.is_empty() {
        out.push(combined);
    }
    out
}

/// Hands out fresh node numbers and remembers transition labels.
struct Converter {
    labels: Vec<Option<Label>>,
    next_place: u32,
}

impl Converter {
    fn place(&mut self) -> u32 {
        let p = self.next_place;
        self.next_place += 1;
        p
    }

    fn silent(&mut self) -> u32 {
        self.labels.push(None);
        u32::try_from(self.labels.len() - 1).expect("at most u32::MAX transitions")
    }

    /// pm4py's `preprocess`: removes a place that duplicates another, and
    /// splits two places with the same inputs (or outputs) that share an
    /// output (or input) behind a new place and a silent transition. It
    /// starts over after each change.
    fn preprocess(&mut self, net: &mut Net) {
        'restart: loop {
            let places: Vec<u32> = net.places.iter().copied().collect();
            for (i, &p1) in places.iter().enumerate() {
                for &p2 in &places[i + 1..] {
                    let (pre1, pre2) = (net.pre(P(p1)), net.pre(P(p2)));
                    let (post1, post2) = (net.post(P(p1)), net.post(P(p2)));
                    if pre1 == pre2 && post1 == post2 {
                        net.remove_node(P(p2));
                        continue 'restart;
                    }
                    if pre1 == pre2 {
                        let common: Vec<Node> = post1.intersection(&post2).copied().collect();
                        if pre1.len() > 1 || !common.is_empty() {
                            let new = self.place();
                            net.places.insert(new);
                            for &t in &pre1 {
                                net.add_arc(t, P(new));
                                net.remove_arc(t, P(p1));
                                net.remove_arc(t, P(p2));
                            }
                            for &t in &common {
                                net.add_arc(P(new), t);
                                net.remove_arc(P(p1), t);
                                net.remove_arc(P(p2), t);
                            }
                            let silent = self.silent();
                            net.transitions.insert(silent);
                            net.add_arc(P(new), T(silent));
                            net.add_arc(T(silent), P(p1));
                            net.add_arc(T(silent), P(p2));
                            continue 'restart;
                        }
                    }
                    if post1 == post2 {
                        let common: Vec<Node> = pre1.intersection(&pre2).copied().collect();
                        if post1.len() > 1 || !common.is_empty() {
                            let new = self.place();
                            net.places.insert(new);
                            for &t in &post1 {
                                net.add_arc(P(new), t);
                                net.remove_arc(P(p1), t);
                                net.remove_arc(P(p2), t);
                            }
                            for &t in &common {
                                net.add_arc(t, P(new));
                                net.remove_arc(t, P(p1));
                                net.remove_arc(t, P(p2));
                            }
                            let silent = self.silent();
                            net.transitions.insert(silent);
                            net.add_arc(P(p1), T(silent));
                            net.add_arc(P(p2), T(silent));
                            net.add_arc(T(silent), P(new));
                            continue 'restart;
                        }
                    }
                }
            }
            return;
        }
    }

    /// pm4py's `__translate_petri_to_powl`.
    fn translate(&mut self, mut net: Net, start: u32, end: u32) -> Result<Powl> {
        if net.transitions.len() == 1 && net.places.len() == 2 && net.arc_count() == 2 {
            let t = *net.transitions.first().expect("one transition");
            // pm4py tests the label for truth, so an empty label is silent.
            return Ok(match &self.labels[t as usize] {
                Some(label) if !label.is_empty() => Powl::Activity(label.clone()),
                _ => Powl::Silent,
            });
        }
        let reach = net.reachability();

        // A choice: transitions that never reach each other.
        let mut branches: Vec<BTreeSet<u32>> = net
            .transitions
            .iter()
            .map(|&t| BTreeSet::from([t]))
            .collect();
        let transitions: Vec<u32> = net.transitions.iter().copied().collect();
        for (i, &t1) in transitions.iter().enumerate() {
            for &t2 in &transitions[i + 1..] {
                if reach[&t2].contains(&T(t1)) || reach[&t1].contains(&T(t2)) {
                    branches = combine(&BTreeSet::from([t1, t2]), branches);
                }
            }
        }
        if branches.len() > 1 {
            let children = branches
                .iter()
                .map(|branch| self.subnet(&net, branch, start, end))
                .collect::<Result<Vec<_>>>()?;
            return Ok(Powl::xor(children));
        }

        // A self-loop on one place: split it into a do and a redo part.
        if start == end {
            let copy = self.place();
            net.places.insert(copy);
            let redo = net.transitions.clone();
            for target in net.post(P(start)) {
                net.remove_arc(P(start), target);
                net.add_arc(P(copy), target);
            }
            let silent = self.silent();
            net.transitions.insert(silent);
            net.add_arc(P(start), T(silent));
            net.add_arc(T(silent), P(copy));
            return self.looped(&net, &BTreeSet::from([silent]), &redo, start, copy);
        }

        // A loop: the transitions from end back to start form the redo part.
        let redo = net.transitions_between(end, start);
        if !redo.is_empty() {
            let do_part = net.transitions_between(start, end);
            if do_part.is_empty() {
                return Err(WfNetToPowlError::NoStructure);
            }
            // pm4py: "This could happen if we have ->(..., Loop)".
            if do_part.is_disjoint(&redo) {
                if do_part.union(&redo).copied().collect::<BTreeSet<_>>() != net.transitions {
                    return Err(WfNetToPowlError::NoStructure);
                }
                return self.looped(&net, &do_part, &redo, start, end);
            }
        }

        // A partial order: merge transitions that a choice keeps together.
        let mut parts: Vec<BTreeSet<u32>> = net
            .transitions
            .iter()
            .map(|&t| BTreeSet::from([t]))
            .collect();
        for &p in &net.places {
            let outputs = Net::transitions_of(&net.post(P(p)));
            if outputs.len() > 1 || (p == end && !outputs.is_empty()) {
                let branches: Vec<BTreeSet<u32>> = outputs
                    .iter()
                    .map(|t| Net::transitions_of(&reach[t]))
                    .collect();
                let union: BTreeSet<u32> = branches.iter().flatten().copied().collect();
                let not_in_every: BTreeSet<u32> = if p == end {
                    union
                } else {
                    union
                        .into_iter()
                        .filter(|t| !branches.iter().all(|b| b.contains(t)))
                        .collect()
                };
                if not_in_every.len() > 1 {
                    parts = combine(&not_in_every, parts);
                }
            }
        }
        if parts.len() > 1 {
            return self.partial_order(&net, &parts, start, end);
        }
        Err(WfNetToPowlError::NoStructure)
    }

    /// pm4py's `__translate_loop`.
    fn looped(
        &mut self,
        net: &Net,
        do_part: &BTreeSet<u32>,
        redo: &BTreeSet<u32>,
        start: u32,
        end: u32,
    ) -> Result<Powl> {
        let do_powl = self.subnet(net, do_part, start, end)?;
        let redo_powl = self.subnet(net, redo, end, start)?;
        Ok(Powl::looped(do_powl, redo_powl))
    }

    /// pm4py's `__create_sub_powl_model`: converts the subnet of `branch`
    /// and the places next to it.
    fn subnet(&mut self, net: &Net, branch: &BTreeSet<u32>, start: u32, end: u32) -> Result<Powl> {
        let mut sub = Net {
            transitions: branch.clone(),
            ..Net::default()
        };
        for (source, target) in net.arcs() {
            let touches = |n: Node| matches!(n, T(t) if branch.contains(&t));
            if touches(source) || touches(target) {
                for n in [source, target] {
                    if let P(p) = n {
                        sub.places.insert(p);
                    }
                }
                sub.add_arc(source, target);
            }
        }
        // pm4py raises `KeyError` when a branch does not touch both places.
        if !sub.places.contains(&start) || !sub.places.contains(&end) {
            return Err(WfNetToPowlError::NoStructure);
        }
        self.translate(sub, start, end)
    }

    /// pm4py's `__translate_partial_order`.
    fn partial_order(
        &mut self,
        net: &Net,
        parts: &[BTreeSet<u32>],
        start: u32,
        end: u32,
    ) -> Result<Powl> {
        let part_of: BTreeMap<u32, usize> = parts
            .iter()
            .enumerate()
            .flat_map(|(i, part)| part.iter().map(move |&t| (t, i)))
            .collect();
        let mut starts = vec![BTreeSet::new(); parts.len()];
        let mut ends = vec![BTreeSet::new(); parts.len()];
        let mut edges = BTreeSet::new();
        for &p in &net.places {
            let sources = Net::transitions_of(&net.pre(P(p)));
            let targets = Net::transitions_of(&net.post(P(p)));
            if p == start {
                for t in &targets {
                    starts[part_of[t]].insert(p);
                }
            }
            if p == end {
                for t in &sources {
                    ends[part_of[t]].insert(p);
                }
            }
            for t1 in &sources {
                for t2 in &targets {
                    let (g1, g2) = (part_of[t1], part_of[t2]);
                    if g1 != g2 {
                        edges.insert((g1, g2));
                        ends[g1].insert(p);
                        starts[g2].insert(p);
                    }
                }
            }
        }
        let mut children = Vec::with_capacity(parts.len());
        for (i, part) in parts.iter().enumerate() {
            let (sub, s, e) = self.projection(net, part, &starts[i], &ends[i])?;
            children.push(self.translate(sub, s, e)?);
        }
        let mut po = StrictPartialOrder::new(children);
        for (g1, g2) in edges {
            po.add_edge(g1, g2);
        }
        po.order_mut().add_transitive_edges();
        if !po.order().is_irreflexive() {
            return Err(WfNetToPowlError::CyclicOrder);
        }
        Ok(Powl::PartialOrder(po))
    }

    /// pm4py's `apply_partial_order_projection`: the subnet of `part`, with
    /// its start places merged into one new place and its end places into
    /// another.
    fn projection(
        &mut self,
        net: &Net,
        part: &BTreeSet<u32>,
        starts: &BTreeSet<u32>,
        ends: &BTreeSet<u32>,
    ) -> Result<(Net, u32, u32)> {
        // pm4py raises `IndexError` when a part has no start or end place.
        let (Some(&old_start), Some(&old_end)) = (starts.first(), ends.first()) else {
            return Err(WfNetToPowlError::NoStructure);
        };
        let unique = |places: &BTreeSet<u32>, first: u32| {
            places
                .iter()
                .all(|&p| net.locally_identical(p, first, part))
        };
        if !unique(starts, old_start) {
            return Err(WfNetToPowlError::NoUniqueLocalStartOrEnd);
        }
        let mut sub = Net {
            transitions: part.clone(),
            ..Net::default()
        };
        let mut map: BTreeMap<u32, u32> = BTreeMap::new();
        let new_start = self.place();
        sub.places.insert(new_start);
        map.insert(old_start, new_start);
        let new_end = if starts == ends {
            new_start
        } else {
            if !unique(ends, old_end) {
                return Err(WfNetToPowlError::NoUniqueLocalStartOrEnd);
            }
            let new_end = self.place();
            sub.places.insert(new_end);
            map.insert(old_end, new_end);
            new_end
        };
        for (source, target) in net.arcs() {
            let touches = |n: Node| matches!(n, T(t) if part.contains(&t));
            if !touches(source) && !touches(target) {
                continue;
            }
            let mut ends_of_arc = [source, target];
            let mut skip = false;
            for n in &mut ends_of_arc {
                if let P(p) = *n {
                    if let Some(&mapped) = map.get(&p) {
                        *n = P(mapped);
                    } else if starts.contains(&p) || ends.contains(&p) {
                        skip = true;
                    } else {
                        sub.places.insert(p);
                    }
                }
            }
            if !skip {
                sub.add_arc(ends_of_arc[0], ends_of_arc[1]);
            }
        }
        Ok((sub, new_start, new_end))
    }
}

impl PetriNet {
    /// Converts a workflow net to a POWL model (pm4py's
    /// `pm4py.convert_to_powl` on a Petri net). Markings play no part, as in
    /// pm4py.
    ///
    /// The result follows node order, so it is deterministic; pm4py follows
    /// set order, and its choice and partial-order children can come in
    /// another order. pm4py also changes the net it converts; this method
    /// works on a copy. Arc weights are ignored, as in pm4py.
    ///
    /// Fails on a net with inhibitor or reset arcs, on a net that is not a
    /// workflow net, and where pm4py's algorithm finds no structure.
    pub fn to_powl(&self) -> std::result::Result<Powl, WfNetToPowlError> {
        if self.has_special_arcs() {
            return Err(WfNetToPowlError::SpecialArcs);
        }
        if !self.is_workflow_net() {
            return Err(WfNetToPowlError::NotWorkflowNet);
        }
        let index = |p: PlaceId| u32::try_from(p.index()).expect("at most u32::MAX places");
        let mut net = Net::default();
        let mut converter = Converter {
            labels: vec![None; self.transition_index_bound()],
            next_place: u32::try_from(self.place_index_bound()).expect("at most u32::MAX places"),
        };
        net.places.extend(self.place_ids().map(index));
        for (id, transition) in self.transitions() {
            let t = u32::try_from(id.index()).expect("at most u32::MAX transitions");
            converter.labels[t as usize] = transition.label.clone();
            net.transitions.insert(t);
            for p in self.preset(id) {
                net.add_arc(P(index(p)), T(t));
            }
            for p in self.postset(id) {
                net.add_arc(T(t), P(index(p)));
            }
        }
        let place = |inputs: bool| {
            self.places()
                .find(|(_, p)| {
                    if inputs {
                        p.in_arcs().is_empty()
                    } else {
                        p.out_arcs().is_empty()
                    }
                })
                .map(|(id, _)| index(id))
                .expect("a workflow net has a source and a sink place")
        };
        let (start, end) = (place(true), place(false));
        converter.preprocess(&mut net);
        converter.translate(net, start, end)
    }
}
