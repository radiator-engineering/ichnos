//! Heuristics net to Petri net, ported from pm4py's
//! `objects/conversion/heuristics_net/variants/to_petri_net.py`.

use std::collections::{BTreeMap, BTreeSet};

use crate::Label;
use crate::heuristics_net::{HeuristicsNet, Matrix};
use crate::petri::{AcceptingPetriNet, Marking, PetriNet, PlaceId, TransitionId};

/// Where an activity's token comes from or goes to: another activity, or
/// the source or sink place of one of the merged nets.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum End {
    Activity(Label),
    Terminal(usize),
}

/// Maximal cliques of the undirected graph whose edges are the pairs in
/// `measures` (pm4py's `find_bindings`, which uses networkx's
/// `find_cliques`). Bron–Kerbosch with pivoting.
fn maximal_cliques(measures: &Matrix<f64>) -> Vec<Vec<Label>> {
    let mut adj: BTreeMap<&Label, BTreeSet<&Label>> = BTreeMap::new();
    for (a, row) in measures {
        adj.entry(a).or_default();
        for b in row.keys() {
            if a != b {
                adj.entry(a).or_default().insert(b);
                adj.entry(b).or_default().insert(a);
            }
        }
    }
    fn expand<'a>(
        adj: &BTreeMap<&'a Label, BTreeSet<&'a Label>>,
        r: &mut Vec<&'a Label>,
        mut p: BTreeSet<&'a Label>,
        mut x: BTreeSet<&'a Label>,
        out: &mut Vec<Vec<Label>>,
    ) {
        if p.is_empty() && x.is_empty() {
            out.push(r.iter().map(|&l| l.clone()).collect());
            return;
        }
        let pivot = p
            .iter()
            .chain(&x)
            .max_by_key(|u| adj[*u].intersection(&p).count())
            .copied()
            .expect("p or x is non-empty");
        let candidates: Vec<&Label> = p.difference(&adj[pivot]).copied().collect();
        for v in candidates {
            r.push(v);
            let np = p.intersection(&adj[v]).copied().collect();
            let nx = x.intersection(&adj[v]).copied().collect();
            expand(adj, r, np, nx, out);
            r.pop();
            p.remove(v);
            x.insert(v);
        }
    }
    let mut out = Vec::new();
    let all: BTreeSet<&Label> = adj.keys().copied().collect();
    expand(&adj, &mut Vec::new(), all, BTreeSet::new(), &mut out);
    out
}

struct Builder {
    net: PetriNet,
    hidden: usize,
}

impl Builder {
    fn hidden(&mut self) -> TransitionId {
        self.hidden += 1;
        self.net
            .add_transition(format!("hid_{}", self.hidden), None::<Label>)
    }

    fn pt(&mut self, p: PlaceId, t: TransitionId) {
        self.net.add_input_arc(p, t).expect("builder ids are live");
    }

    fn tp(&mut self, t: TransitionId, p: PlaceId) {
        self.net.add_output_arc(t, p).expect("builder ids are live");
    }
}

/// Removes every silent transition whose preset equals another silent
/// transition's and whose postset is a strict subset of it (pm4py's
/// `remove_rendundant_invisible_transitions`).
fn remove_redundant_silent(net: &mut PetriNet) {
    let silent: Vec<(TransitionId, BTreeSet<PlaceId>, BTreeSet<PlaceId>)> = net
        .transitions()
        .filter(|(_, t)| t.is_silent())
        .map(|(id, _)| (id, net.preset(id).collect(), net.postset(id).collect()))
        .collect();
    let dominated: Vec<TransitionId> = silent
        .iter()
        .filter(|(j, pre_j, post_j)| {
            silent.iter().any(|(i, pre_i, post_i)| {
                i != j && pre_i == pre_j && post_j.len() < post_i.len() && post_j.is_subset(post_i)
            })
        })
        .map(|(j, _, _)| *j)
        .collect();
    for t in dominated {
        net.remove_transition(t);
    }
}

impl HeuristicsNet {
    /// Converts the heuristics net to an accepting Petri net (pm4py's
    /// `heuristics_net.converter.apply`, used by `pm4py.convert_to_petri_net`).
    ///
    /// Each node becomes a visible transition named after its activity.
    /// Predecessors whose AND measures form a clique join through one silent
    /// transition; the others join as alternatives. Successors split the same
    /// way. Each start (end) activity map gets a source (sink) place
    /// `source0`, `source1`, ... (`sink0`, ...) with one token in the initial
    /// (final) marking. Redundant silent transitions are removed and the net
    /// is reduced with [`PetriNet::apply_simple_reduction`].
    pub fn to_petri_net(&self) -> AcceptingPetriNet {
        let mut b = Builder {
            net: PetriNet::new(""),
            hidden: 0,
        };
        let sources: Vec<PlaceId> = (0..self.start_activities.len())
            .map(|i| b.net.add_place(format!("source{i}")))
            .collect();
        let sinks: Vec<PlaceId> = (0..self.end_activities.len())
            .map(|i| b.net.add_place(format!("sink{i}")))
            .collect();

        // Activities in node order, then the targets of their connections.
        let mut acts: Vec<&Label> = Vec::new();
        let mut seen: BTreeSet<&Label> = BTreeSet::new();
        for (a1, node) in &self.nodes {
            for a in std::iter::once(a1).chain(node.outputs.keys()) {
                if seen.insert(a) {
                    acts.push(a);
                }
            }
        }
        let mut act_trans: BTreeMap<Label, TransitionId> = BTreeMap::new();
        let mut entering: BTreeMap<Label, BTreeSet<End>> = BTreeMap::new();
        let mut exiting: BTreeMap<Label, BTreeSet<End>> = BTreeMap::new();
        for &act in &acts {
            let t = b.net.add_transition(act.as_str(), Some(act.clone()));
            act_trans.insert(act.clone(), t);
            entering.insert(
                act.clone(),
                (0..self.start_activities.len())
                    .filter(|&i| self.start_activities[i].contains_key(act))
                    .map(End::Terminal)
                    .collect(),
            );
            exiting.insert(
                act.clone(),
                (0..self.end_activities.len())
                    .filter(|&i| self.end_activities[i].contains_key(act))
                    .map(End::Terminal)
                    .collect(),
            );
        }
        for (a1, node) in &self.nodes {
            for a2 in node.outputs.keys() {
                entering
                    .get_mut(a2)
                    .expect("every target has an entry")
                    .insert(End::Activity(a1.clone()));
                exiting
                    .get_mut(a1)
                    .expect("every node has an entry")
                    .insert(End::Activity(a2.clone()));
            }
        }
        let empty = Matrix::new();

        // places_entering[act][pred]: the place where `pred` puts the token
        // that enables `act`.
        let mut places_entering: BTreeMap<Label, BTreeMap<Label, PlaceId>> = BTreeMap::new();
        for (act, ins) in &entering {
            let t_act = act_trans[act];
            let preds: Vec<&Label> = ins
                .iter()
                .filter_map(|e| match e {
                    End::Activity(a) => Some(a),
                    End::Terminal(_) => None,
                })
                .collect();
            let srcs: Vec<usize> = ins
                .iter()
                .filter_map(|e| match e {
                    End::Terminal(i) => Some(*i),
                    End::Activity(_) => None,
                })
                .collect();
            let pe = places_entering.entry(act.clone()).or_default();
            let master = (!preds.is_empty() || ins.len() > 1).then(|| {
                let p = b.net.add_place(format!("pre_{act}"));
                b.pt(p, t_act);
                p
            });
            let measures = self.nodes.get(act).map_or(&empty, |n| &n.and_measures_in);
            let pred_set: BTreeSet<&Label> = preds.iter().copied().collect();
            let mut and_members: BTreeSet<&Label> = BTreeSet::new();
            for clique in maximal_cliques(measures) {
                let clique: Vec<&Label> = clique
                    .iter()
                    .filter_map(|n| pred_set.get(n).copied())
                    .collect();
                if clique.len() < 2 {
                    continue;
                }
                and_members.extend(clique.iter().copied());
                let hid = b.hidden();
                b.tp(
                    hid,
                    master.expect("an activity with predecessors has a master place"),
                );
                for pred in clique {
                    let s = *pe
                        .entry(pred.clone())
                        .or_insert_with(|| b.net.add_place(format!("splace_in_{act}_{pred}")));
                    b.pt(s, hid);
                }
            }
            for &pred in &preds {
                if and_members.contains(pred) {
                    continue;
                }
                let master = master.expect("an activity with predecessors has a master place");
                if ins.len() == 1 {
                    pe.insert(pred.clone(), master);
                } else {
                    let s = *pe
                        .entry(pred.clone())
                        .or_insert_with(|| b.net.add_place(format!("splace_in_{act}_{pred}")));
                    let hid = b.hidden();
                    b.pt(s, hid);
                    b.tp(hid, master);
                }
            }
            for i in srcs {
                if ins.len() == 1 {
                    b.pt(sources[i], t_act);
                } else {
                    let hid = b.hidden();
                    b.pt(sources[i], hid);
                    b.tp(hid, master.expect("several entries need a master place"));
                }
            }
        }

        for (act, outs) in &exiting {
            let t_act = act_trans[act];
            let succs: Vec<&Label> = outs
                .iter()
                .filter_map(|e| match e {
                    End::Activity(a) => Some(a),
                    End::Terminal(_) => None,
                })
                .collect();
            let snks: Vec<usize> = outs
                .iter()
                .filter_map(|e| match e {
                    End::Terminal(i) => Some(*i),
                    End::Activity(_) => None,
                })
                .collect();
            let entry_of =
                |succ: &Label| places_entering.get(succ).and_then(|m| m.get(act)).copied();
            if outs.len() == 1 {
                if let Some(succ) = succs.first() {
                    if let Some(p) = entry_of(succ) {
                        b.tp(t_act, p);
                    }
                    continue;
                }
                if let Some(&i) = snks.first() {
                    b.tp(t_act, sinks[i]);
                    continue;
                }
            }
            let int_place = b.net.add_place(format!("intplace_{act}"));
            b.tp(t_act, int_place);
            let measures = self.nodes.get(act).map_or(&empty, |n| &n.and_measures_out);
            let succ_set: BTreeSet<&Label> = succs.iter().copied().collect();
            let mut and_members: BTreeSet<&Label> = BTreeSet::new();
            for clique in maximal_cliques(measures) {
                let clique: Vec<&Label> = clique
                    .iter()
                    .filter_map(|n| succ_set.get(n).copied())
                    .collect();
                if clique.len() < 2 {
                    continue;
                }
                and_members.extend(clique.iter().copied());
                let hid = b.hidden();
                b.pt(int_place, hid);
                for succ in clique {
                    if let Some(p) = entry_of(succ) {
                        b.tp(hid, p);
                    }
                }
            }
            for &succ in &succs {
                if and_members.contains(succ) {
                    continue;
                }
                if let Some(p) = entry_of(succ) {
                    let hid = b.hidden();
                    b.pt(int_place, hid);
                    b.tp(hid, p);
                }
            }
            for i in snks {
                let hid = b.hidden();
                b.pt(int_place, hid);
                b.tp(hid, sinks[i]);
            }
        }

        let mut net = b.net;
        remove_redundant_silent(&mut net);
        net.apply_simple_reduction();
        let map = net.compact();
        let im: Marking = sources.iter().map(|&p| (p, 1)).collect();
        let fm: Marking = sinks.iter().map(|&p| (p, 1)).collect();
        AcceptingPetriNet::new(net, map.marking(&im), map.marking(&fm))
    }
}
