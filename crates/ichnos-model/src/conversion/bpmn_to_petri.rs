//! BPMN to Petri net, ported from pm4py's
//! `objects/conversion/bpmn/variants/to_petri_net.py`.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::Label;
use crate::bpmn::{Bpmn, FlowId, FlowKind, GatewayKind, NodeId, NodeKind};
use crate::petri::{AcceptingPetriNet, Marking, PetriNet, PlaceId, TransitionId};

/// Options for [`Bpmn::to_petri_net`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BpmnToPetriOptions {
    /// Label every transition with its node's string id (pm4py's
    /// `USE_ID`). Off by default: tasks are labelled with their name and
    /// other nodes are silent.
    pub use_id: bool,
    /// Run [`PetriNet::apply_simple_reduction`] (pm4py's
    /// `ENABLE_REDUCTION`). On by default.
    pub reduce: bool,
}

impl Default for BpmnToPetriOptions {
    fn default() -> Self {
        BpmnToPetriOptions {
            use_id: false,
            reduce: true,
        }
    }
}

/// The result of [`Bpmn::to_petri_net`]: the net and where each BPMN
/// element went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BpmnPetriNet {
    /// The accepting Petri net, with one token in `source` initially and
    /// one in `sink` finally.
    pub net: AcceptingPetriNet,
    /// The place of each converted sequence flow that survived reduction.
    pub flow_places: BTreeMap<FlowId, PlaceId>,
    /// The transitions made for each converted node that survived
    /// reduction: the node's own transition first, then helper silent
    /// transitions.
    pub node_transitions: BTreeMap<NodeId, Vec<TransitionId>>,
}

/// Where a flow enters or leaves a converted node.
#[derive(Debug, Clone, Copy)]
enum Port {
    Place(PlaceId),
    Transition(TransitionId),
}

fn converts(kind: &NodeKind) -> bool {
    kind.is_task()
        || kind.is_start_event()
        || kind.is_end_event()
        || matches!(
            kind.gateway_kind(),
            Some(GatewayKind::Exclusive | GatewayKind::Parallel | GatewayKind::Inclusive)
        )
}

impl Bpmn {
    /// Converts the diagram to an accepting Petri net (pm4py's
    /// `bpmn.converter.apply`, used by `pm4py.convert_to_petri_net`).
    ///
    /// Each sequence flow becomes a place named after the flow's string id.
    /// Each task, start event, end event and exclusive, parallel or
    /// inclusive gateway becomes a transition named after the node's id,
    /// between places `ent_<id>` and `exi_<id>`. Other nodes and their
    /// flows are left out, as in pm4py. A parallel or inclusive gateway
    /// with several outgoing (incoming) flows gets a silent split (join)
    /// transition `<id>_split` (`<id>_join`). Silent transitions
    /// `sfl_<flow>` and `tfl_<flow>` link flow places to node places.
    /// Start events take their token from `source` through `<id>_start`;
    /// end events put theirs in `sink` through `<id>_end`. pm4py names the
    /// helper transitions with random UUIDs.
    ///
    /// When there are inclusive gateways, each place after a diverging
    /// inclusive gateway gets a silent transition `<place>_skip` to the
    /// nearest place before a converging one, as in pm4py. Ties between
    /// equally near places go to the smallest name; pm4py's choice depends
    /// on set order.
    ///
    /// Places left without arcs, other than `source` and `sink`, are
    /// removed. pm4py turns the reduction off when it returns the element
    /// maps; here the maps leave out what reduction removed.
    pub fn to_petri_net(&self, options: BpmnToPetriOptions) -> BpmnPetriNet {
        let mut net = PetriNet::new("");
        let source = net.add_place("source");
        let sink = net.add_place("sink");
        let arc_pt = |net: &mut PetriNet, p, t| {
            net.add_input_arc(p, t).expect("builder ids are live");
        };
        let arc_tp = |net: &mut PetriNet, t, p| {
            net.add_output_arc(t, p).expect("builder ids are live");
        };

        let sequence: Vec<FlowId> = self
            .flows()
            .filter(|(_, f)| f.kind == FlowKind::Sequence)
            .map(|(id, _)| id)
            .collect();
        let mut flow_places = BTreeMap::new();
        let mut out_count: BTreeMap<NodeId, usize> = BTreeMap::new();
        let mut in_count: BTreeMap<NodeId, usize> = BTreeMap::new();
        for &f in &sequence {
            let flow = self.flow(f);
            flow_places.insert(f, net.add_place(flow.id.as_str()));
            *out_count.entry(flow.source()).or_default() += 1;
            *in_count.entry(flow.target()).or_default() += 1;
        }
        let is_inclusive =
            |n: NodeId| self.node(n).kind.gateway_kind() == Some(GatewayKind::Inclusive);
        let mut or_exit: BTreeSet<String> = BTreeSet::new();
        let mut or_entry: BTreeSet<String> = BTreeSet::new();
        for &f in &sequence {
            let flow = self.flow(f);
            if is_inclusive(flow.source()) && out_count[&flow.source()] > 1 {
                or_exit.insert(flow.id.clone());
            } else if is_inclusive(flow.target()) && in_count[&flow.target()] > 1 {
                or_entry.insert(flow.id.clone());
            }
        }
        let both: BTreeSet<String> = or_exit.intersection(&or_entry).cloned().collect();
        or_exit.retain(|p| !both.contains(p));
        or_entry.retain(|p| !both.contains(p));

        let mut entering: BTreeMap<NodeId, Port> = BTreeMap::new();
        let mut exiting: BTreeMap<NodeId, Port> = BTreeMap::new();
        let mut node_transitions: BTreeMap<NodeId, Vec<TransitionId>> = BTreeMap::new();
        for (id, node) in self.nodes() {
            if !converts(&node.kind) {
                continue;
            }
            let entry = net.add_place(format!("ent_{}", node.id));
            let exit = net.add_place(format!("exi_{}", node.id));
            let label = if options.use_id {
                Some(Label::from(node.id.as_str()))
            } else if node.kind.is_task() && !node.name.is_empty() {
                Some(Label::from(node.name.as_str()))
            } else {
                None
            };
            let t = net.add_transition(node.id.as_str(), label);
            let ts = node_transitions.entry(id).or_insert_with(|| vec![t]);
            arc_pt(&mut net, entry, t);
            arc_tp(&mut net, t, exit);
            let and_or = matches!(
                node.kind.gateway_kind(),
                Some(GatewayKind::Parallel | GatewayKind::Inclusive)
            );
            let mut ent_port = Port::Place(entry);
            let mut exi_port = Port::Place(exit);
            if and_or && out_count.get(&id).copied().unwrap_or(0) > 1 {
                let s = net.add_transition(format!("{}_split", node.id), None::<Label>);
                arc_pt(&mut net, exit, s);
                ts.push(s);
                exi_port = Port::Transition(s);
            }
            if and_or && in_count.get(&id).copied().unwrap_or(0) > 1 {
                let j = net.add_transition(format!("{}_join", node.id), None::<Label>);
                arc_tp(&mut net, j, entry);
                ts.push(j);
                ent_port = Port::Transition(j);
            }
            if node.kind.is_start_event() {
                let s = net.add_transition(format!("{}_start", node.id), None::<Label>);
                arc_pt(&mut net, source, s);
                arc_tp(&mut net, s, entry);
                ts.push(s);
            } else if node.kind.is_end_event() {
                let e = net.add_transition(format!("{}_end", node.id), None::<Label>);
                arc_pt(&mut net, exit, e);
                arc_tp(&mut net, e, sink);
                ts.push(e);
            }
            entering.insert(id, ent_port);
            exiting.insert(id, exi_port);
        }

        for &f in &sequence {
            let flow = self.flow(f);
            let (Some(&from), Some(&to)) =
                (exiting.get(&flow.source()), entering.get(&flow.target()))
            else {
                continue;
            };
            let from = match from {
                Port::Transition(t) => t,
                Port::Place(p) => {
                    let t = net.add_transition(format!("sfl_{}", flow.id), None::<Label>);
                    arc_pt(&mut net, p, t);
                    node_transitions
                        .get_mut(&flow.source())
                        .expect("converted node")
                        .push(t);
                    t
                }
            };
            let to = match to {
                Port::Transition(t) => t,
                Port::Place(p) => {
                    let t = net.add_transition(format!("tfl_{}", flow.id), None::<Label>);
                    arc_tp(&mut net, t, p);
                    node_transitions
                        .get_mut(&flow.target())
                        .expect("converted node")
                        .push(t);
                    t
                }
            };
            arc_tp(&mut net, from, flow_places[&f]);
            arc_pt(&mut net, flow_places[&f], to);
        }

        if !or_exit.is_empty() && !or_entry.is_empty() {
            add_inclusive_skips(&mut net, &or_exit, &or_entry);
        }
        if options.reduce {
            net.apply_simple_reduction();
        }
        let isolated: Vec<PlaceId> = net
            .places()
            .filter(|(p, place)| {
                *p != source
                    && *p != sink
                    && place.in_arcs().is_empty()
                    && place.out_arcs().is_empty()
            })
            .map(|(p, _)| p)
            .collect();
        for p in isolated {
            net.remove_place(p);
        }
        let map = net.compact();
        let im: Marking = [(source, 1)].into_iter().collect();
        let fm: Marking = [(sink, 1)].into_iter().collect();
        BpmnPetriNet {
            net: AcceptingPetriNet::new(net, map.marking(&im), map.marking(&fm)),
            flow_places: flow_places
                .into_iter()
                .filter_map(|(f, p)| map.place(p).map(|p| (f, p)))
                .collect(),
            node_transitions: node_transitions
                .into_iter()
                .filter_map(|(n, ts)| {
                    let ts: Vec<TransitionId> =
                        ts.into_iter().filter_map(|t| map.transition(t)).collect();
                    (!ts.is_empty()).then_some((n, ts))
                })
                .collect(),
        }
    }
}

/// For each place in `exits`, adds a silent transition to the nearest place
/// in `entries`, measured on the place graph where `p -> q` if a
/// transition consumes from `p` and produces into `q`.
fn add_inclusive_skips(net: &mut PetriNet, exits: &BTreeSet<String>, entries: &BTreeSet<String>) {
    let mut succ: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for (t, _) in net.transitions() {
        for p in net.preset(t) {
            for q in net.postset(t) {
                succ.entry(net.place(p).name.as_str())
                    .or_default()
                    .insert(net.place(q).name.as_str());
            }
        }
    }
    let mut skips: Vec<(String, String)> = Vec::new();
    for start in exits {
        let mut dist: BTreeMap<&str, usize> = BTreeMap::from([(start.as_str(), 0)]);
        let mut queue = VecDeque::from([start.as_str()]);
        while let Some(p) = queue.pop_front() {
            for &q in succ.get(p).into_iter().flatten() {
                if !dist.contains_key(q) {
                    dist.insert(q, dist[p] + 1);
                    queue.push_back(q);
                }
            }
        }
        if let Some((target, _)) = dist
            .iter()
            .filter(|(p, _)| entries.contains(**p))
            .min_by_key(|(p, d)| (**d, **p))
        {
            skips.push((start.clone(), (*target).to_owned()));
        }
    }
    for (from, to) in skips {
        let (Some(p), Some(q)) = (net.place_by_name(&from), net.place_by_name(&to)) else {
            continue;
        };
        let t = net.add_transition(format!("{from}_skip"), None::<Label>);
        net.add_input_arc(p, t).expect("live ids");
        net.add_output_arc(t, q).expect("live ids");
    }
}
