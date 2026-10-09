//! Transition systems, ported from pm4py's `objects/transition_system/`.
//!
//! A [`TransitionSystem`] is a directed graph of named states and named
//! edges. pm4py calls the edges "transitions"; this crate calls them edges so
//! they are not confused with Petri net transitions. Reachability graphs of
//! Petri nets are transition systems (see
//! [`PetriNet::to_transition_system`](crate::PetriNet::to_transition_system)).

use std::collections::VecDeque;

/// Index of a state in a [`TransitionSystem`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StateId(pub(crate) u32);

/// Index of an edge in a [`TransitionSystem`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EdgeId(pub(crate) u32);

impl StateId {
    /// Returns the position of this state in its arena.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

impl EdgeId {
    /// Returns the position of this edge in its arena.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

fn to_u32(n: usize) -> u32 {
    u32::try_from(n).expect("a transition system holds at most u32::MAX states and edges")
}

/// A state of a transition system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    /// The state name. Names need not be unique.
    pub name: String,
    incoming: Vec<EdgeId>,
    outgoing: Vec<EdgeId>,
}

impl State {
    /// Edges that end in this state, in insertion order.
    pub fn incoming(&self) -> &[EdgeId] {
        &self.incoming
    }

    /// Edges that start in this state, in insertion order.
    pub fn outgoing(&self) -> &[EdgeId] {
        &self.outgoing
    }
}

/// A named edge between two states (pm4py's `TransitionSystem.Transition`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    /// The edge name.
    pub name: String,
    from: StateId,
    to: StateId,
}

impl Edge {
    /// The source state.
    pub fn from(&self) -> StateId {
        self.from
    }

    /// The target state.
    pub fn to(&self) -> StateId {
        self.to
    }
}

/// Errors raised by transition system operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TsError {
    /// A state id does not belong to this transition system.
    #[error("state {0:?} does not exist in this transition system")]
    UnknownState(StateId),
    /// The operation needs an acyclic transition system.
    #[error("the transition system has a cycle")]
    Cyclic,
}

/// A transition system: states and named edges, stored in arenas.
///
/// Like pm4py, it holds at most one edge per (name, source, target).
/// States cannot be removed. Removing an edge leaves a hole, so the other
/// edge ids stay valid.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TransitionSystem {
    /// The name of the transition system.
    pub name: String,
    states: Vec<State>,
    edges: Vec<Option<Edge>>,
    live_edges: usize,
}

impl TransitionSystem {
    /// Creates an empty transition system with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }

    /// Adds a state and returns its id.
    pub fn add_state(&mut self, name: impl Into<String>) -> StateId {
        let id = StateId(to_u32(self.states.len()));
        self.states.push(State {
            name: name.into(),
            incoming: Vec::new(),
            outgoing: Vec::new(),
        });
        id
    }

    /// Adds an edge (pm4py's `utils.add_arc_from_to`). If an edge with the
    /// same name, source and target exists, returns its id instead.
    pub fn add_edge(
        &mut self,
        name: impl Into<String>,
        from: StateId,
        to: StateId,
    ) -> Result<EdgeId, TsError> {
        for s in [from, to] {
            if s.index() >= self.states.len() {
                return Err(TsError::UnknownState(s));
            }
        }
        let name = name.into();
        if let Some(&e) = self.states[from.index()]
            .outgoing
            .iter()
            .find(|&&e| self.edge(e).to == to && self.edge(e).name == name)
        {
            return Ok(e);
        }
        let id = EdgeId(to_u32(self.edges.len()));
        self.edges.push(Some(Edge { name, from, to }));
        self.states[from.index()].outgoing.push(id);
        self.states[to.index()].incoming.push(id);
        self.live_edges += 1;
        Ok(id)
    }

    /// Removes an edge. Does nothing if it was already removed.
    pub fn remove_edge(&mut self, id: EdgeId) {
        let Some(edge) = self.edges.get_mut(id.index()).and_then(Option::take) else {
            return;
        };
        self.states[edge.from.index()].outgoing.retain(|&e| e != id);
        self.states[edge.to.index()].incoming.retain(|&e| e != id);
        self.live_edges -= 1;
    }

    /// Removes the edge named `name` from `from` to `to`, if any (pm4py's
    /// `utils.remove_arc_from_to`). Returns `true` if an edge was removed.
    ///
    /// pm4py removes every edge with that name, wherever it is; this
    /// removes only the one between the two states.
    pub fn remove_edge_named(&mut self, name: &str, from: StateId, to: StateId) -> bool {
        let found = self
            .edges_between(from, to)
            .find(|&e| self.edge(e).name == name);
        if let Some(e) = found {
            self.remove_edge(e);
        }
        found.is_some()
    }

    /// Removes every edge from `from` to `to` (pm4py's
    /// `utils.remove_all_arcs_from_to`). Returns how many were removed.
    ///
    /// pm4py also removes edges elsewhere that share a name with one of
    /// these; this removes only the edges between the two states.
    pub fn remove_edges_between(&mut self, from: StateId, to: StateId) -> usize {
        let found: Vec<EdgeId> = self.edges_between(from, to).collect();
        for &e in &found {
            self.remove_edge(e);
        }
        found.len()
    }

    /// Edges from `from` to `to`.
    pub fn edges_between(&self, from: StateId, to: StateId) -> impl Iterator<Item = EdgeId> + '_ {
        self.state(from)
            .outgoing
            .iter()
            .copied()
            .filter(move |&e| self.edge(e).to == to)
    }

    /// Returns the state with the given id.
    ///
    /// # Panics
    ///
    /// Panics if the id does not belong to this transition system.
    pub fn state(&self, id: StateId) -> &State {
        &self.states[id.index()]
    }

    /// Returns the edge with the given id.
    ///
    /// # Panics
    ///
    /// Panics if the edge was removed or does not belong to this transition
    /// system.
    pub fn edge(&self, id: EdgeId) -> &Edge {
        self.edges[id.index()].as_ref().expect("edge was removed")
    }

    /// Returns `true` if the edge exists and was not removed.
    pub fn contains_edge(&self, id: EdgeId) -> bool {
        matches!(self.edges.get(id.index()), Some(Some(_)))
    }

    /// Number of states.
    pub fn state_count(&self) -> usize {
        self.states.len()
    }

    /// Number of edges that were not removed.
    pub fn edge_count(&self) -> usize {
        self.live_edges
    }

    /// State ids in insertion order.
    pub fn state_ids(&self) -> impl Iterator<Item = StateId> + '_ {
        (0..self.states.len()).map(|i| StateId(to_u32(i)))
    }

    /// States with their ids, in insertion order.
    pub fn states(&self) -> impl Iterator<Item = (StateId, &State)> {
        self.states
            .iter()
            .enumerate()
            .map(|(i, s)| (StateId(to_u32(i)), s))
    }

    /// Edges that were not removed, with their ids, in insertion order.
    pub fn edges(&self) -> impl Iterator<Item = (EdgeId, &Edge)> {
        self.edges
            .iter()
            .enumerate()
            .filter_map(|(i, e)| e.as_ref().map(|e| (EdgeId(to_u32(i)), e)))
    }

    /// The first state with the given name.
    pub fn state_by_name(&self, name: &str) -> Option<StateId> {
        self.states()
            .find(|(_, s)| s.name == name)
            .map(|(id, _)| id)
    }

    /// Target states of the edges leaving `s`, without duplicates, in edge
    /// order.
    pub fn successors(&self, s: StateId) -> Vec<StateId> {
        let mut out: Vec<StateId> = Vec::new();
        for &e in &self.state(s).outgoing {
            let to = self.edge(e).to;
            if !out.contains(&to) {
                out.push(to);
            }
        }
        out
    }

    /// States in a topological order, or `None` if there is a cycle
    /// (including a self-loop).
    pub fn topological_order(&self) -> Option<Vec<StateId>> {
        let mut indegree: Vec<usize> = self.states.iter().map(|s| s.incoming.len()).collect();
        let mut queue: VecDeque<StateId> = self
            .state_ids()
            .filter(|s| indegree[s.index()] == 0)
            .collect();
        let mut order = Vec::with_capacity(self.states.len());
        while let Some(s) = queue.pop_front() {
            order.push(s);
            for &e in &self.state(s).outgoing {
                let to = self.edge(e).to;
                indegree[to.index()] -= 1;
                if indegree[to.index()] == 0 {
                    queue.push_back(to);
                }
            }
        }
        (order.len() == self.states.len()).then_some(order)
    }

    /// Removes every edge `s -> v` where `v` can also be reached from `s`
    /// through another successor of `s` (pm4py's
    /// `utils.transitive_reduction`). Fails on a cyclic transition system,
    /// where pm4py's result is undefined.
    ///
    /// It also differs from pm4py on acyclic systems whose edge names repeat,
    /// which is the normal case in a reachability graph: pm4py removes edges
    /// by name across the whole system, so it also deletes unrelated edges.
    /// On `a -e-> b, b -e-> c, a -e-> c, x -e-> y` pm4py removes all four
    /// edges; this keeps `a -> b`, `b -> c` and `x -> y`.
    pub fn transitive_reduction(&mut self) -> Result<(), TsError> {
        let order = self.topological_order().ok_or(TsError::Cyclic)?;
        let n = self.states.len();
        let words = n.div_ceil(64);
        // descendants[s]: states reachable from s by one or more edges.
        let mut descendants = vec![vec![0u64; words]; n];
        for &s in order.iter().rev() {
            let mut acc = vec![0u64; words];
            for c in self.successors(s) {
                acc[c.index() / 64] |= 1 << (c.index() % 64);
                for (a, d) in acc.iter_mut().zip(&descendants[c.index()]) {
                    *a |= d;
                }
            }
            descendants[s.index()] = acc;
        }
        let has = |s: StateId, v: StateId| {
            descendants[s.index()][v.index() / 64] >> (v.index() % 64) & 1 == 1
        };
        for s in self.state_ids().collect::<Vec<_>>() {
            let children = self.successors(s);
            for &v in &children {
                if children.iter().any(|&w| w != v && has(w, v)) {
                    self.remove_edges_between(s, v);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod tests;
