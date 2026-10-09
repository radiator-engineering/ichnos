//! BPMN diagrams, ported from pm4py's `visualization/bpmn/variants/classic.py`.

use std::collections::{BTreeMap, BTreeSet};

use ichnos_model::Bpmn;
use ichnos_model::bpmn::{GatewayKind, Node, NodeId, NodeKind};

use crate::dot::{Dot, title_label};

/// Options for [`bpmn_dot`], with pm4py's defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BpmnDotOptions {
    /// The background colour. Default `white`.
    pub bgcolor: String,
    /// The Graphviz `rankdir`. Default `LR`.
    pub rankdir: String,
    /// A title above the graph. `None` or an empty title draws none.
    pub graph_title: Option<String>,
    /// The font size. Default 12.
    pub font_size: u32,
    /// Draw each participant's process in its own cluster. Default `true`.
    pub enable_swimlanes: bool,
    /// Label intermediate and boundary events with their names. Default
    /// `true`.
    pub include_name_in_events: bool,
    /// The cluster margin. Default 35.
    pub swimlanes_margin: u32,
    /// The Graphviz shape of start and end events. Default `circle`.
    pub endpoints_shape: String,
}

impl Default for BpmnDotOptions {
    fn default() -> Self {
        BpmnDotOptions {
            bgcolor: "white".to_owned(),
            rankdir: "LR".to_owned(),
            graph_title: None,
            font_size: 12,
            enable_swimlanes: true,
            include_name_in_events: true,
            swimlanes_margin: 35,
            endpoints_shape: "circle".to_owned(),
        }
    }
}

/// The DOT text of a BPMN diagram, as pm4py's `save_vis_bpmn` draws it with
/// the `classic` variant.
///
/// Tasks and text annotations are boxes. Start events are green and end
/// events orange. Other events are underlined names. Gateways are diamonds
/// marked `X` (exclusive), `+` (parallel), `O` (inclusive) or `E` (event
/// based). Subprocesses, participants and collaborations are not drawn,
/// nor are flows that touch them. When the diagram has participants and
/// swimlanes are on, only the nodes of a participant's process are drawn,
/// one cluster per participant.
pub fn bpmn_dot(bpmn: &Bpmn, options: &BpmnDotOptions) -> String {
    let font_size = options.font_size.to_string();
    let mut dot = Dot::new(
        true,
        false,
        "",
        vec![
            ("bgcolor", Some(options.bgcolor.clone())),
            ("rankdir", Some(options.rankdir.clone())),
        ],
    );
    if let Some(title) = options.graph_title.as_deref().filter(|t| !t.is_empty()) {
        dot.set(vec![
            ("label", Some(title_label(title, options.font_size))),
            ("labelloc", Some("top".to_owned())),
        ]);
    }
    let (nodes, edges) = sorted_nodes_edges(bpmn);
    let id = |n: NodeId| format!("n{}", n.index());

    let mut process_ids: Vec<&str> = Vec::new();
    let mut members: BTreeMap<&str, Vec<NodeId>> = BTreeMap::new();
    for &n in &nodes {
        let process = bpmn.node(n).process.as_str();
        if !process_ids.contains(&process) {
            process_ids.push(process);
        }
        members.entry(process).or_default().push(n);
    }
    let mut pool_name: BTreeMap<&str, &str> = BTreeMap::new();
    let mut pool_id: BTreeMap<&str, NodeId> = BTreeMap::new();
    for &n in &nodes {
        let node = bpmn.node(n);
        if let NodeKind::Participant {
            process_ref: Some(process),
        } = &node.kind
        {
            pool_name.insert(process, &node.name);
            pool_id.insert(process, n);
        }
    }
    let participants = nodes
        .iter()
        .filter(|&&n| matches!(bpmn.node(n).kind, NodeKind::Participant { .. }))
        .count();

    let mut added: BTreeSet<NodeId> = BTreeSet::new();
    if participants < 1 || !options.enable_swimlanes {
        for &n in &nodes {
            if add_node(&mut dot, &id(n), bpmn.node(n), &font_size, options) {
                added.insert(n);
            }
        }
    } else {
        let invis = || vec![("style", Some("invis".to_owned()))];
        dot.node("@@anchorStart", None, invis());
        dot.node("@@anchorEnd", None, invis());
        for process in process_ids {
            let (Some(name), Some(&pool)) = (pool_name.get(process), pool_id.get(process)) else {
                continue;
            };
            let lane: Vec<NodeId> = members[process]
                .iter()
                .copied()
                .filter(|&n| drawn(&bpmn.node(n).kind))
                .collect();
            // pm4py adds these edges to the graph while the cluster is
            // open, so they come before it.
            if let (Some(&first), Some(&last)) = (lane.first(), lane.last()) {
                dot.edge("@@anchorStart", &id(first), None, invis());
                dot.edge(&id(last), "@@anchorEnd", None, invis());
            }
            dot.subgraph(&format!("cluster{}", id(pool)), |c| {
                c.set(vec![("label", Some((*name).to_owned()))]);
                c.set(vec![("margin", Some(options.swimlanes_margin.to_string()))]);
                for &n in &lane {
                    add_node(c, &id(n), bpmn.node(n), &font_size, options);
                }
            });
            added.extend(lane);
        }
    }
    for (s, t) in edges {
        if added.contains(&s) && added.contains(&t) {
            dot.edge(&id(s), &id(t), None, vec![]);
        }
    }
    dot.set(vec![("overlap", Some("false".to_owned()))]);
    dot.finish()
}

/// Whether pm4py's `add_bpmn_node` draws a node of this type.
fn drawn(kind: &NodeKind) -> bool {
    !matches!(
        kind,
        NodeKind::SubProcess { .. } | NodeKind::Participant { .. } | NodeKind::Collaboration
    )
}

/// pm4py's `add_bpmn_node`. Returns `false` for node types it does not
/// draw.
fn add_node(dot: &mut Dot, id: &str, n: &Node, font_size: &str, options: &BpmnDotOptions) -> bool {
    let s = |v: &str| Some(v.to_owned());
    let fs = s(font_size);
    match &n.kind {
        NodeKind::Task(_) => dot.node(
            id,
            Some(&n.name),
            vec![("shape", s("box")), ("fontsize", fs)],
        ),
        NodeKind::StartEvent { .. } | NodeKind::EndEvent(_) => {
            let fill = if n.kind.is_start_event() {
                "green"
            } else {
                "orange"
            };
            dot.node(
                id,
                Some(""),
                vec![
                    ("shape", Some(options.endpoints_shape.clone())),
                    ("style", s("filled")),
                    ("fillcolor", s(fill)),
                    ("fontsize", fs),
                ],
            );
        }
        NodeKind::IntermediateCatchEvent(_)
        | NodeKind::IntermediateThrowEvent(_)
        | NodeKind::BoundaryEvent { .. } => {
            let label = if options.include_name_in_events {
                n.name.as_str()
            } else {
                ""
            };
            dot.node(
                id,
                Some(label),
                vec![("shape", s("underline")), ("fontsize", fs)],
            );
        }
        NodeKind::TextAnnotation { text } => {
            dot.node(
                id,
                text.as_deref(),
                vec![("shape", s("box")), ("fontsize", fs)],
            );
        }
        NodeKind::Gateway { kind, .. } => {
            let mark = match kind {
                GatewayKind::Parallel => "+",
                GatewayKind::Exclusive => "X",
                GatewayKind::EventBased => "E",
                GatewayKind::Inclusive => "O",
            };
            dot.node(
                id,
                Some(mark),
                vec![("shape", s("diamond")), ("fontsize", fs)],
            );
        }
        NodeKind::SubProcess { .. } | NodeKind::Participant { .. } | NodeKind::Collaboration => {
            return false;
        }
    }
    true
}

/// pm4py's `get_sorted_nodes_edges`: nodes and flows in breadth-first order
/// from the start events.
///
/// The search does not enter end events. Nodes it does not reach, end
/// events among them, come last. Ties keep the order of pm4py's networkx
/// graph: nodes in id order, and flows by source node, then by target in
/// the order of the first flow between the two.
fn sorted_nodes_edges(bpmn: &Bpmn) -> (Vec<NodeId>, Vec<(NodeId, NodeId)>) {
    let mut targets: BTreeMap<NodeId, Vec<(NodeId, usize)>> = BTreeMap::new();
    for (_, f) in bpmn.flows() {
        let out = targets.entry(f.source()).or_default();
        match out.iter_mut().find(|(t, _)| *t == f.target()) {
            Some((_, n)) => *n += 1,
            None => out.push((f.target(), 1)),
        }
    }
    let edges: Vec<(NodeId, NodeId)> = bpmn
        .node_ids()
        .flat_map(|s| {
            targets
                .get(&s)
                .into_iter()
                .flatten()
                .flat_map(move |&(t, n)| std::iter::repeat_n((s, t), n))
        })
        .collect();
    let mut level: BTreeMap<NodeId, u32> = bpmn
        .nodes()
        .filter(|(_, n)| n.kind.is_start_event())
        .map(|(id, _)| (id, 0))
        .collect();
    let mut depth = 0;
    loop {
        depth += 1;
        let visit: Vec<NodeId> = edges
            .iter()
            .filter(|(s, t)| {
                level.contains_key(s)
                    && !level.contains_key(t)
                    && !bpmn.node(*t).kind.is_end_event()
            })
            .map(|&(_, t)| t)
            .collect();
        if visit.is_empty() {
            break;
        }
        for n in visit {
            level.insert(n, depth);
        }
    }
    depth += 1;
    for (n, _) in bpmn.nodes() {
        level.entry(n).or_insert(depth);
    }
    let mut nodes: Vec<NodeId> = bpmn.node_ids().collect();
    nodes.sort_by_key(|n| level[n]);
    let mut sorted = edges;
    sorted.sort_by_key(|(s, t)| (level[s], level[t]));
    (nodes, sorted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ichnos_model::bpmn::GatewayDirection;

    #[test]
    fn flows_sort_by_level_then_as_networkx_lists_them() {
        let mut bpmn = Bpmn::new("p");
        let start = bpmn.add_node(NodeKind::start_event(), "start");
        let a = bpmn.add_node(NodeKind::task(), "a");
        let split = bpmn.add_node(
            NodeKind::gateway(GatewayKind::Exclusive, GatewayDirection::Diverging),
            "split",
        );
        let b = bpmn.add_node(NodeKind::task(), "b");
        let end = bpmn.add_node(NodeKind::end_event(), "end");
        for (s, t) in [
            (start, split),
            (split, b),
            (a, end),
            (split, a),
            (b, end),
            (split, b),
        ] {
            bpmn.add_flow(s, t).expect("flow");
        }
        let (nodes, edges) = sorted_nodes_edges(&bpmn);
        // Levels: start 0, split 1, a and b 2; the search does not enter
        // the end event, which comes last.
        assert_eq!(nodes, [start, split, a, b, end]);
        // Ties keep networkx's order: by source node, and the two flows
        // from `split` to `b` together, before `split` to `a`.
        assert_eq!(
            edges,
            [
                (start, split),
                (split, b),
                (split, b),
                (split, a),
                (a, end),
                (b, end)
            ]
        );
    }
}
