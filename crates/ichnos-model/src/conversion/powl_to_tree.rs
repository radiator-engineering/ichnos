//! POWL to process tree, ported from pm4py's
//! `objects/conversion/powl/variants/to_process_tree.py`.

use std::collections::VecDeque;

use crate::powl::{Powl, PowlError, StrictPartialOrder};
use crate::{Operator, ProcessTree};

/// The transitive reduction of the order of `po` as adjacency lists, with
/// pairs of a child with itself left out, as pm4py builds its graph.
///
/// Returns [`PowlError::Cyclic`] when the order has a cycle.
fn reduced_graph(po: &StrictPartialOrder) -> Result<Vec<Vec<usize>>, PowlError> {
    let n = po.children().len();
    let succ: Vec<Vec<usize>> = (0..n)
        .map(|i| (0..n).filter(|&j| i != j && po.is_edge(i, j)).collect())
        .collect();

    // Topological order (Kahn), which also finds cycles.
    let mut indegree = vec![0usize; n];
    for &j in succ.iter().flatten() {
        indegree[j] += 1;
    }
    let mut queue: VecDeque<usize> = (0..n).filter(|&i| indegree[i] == 0).collect();
    let mut topo = Vec::with_capacity(n);
    while let Some(i) = queue.pop_front() {
        topo.push(i);
        for &j in &succ[i] {
            indegree[j] -= 1;
            if indegree[j] == 0 {
                queue.push_back(j);
            }
        }
    }
    if topo.len() < n {
        return Err(PowlError::Cyclic);
    }

    // reach[i][j]: a path of one or more pairs leads from i to j.
    let mut reach = vec![vec![false; n]; n];
    for &i in topo.iter().rev() {
        for &j in &succ[i] {
            let via = reach[j].clone();
            for (r, v) in reach[i].iter_mut().zip(via) {
                *r |= v;
            }
            reach[i][j] = true;
        }
    }
    // Keep i -> j unless some other successor of i reaches j.
    Ok((0..n)
        .map(|i| {
            succ[i]
                .iter()
                .copied()
                .filter(|&j| !succ[i].iter().any(|&k| k != j && reach[k][j]))
                .collect()
        })
        .collect())
}

/// Converts `powl`, clearing `precise` when a partial order is not a
/// series of fully connected levels.
fn convert(powl: &Powl, precise: &mut bool) -> Result<ProcessTree, PowlError> {
    let po = match powl {
        Powl::Silent => return Ok(ProcessTree::Tau),
        Powl::Activity(_) | Powl::Frequent(_) => {
            return Ok(ProcessTree::Activity(
                powl.label().expect("a visible transition has a label"),
            ));
        }
        Powl::Xor(children) => {
            let children = children
                .iter()
                .map(|c| convert(c, precise))
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(ProcessTree::Node(Operator::Xor, children));
        }
        Powl::Loop(children) => {
            let [d, r] = &**children;
            return Ok(ProcessTree::looped(
                convert(d, precise)?,
                convert(r, precise)?,
            ));
        }
        Powl::PartialOrder(po) => po,
    };

    let succ = reduced_graph(po)?;
    let n = succ.len();
    let mut pred = vec![Vec::new(); n];
    for (i, s) in succ.iter().enumerate() {
        for &j in s {
            pred[j].push(i);
        }
    }
    let adjacent = |a: usize, b: usize| succ[a].contains(&b) || succ[b].contains(&a);

    // Connected components of the undirected graph, in order of their
    // smallest child.
    let mut component = vec![usize::MAX; n];
    let mut components: Vec<Vec<usize>> = Vec::new();
    for start in 0..n {
        if component[start] != usize::MAX {
            continue;
        }
        let id = components.len();
        let mut members = vec![start];
        component[start] = id;
        let mut k = 0;
        while k < members.len() {
            let i = members[k];
            for &j in succ[i].iter().chain(&pred[i]) {
                if component[j] == usize::MAX {
                    component[j] = id;
                    members.push(j);
                }
            }
            k += 1;
        }
        members.sort_unstable();
        components.push(members);
    }

    let mut subtrees = Vec::with_capacity(components.len());
    for members in components {
        if let [only] = members.as_slice() {
            subtrees.push(convert(&po.children()[*only], precise)?);
            continue;
        }
        // Level = distance from the nearest child without predecessors.
        let mut level = vec![usize::MAX; n];
        let mut queue: VecDeque<usize> = members
            .iter()
            .copied()
            .filter(|&i| pred[i].is_empty())
            .collect();
        for &i in &queue {
            level[i] = 0;
        }
        while let Some(i) = queue.pop_front() {
            for &j in &succ[i] {
                if level[j] == usize::MAX {
                    level[j] = level[i] + 1;
                    queue.push_back(j);
                }
            }
        }
        let depth = members.iter().map(|&i| level[i]).max().unwrap_or(0) + 1;
        let mut levels: Vec<Vec<usize>> = vec![Vec::new(); depth];
        for &i in &members {
            levels[level[i]].push(i);
        }
        for pair in levels.windows(2) {
            if pair[0]
                .iter()
                .any(|&a| pair[1].iter().any(|&b| !adjacent(a, b)))
            {
                *precise = false;
            }
        }
        let mut steps = Vec::with_capacity(levels.len());
        for lvl in levels {
            let mut nodes = lvl
                .iter()
                .map(|&i| convert(&po.children()[i], precise))
                .collect::<Result<Vec<_>, _>>()?;
            steps.push(if nodes.len() == 1 {
                nodes.pop().expect("one node")
            } else {
                ProcessTree::Node(Operator::Parallel, nodes)
            });
        }
        subtrees.push(ProcessTree::Node(Operator::Sequence, steps));
    }
    Ok(if subtrees.len() == 1 {
        subtrees.pop().expect("one subtree")
    } else {
        ProcessTree::Node(Operator::Parallel, subtrees)
    })
}

impl Powl {
    /// Converts the model to a process tree where it can (pm4py's
    /// `powl.variants.to_process_tree.apply`). Same as
    /// [`to_process_tree_checked`](Self::to_process_tree_checked) without
    /// the precision flag.
    ///
    /// # Errors
    ///
    /// [`PowlError::Cyclic`] when a partial order has a cycle.
    pub fn to_process_tree(&self) -> Result<ProcessTree, PowlError> {
        self.to_process_tree_checked().map(|(tree, _)| tree)
    }

    /// Converts the model to a process tree and reports whether the
    /// conversion is precise.
    ///
    /// A partial order becomes a parallel node over its connected
    /// components. A component with more than one child becomes a sequence
    /// of levels, where a child's level is its distance from the nearest
    /// child without predecessors in the transitive reduction; a level with
    /// several children becomes a parallel node. The flag is `false` when
    /// some child of a level is not ordered directly before some child of
    /// the next level; pm4py warns "This POWL model cannot be converted
    /// precisely." then.
    ///
    /// Parallel children follow the children's order. pm4py's order there
    /// depends on hashing.
    ///
    /// # Errors
    ///
    /// [`PowlError::Cyclic`] when a partial order has a cycle.
    pub fn to_process_tree_checked(&self) -> Result<(ProcessTree, bool), PowlError> {
        let mut precise = true;
        let tree = convert(self, &mut precise)?;
        Ok((tree, precise))
    }
}
