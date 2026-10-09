//! Petri net to BPMN, ported from pm4py's
//! `objects/conversion/wf_net/variants/to_bpmn.py`.

use std::collections::BTreeMap;

use crate::bpmn::{Bpmn, GatewayDirection, GatewayKind, NodeId, NodeKind};
use crate::petri::{AcceptingPetriNet, ArcEnds, PlaceId, TransitionId};

fn xor() -> NodeKind {
    NodeKind::gateway(GatewayKind::Exclusive, GatewayDirection::Unspecified)
}

fn and(direction: GatewayDirection) -> NodeKind {
    NodeKind::gateway(GatewayKind::Parallel, direction)
}

impl AcceptingPetriNet {
    /// Converts the net to a BPMN diagram (pm4py's `wf_net.converter.apply`
    /// with the `TO_BPMN` variant, used by `pm4py.convert_to_bpmn`). The
    /// result reads well for sound workflow nets.
    ///
    /// Each place becomes an exclusive gateway. A silent transition becomes
    /// a converging parallel gateway if it has several input arcs, else a
    /// diverging one if it has several output arcs, else an exclusive
    /// gateway. A visible transition becomes a task between an entry and
    /// an exit gateway, chosen the same way. Each arc becomes a sequence
    /// flow. A start event `start` leads to the places of the initial
    /// marking, and the places of the final marking lead to an end event
    /// `end`. Finally [`Bpmn::reduce_xor_gateways`] removes exclusive
    /// gateways with one incoming and one outgoing flow.
    ///
    /// As in pm4py, arc weights are ignored, and inhibitor and reset arcs
    /// become ordinary flows. Node ids are `id_<n>`, where pm4py uses
    /// random UUIDs.
    pub fn to_bpmn(&self) -> Bpmn {
        let net = &self.net;
        let mut bpmn = Bpmn::default();
        let mut place_node: BTreeMap<PlaceId, NodeId> = BTreeMap::new();
        for (p, _) in net.places() {
            place_node.insert(p, bpmn.add_node(xor(), ""));
        }
        let mut entering: BTreeMap<TransitionId, NodeId> = BTreeMap::new();
        let mut exiting: BTreeMap<TransitionId, NodeId> = BTreeMap::new();
        for (t, tr) in net.transitions() {
            let (ins, outs) = (tr.in_arcs().len(), tr.out_arcs().len());
            match &tr.label {
                None => {
                    let kind = if ins > 1 {
                        and(GatewayDirection::Converging)
                    } else if outs > 1 {
                        and(GatewayDirection::Diverging)
                    } else {
                        xor()
                    };
                    let g = bpmn.add_node(kind, "");
                    entering.insert(t, g);
                    exiting.insert(t, g);
                }
                Some(label) => {
                    let entry = if ins > 1 {
                        and(GatewayDirection::Converging)
                    } else {
                        xor()
                    };
                    let exit = if outs > 1 {
                        and(GatewayDirection::Diverging)
                    } else {
                        xor()
                    };
                    let entry = bpmn.add_node(entry, "");
                    let exit = bpmn.add_node(exit, "");
                    let task = bpmn.add_node(NodeKind::task(), label.as_str());
                    bpmn.add_flow(entry, task).expect("live nodes");
                    bpmn.add_flow(task, exit).expect("live nodes");
                    entering.insert(t, entry);
                    exiting.insert(t, exit);
                }
            }
        }
        for (_, arc) in net.arcs() {
            let (from, to) = match arc.ends {
                ArcEnds::PlaceToTransition(p, t) => (place_node[&p], entering[&t]),
                ArcEnds::TransitionToPlace(t, p) => (exiting[&t], place_node[&p]),
            };
            bpmn.add_flow(from, to).expect("live nodes");
        }
        let start = bpmn.add_node(NodeKind::start_event(), "start");
        let end = bpmn.add_node(NodeKind::end_event(), "end");
        for (p, _) in self.initial_marking.iter() {
            bpmn.add_flow(start, place_node[&p]).expect("live nodes");
        }
        for (p, _) in self.final_marking.iter() {
            bpmn.add_flow(place_node[&p], end).expect("live nodes");
        }
        bpmn.reduce_xor_gateways();
        bpmn
    }
}
