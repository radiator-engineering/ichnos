//! Ordered triconnected-component DFS used by the RPST refinement.
use super::rpst::{Component, Link};
#[derive(Clone, Copy)]
struct SeparationTriple {
    h: i64,
    a: usize,
    b: usize,
}
// Edge IDs remain stable while orientation and active adjacency change.
// A shared virtual-edge ID connects two components; real edges occur once.
struct Decomposer<'a> {
    links: &'a mut Vec<Link>,
    active: Vec<usize>,
    vertices: Vec<usize>,
    adj: Vec<Vec<usize>>,
    num: Vec<i64>,
    state: Vec<u8>,
    etype: Vec<u8>,
    parent: Vec<Option<usize>>,
    tree: Vec<Option<usize>>,
    low1: Vec<i64>,
    low2: Vec<i64>,
    low1_vertex: Vec<usize>,
    low2_vertex: Vec<usize>,
    descendants: Vec<i64>,
    starts: Vec<bool>,
    number: Vec<i64>,
    tree_count: Vec<i64>,
    low1_number: Vec<i64>,
    low2_number: Vec<i64>,
    high: Vec<Vec<usize>>,
    count: Vec<i64>,
    remaining: Vec<i64>,
    hidden: Vec<bool>,
    assigned: Vec<Option<usize>>,
    components: Vec<Vec<usize>>,
    edge_stack: Vec<usize>,
    separation_stack: Vec<Option<SeparationTriple>>,
    dfsnum: i64,
    m: i64,
    new_path: bool,
    root: usize,
}
impl<'a> Decomposer<'a> {
    fn new(links: &'a mut Vec<Link>, n: usize, root: usize) -> Self {
        let active: Vec<_> = (0..links.len()).collect();
        let mut vertices = Vec::new();
        let mut seen = vec![false; n];
        let mut adj = vec![Vec::new(); n];
        for &e in &active {
            for v in [links[e].ends.0, links[e].ends.1] {
                if !seen[v] {
                    seen[v] = true;
                    vertices.push(v);
                }
                adj[v].push(e);
            }
        }
        let ne = links.len();
        Self {
            links,
            active,
            vertices,
            adj,
            num: vec![-1; n],
            state: vec![0; n],
            etype: vec![0; ne],
            parent: vec![None; n],
            tree: vec![None; n],
            low1: vec![-1; n],
            low2: vec![-1; n],
            low1_vertex: vec![0; n],
            low2_vertex: vec![0; n],
            descendants: vec![-1; n],
            starts: vec![false; ne],
            number: vec![-1; n],
            tree_count: vec![0; n],
            low1_number: vec![-1; n],
            low2_number: vec![-1; n],
            high: vec![Vec::new(); n],
            count: vec![0; n],
            remaining: vec![0; n],
            hidden: vec![false; ne],
            assigned: vec![None; ne],
            components: Vec::new(),
            edge_stack: Vec::new(),
            separation_stack: Vec::new(),
            dfsnum: 0,
            m: 0,
            new_path: true,
            root,
        }
    }
    fn other(&self, e: usize, v: usize) -> usize {
        let (a, b) = self.links[e].ends;
        if b == v { a } else { b }
    }
    fn same(&self, e: usize, a: usize, b: usize) -> bool {
        let (u, v) = self.links[e].ends;
        (u == a && v == b) || (u == b && v == a)
    }
    fn incident(&self, v: usize) -> Vec<usize> {
        self.active
            .iter()
            .copied()
            .filter(|&e| {
                let (a, b) = self.links[e].ends;
                a == v || b == v
            })
            .collect()
    }
    fn push_virtual(&mut self, a: usize, b: usize) -> usize {
        let e = self.links.len();
        self.links.push(Link {
            ends: (a, b),
            virtual_edge: true,
        });
        self.active.push(e);
        self.etype.push(0);
        self.starts.push(false);
        self.hidden.push(false);
        self.assigned.push(None);
        e
    }
    fn multiple(&mut self) {
        let mut ordered = Vec::new();
        for &v in &self.vertices {
            let mut groups: Vec<(usize, Vec<usize>)> = Vec::new();
            for &e in &self.active {
                let (a, b) = self.links[e].ends;
                let ia = self.vertices.iter().position(|&x| x == a).unwrap();
                let ib = self.vertices.iter().position(|&x| x == b).unwrap();
                if self.vertices[ia.min(ib)] != v {
                    continue;
                }
                let key = ia + ib;
                if let Some((_, g)) = groups.iter_mut().find(|(k, _)| *k == key) {
                    g.push(e);
                } else {
                    groups.push((key, vec![e]));
                }
            }
            for (_, g) in groups {
                ordered.extend(g);
            }
        }
        let mut i = 0;
        while i < ordered.len() {
            let mut j = i + 1;
            let (a, b) = self.links[ordered[i]].ends;
            while j < ordered.len() && self.same(ordered[j], a, b) {
                j += 1;
            }
            if j - i > 1 {
                let mut g = ordered[i..j].to_vec();
                for &e in &g {
                    self.active.retain(|&x| x != e);
                    self.hidden[e] = true;
                }
                let e = self.push_virtual(a, b);
                g.insert(0, e);
                for &x in &g {
                    self.assigned[x] = Some(e);
                }
                self.components.push(g);
            }
            i = j;
        }
    }
    fn dfs(&mut self, v: usize, number: bool) {
        self.dfsnum += 1;
        self.num[v] = self.dfsnum;
        self.state[v] = 1;
        if number {
            self.number[v] = self.m - self.descendants[v] + 1;
            self.tree_count[v] = 0;
        } else {
            self.low1[v] = self.dfsnum;
            self.low2[v] = self.dfsnum;
            self.low1_vertex[v] = v;
            self.low2_vertex[v] = v;
            self.descendants[v] = 1;
        }
        for e in self.adj[v].clone() {
            if self.etype[e] != 0 {
                continue;
            }
            let w = self.other(e, v);
            self.links[e].ends = (v, w);
            let tree = self.state[w] == 0;
            self.etype[e] = if tree { 1 } else { 2 };
            if tree {
                self.parent[w] = Some(v);
                self.tree[w] = Some(e);
                if self.new_path {
                    self.starts[e] = true;
                    self.new_path = false;
                }
            } else {
                if self.new_path {
                    self.starts[e] = true;
                }
                self.new_path = true;
            }
            if !tree {
                if number {
                    self.high[w].push(v);
                } else if self.num[w] < self.low1[v] {
                    self.low2[v] = self.low1[v];
                    self.low2_vertex[v] = self.low1_vertex[v];
                    self.low1[v] = self.num[w];
                    self.low1_vertex[v] = w;
                } else if self.num[w] > self.low1[v] && self.num[w] < self.low2[v] {
                    self.low2[v] = self.num[w];
                    self.low2_vertex[v] = w;
                }
            }
            if tree {
                self.dfs(w, number);
                if number {
                    self.m -= 1;
                    self.tree_count[v] += 1;
                } else {
                    if self.low1[w] < self.low1[v] {
                        let m = self.low1[v].min(self.low2[w]);
                        self.low2[v] = m;
                        self.low2_vertex[v] = if m == self.low1[v] {
                            self.low1_vertex[v]
                        } else {
                            self.low2_vertex[w]
                        };
                        self.low1[v] = self.low1[w];
                        self.low1_vertex[v] = self.low1_vertex[w];
                    } else if self.low1[w] == self.low1[v] {
                        if self.low2[w] < self.low2[v] {
                            self.low2[v] = self.low2[w];
                            self.low2_vertex[v] = self.low2_vertex[w];
                        }
                    } else if self.low1[w] < self.low2[v] {
                        self.low2[v] = self.low1[w];
                        self.low2_vertex[v] = self.low1_vertex[w];
                    }
                    self.descendants[v] += self.descendants[w];
                }
            }
        }
        self.state[v] = 2;
        if number {
            self.low1_number[v] = self.number[self.low1_vertex[v]];
            self.low2_number[v] = self.number[self.low2_vertex[v]];
        }
    }
    fn order(&mut self) {
        let mut edges = self.active.clone();
        edges.sort_by_key(|&e| {
            let (a, b) = self.links[e].ends;
            if self.etype[e] == 1 {
                3 * self.low1[b] + if self.low2[b] < self.num[a] { 0 } else { 2 }
            } else {
                3 * self.num[b] + 1
            }
        });
        self.adj.fill(Vec::new());
        for e in edges {
            self.adj[self.links[e].ends.0].push(e);
        }
    }
    fn remove(&mut self, edges: &[usize]) {
        for &e in edges {
            let (a, b) = self.links[e].ends;
            self.adj[a].retain(|&x| x != e);
            if self.active.contains(&e) {
                self.active.retain(|&x| x != e);
                self.count[a] -= 1;
                self.count[b] -= 1;
            }
            self.hidden[e] = true;
        }
    }
    fn component(&mut self, edges: Vec<usize>) -> usize {
        self.remove(&edges);
        let c = self.components.len();
        self.components.push(edges);
        c
    }
    fn add_component(&mut self, c: usize, edges: &[usize]) {
        self.remove(edges);
        self.components[c].extend(edges);
    }
    fn virtual_edge(&mut self, c: usize, a: usize, b: usize) -> usize {
        let e = self.push_virtual(a, b);
        self.count[a] += 1;
        self.count[b] += 1;
        self.components[c].insert(0, e);
        self.adj[a].push(e);
        for &x in &self.components[c] {
            self.assigned[x] = Some(e);
        }
        e
    }
    fn tree_edge(&mut self, e: usize, a: usize, b: usize) {
        self.links[e].ends = (a, b);
        self.etype[e] = 1;
    }
    fn highnum(&self, v: usize) -> i64 {
        self.high[v].first().map_or(0, |&w| self.number[w])
    }
    fn update_ts(&mut self, v: usize, w: usize, tree: bool) {
        let mut last = None;
        let mut y = -1;
        let threshold = if tree {
            self.low1_number[w]
        } else {
            self.number[w]
        };
        while self
            .separation_stack
            .last()
            .copied()
            .flatten()
            .is_some_and(|t| self.number[t.a] > threshold)
        {
            let t = self.separation_stack.pop().unwrap().unwrap();
            y = y.max(t.h);
            last = Some(t);
        }
        let item = if tree {
            SeparationTriple {
                h: y.max(self.number[w] + self.descendants[w] - 1),
                a: self.low1_vertex[w],
                b: last.map_or(v, |t| t.b),
            }
        } else {
            SeparationTriple {
                h: if last.is_none() { self.number[v] } else { y },
                a: w,
                b: last.map_or(v, |t| t.b),
            }
        };
        self.separation_stack.push(Some(item));
        if tree {
            self.separation_stack.push(None);
        }
    }
    fn pre(&mut self, e: usize, w: usize, tree: bool) {
        let v = self.other(e, w);
        self.remaining[v] -= 1;
        if self.starts[e] {
            self.update_ts(v, w, tree);
        }
        if !tree {
            if self.parent[v] == Some(w) {
                let c = self.component(vec![e, self.tree[v].unwrap()]);
                let ve = self.virtual_edge(c, w, v);
                self.tree_edge(ve, w, v);
            } else {
                self.edge_stack.push(e);
            }
        }
    }
    fn type1(&mut self, v: usize, w: usize) {
        if self.low2_number[w] >= self.number[v]
            && self.low1_number[w] < self.number[v]
            && (self.parent[v] != Some(self.root) || self.remaining[v] > 0)
        {
            let low = self.low1_vertex[w];
            let c = self.component(Vec::new());
            let h = self.number[w] + self.descendants[w] - 1;
            while self.edge_stack.last().is_some_and(|&e| {
                let (a, b) = self.links[e].ends;
                (self.number[w]..=h).contains(&self.number[a])
                    || (self.number[w]..=h).contains(&self.number[b])
            }) {
                let e = self.edge_stack.pop().unwrap();
                self.add_component(c, &[e]);
            }
            let mut ve = self.virtual_edge(c, v, low);
            if self
                .edge_stack
                .last()
                .is_some_and(|&e| self.same(e, v, low))
            {
                let e = self.edge_stack.pop().unwrap();
                let c = self.component(vec![e, ve]);
                ve = self.virtual_edge(c, v, low);
            }
            if self.parent[v] != Some(low) {
                self.edge_stack.push(ve);
            } else {
                let c = self.component(vec![self.tree[v].unwrap(), ve]);
                ve = self.virtual_edge(c, low, v);
                self.tree[v] = Some(ve);
            }
            self.adj[v].push(ve);
            self.tree_edge(ve, low, v);
        }
    }
    fn type2(&mut self, v: usize, mut w: usize) {
        loop {
            let top = self.separation_stack.last().copied().flatten();
            let first = self.adj[w].first().map(|&e| self.other(e, w));
            let simple =
                self.count[w] == 2 && first.is_some_and(|x| self.number[x] > self.number[w]);
            if v == self.root || !(top.is_some_and(|t| t.a == v) || simple) {
                break;
            }
            if top.is_some_and(|t| t.a == v && self.parent[t.b] == Some(t.a)) {
                self.separation_stack.pop();
                continue;
            }
            let c = self.component(Vec::new());
            let mut ab = Vec::new();
            let mut ve;
            let t;
            if simple {
                t = top;
                let e1 = self.edge_stack.pop().unwrap();
                let e2 = self.edge_stack.pop().unwrap();
                self.add_component(c, &[e1, e2]);
                let fc = first.unwrap();
                ve = self.virtual_edge(c, v, fc);
                if self.edge_stack.last().is_some_and(|&e| {
                    top.is_some_and(|x| self.same(e, v, x.b)) || self.same(e, v, fc)
                }) {
                    ab.push(self.edge_stack.pop().unwrap());
                }
            } else {
                let item = self.separation_stack.pop().unwrap().unwrap();
                t = Some(item);
                while self.edge_stack.last().is_some_and(|&e| {
                    let (a, b) = self.links[e].ends;
                    self.number[a] >= self.number[item.a]
                        && self.number[b] >= self.number[item.a]
                        && self.number[a] <= item.h
                        && self.number[b] <= item.h
                }) {
                    let e = self.edge_stack.pop().unwrap();
                    if self.same(e, item.a, item.b) {
                        ab.push(e);
                    } else {
                        self.add_component(c, &[e]);
                    }
                }
                ve = self.virtual_edge(c, item.a, item.b);
            }
            if !ab.is_empty() {
                let b = match t {
                    Some(t) if !first.is_some_and(|fc| self.same(ab[0], v, fc)) => t.b,
                    _ => first.expect("simple separation pair has a child"),
                };
                ab.push(ve);
                let c = self.component(ab);
                ve = self.virtual_edge(c, v, b);
            }
            self.edge_stack.push(ve);
            w = self.other(ve, v);
            self.tree_edge(ve, v, w);
            self.parent[w] = Some(v);
        }
    }
    fn split_dfs(&mut self, v: usize) {
        self.state[v] = 1;
        for e in self.adj[v].clone() {
            let w = self.other(e, v);
            self.links[e].ends = (v, w);
            let tree = self.state[w] == 0;
            if self.etype[e] != 1 && self.etype[e] != 2 {
                self.etype[e] = if tree { 1 } else { 2 };
            }
            self.pre(e, w, tree);
            if tree {
                self.split_dfs(w);
                let v = self.other(e, w);
                let mut pushed = e;
                while self.hidden[pushed] {
                    pushed = self.assigned[pushed].unwrap();
                }
                self.edge_stack.push(pushed);
                self.type2(v, w);
                self.type1(v, w);
                if self.starts[e] {
                    while self.separation_stack.last().is_some_and(Option::is_some) {
                        self.separation_stack.pop();
                    }
                    self.separation_stack.pop();
                }
                let high = self.highnum(v);
                while self
                    .separation_stack
                    .last()
                    .copied()
                    .flatten()
                    .is_some_and(|t| t.a != v && t.b != v && high > t.h)
                {
                    self.separation_stack.pop();
                }
            }
        }
        self.state[v] = 2;
    }
    // Low points determine stable adjacency buckets; the second DFS numbers
    // paths before the edge/separation stacks emit ordered components.
    fn run(mut self) -> Vec<Component> {
        self.multiple();
        for &v in &self.vertices {
            self.adj[v] = self.incident(v);
        }
        self.dfs(self.root, false);
        self.order();
        self.state.fill(0);
        self.etype.fill(0);
        self.starts.fill(false);
        self.dfsnum = 0;
        self.new_path = true;
        self.m = self.vertices.len() as i64;
        self.dfs(self.root, true);
        for &v in &self.vertices {
            self.count[v] = self.incident(v).len() as i64;
            self.remaining[v] = self.tree_count[v];
        }
        self.state.fill(0);
        self.etype.fill(0);
        self.separation_stack.push(None);
        self.split_dfs(self.root);
        if !self.edge_stack.is_empty() {
            let edges = std::mem::take(&mut self.edge_stack);
            self.component(edges);
        }
        let mut queue: std::collections::VecDeque<_> = self
            .components
            .into_iter()
            .map(|edges| Component {
                kind: super::rpst::classify(&edges, self.links),
                edges,
            })
            .collect();
        let mut result = Vec::new();
        while let Some(node) = queue.pop_front() {
            let found = if node.kind == 'R' {
                None
            } else {
                queue.iter().enumerate().find_map(|(i, other)| {
                    if node.kind == other.kind {
                        node.edges
                            .iter()
                            .copied()
                            .find(|&e| self.links[e].virtual_edge && other.edges.contains(&e))
                            .map(|e| (i, e))
                    } else {
                        None
                    }
                })
            };
            if let Some((i, e)) = found {
                let other = queue.remove(i).unwrap();
                let edges = node
                    .edges
                    .into_iter()
                    .chain(other.edges)
                    .filter(|&x| x != e)
                    .collect();
                queue.push_back(Component {
                    kind: node.kind,
                    edges,
                });
            } else {
                result.push(node);
            }
        }
        result
    }
}
pub(super) fn decompose(links: &mut Vec<Link>, n: usize, root: usize) -> Vec<Component> {
    Decomposer::new(links, n, root).run()
}
