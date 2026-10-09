//! Alignments against a process tree (pm4py's
//! `algo/conformance/alignments/process_tree/variants/search_graph_pt.py`,
//! the default of `conformance_diagnostics_alignments` with a tree).
//!
//! The search runs on tree states: every node is *future*, *enabled*,
//! *open* or *closed*. For the next event it takes, for each leaf with the
//! event's activity, the shortest way to enable and then close that leaf
//! (a synchronous move plus the visible leaves it had to open as model
//! moves), or a log move. At the end of the trace it closes the tree.
//! Costs are pm4py's for this method: 1 per log move and per visible model
//! move, 0 for silent leaves.
//!
//! The "shortest way" is computed greedily, so the search is not an exact
//! shortest-path search and the cost can exceed the optimum. ichnos
//! follows pm4py step for step, including its heap, whose entries can lose
//! their order when a cheaper path to a known state lowers their cost, so
//! costs match pm4py exactly.
//!
//! Interleaving nodes are not supported: pm4py's tree semantics have no
//! rules for them.

use std::collections::HashMap;

use ichnos_core::{EventKeys, EventLog};
use ichnos_model::{Label, Operator, ProcessTree};

use super::py_heap::PyHeap;
use super::result::{LogAlignment, SequenceAlignment, SequenceMove};
use crate::error::{Error, Result};

const FUTURE: u8 = 0;
const ENABLED: u8 = 1;
const OPEN: u8 = 2;
const CLOSED: u8 = 3;

type State = Vec<u8>;
type Path = Vec<(u32, u8)>;

#[derive(Debug, Clone)]
struct Node {
    op: Option<Operator>,
    label: Option<Label>,
    parent: Option<u32>,
    children: Vec<u32>,
    /// pm4py's `parent.children.index(node)`: the position of the first
    /// sibling equal to this node in structure.
    first_equal: usize,
}

/// A process tree prepared for alignments.
#[derive(Debug, Clone)]
pub struct TreeAligner {
    nodes: Vec<Node>,
    /// Visible leaves by activity, in depth-first order.
    leaves: HashMap<Label, Vec<u32>>,
    initial: State,
    empty_cost: u64,
}

impl TreeAligner {
    /// Prepares `tree`. Fails with [`Error::ProcessTree`] for a loop
    /// without exactly two children or an operator without children, and
    /// with [`Error::UnsupportedOperator`] for interleaving nodes.
    pub fn new(tree: &ProcessTree) -> Result<Self> {
        tree.validate()?;
        let mut nodes = Vec::new();
        flatten(tree, None, 0, &mut nodes)?;
        let mut leaves: HashMap<Label, Vec<u32>> = HashMap::new();
        for (i, n) in nodes.iter().enumerate() {
            if let Some(l) = &n.label {
                leaves.entry(l.clone()).or_default().push(to_u32(i));
            }
        }
        let mut aligner = Self {
            nodes,
            leaves,
            initial: Vec::new(),
            empty_cost: 0,
        };
        // pm4py's get_initial_state: the root is future, then enabled.
        let state = vec![FUTURE; aligner.nodes.len()];
        aligner.initial = aligner
            .enable_vertex(0, &state)
            .map(|(_, s)| s)
            .expect("the root can be enabled");
        aligner.empty_cost = aligner.search::<&str>(&[]).cost;
        Ok(aligner)
    }

    /// The cost of aligning the empty trace (pm4py's `bwc` for trees).
    pub fn empty_trace_cost(&self) -> u64 {
        self.empty_cost
    }

    /// Aligns one trace, given as activity names.
    pub fn align<S: AsRef<str>>(&self, trace: &[S]) -> SequenceAlignment {
        let mut a = self.search(trace);
        let bwc = trace.len() as u64 + self.empty_cost;
        a.best_worst_cost = bwc;
        a.fitness = if bwc > 0 {
            1.0 - a.cost as f64 / bwc as f64
        } else {
            0.0
        };
        a
    }

    /// Aligns every trace of `log`, once per variant.
    pub fn align_log(
        &self,
        log: &EventLog,
        keys: &EventKeys,
    ) -> Result<LogAlignment<SequenceAlignment>> {
        let variants = log.variants(keys)?;
        let alignments = variants
            .iter()
            .map(|v| Some(self.align(&variants.names(v).collect::<Vec<_>>())))
            .collect();
        Ok(LogAlignment::new(variants, alignments))
    }

    fn op(&self, n: u32) -> Option<Operator> {
        self.nodes[n as usize].op
    }

    fn is_leaf(&self, n: u32) -> bool {
        self.nodes[n as usize].children.is_empty()
    }

    fn child(&self, n: u32, i: usize) -> u32 {
        self.nodes[n as usize].children[i]
    }

    /// The set of the children's states equals `want` (pm4py compares
    /// frozensets).
    fn children_states_are(&self, n: u32, state: &State, want: &[u8]) -> bool {
        let mut seen = [false; 4];
        for &c in &self.nodes[n as usize].children {
            seen[state[c as usize] as usize] = true;
        }
        (0..4u8).all(|s| seen[s as usize] == want.contains(&s))
    }

    fn transform_tree(&self, n: u32, to: u8, state: &mut State, path: &mut Path) {
        if state[n as usize] != to {
            state[n as usize] = to;
            path.push((n, to));
        }
        for i in 0..self.nodes[n as usize].children.len() {
            self.transform_tree(self.child(n, i), to, state, path);
        }
    }

    fn can_enable(&self, n: u32, state: &State) -> bool {
        if state[n as usize] != FUTURE {
            return false;
        }
        let Some(p) = self.nodes[n as usize].parent else {
            return true;
        };
        if state[p as usize] != OPEN {
            return false;
        }
        match self.op(p) {
            Some(Operator::Parallel | Operator::Or) => true,
            Some(Operator::Xor) => self.children_states_are(p, state, &[FUTURE]),
            Some(Operator::Sequence) => {
                let i = self.nodes[n as usize].first_equal;
                i == 0 || state[self.child(p, i - 1) as usize] == CLOSED
            }
            Some(Operator::Loop) => self.children_states_are(p, state, &[FUTURE, CLOSED]),
            _ => false,
        }
    }

    fn can_close(&self, n: u32, state: &State) -> bool {
        if self.is_leaf(n) {
            return state[n as usize] == OPEN;
        }
        match self.op(n) {
            Some(Operator::Sequence | Operator::Parallel | Operator::Xor) => {
                self.children_states_are(n, state, &[CLOSED])
            }
            Some(Operator::Or) => self.children_states_are(n, state, &[CLOSED, FUTURE]),
            Some(Operator::Loop) => {
                state[self.child(n, 0) as usize] == CLOSED
                    && state[self.child(n, 1) as usize] == FUTURE
            }
            _ => false,
        }
    }

    fn close_vertex(&self, n: u32, state: &State) -> Option<(Path, State)> {
        if !self.can_close(n, state) {
            return None;
        }
        let current = state[n as usize];
        let mut state = state.clone();
        let mut path = Path::new();
        for i in 0..self.nodes[n as usize].children.len() {
            let c = self.child(n, i);
            if state[c as usize] != CLOSED {
                self.transform_tree(c, CLOSED, &mut state, &mut path);
            }
        }
        state[n as usize] = CLOSED;
        path.push((n, CLOSED));
        if let Some(p) = self.nodes[n as usize].parent
            && self.op(p) == Some(Operator::Loop)
            && self.child(p, 1) == n
            && current == OPEN
        {
            let (e_path, s) = self.enable_vertex(self.child(p, 0), &state)?;
            path.extend(e_path);
            state = s;
        }
        Some((path, state))
    }

    fn enable_vertex(&self, n: u32, state: &State) -> Option<(Path, State)> {
        if state[n as usize] == ENABLED {
            return Some((Path::new(), state.clone()));
        }
        if !self.can_enable(n, state) {
            return None;
        }
        let mut state = state.clone();
        let mut path = vec![(n, ENABLED)];
        state[n as usize] = ENABLED;
        if let Some(p) = self.nodes[n as usize].parent {
            match self.op(p) {
                Some(Operator::Loop) => {
                    if self.child(p, 0) == n {
                        self.transform_tree(self.child(p, 1), FUTURE, &mut state, &mut path);
                    }
                    if self.child(p, 1) == n {
                        self.transform_tree(self.child(p, 0), FUTURE, &mut state, &mut path);
                    }
                }
                Some(Operator::Xor) => {
                    for i in 0..self.nodes[p as usize].children.len() {
                        let c = self.child(p, i);
                        if c != n {
                            self.transform_tree(c, CLOSED, &mut state, &mut path);
                        }
                    }
                }
                _ => {}
            }
        }
        for i in 0..self.nodes[n as usize].children.len() {
            self.transform_tree(self.child(n, i), FUTURE, &mut state, &mut path);
        }
        Some((path, state))
    }

    fn open_vertex(&self, n: u32, state: &State) -> Option<(Path, State)> {
        if state[n as usize] != ENABLED {
            return None;
        }
        let mut state = state.clone();
        let mut path = vec![(n, OPEN)];
        state[n as usize] = OPEN;
        let children = &self.nodes[n as usize].children;
        match self.op(n) {
            Some(Operator::Xor | Operator::Or | Operator::Parallel) => {
                for &c in children {
                    state[c as usize] = FUTURE;
                    path.push((c, FUTURE));
                }
            }
            Some(Operator::Sequence | Operator::Loop) => {
                state[children[0] as usize] = ENABLED;
                path.push((children[0], ENABLED));
                for &c in &children[1..] {
                    state[c as usize] = FUTURE;
                    path.push((c, FUTURE));
                }
            }
            _ => {}
        }
        Some((path, state))
    }

    fn shortest_path_to_open(&self, n: u32, state: &State) -> Option<(Path, State)> {
        if state[n as usize] == OPEN {
            return Some((Path::new(), state.clone()));
        }
        if let Some(fast) = self.open_vertex(n, state) {
            return Some(fast);
        }
        let (mut path, state) = self.shortest_path_to_enable(n, state)?;
        let (e_path, state) = self.open_vertex(n, &state)?;
        path.extend(e_path);
        Some((path, state))
    }

    fn shortest_path_to_close(&self, n: u32, state: &State) -> Option<(Path, State)> {
        if state[n as usize] == CLOSED {
            return Some((Path::new(), state.clone()));
        }
        if let Some(fast) = self.close_vertex(n, state) {
            return Some(fast);
        }
        let (mut path, mut state) = self.shortest_path_to_open(n, state)?;
        let then = |p: Option<(Path, State)>, path: &mut Path, state: &mut State| {
            let (e, s) = p?;
            path.extend(e);
            *state = s;
            Some(())
        };
        if self.is_leaf(n) {
            let p = self.close_vertex(n, &state);
            then(p, &mut path, &mut state)?;
            return Some((path, state));
        }
        let children = &self.nodes[n as usize].children;
        match self.op(n) {
            Some(Operator::Sequence | Operator::Parallel) => {
                for &c in children {
                    let p = self.shortest_path_to_close(c, &state);
                    then(p, &mut path, &mut state)?;
                }
            }
            Some(Operator::Loop) => {
                let (c0, c1) = (children[0], children[1]);
                if matches!(state[c0 as usize], ENABLED | OPEN) {
                    let p = self.shortest_path_to_close(c0, &state);
                    then(p, &mut path, &mut state)?;
                } else if matches!(state[c1 as usize], ENABLED | OPEN) {
                    let p = self.shortest_path_to_close(c1, &state);
                    then(p, &mut path, &mut state)?;
                    let p = self.shortest_path_to_open(c0, &state);
                    then(p, &mut path, &mut state)?;
                    let p = self.shortest_path_to_close(c0, &state);
                    then(p, &mut path, &mut state)?;
                }
            }
            Some(Operator::Xor | Operator::Or) => {
                let mut busy = false;
                for &c in children {
                    if matches!(state[c as usize], ENABLED | OPEN) {
                        let p = self.shortest_path_to_close(c, &state);
                        then(p, &mut path, &mut state)?;
                        busy = true;
                    }
                }
                if !busy {
                    let mut best: Option<(Path, State, usize)> = None;
                    for &c in children {
                        if state[c as usize] != CLOSED {
                            let (p, s) = self.shortest_path_to_close(c, &state)?;
                            let cost = self.visible_opened(&p).count();
                            if best.as_ref().is_none_or(|b| cost < b.2) {
                                best = Some((p, s, cost));
                            }
                        }
                    }
                    if let Some((p, s, _)) = best {
                        path.extend(p);
                        state = s;
                    }
                }
            }
            _ => {}
        }
        let p = self.close_vertex(n, &state);
        then(p, &mut path, &mut state)?;
        Some((path, state))
    }

    fn shortest_path_to_enable(&self, n: u32, state: &State) -> Option<(Path, State)> {
        if state[n as usize] == ENABLED {
            return Some((Path::new(), state.clone()));
        }
        if let Some(fast) = self.enable_vertex(n, state) {
            return Some(fast);
        }
        let then = |p: Option<(Path, State)>, path: &mut Path, state: &mut State| {
            let (e, s) = p?;
            path.extend(e);
            *state = s;
            Some(())
        };
        if state[n as usize] == FUTURE {
            let p = self.nodes[n as usize].parent?;
            let (mut path, mut state) = self.shortest_path_to_open(p, state)?;
            match self.op(p) {
                Some(Operator::Xor | Operator::Parallel | Operator::Or) => {
                    // pm4py keeps the path but loses the state when another
                    // choice was taken; it then fails, so stop here.
                    then(self.enable_vertex(n, &state), &mut path, &mut state)?;
                }
                Some(Operator::Sequence) => {
                    for i in 0..self.nodes[p as usize].children.len() {
                        let c = self.child(p, i);
                        if c == n {
                            if i > 0 {
                                then(self.enable_vertex(n, &state), &mut path, &mut state)?;
                            }
                            break;
                        }
                        then(
                            self.shortest_path_to_close(c, &state),
                            &mut path,
                            &mut state,
                        )?;
                    }
                }
                Some(Operator::Loop) => {
                    let (c0, c1) = (self.child(p, 0), self.child(p, 1));
                    if c0 == n {
                        if !matches!(state[c1 as usize], FUTURE | CLOSED) {
                            then(
                                self.shortest_path_to_close(c1, &state),
                                &mut path,
                                &mut state,
                            )?;
                        }
                    } else {
                        then(
                            self.shortest_path_to_close(c0, &state),
                            &mut path,
                            &mut state,
                        )?;
                    }
                    then(self.enable_vertex(n, &state), &mut path, &mut state)?;
                }
                _ => {}
            }
            return Some((path, state));
        }
        let (mut path, mut state) = self.shortest_path_to_close(n, state)?;
        let mut parent = self.nodes[n as usize].parent;
        while let Some(p) = parent {
            if self.op(p) == Some(Operator::Loop) && state[p as usize] == OPEN {
                break;
            }
            parent = self.nodes[p as usize].parent;
        }
        let p = parent?;
        let (c0, c1) = (self.child(p, 0), self.child(p, 1));
        if state[c0 as usize] == OPEN {
            then(
                self.shortest_path_to_close(c0, &state),
                &mut path,
                &mut state,
            )?;
            then(
                self.shortest_path_to_enable(c1, &state),
                &mut path,
                &mut state,
            )?;
        } else if state[c1 as usize] == OPEN {
            then(
                self.shortest_path_to_close(c1, &state),
                &mut path,
                &mut state,
            )?;
            then(
                self.shortest_path_to_enable(c0, &state),
                &mut path,
                &mut state,
            )?;
        } else if state[c0 as usize] == FUTURE {
            then(
                self.shortest_path_to_enable(c0, &state),
                &mut path,
                &mut state,
            )?;
        } else if state[c1 as usize] == FUTURE {
            then(
                self.shortest_path_to_enable(c1, &state),
                &mut path,
                &mut state,
            )?;
        }
        then(
            self.shortest_path_to_enable(n, &state),
            &mut path,
            &mut state,
        )?;
        Some((path, state))
    }

    /// Leaves opened along `path`, silent ones included.
    fn opened<'a>(&'a self, path: &'a Path) -> impl Iterator<Item = u32> + 'a {
        path.iter()
            .filter(|&&(n, s)| s == OPEN && self.is_leaf(n))
            .map(|&(n, _)| n)
    }

    /// Visible leaves opened along `path`: the model moves.
    fn visible_opened<'a>(&'a self, path: &'a Path) -> impl Iterator<Item = u32> + 'a {
        self.opened(path)
            .filter(|&n| self.nodes[n as usize].label.is_some())
    }

    /// pm4py's `_need_log_move`: also offer a log move when reaching the
    /// leaf opened other leaves or decided a choice or loop.
    fn need_log_move(&self, old: &State, new: &State, path: &Path) -> bool {
        if self.opened(path).next().is_some() {
            return true;
        }
        let decided = |n: u32| {
            matches!(old[n as usize], FUTURE | CLOSED) && old[n as usize] != new[n as usize]
        };
        self.nodes
            .iter()
            .enumerate()
            .any(|(i, node)| match node.op {
                Some(Operator::Xor) => decided(to_u32(i)),
                Some(Operator::Loop) => node.children.iter().any(|&c| decided(c)),
                _ => false,
            })
    }

    fn is_final(&self, state: &State) -> bool {
        state[0] == CLOSED
    }

    fn search<S: AsRef<str>>(&self, variant: &[S]) -> SequenceAlignment {
        let len = variant.len();
        let mut s = Search {
            states: vec![SearchState {
                cost: 0,
                index: 0,
                state: self.initial.clone(),
                leaves: Vec::new(),
                closing: Vec::new(),
                parent: None,
                children: Vec::new(),
                in_open: true,
                in_closed: false,
            }],
            by_key: HashMap::new(),
        };
        let mut open = PyHeap::new();
        s.index_state(0);
        open.push(0, |_, _| false);
        let mut visited = 0;
        loop {
            let Some(cur) = open.pop(|a, b| s.lt(a, b)) else {
                // pm4py returns the initial state's alignment here; the root
                // can always be closed, so this is not reached.
                return self.result(&s, 0, variant, visited);
            };
            visited += 1;
            let c = cur as usize;
            s.states[c].in_open = false;
            if self.is_final(&s.states[c].state) && s.states[c].index == len {
                return self.result(&s, cur, variant, visited);
            }
            s.states[c].in_closed = true;
            let index = s.states[c].index;
            if index < len {
                let candidates = self
                    .leaves
                    .get(variant[index].as_ref())
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                let mut need_log_move = candidates.is_empty();
                for &leaf in candidates {
                    let old = s.states[c].state.clone();
                    let enabled = self.shortest_path_to_enable(leaf, &old);
                    let closed = enabled.and_then(|(path, new)| {
                        let close = self.shortest_path_to_close(leaf, &new)?;
                        Some((path, new, close))
                    });
                    let Some((mut path, new, (sync_path, after))) = closed else {
                        need_log_move = true;
                        continue;
                    };
                    let model_moves = self.visible_opened(&path).count() as u64;
                    need_log_move = need_log_move || self.need_log_move(&old, &new, &path);
                    path.extend(sync_path);
                    let leaves: Vec<u32> = self.opened(&path).collect();
                    let new_cost = s.states[c].cost + model_moves;
                    s.add(&mut open, cur, new_cost, index + 1, after, leaves);
                }
                if need_log_move {
                    let state = s.states[c].state.clone();
                    let new_cost = s.states[c].cost + 1;
                    s.add(&mut open, cur, new_cost, index + 1, state, Vec::new());
                }
            } else {
                let state = s.states[c].state.clone();
                let Some((path, after)) = self.shortest_path_to_close(0, &state) else {
                    continue;
                };
                s.unindex_state(cur);
                let model_moves = self.visible_opened(&path).count() as u64;
                let st = &mut s.states[c];
                st.state = after;
                st.cost += model_moves;
                st.closing.extend(self.opened(&path));
                st.in_open = true;
                s.index_state(cur);
                open.push(cur, |a, b| s.lt(a, b));
            }
        }
    }

    fn result<S: AsRef<str>>(
        &self,
        s: &Search,
        last: u32,
        variant: &[S],
        visited: usize,
    ) -> SequenceAlignment {
        let leaf_move = |n: u32| SequenceMove::Model {
            activity: self.nodes[n as usize].label.clone(),
        };
        let mut moves = Vec::new();
        let mut cur = last as usize;
        moves.extend(s.states[cur].closing.iter().rev().map(|&n| leaf_move(n)));
        while let Some(p) = s.states[cur].parent {
            let event = s.states[p as usize].index;
            let st = &s.states[cur];
            match st.leaves.split_last() {
                None => moves.push(SequenceMove::Log { event }),
                Some((_, before)) => {
                    moves.push(SequenceMove::Sync { event });
                    moves.extend(before.iter().rev().map(|&n| leaf_move(n)));
                }
            }
            cur = p as usize;
        }
        moves.reverse();
        debug_assert_eq!(
            moves.iter().filter_map(SequenceMove::event).count(),
            variant.len()
        );
        SequenceAlignment {
            moves,
            cost: s.states[last as usize].cost,
            fitness: 0.0,
            best_worst_cost: 0,
            visited_states: visited,
            closed_states: 0,
        }
    }
}

#[derive(Debug, Clone)]
struct SearchState {
    cost: u64,
    index: usize,
    state: State,
    /// Leaves opened to reach this state, the synchronous one last; empty
    /// for a log move.
    leaves: Vec<u32>,
    /// Leaves opened to close the tree after the last event.
    closing: Vec<u32>,
    parent: Option<u32>,
    children: Vec<u32>,
    in_open: bool,
    in_closed: bool,
}

/// pm4py's search states and closed set, with an index by (event index,
/// tree state) in place of its linear scans. The open heap holds state ids.
struct Search {
    states: Vec<SearchState>,
    by_key: HashMap<(usize, State), Vec<u32>>,
}

impl Search {
    /// pm4py's `SGASearchState.__lt__`: lower cost first, then more events
    /// explained.
    fn lt(&self, a: u32, b: u32) -> bool {
        let (a, b) = (&self.states[a as usize], &self.states[b as usize]);
        a.cost < b.cost || (a.cost == b.cost && a.index > b.index)
    }

    fn index_state(&mut self, id: u32) {
        let st = &self.states[id as usize];
        self.by_key
            .entry((st.index, st.state.clone()))
            .or_default()
            .push(id);
    }

    fn unindex_state(&mut self, id: u32) {
        let st = &self.states[id as usize];
        if let Some(ids) = self.by_key.get_mut(&(st.index, st.state.clone())) {
            ids.retain(|&x| x != id);
        }
    }

    /// pm4py's `_add_new_state`: if a closed state, or else an open state,
    /// has the same event index and tree state, lower its cost (and its
    /// descendants') when the new path is cheaper; otherwise queue the new
    /// state.
    fn add(
        &mut self,
        open: &mut PyHeap<u32>,
        parent: u32,
        cost: u64,
        index: usize,
        state: State,
        leaves: Vec<u32>,
    ) {
        let matches: Vec<u32> = self
            .by_key
            .get(&(index, state.clone()))
            .cloned()
            .unwrap_or_default();
        for want_closed in [true, false] {
            let hits: Vec<u32> = matches
                .iter()
                .copied()
                .filter(|&m| {
                    let st = &self.states[m as usize];
                    if want_closed {
                        st.in_closed
                    } else {
                        st.in_open
                    }
                })
                .collect();
            if hits.is_empty() {
                continue;
            }
            for m in hits {
                let alt_cost = self.states[m as usize].cost;
                if cost < alt_cost {
                    let delta = alt_cost - cost;
                    let children = self.states[m as usize].children.clone();
                    self.lower(delta, &children);
                    let alt = &mut self.states[m as usize];
                    alt.cost = cost;
                    alt.parent = Some(parent);
                    alt.leaves = leaves.clone();
                }
            }
            return;
        }
        let id = u32::try_from(self.states.len()).expect("state count fits u32");
        self.states.push(SearchState {
            cost,
            index,
            state,
            leaves,
            closing: Vec::new(),
            parent: Some(parent),
            children: Vec::new(),
            in_open: true,
            in_closed: false,
        });
        self.states[parent as usize].children.push(id);
        self.index_state(id);
        open.push(id, |a, b| self.lt(a, b));
    }

    /// pm4py's `_update_costs_recursive`.
    fn lower(&mut self, delta: u64, ids: &[u32]) {
        for &id in ids {
            self.states[id as usize].cost -= delta;
            let children = self.states[id as usize].children.clone();
            self.lower(delta, &children);
        }
    }
}

fn flatten(
    tree: &ProcessTree,
    parent: Option<u32>,
    first_equal: usize,
    nodes: &mut Vec<Node>,
) -> Result<u32> {
    let id = to_u32(nodes.len());
    nodes.push(Node {
        op: tree.operator(),
        label: tree.label().cloned(),
        parent,
        children: Vec::new(),
        first_equal,
    });
    if tree.operator() == Some(Operator::Interleaving) {
        return Err(Error::UnsupportedOperator(Operator::Interleaving));
    }
    let children = tree.children();
    for (i, c) in children.iter().enumerate() {
        let first = children.iter().position(|x| x == c).unwrap_or(i);
        let cid = flatten(c, Some(id), first, nodes)?;
        nodes[id as usize].children.push(cid);
    }
    Ok(id)
}

/// Aligns every trace of `log` against `tree` (pm4py's
/// `conformance_diagnostics_alignments` with a process tree).
pub fn align_log_tree(
    log: &EventLog,
    tree: &ProcessTree,
    keys: &EventKeys,
) -> Result<LogAlignment<SequenceAlignment>> {
    TreeAligner::new(tree)?.align_log(log, keys)
}

fn to_u32(i: usize) -> u32 {
    u32::try_from(i).expect("tree node count fits u32")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(s: &str) -> ProcessTree {
        ProcessTree::parse(s).unwrap()
    }

    #[test]
    fn sequence_with_choice() {
        let al = TreeAligner::new(&tree("->( 'a', X( 'b', 'c' ), 'd' )")).unwrap();
        assert_eq!(al.empty_trace_cost(), 3);
        let a = al.align(&["a", "b", "d"]);
        assert_eq!(a.cost, 0);
        assert_eq!(a.fitness, 1.0);
        let a = al.align(&["a", "d"]);
        assert_eq!(a.cost, 1);
        assert_eq!(a.best_worst_cost, 5);
        let a = al.align(&["a", "x", "c", "d"]);
        assert_eq!(a.cost, 1);
        assert_eq!(
            a.moves
                .iter()
                .filter(|m| matches!(m, SequenceMove::Log { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn loops_repeat() {
        let al = TreeAligner::new(&tree("*( 'a', 'b' )")).unwrap();
        assert_eq!(al.align(&["a", "b", "a"]).cost, 0);
        assert_eq!(al.align(&["a", "b"]).cost, 1);
    }

    #[test]
    fn interleaving_is_rejected() {
        let err = TreeAligner::new(&tree("<>( 'a', 'b' )")).unwrap_err();
        assert!(matches!(
            err,
            Error::UnsupportedOperator(Operator::Interleaving)
        ));
    }
}
