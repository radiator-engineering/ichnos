//! RPST fragments from an SPQR decomposition of the normalized undirected graph.
//! Component and sibling order follow the low-point, numbering and split DFS passes.
use super::Edge;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
#[derive(Clone)]
pub(super) struct Link {
    pub ends: Edge,
    pub virtual_edge: bool,
}
pub(super) struct Component {
    pub kind: char,
    pub edges: Vec<usize>,
}
struct Tree {
    kind: char,
    boundary: Edge,
    children: Vec<Tree>,
}
pub(super) struct Fragment {
    pub kind: char,
    pub entry: usize,
    pub exit: usize,
    pub edges: BTreeSet<Edge>,
}
fn vertices(edges: &[usize], links: &[Link]) -> Vec<usize> {
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    for &e in edges {
        for v in [links[e].ends.0, links[e].ends.1] {
            if seen.insert(v) {
                result.push(v);
            }
        }
    }
    result
}
pub(super) fn classify(edges: &[usize], links: &[Link]) -> char {
    let vs = vertices(edges, links);
    if vs.len() == 2 {
        return 'B';
    }
    let mut deg = BTreeMap::new();
    for &e in edges {
        let (a, b) = links[e].ends;
        *deg.entry(a).or_insert(0) += 1;
        *deg.entry(b).or_insert(0) += 1;
    }
    if deg.values().all(|&d| d == 2) {
        'P'
    } else {
        'R'
    }
}
fn build(
    c: usize,
    boundary: Edge,
    comps: &[Component],
    links: &[Link],
    seen: &mut BTreeSet<usize>,
    quasi: &BTreeSet<usize>,
) -> Tree {
    seen.insert(c);
    let mut children = Vec::new();
    for &e in &comps[c].edges {
        if !links[e].virtual_edge {
            continue;
        }
        for other in 0..comps.len() {
            if !seen.contains(&other) && comps[other].edges.contains(&e) {
                children.push(build(other, links[e].ends, comps, links, seen, quasi));
            }
        }
    }
    for &e in &comps[c].edges {
        if !links[e].virtual_edge && !quasi.contains(&e) {
            children.push(Tree {
                kind: 'T',
                boundary: links[e].ends,
                children: Vec::new(),
            });
        }
    }
    Tree {
        kind: comps[c].kind,
        boundary,
        children,
    }
}
fn contract(mut t: Tree) -> Tree {
    t.children = t.children.into_iter().map(contract).collect();
    if t.children.len() == 1 {
        t.children.pop().unwrap()
    } else {
        t
    }
}
fn flatten(
    t: Tree,
    real: &[usize],
    directed: &BTreeSet<Edge>,
    start: usize,
    end: usize,
    is_root: bool,
    all: &mut Vec<(Fragment, Vec<usize>)>,
) -> Option<usize> {
    let (a, b) = (real[t.boundary.0], real[t.boundary.1]);
    if t.kind == 'T' && ((a == start && b == end) || (a == end && b == start)) {
        return None;
    }
    let id = all.len();
    all.push((
        Fragment {
            kind: t.kind,
            entry: a,
            exit: b,
            edges: BTreeSet::new(),
        },
        Vec::new(),
    ));
    let children: Vec<_> = t
        .children
        .into_iter()
        .filter_map(|t| flatten(t, real, directed, start, end, false, all))
        .collect();
    let mut edges = BTreeSet::new();
    for &c in &children {
        edges.extend(all[c].0.edges.iter().copied());
    }
    if t.kind == 'T' && a != b {
        if directed.contains(&(a, b)) {
            edges.insert((a, b));
        }
        if directed.contains(&(b, a)) {
            edges.insert((b, a));
        }
    }
    let incoming = |v| edges.iter().any(|e| e.1 == v);
    let entry_exit = if is_root {
        (start, end)
    } else if a == end || b == end {
        (if a == end { b } else { a }, end)
    } else if a == start || b == start {
        (start, if a == start { b } else { a })
    } else if !incoming(a) && incoming(b) {
        (a, b)
    } else if incoming(a)
        && (!incoming(b)
            || edges.iter().filter(|e| e.0 == a).count()
                != directed.iter().filter(|e| e.0 == a).count())
    {
        (b, a)
    } else {
        (a, b)
    };
    all[id] = (
        Fragment {
            kind: t.kind,
            entry: entry_exit.0,
            exit: entry_exit.1,
            edges,
        },
        children,
    );
    Some(id)
}
pub(super) fn fragments(directed: &[Edge], n: usize) -> Option<Vec<Fragment>> {
    let nodes: BTreeSet<_> = directed.iter().flat_map(|&(a, b)| [a, b]).collect();
    let sources: Vec<_> = nodes
        .iter()
        .copied()
        .filter(|v| !directed.iter().any(|e| e.1 == *v))
        .collect();
    let sinks: Vec<_> = nodes
        .iter()
        .copied()
        .filter(|v| !directed.iter().any(|e| e.0 == *v))
        .collect();
    if sources.len() != 1 || sinks.len() != 1 {
        return None;
    }
    let (start, end) = (sources[0], sinks[0]);
    let mut first_seen = Vec::new();
    for &(a, b) in directed {
        for v in [a, b] {
            if !first_seen.contains(&v) {
                first_seen.push(v);
            }
        }
    }
    let mut star = BTreeMap::new();
    let mut real: Vec<_> = (0..n).collect();
    for &v in &first_seen {
        if directed.iter().filter(|e| e.0 == v).count() > 1
            && directed.iter().filter(|e| e.1 == v).count() > 1
        {
            star.insert(v, real.len());
            real.push(v);
        }
    }
    let mut links = Vec::new();
    for &(a, b) in directed {
        links.push(Link {
            ends: (star.get(&a).copied().unwrap_or(a), b),
            virtual_edge: false,
        });
    }
    let mut quasi = BTreeSet::new();
    for &a in &first_seen {
        let Some(&b) = star.get(&a) else {
            continue;
        };
        quasi.insert(links.len());
        links.push(Link {
            ends: (a, b),
            virtual_edge: false,
        });
    }
    let back = links.len();
    links.push(Link {
        ends: (end, start),
        virtual_edge: true,
    });
    let comps = super::spqr::decompose(&mut links, real.len(), end);
    let root = comps.iter().position(|c| c.edges.contains(&back))?;
    let tree = contract(build(
        root,
        (end, start),
        &comps,
        &links,
        &mut BTreeSet::new(),
        &quasi,
    ));
    let mut all = Vec::new();
    let root = flatten(
        tree,
        &real,
        &directed.iter().copied().collect(),
        start,
        end,
        true,
        &mut all,
    )?;
    let mut todo = VecDeque::from([root]);
    let mut order = Vec::new();
    while let Some(i) = todo.pop_front() {
        order.push(i);
        todo.extend(all[i].1.iter().copied());
    }
    order.reverse();
    let mut slots: Vec<_> = all.into_iter().map(|(f, _)| Some(f)).collect();
    Some(
        order
            .into_iter()
            .map(|i| slots[i].take().unwrap())
            .collect(),
    )
}
