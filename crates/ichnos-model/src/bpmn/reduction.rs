//! Gateway reductions, ported from pm4py's `objects/bpmn/util/reduction.py`.

use super::{Bpmn, FlowId, GatewayKind, Node, NodeId, NodeKind};

impl Bpmn {
    /// The first live node, in id order, that satisfies `f`.
    fn find_node(&self, f: impl Fn(&Node) -> bool) -> Option<NodeId> {
        self.nodes().find(|(_, n)| f(n)).map(|(id, _)| id)
    }

    /// The live flows into and out of a node.
    fn in_out(&self, n: NodeId) -> (Vec<FlowId>, Vec<FlowId>) {
        let node = self.node(n);
        (node.in_flows.clone(), node.out_flows.clone())
    }

    fn add_sequence_flow(&mut self, source: NodeId, target: NodeId) {
        self.add_flow(source, target)
            .expect("both ends are in the diagram");
    }

    /// Removes the gateway `g` with one incoming flow `a -> g` and one
    /// outgoing flow `g -> b`. With `link`, adds a sequence flow `a -> b`.
    fn splice(&mut self, g: NodeId, link: bool) {
        let (ins, outs) = self.in_out(g);
        let source = self.flow(ins[0]).source;
        let target = self.flow(outs[0]).target;
        self.remove_node(g);
        if link {
            self.add_sequence_flow(source, target);
        }
    }

    /// Removes every exclusive gateway with exactly one incoming and one
    /// outgoing flow, linking its neighbours with a new sequence flow
    /// (pm4py's `reduce_xor_gateways`). A gateway whose only flow is a
    /// self-loop stays: splicing it out would leave the same self-loop, so
    /// pm4py, which tries anyway, never ends on such a diagram.
    pub fn reduce_xor_gateways(&mut self) {
        while let Some(g) = self.find_node(|node| {
            node.kind.gateway_kind() == Some(GatewayKind::Exclusive)
                && node.in_flows.len() == 1
                && node.out_flows.len() == 1
                && node.in_flows != node.out_flows
        }) {
            self.splice(g, true);
        }
    }

    /// Removes every gateway with exactly one incoming and one outgoing
    /// flow (pm4py's `remove_trivial_gateways`). Its neighbours get a new
    /// sequence flow, unless they are the same node.
    pub fn remove_trivial_gateways(&mut self) {
        while let Some(g) = self.find_node(|node| {
            node.kind.is_gateway() && node.in_flows.len() == 1 && node.out_flows.len() == 1
        }) {
            let (ins, outs) = self.in_out(g);
            let link = self.flow(ins[0]).source != self.flow(outs[0]).target;
            self.splice(g, link);
        }
    }

    /// Merges a split `s` into a gateway `g` of the same XOR or AND type
    /// when every flow into `s` comes from `g` (pm4py's
    /// `collapse_split_gateways`). `g` takes over the outgoing flows of
    /// `s`. Repeats until nothing changes.
    pub fn collapse_split_gateways(&mut self) {
        while let Some((g, s)) = self.mergeable_pair(true) {
            let (s_in, s_out) = self.in_out(s);
            for f in s_out {
                let t = self.flow(f).target;
                self.remove_flow(f);
                if t != g {
                    self.add_sequence_flow(g, t);
                }
            }
            for f in s_in {
                self.remove_flow(f);
            }
            self.remove_node(s);
        }
    }

    /// Merges a join `p` into a gateway `g` of the same XOR or AND type
    /// when every flow out of `p` goes to `g` (pm4py's
    /// `collapse_join_gateways`). `g` takes over the incoming flows of
    /// `p`. Repeats until nothing changes.
    pub fn collapse_join_gateways(&mut self) {
        while let Some((g, p)) = self.mergeable_pair(false) {
            let (p_in, p_out) = self.in_out(p);
            for f in p_in {
                let s = self.flow(f).source;
                self.remove_flow(f);
                if s != g {
                    self.add_sequence_flow(s, g);
                }
            }
            for f in p_out {
                self.remove_flow(f);
            }
            self.remove_node(p);
        }
    }

    /// The first XOR or AND gateway `g`, in id order, with a neighbour of
    /// the same kind that feeds only `g` (`split == false`) or is fed only
    /// by `g` (`split == true`). Self-loops do not count.
    fn mergeable_pair(&self, split: bool) -> Option<(NodeId, NodeId)> {
        let mergeable = |k: &NodeKind| {
            matches!(
                k.gateway_kind(),
                Some(GatewayKind::Exclusive | GatewayKind::Parallel)
            )
        };
        for (g, node) in self.nodes() {
            if !mergeable(&node.kind) {
                continue;
            }
            let flows = if split {
                &node.out_flows
            } else {
                &node.in_flows
            };
            for &f in flows {
                let flow = self.flow(f);
                let other = if split { flow.target } else { flow.source };
                if other == g || self.node(other).kind.gateway_kind() != node.kind.gateway_kind() {
                    continue;
                }
                let o = self.node(other);
                let only_g = if split {
                    o.in_flows.iter().all(|&f| self.flow(f).source == g)
                } else {
                    o.out_flows.iter().all(|&f| self.flow(f).target == g)
                };
                if only_g {
                    return Some((g, other));
                }
            }
        }
        None
    }

    /// Collapses splits, joins, splits and joins again, then removes
    /// trivial gateways (pm4py's `collapse_gateways`).
    pub fn collapse_gateways(&mut self) {
        self.collapse_split_gateways();
        self.collapse_join_gateways();
        self.collapse_split_gateways();
        self.collapse_join_gateways();
        self.remove_trivial_gateways();
    }

    /// pm4py's `bpmn.util.reduction.apply`: [`Bpmn::reduce_xor_gateways`],
    /// then [`Bpmn::collapse_gateways`] if `collapse` is set (pm4py's
    /// `COLLAPSE_GATEWAYS`, off by default).
    pub fn reduce(&mut self, collapse: bool) {
        self.reduce_xor_gateways();
        if collapse {
            self.collapse_gateways();
        }
    }
}
