//! Reachability graphs, ported from pm4py's
//! `petri_net/utils/reachability_graph.py` (`marking_flow_petri`).

use std::collections::HashMap;

use super::{Marking, PetriNet, TransitionId};

/// Limits for building a reachability graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReachabilityOptions {
    /// Stop with [`ReachabilityError::TooManyMarkings`] once this many
    /// markings have been found. pm4py has a one-day time limit instead.
    pub max_markings: usize,
}

impl Default for ReachabilityOptions {
    fn default() -> Self {
        Self {
            max_markings: 1_000_000,
        }
    }
}

/// Errors raised while building a reachability graph.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ReachabilityError {
    /// The net has more reachable markings than allowed; it may be unbounded.
    #[error("more than {0} reachable markings; the net may be unbounded")]
    TooManyMarkings(usize),
}

/// The reachability graph of a Petri net from an initial marking.
///
/// Markings are numbered in discovery order; marking 0 is the initial one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReachabilityGraph {
    markings: Vec<Marking>,
    index: HashMap<Marking, usize>,
    /// Outgoing edges per marking: `(transition, target marking)`, in
    /// transition-id order.
    outgoing: Vec<Vec<(TransitionId, usize)>>,
}

impl ReachabilityGraph {
    /// All reachable markings, in discovery order.
    pub fn markings(&self) -> &[Marking] {
        &self.markings
    }

    /// The position of a marking, if reachable.
    pub fn index_of(&self, m: &Marking) -> Option<usize> {
        self.index.get(m).copied()
    }

    /// Edges leaving marking `i`, as `(transition, target)` pairs.
    pub fn outgoing(&self, i: usize) -> &[(TransitionId, usize)] {
        &self.outgoing[i]
    }

    /// Iterates over all edges as `(source, transition, target)`.
    pub fn edges(&self) -> impl Iterator<Item = (usize, TransitionId, usize)> + '_ {
        self.outgoing
            .iter()
            .enumerate()
            .flat_map(|(s, es)| es.iter().map(move |&(t, d)| (s, t, d)))
    }

    /// Number of reachable markings.
    pub fn len(&self) -> usize {
        self.markings.len()
    }

    /// Always `false`: the initial marking is always reachable.
    pub fn is_empty(&self) -> bool {
        self.markings.is_empty()
    }
}

impl PetriNet {
    /// Builds the reachability graph from `initial`.
    pub fn reachability_graph(
        &self,
        initial: &Marking,
        options: ReachabilityOptions,
    ) -> Result<ReachabilityGraph, ReachabilityError> {
        let mut g = ReachabilityGraph {
            markings: vec![initial.clone()],
            index: HashMap::from([(initial.clone(), 0)]),
            outgoing: vec![Vec::new()],
        };
        let mut stack = vec![0usize];
        while let Some(i) = stack.pop() {
            let m = g.markings[i].clone();
            for t in self.enabled_transitions(&m) {
                let next = self.weak_fire(t, &m);
                let j = match g.index.get(&next) {
                    Some(&j) => j,
                    None => {
                        if g.markings.len() >= options.max_markings {
                            return Err(ReachabilityError::TooManyMarkings(options.max_markings));
                        }
                        let j = g.markings.len();
                        g.index.insert(next.clone(), j);
                        g.markings.push(next);
                        g.outgoing.push(Vec::new());
                        stack.push(j);
                        j
                    }
                };
                g.outgoing[i].push((t, j));
            }
        }
        Ok(g)
    }
}
