//! Petri net to a graph, ported from pm4py's
//! `pm4py.convert_petri_net_to_networkx`.

use std::collections::BTreeMap;

use petgraph::graph::{DiGraph, NodeIndex};

use crate::Label;
use crate::petri::{AcceptingPetriNet, ArcEnds, ArcKind};

/// A Petri net as a directed graph. Build it with
/// [`AcceptingPetriNet::to_graph`].
pub type PetriGraph = DiGraph<PetriGraphNode, PetriGraphArc>;

/// A node of a [`PetriGraph`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PetriGraphNode {
    /// A place (pm4py type `place`).
    Place {
        /// The place name.
        name: String,
        /// The place holds tokens in the initial marking.
        in_initial_marking: bool,
        /// The place holds tokens in the final marking.
        in_final_marking: bool,
    },
    /// A transition (pm4py type `transition`).
    Transition {
        /// The transition name.
        name: String,
        /// The label, or `None` for a silent transition.
        label: Option<Label>,
    },
}

/// An edge of a [`PetriGraph`]: one arc of the net.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetriGraphArc {
    /// The arc weight.
    pub weight: u32,
    /// The arc kind. pm4py keeps it in the arc properties as `arctype`.
    pub kind: ArcKind,
}

impl AcceptingPetriNet {
    /// The net as a directed graph (pm4py's
    /// `pm4py.convert_petri_net_to_networkx`): places first, then
    /// transitions, each in id order, and one edge per pair of nodes joined
    /// by an arc.
    ///
    /// pm4py keys its nodes by name, so a place and a transition with the
    /// same name are one node there; here they stay apart. When two arcs
    /// join the same nodes, pm4py and this method both keep one edge, with
    /// the data of the arc added last; pm4py adds arcs in set order.
    pub fn to_graph(&self) -> PetriGraph {
        let net = &self.net;
        let mut graph = PetriGraph::new();
        let places: BTreeMap<_, NodeIndex> = net
            .places()
            .map(|(id, p)| {
                let node = graph.add_node(PetriGraphNode::Place {
                    name: p.name.clone(),
                    in_initial_marking: self.initial_marking.get(id) > 0,
                    in_final_marking: self.final_marking.get(id) > 0,
                });
                (id, node)
            })
            .collect();
        let transitions: BTreeMap<_, NodeIndex> = net
            .transitions()
            .map(|(id, t)| {
                let node = graph.add_node(PetriGraphNode::Transition {
                    name: t.name.clone(),
                    label: t.label.clone(),
                });
                (id, node)
            })
            .collect();
        for (_, arc) in net.arcs() {
            let (source, target) = match arc.ends {
                ArcEnds::PlaceToTransition(p, t) => (places[&p], transitions[&t]),
                ArcEnds::TransitionToPlace(t, p) => (transitions[&t], places[&p]),
            };
            let data = PetriGraphArc {
                weight: arc.weight,
                kind: arc.kind,
            };
            graph.update_edge(source, target, data);
        }
        graph
    }
}
