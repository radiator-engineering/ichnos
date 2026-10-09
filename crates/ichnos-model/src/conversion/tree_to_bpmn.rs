//! Process tree to BPMN, ported from pm4py's
//! `objects/conversion/process_tree/variants/to_bpmn.py`.

use crate::bpmn::{Bpmn, GatewayDirection, GatewayKind, NodeId, NodeKind};
use crate::process_tree::{Operator, ProcessTree};

/// [`ProcessTree::to_bpmn`] met an operator that pm4py does not convert.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("process tree to BPMN does not support the {0} operator")]
pub struct UnsupportedOperator(pub Operator);

struct Builder {
    bpmn: Bpmn,
    taus: Vec<NodeId>,
}

impl Builder {
    fn flow(&mut self, a: NodeId, b: NodeId) {
        self.bpmn.add_flow(a, b).expect("builder nodes are live");
    }

    fn tau(&mut self) -> NodeId {
        let name = format!("tau_{}", self.taus.len() + 1);
        let t = self.bpmn.add_node(NodeKind::task(), name);
        self.taus.push(t);
        t
    }

    fn gateways(&mut self, kind: GatewayKind) -> (NodeId, NodeId) {
        let split = self
            .bpmn
            .add_node(NodeKind::gateway(kind, GatewayDirection::Diverging), "");
        let join = self
            .bpmn
            .add_node(NodeKind::gateway(kind, GatewayDirection::Converging), "");
        (split, join)
    }

    /// Adds `tree` between `initial` and `fin`. Returns the first and last
    /// node of the fragment (pm4py's `recursively_add_tree`).
    fn add(
        &mut self,
        tree: &ProcessTree,
        initial: NodeId,
        fin: NodeId,
    ) -> Result<(NodeId, NodeId), UnsupportedOperator> {
        let children = tree.children();
        Ok(match tree {
            ProcessTree::Tau | ProcessTree::Activity(_) => {
                let task = match tree.label() {
                    Some(l) => self.bpmn.add_node(NodeKind::task(), l.as_str()),
                    None => self.tau(),
                };
                self.flow(initial, task);
                self.flow(task, fin);
                (task, task)
            }
            ProcessTree::Node(op @ (Operator::Xor | Operator::Parallel | Operator::Or), _) => {
                let kind = match op {
                    Operator::Xor => GatewayKind::Exclusive,
                    Operator::Parallel => GatewayKind::Parallel,
                    _ => GatewayKind::Inclusive,
                };
                let (split, join) = self.gateways(kind);
                for c in children {
                    self.add(c, split, join)?;
                }
                self.flow(initial, split);
                self.flow(join, fin);
                (split, join)
            }
            ProcessTree::Node(Operator::Sequence, _) => {
                // pm4py chains the children through tau tasks that it
                // removes afterwards; chaining them directly gives the same
                // flows. A one-child sequence also reaches `fin`, which
                // pm4py's chaining misses.
                let mut from = initial;
                let mut first = None;
                for (i, c) in children.iter().enumerate() {
                    let to = if i + 1 == children.len() {
                        fin
                    } else {
                        self.tau()
                    };
                    let (a, b) = self.add(c, from, to)?;
                    first.get_or_insert(a);
                    from = b;
                }
                match first {
                    Some(a) => (a, from),
                    None => {
                        self.flow(initial, fin);
                        (initial, fin)
                    }
                }
            }
            ProcessTree::Node(Operator::Loop, _) => {
                let (split, join) = self.gateways(GatewayKind::Exclusive);
                if let Some((body, redos)) = children.split_first() {
                    self.add(body, join, split)?;
                    for r in redos {
                        self.add(r, split, join)?;
                    }
                }
                self.flow(initial, join);
                self.flow(split, fin);
                (join, split)
            }
            ProcessTree::Node(op @ Operator::Interleaving, _) => {
                return Err(UnsupportedOperator(*op));
            }
        })
    }

    /// Replaces each tau task by a flow from its predecessor to its
    /// successor, or drops it if it lacks one (pm4py's
    /// `delete_tau_transitions`).
    fn delete_taus(&mut self) {
        for t in std::mem::take(&mut self.taus) {
            let node = self.bpmn.node(t);
            let link = match (node.in_flows(), node.out_flows()) {
                ([i], [o]) => Some((self.bpmn.flow(*i).source(), self.bpmn.flow(*o).target())),
                (ins, outs) => {
                    debug_assert!(
                        ins.len() <= 1 && outs.len() <= 1,
                        "a tau has one flow in and out"
                    );
                    None
                }
            };
            self.bpmn.remove_node(t);
            if let Some((a, b)) = link {
                self.flow(a, b);
            }
        }
    }
}

impl ProcessTree {
    /// Converts the tree to a BPMN diagram (pm4py's
    /// `process_tree.converter.apply` with the `TO_BPMN` variant, used by
    /// `pm4py.convert_to_bpmn`).
    ///
    /// The diagram has a start event `start` and an end event `end`. Each
    /// activity becomes a task. XOR, parallel and OR nodes become a
    /// diverging and a converging gateway of the matching type. A loop
    /// becomes an exclusive join, its do part, an exclusive split, and its
    /// redo parts back from the split to the join. Silent leaves become
    /// flows. Node ids are `id_<n>`, where pm4py uses random UUIDs.
    ///
    /// Fails on an [`Operator::Interleaving`] node, which pm4py drops
    /// without a word.
    pub fn to_bpmn(&self) -> Result<Bpmn, UnsupportedOperator> {
        let mut b = Builder {
            bpmn: Bpmn::default(),
            taus: Vec::new(),
        };
        let start = b.bpmn.add_node(NodeKind::start_event(), "start");
        let end = b.bpmn.add_node(NodeKind::end_event(), "end");
        b.add(self, start, end)?;
        b.delete_taus();
        Ok(b.bpmn)
    }
}
