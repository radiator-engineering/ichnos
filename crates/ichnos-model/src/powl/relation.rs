//! Binary relations over the children of a partial order, ported from
//! pm4py's `powl/BinaryRelation.py`.

use super::PowlError;

/// A binary relation over the nodes `0..len()`.
///
/// pm4py keys the relation by node objects. Here nodes are positions, the
/// same positions as the children of the [`StrictPartialOrder`] that owns
/// the relation.
///
/// Methods that take node positions panic when a position is out of range.
///
/// [`StrictPartialOrder`]: super::StrictPartialOrder
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct BinaryRelation {
    len: usize,
    edges: Vec<bool>,
}

impl BinaryRelation {
    /// Creates the empty relation over `len` nodes.
    pub fn new(len: usize) -> Self {
        Self {
            len,
            edges: vec![false; len * len],
        }
    }

    /// The number of nodes.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` when the relation has no nodes.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn at(&self, source: usize, target: usize) -> usize {
        assert!(
            source < self.len && target < self.len,
            "node ({source}, {target}) out of range for a relation over {} nodes",
            self.len
        );
        source * self.len + target
    }

    /// Relates `source` to `target`.
    pub fn add_edge(&mut self, source: usize, target: usize) {
        let i = self.at(source, target);
        self.edges[i] = true;
    }

    /// Removes the pair `(source, target)`.
    pub fn remove_edge(&mut self, source: usize, target: usize) {
        let i = self.at(source, target);
        self.edges[i] = false;
    }

    /// Returns `true` when `source` is related to `target`.
    pub fn is_edge(&self, source: usize, target: usize) -> bool {
        self.edges[self.at(source, target)]
    }

    /// The related pairs, in row-major order.
    pub fn edges(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        let n = self.len;
        self.edges
            .iter()
            .enumerate()
            .filter(|&(_, &e)| e)
            .map(move |(i, _)| (i / n, i % n))
    }

    /// The number of related pairs.
    pub fn edge_count(&self) -> usize {
        self.edges.iter().filter(|&&e| e).count()
    }

    /// Adds a node with no pairs and returns its position.
    pub(crate) fn push_node(&mut self) -> usize {
        let n = self.len;
        let mut edges = vec![false; (n + 1) * (n + 1)];
        for (i, row) in self.edges.chunks(n.max(1)).enumerate().take(n) {
            edges[i * (n + 1)..i * (n + 1) + n].copy_from_slice(row);
        }
        self.len = n + 1;
        self.edges = edges;
        n
    }

    /// Nodes no pair leads into (pm4py's `get_start_nodes`), ascending.
    pub fn start_nodes(&self) -> Vec<usize> {
        (0..self.len)
            .filter(|&j| (0..self.len).all(|i| !self.is_edge(i, j)))
            .collect()
    }

    /// Nodes no pair leaves (pm4py's `get_end_nodes`), ascending.
    pub fn end_nodes(&self) -> Vec<usize> {
        (0..self.len)
            .filter(|&i| (0..self.len).all(|j| !self.is_edge(i, j)))
            .collect()
    }

    /// Returns `true` when no node is related to itself.
    pub fn is_irreflexive(&self) -> bool {
        (0..self.len).all(|i| !self.is_edge(i, i))
    }

    /// Returns `true` when `i -> j` and `j -> k` always give `i -> k`.
    pub fn is_transitive(&self) -> bool {
        self.transitivity_gap().is_none()
    }

    /// The first `(i, j, k)` with `i -> j`, `j -> k` and not `i -> k`.
    pub(crate) fn transitivity_gap(&self) -> Option<(usize, usize, usize)> {
        let n = self.len;
        (0..n)
            .flat_map(|i| (0..n).flat_map(move |j| (0..n).map(move |k| (i, j, k))))
            .find(|&(i, j, k)| self.is_edge(i, j) && self.is_edge(j, k) && !self.is_edge(i, k))
    }

    /// Returns `true` for an irreflexive, transitive relation.
    pub fn is_strict_partial_order(&self) -> bool {
        self.is_irreflexive() && self.is_transitive()
    }

    /// pm4py's `get_transitive_reduction`: drops each pair `i -> k` for
    /// which some `j` has `i -> j` and `j -> k`.
    ///
    /// On a transitive relation this is the transitive reduction. On other
    /// relations it can keep pairs that a longer path implies, as pm4py's
    /// does.
    ///
    /// # Errors
    ///
    /// [`PowlError::Reflexive`] when a node is related to itself.
    pub fn transitive_reduction(&self) -> Result<BinaryRelation, PowlError> {
        if let Some(i) = (0..self.len).find(|&i| self.is_edge(i, i)) {
            return Err(PowlError::Reflexive(i));
        }
        let mut res = self.clone();
        let n = self.len;
        for i in 0..n {
            for j in 0..n {
                for k in 0..n {
                    if i != j && j != k && self.is_edge(i, j) && self.is_edge(j, k) {
                        res.remove_edge(i, k);
                    }
                }
            }
        }
        Ok(res)
    }

    /// Adds pairs until the relation is transitive (pm4py's
    /// `add_transitive_edges`). As in pm4py, a cycle relates its nodes to
    /// themselves.
    pub fn add_transitive_edges(&mut self) {
        let n = self.len;
        let mut changed = true;
        while changed {
            changed = false;
            for i in 0..n {
                for j in 0..n {
                    for k in 0..n {
                        if i != j
                            && j != k
                            && self.is_edge(i, j)
                            && self.is_edge(j, k)
                            && !self.is_edge(i, k)
                        {
                            self.add_edge(i, k);
                            changed = true;
                        }
                    }
                }
            }
        }
    }

    /// pm4py's `remove_edge_without_violating_transitivity`: removes
    /// `(source, target)`, then, while some `i -> j`, `j -> k` lacks
    /// `i -> k`, removes `j -> k`.
    pub fn remove_edge_without_violating_transitivity(&mut self, source: usize, target: usize) {
        self.remove_edge(source, target);
        let n = self.len;
        let mut changed = true;
        while changed {
            changed = false;
            for i in 0..n {
                for j in 0..n {
                    for k in 0..n {
                        if i != j
                            && j != k
                            && self.is_edge(i, j)
                            && self.is_edge(j, k)
                            && !self.is_edge(i, k)
                        {
                            self.remove_edge(j, k);
                            changed = true;
                        }
                    }
                }
            }
        }
    }
}
