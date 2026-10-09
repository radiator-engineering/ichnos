//! Maximal decomposition of a Petri net, ported from pm4py's
//! `objects/petri_net/utils/decomposition.py`.

use std::collections::{BTreeMap, HashMap};

use crate::Label;
use crate::petri::{AcceptingPetriNet, Marking, PetriNet, PlaceId, TransitionId};

/// A node of pm4py's decomposition graph. pm4py keys nodes by name, so a
/// duplicated label is a node of its own, apart from the transitions that
/// carry it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Node {
    Place(PlaceId),
    Transition(TransitionId),
    Label(usize),
}

/// Union-find over node indices.
struct Components {
    parent: Vec<usize>,
}

impl Components {
    fn find(&mut self, mut x: usize) -> usize {
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]];
            x = self.parent[x];
        }
        x
    }

    fn union(&mut self, a: usize, b: usize) {
        let (a, b) = (self.find(a), self.find(b));
        if a != b {
            self.parent[b.max(a)] = a.min(b);
        }
    }
}

impl AcceptingPetriNet {
    /// Splits the net into its maximal decomposition (pm4py's
    /// `maximal_decomposition`).
    ///
    /// Places and silent transitions joined by arcs form one component. A
    /// visible transition with a unique label splits components, and each
    /// component gets its own copy of it. Of the transitions that share a
    /// label, one joins the components it produces into, as in pm4py.
    /// pm4py keeps the last of them in set order, which follows memory
    /// addresses and changes from run to run; here it is the one whose name
    /// sorts last (the lowest id on a tie). Use
    /// [`AcceptingPetriNet::maximal_decomposition_with`] to pick another.
    ///
    /// Each component keeps the names, labels and markings of its places
    /// and transitions. Arcs become normal arcs of weight 1, as in pm4py.
    /// Components come in the order of their first place or transition by
    /// id; pm4py's order depends on hashing.
    pub fn maximal_decomposition(&self) -> Vec<AcceptingPetriNet> {
        let net = &self.net;
        self.maximal_decomposition_with(|_, candidates| last_by_name(net, candidates))
    }

    /// [`AcceptingPetriNet::maximal_decomposition`], with `choose` picking
    /// the transition that joins components for each duplicated label. It
    /// gets the label and its transitions in id order.
    ///
    /// # Panics
    ///
    /// If `choose` returns a transition that is not one of the candidates.
    pub fn maximal_decomposition_with(
        &self,
        choose: impl FnMut(&Label, &[TransitionId]) -> TransitionId,
    ) -> Vec<AcceptingPetriNet> {
        let net = &self.net;
        let mut out = Vec::new();
        for part in self.maximal_decomposition_parts_with(choose) {
            let mut sub = PetriNet::new("");
            let mut im = Marking::new();
            let mut fm = Marking::new();
            let mut places = HashMap::new();
            let mut transitions = HashMap::new();
            for &p in &part.places {
                places.insert(p, sub.add_place(net.place(p).name.clone()));
            }
            for &t in &part.transitions {
                let tr = net.transition(t);
                transitions.insert(t, sub.add_transition(tr.name.clone(), tr.label.clone()));
            }
            for &p in &part.places {
                let q = places[&p];
                for t in net.place_preset(p) {
                    sub.add_output_arc(transitions[&t], q).expect("live ids");
                }
                for t in net.place_postset(p) {
                    sub.add_input_arc(q, transitions[&t]).expect("live ids");
                }
                let (a, b) = (self.initial_marking.get(p), self.final_marking.get(p));
                if a > 0 {
                    im.set(q, a);
                }
                if b > 0 {
                    fm.set(q, b);
                }
            }
            out.push(AcceptingPetriNet::new(sub, im, fm));
        }
        out
    }

    /// The components of [`AcceptingPetriNet::maximal_decomposition`] as
    /// ids of this net, in the same order.
    pub fn maximal_decomposition_parts(&self) -> Vec<DecompositionPart> {
        let net = &self.net;
        self.maximal_decomposition_parts_with(|_, candidates| last_by_name(net, candidates))
    }

    /// [`AcceptingPetriNet::maximal_decomposition_with`], as ids of this
    /// net.
    ///
    /// # Panics
    ///
    /// If `choose` returns a transition that is not one of the candidates.
    pub fn maximal_decomposition_parts_with(
        &self,
        mut choose: impl FnMut(&Label, &[TransitionId]) -> TransitionId,
    ) -> Vec<DecompositionPart> {
        let net = &self.net;
        let mut by_label: BTreeMap<&Label, Vec<TransitionId>> = BTreeMap::new();
        for (id, t) in net.transitions() {
            if let Some(l) = &t.label {
                by_label.entry(l).or_default().push(id);
            }
        }
        let duplicated: Vec<(&Label, TransitionId)> = by_label
            .iter()
            .filter(|(_, ts)| ts.len() > 1)
            .map(|(&l, ts)| {
                let t = choose(l, ts);
                assert!(ts.contains(&t), "{t:?} does not carry the label {l}");
                (l, t)
            })
            .collect();

        let mut nodes: Vec<Node> = net.place_ids().map(Node::Place).collect();
        nodes.extend(
            net.transitions()
                .filter(|(_, t)| t.label.is_none())
                .map(|(id, _)| Node::Transition(id)),
        );
        nodes.extend((0..duplicated.len()).map(Node::Label));
        nodes.extend(duplicated.iter().map(|&(_, t)| Node::Transition(t)));
        let index: HashMap<Node, usize> = nodes.iter().enumerate().map(|(i, &n)| (n, i)).collect();
        let mut comps = Components {
            parent: (0..nodes.len()).collect(),
        };
        // Arcs out of places, silent transitions and the chosen duplicated
        // transitions, into nodes of the graph.
        for &n in &nodes {
            let targets: Vec<Node> = match n {
                Node::Place(p) => net.place_postset(p).map(Node::Transition).collect(),
                Node::Transition(t) => net.postset(t).map(Node::Place).collect(),
                Node::Label(_) => Vec::new(),
            };
            for m in targets {
                if let Some(&j) = index.get(&m) {
                    comps.union(index[&n], j);
                }
            }
        }
        // The label nodes have no edges besides self-loops, and the chosen
        // duplicated transitions are named after themselves, not their
        // label: pm4py's graph keeps them apart too.
        let mut groups: BTreeMap<usize, Vec<Node>> = BTreeMap::new();
        for (i, &n) in nodes.iter().enumerate() {
            let root = comps.find(i);
            groups.entry(root).or_default().push(n);
        }

        let mut out = Vec::new();
        for members in groups.into_values() {
            let mut part = DecompositionPart::default();
            let add_transition = |part: &mut DecompositionPart, t: TransitionId| {
                if !part.transitions.contains(&t) {
                    part.transitions.push(t);
                }
            };
            for &n in &members {
                match n {
                    Node::Place(p) => part.places.push(p),
                    Node::Transition(t) => add_transition(&mut part, t),
                    Node::Label(_) => {}
                }
            }
            for &n in &members {
                let Node::Place(p) = n else { continue };
                for t in net.place_preset(p).chain(net.place_postset(p)) {
                    add_transition(&mut part, t);
                }
            }
            if !part.places.is_empty() || !part.transitions.is_empty() {
                out.push(part);
            }
        }
        out
    }
}

/// One component of a maximal decomposition, as ids of the decomposed net.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DecompositionPart {
    /// The places of the component, by id.
    pub places: Vec<PlaceId>,
    /// The transitions of the component: its silent and joining
    /// transitions, then the others around its places, without repeats.
    pub transitions: Vec<TransitionId>,
}

/// The transition whose name sorts last, the lowest id on a tie.
fn last_by_name(net: &PetriNet, candidates: &[TransitionId]) -> TransitionId {
    *candidates
        .iter()
        .rev()
        .max_by(|&&a, &&b| net.transition(a).name.cmp(&net.transition(b).name))
        .expect("candidates")
}
