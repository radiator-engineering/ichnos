//! Random playout, ported from pm4py's `process_tree/semantics.py`
//! (`generate_log` / `execute`).
//!
//! The run keeps a set of enabled nodes and opens one of them, chosen
//! uniformly, at each step, as pm4py does. Every node costs one step, and a
//! loop gets an extra silent exit child, so the trace distribution under
//! parallel nodes follows pm4py's.

use rand::seq::SliceRandom;
use rand::{Rng, RngExt};

use super::{Operator, ProcessTree};
use crate::Label;

#[derive(Debug)]
enum Kind {
    Tau,
    Activity(Label),
    Op(Operator),
}

#[derive(Debug)]
struct FlatNode {
    kind: Kind,
    parent: Option<usize>,
    children: Vec<usize>,
}

/// The tree flattened into an arena. Loops get the children
/// `[do, redo, exit]`, where exit is a tau leaf.
#[derive(Debug)]
struct Flat {
    nodes: Vec<FlatNode>,
}

impl Flat {
    fn new(tree: &ProcessTree) -> Self {
        let mut flat = Flat { nodes: Vec::new() };
        flat.push(tree, None);
        flat
    }

    fn push(&mut self, tree: &ProcessTree, parent: Option<usize>) -> usize {
        let id = self.nodes.len();
        let kind = match tree {
            ProcessTree::Tau => Kind::Tau,
            ProcessTree::Activity(l) => Kind::Activity(l.clone()),
            ProcessTree::Node(op, _) => Kind::Op(*op),
        };
        self.nodes.push(FlatNode {
            kind,
            parent,
            children: Vec::new(),
        });
        let children: Vec<usize> = match tree.loop_parts() {
            Some((do_part, redo)) => {
                let redo_tree = match redo {
                    super::LoopRedo::Tau => ProcessTree::Tau,
                    super::LoopRedo::One(r) => r.clone(),
                    super::LoopRedo::Choice(rs) => ProcessTree::xor(rs.iter().cloned()),
                };
                vec![
                    self.push(do_part, Some(id)),
                    self.push(&redo_tree, Some(id)),
                    self.push(&ProcessTree::Tau, Some(id)),
                ]
            }
            None => tree
                .children()
                .iter()
                .map(|c| self.push(c, Some(id)))
                .collect(),
        };
        self.nodes[id].children = children;
        id
    }
}

/// Per-run state.
struct Run<'a, R: ?Sized> {
    flat: &'a Flat,
    rng: &'a mut R,
    enabled: Vec<usize>,
    closed: Vec<bool>,
    /// The children an OR node chose, or the order an interleaving node
    /// picked; for other nodes, the children in tree order.
    active: Vec<Vec<usize>>,
    trace: Vec<Label>,
}

impl<R: Rng + ?Sized> Run<'_, R> {
    fn enable(&mut self, n: usize) {
        self.closed[n] = false;
        self.enabled.push(n);
    }

    fn open(&mut self, v: usize) {
        let flat = self.flat;
        let node = &flat.nodes[v];
        let op = match &node.kind {
            Kind::Tau => return self.close(v),
            Kind::Activity(l) => {
                self.trace.push(l.clone());
                return self.close(v);
            }
            Kind::Op(op) => *op,
        };
        let children = &node.children;
        if children.is_empty() {
            return self.close(v);
        }
        let mut active = children.clone();
        match op {
            Operator::Sequence | Operator::Loop => self.enable(children[0]),
            Operator::Parallel => children.iter().for_each(|&c| self.enable(c)),
            Operator::Xor => {
                let c = children[self.rng.random_range(0..children.len())];
                self.enable(c);
            }
            Operator::Or => {
                // pm4py may pick no child, which leaves the OR node open for
                // ever. Draw again until at least one child is picked.
                loop {
                    active = children
                        .iter()
                        .copied()
                        .filter(|_| self.rng.random_bool(0.5))
                        .collect();
                    if !active.is_empty() {
                        break;
                    }
                }
                active.iter().for_each(|&c| self.enable(c));
            }
            Operator::Interleaving => {
                active.shuffle(self.rng);
                self.enable(active[0]);
            }
        }
        self.active[v] = active;
    }

    fn close(&mut self, mut v: usize) {
        loop {
            self.closed[v] = true;
            let Some(p) = self.flat.nodes[v].parent else {
                return;
            };
            let Kind::Op(op) = self.flat.nodes[p].kind else {
                unreachable!("only operator nodes have children");
            };
            let siblings = &self.active[p];
            let pos = siblings.iter().position(|&c| c == v);
            let should_close = match op {
                Operator::Sequence | Operator::Loop | Operator::Interleaving => {
                    pos == Some(siblings.len() - 1)
                }
                Operator::Xor => true,
                Operator::Parallel | Operator::Or => siblings.iter().all(|&c| self.closed[c]),
            };
            if should_close {
                v = p;
                continue;
            }
            let next = match (op, pos) {
                (Operator::Sequence | Operator::Interleaving, Some(i)) => Some(siblings[i + 1]),
                (Operator::Loop, Some(0)) => Some(siblings[self.rng.random_range(1..=2)]),
                (Operator::Loop, _) => Some(siblings[0]),
                _ => None,
            };
            if let Some(n) = next {
                self.enable(n);
            }
            return;
        }
    }
}

impl ProcessTree {
    /// Plays out one random trace of the tree.
    ///
    /// XOR picks a child uniformly. A loop repeats with probability 1/2 after
    /// each do part. An OR node runs each child with probability 1/2,
    /// redrawing when it picks none.
    pub fn play_out_trace<R: Rng + ?Sized>(&self, rng: &mut R) -> Vec<Label> {
        play_out_one(&Flat::new(self), rng)
    }

    /// Plays out `count` random traces (pm4py's `semantics.generate_log`).
    pub fn play_out<R: Rng + ?Sized>(&self, count: usize, rng: &mut R) -> Vec<Vec<Label>> {
        let flat = Flat::new(self);
        (0..count).map(|_| play_out_one(&flat, rng)).collect()
    }
}

fn play_out_one<R: Rng + ?Sized>(flat: &Flat, rng: &mut R) -> Vec<Label> {
    let n = flat.nodes.len();
    let mut run = Run {
        flat,
        rng,
        enabled: vec![0],
        closed: vec![false; n],
        active: flat.nodes.iter().map(|x| x.children.clone()).collect(),
        trace: Vec::new(),
    };
    while !run.enabled.is_empty() {
        let i = run.rng.random_range(0..run.enabled.len());
        let v = run.enabled.swap_remove(i);
        run.open(v);
    }
    run.trace
}
