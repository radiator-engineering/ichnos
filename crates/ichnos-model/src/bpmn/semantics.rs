//! Token semantics of BPMN diagrams, ported from pm4py's
//! `objects/bpmn/semantics.py`.

use std::collections::{BTreeMap, BTreeSet};

use super::{Bpmn, CatchTrigger, EndTrigger, GatewayDirection, GatewayKind, NodeId, NodeKind};

/// Tokens per node (pm4py's BPMN `Marking`). Nodes without tokens have no
/// entry.
pub type BpmnMarking = BTreeMap<NodeId, u32>;

/// [`Bpmn::fire`] was called on a node that the marking does not enable.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("node {0:?} is not enabled")]
pub struct NodeNotEnabled(pub NodeId);

fn tokens(m: &BpmnMarking, n: NodeId) -> u32 {
    m.get(&n).copied().unwrap_or(0)
}

fn add_token(m: &mut BpmnMarking, n: NodeId) {
    *m.entry(n).or_insert(0) += 1;
}

/// All non-empty subsets of `items`, smallest first.
fn non_empty_subsets<T: Copy>(items: &[T]) -> Vec<Vec<T>> {
    let mut out = Vec::new();
    for size in 1..=items.len() {
        let mut pick: Vec<usize> = (0..size).collect();
        loop {
            out.push(pick.iter().map(|&i| items[i]).collect());
            // Next combination in lexicographic order.
            let Some(i) = (0..size).rev().find(|&i| pick[i] != i + items.len() - size) else {
                break;
            };
            pick[i] += 1;
            for j in i + 1..size {
                pick[j] = pick[j - 1] + 1;
            }
        }
    }
    out
}

impl Bpmn {
    /// One token on each start event of the top-level process (pm4py's
    /// `get_initial_marking`).
    pub fn initial_marking(&self) -> BpmnMarking {
        self.global_start_events()
            .into_iter()
            .map(|n| (n, 1))
            .collect()
    }

    /// Returns `true` if `m` enables `node` (pm4py's `is_enabled`). A
    /// converging parallel or inclusive gateway needs a token per incoming
    /// flow; any other node needs one token.
    pub fn is_enabled(&self, node: NodeId, m: &BpmnMarking) -> bool {
        if !self.contains_node(node) {
            return false;
        }
        let n = self.node(node);
        match n.kind {
            NodeKind::Gateway {
                kind: GatewayKind::Parallel | GatewayKind::Inclusive,
                direction: GatewayDirection::Converging,
            } => tokens(m, node) as usize >= n.in_flows.len(),
            _ => tokens(m, node) >= 1,
        }
    }

    /// The nodes that `m` enables (pm4py's `enabled_nodes`). Empty once an
    /// end event of the top-level process holds a token.
    pub fn enabled_nodes(&self, m: &BpmnMarking) -> BTreeSet<NodeId> {
        let mut enabled = BTreeSet::new();
        for (id, n) in self.nodes() {
            if self.is_enabled(id, m) {
                if n.kind.is_end_event() && n.process == self.process_id {
                    return BTreeSet::new();
                }
                enabled.insert(id);
            }
        }
        enabled
    }

    /// Fires an enabled node (pm4py's `execute`). See [`Bpmn::weak_fire`]
    /// for the result.
    pub fn fire(&self, node: NodeId, m: &BpmnMarking) -> Result<Vec<BpmnMarking>, NodeNotEnabled> {
        if !self.is_enabled(node, m) {
            return Err(NodeNotEnabled(node));
        }
        Ok(self.weak_fire(node, m))
    }

    /// Fires a node whether or not `m` enables it (pm4py's `weak_execute`)
    /// and returns every possible next marking.
    ///
    /// Firing removes all tokens from the node. Events, activities and
    /// converging gateways pass one token along their first outgoing flow.
    /// A diverging parallel gateway passes one along each outgoing flow; a
    /// diverging exclusive gateway gives one marking per outgoing flow; a
    /// diverging inclusive gateway gives one marking per non-empty set of
    /// outgoing flows. Other gateways give no marking.
    ///
    /// A token that reaches a subprocess goes to its start events and its
    /// message boundary events. A normal or terminate end event inside a
    /// subprocess clears the subprocess and passes a token to the
    /// subprocess's successor. A terminate end event of the top-level
    /// process clears every other node. An end event inside a subprocess
    /// that shares its name with a boundary event of the subprocess moves
    /// the token to that boundary event.
    pub fn weak_fire(&self, node: NodeId, m: &BpmnMarking) -> Vec<BpmnMarking> {
        let n = self.node(node);
        let first_target = |of: NodeId| {
            self.node(of)
                .out_flows
                .first()
                .map(|&f| self.flow(f).target)
        };
        if matches!(
            n.kind,
            NodeKind::EndEvent(EndTrigger::Normal | EndTrigger::Terminate)
        ) && n.process != self.process_id
        {
            let mut out = m.clone();
            out.remove(&node);
            let sub = self.node_by_id(&n.process);
            for key in self.nodes_inside_process(&n.process, true) {
                out.remove(&key);
            }
            for key in self.boundary_events_of(&n.process) {
                if key != node {
                    out.remove(&key);
                }
            }
            if let Some(target) = sub.and_then(first_target) {
                self.token_flow(target, &mut out);
            }
            return vec![out];
        }
        let converging = n.kind.gateway_direction() == Some(GatewayDirection::Converging);
        if n.kind.is_event() || n.kind.is_activity() || converging {
            let mut out = m.clone();
            out.remove(&node);
            if let Some(target) = first_target(node) {
                self.token_flow(target, &mut out);
            }
            if let NodeKind::BoundaryEvent {
                trigger: CatchTrigger::Message,
                activity: Some(activity),
            } = &n.kind
            {
                for key in self.nodes_inside_process(activity, true) {
                    out.remove(&key);
                }
                for key in self.boundary_events_of(activity) {
                    if key != node {
                        out.remove(&key);
                    }
                }
            }
            return vec![out];
        }
        let NodeKind::Gateway {
            kind,
            direction: GatewayDirection::Diverging,
        } = n.kind
        else {
            return Vec::new();
        };
        let targets: Vec<NodeId> = n.out_flows.iter().map(|&f| self.flow(f).target).collect();
        let mut base = m.clone();
        base.remove(&node);
        let with = |ts: &[NodeId]| {
            let mut out = base.clone();
            for &t in ts {
                self.token_flow(t, &mut out);
            }
            out
        };
        match kind {
            GatewayKind::Parallel => vec![with(&targets)],
            GatewayKind::Exclusive => targets.iter().map(|&t| with(&[t])).collect(),
            GatewayKind::Inclusive => non_empty_subsets(&targets)
                .iter()
                .map(|ts| with(ts))
                .collect(),
            GatewayKind::EventBased => Vec::new(),
        }
    }

    /// Puts a token on `target` (pm4py's `execute_token_flow`).
    fn token_flow(&self, target: NodeId, m: &mut BpmnMarking) {
        let t = self.node(target);
        match &t.kind {
            NodeKind::SubProcess { .. } => {
                for s in self.start_events_of_subprocess(&t.id) {
                    add_token(m, s);
                }
                for b in self.external_boundary_events_of(&t.id) {
                    add_token(m, b);
                }
            }
            NodeKind::EndEvent(trigger) if t.process == self.process_id => {
                if *trigger == EndTrigger::Terminate {
                    m.retain(|&k, _| k == target);
                }
                add_token(m, target);
            }
            NodeKind::EndEvent(trigger) => {
                let Some(sub) = self.node_by_id(&t.process) else {
                    add_token(m, target);
                    return;
                };
                let sub_id = &self.node(sub).id;
                let boundary = self.boundary_events_of(sub_id);
                if let Some(&b) = boundary.iter().find(|&&b| self.node(b).name == t.name) {
                    add_token(m, b);
                    for key in self.nodes_inside_process(&t.process, true) {
                        m.remove(&key);
                    }
                    for &key in &boundary {
                        if key != b {
                            m.remove(&key);
                        }
                    }
                    return;
                }
                if *trigger == EndTrigger::Terminate {
                    let clear: BTreeSet<NodeId> = self
                        .nodes_inside_process(&t.process, true)
                        .into_iter()
                        .chain(boundary)
                        .filter(|&k| k != target)
                        .collect();
                    m.retain(|k, _| !clear.contains(k));
                    if let Some(&f) = self.node(sub).out_flows.first() {
                        self.token_flow(self.flow(f).target, m);
                        return;
                    }
                }
                add_token(m, target);
            }
            _ => add_token(m, target),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::non_empty_subsets;

    #[test]
    fn subsets_are_listed_smallest_first() {
        assert_eq!(
            non_empty_subsets(&[1, 2, 3]),
            vec![
                vec![1],
                vec![2],
                vec![3],
                vec![1, 2],
                vec![1, 3],
                vec![2, 3],
                vec![1, 2, 3]
            ]
        );
        assert!(non_empty_subsets::<u8>(&[]).is_empty());
    }
}
