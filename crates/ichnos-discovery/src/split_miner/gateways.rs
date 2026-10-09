//! Inclusive-join normalization, dominator analysis and token generation.
use super::graph::{Graph, Kind};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
const ENTRY: usize = usize::MAX - 1;
const EXIT: usize = usize::MAX;
#[derive(Clone)]
struct Flow {
    src: usize,
    tgt: usize,
    first: usize,
    last: usize,
    back: bool,
}
struct Map {
    flows: Vec<Option<Flow>>,
    gates: BTreeSet<usize>,
    ins: BTreeMap<usize, Vec<usize>>,
    out: BTreeMap<usize, Vec<usize>>,
    loop_joins: BTreeSet<usize>,
    depth: BTreeMap<usize, usize>,
    dom: BTreeMap<usize, usize>,
}
#[derive(Default)]
struct Visit {
    unvisited: BTreeSet<usize>,
    visiting: BTreeSet<usize>,
    settled: BTreeMap<usize, bool>,
    flows: BTreeSet<usize>,
    loops: BTreeSet<usize>,
    forward: BTreeSet<usize>,
}
fn key(g: &Graph, v: usize) -> &str {
    match v {
        ENTRY => "__gm_entry__",
        EXIT => "__gm_exit__",
        _ => &g.nodes[v].as_ref().unwrap().key,
    }
}
fn sorted(g: &Graph, ids: impl IntoIterator<Item = usize>) -> Vec<usize> {
    let mut ids: Vec<_> = ids.into_iter().collect();
    ids.sort_by(|&a, &b| key(g, a).cmp(key(g, b)));
    ids
}
fn kind(g: &Graph, v: usize) -> Option<Kind> {
    g.nodes.get(v)?.as_ref().map(|n| n.kind)
}
impl Map {
    fn new() -> Self {
        Self {
            flows: Vec::new(),
            gates: BTreeSet::new(),
            ins: BTreeMap::new(),
            out: BTreeMap::new(),
            loop_joins: BTreeSet::new(),
            depth: BTreeMap::new(),
            dom: BTreeMap::new(),
        }
    }
    fn gate(&mut self, v: usize) {
        self.gates.insert(v);
        self.ins.entry(v).or_default();
        self.out.entry(v).or_default();
    }
    fn add(&mut self, src: usize, tgt: usize, first: usize, last: usize) -> usize {
        let id = self.flows.len();
        self.flows.push(Some(Flow {
            src,
            tgt,
            first,
            last,
            back: false,
        }));
        self.out.entry(src).or_default().push(id);
        self.ins.entry(tgt).or_default().push(id);
        id
    }
    fn remove(&mut self, id: usize) {
        if let Some(f) = self.flows[id].take() {
            self.out.get_mut(&f.src).unwrap().retain(|&v| v != id);
            self.ins.get_mut(&f.tgt).unwrap().retain(|&v| v != id);
        }
    }
    fn target(&mut self, id: usize, to: usize) -> usize {
        let f = self.flows[id].as_ref().unwrap().clone();
        let new = self.add(
            f.src,
            to,
            if f.first == f.tgt { to } else { f.first },
            f.last,
        );
        self.flows[new].as_mut().unwrap().back = f.back;
        self.remove(id);
        new
    }
    fn source(&mut self, id: usize, from: usize) -> usize {
        let f = self.flows[id].as_ref().unwrap().clone();
        let new = self.add(
            from,
            f.tgt,
            f.first,
            if f.last == f.src { from } else { f.last },
        );
        self.flows[new].as_mut().unwrap().back = f.back;
        self.remove(id);
        new
    }
    fn incoming(&self, v: usize) -> Vec<usize> {
        self.ins.get(&v).cloned().unwrap_or_default()
    }
    fn outgoing(&self, v: usize) -> Vec<usize> {
        self.out.get(&v).cloned().unwrap_or_default()
    }
    fn successors(&self, v: usize) -> BTreeSet<usize> {
        self.outgoing(v)
            .iter()
            .map(|&i| self.flows[i].as_ref().unwrap().tgt)
            .collect()
    }
    fn build(&mut self, g: &mut Graph) -> bool {
        for v in g.order.clone() {
            if kind(g, v).is_some_and(Kind::gate) && g.ins[v].len() > 1 && g.out[v].len() > 1 {
                let split = g.add(g.kind(v));
                for to in g.out[v].clone() {
                    g.unedge(v, to);
                    g.edge(split, to);
                }
                g.edge(v, split);
                g.nodes[v].as_mut().unwrap().kind = Kind::Or;
            }
        }
        for &v in &g.order {
            if kind(g, v).is_some_and(Kind::gate) {
                self.gate(v);
            }
        }
        if self.gates.is_empty() {
            return false;
        }
        let mut first = g.start;
        let mut seen = BTreeSet::new();
        while !kind(g, first).is_some_and(Kind::gate) && g.out[first].len() == 1 {
            if !seen.insert(first) {
                return false;
            }
            first = g.out[first][0];
        }
        if !kind(g, first).is_some_and(Kind::gate) {
            return false;
        }
        let mut queue = VecDeque::from([first]);
        let mut visited = BTreeSet::from([g.end]);
        let mut processed = BTreeSet::new();
        let mut last = None;
        while let Some(v) = queue.pop_front() {
            if !processed.insert(v) {
                continue;
            }
            visited.insert(v);
            for child in g.out[v].clone() {
                let (mut cur, mut prev) = (child, v);
                let mut chain = BTreeSet::new();
                while !kind(g, cur).is_some_and(Kind::gate) && cur != g.end && g.out[cur].len() == 1
                {
                    if !chain.insert(cur) {
                        return false;
                    }
                    prev = cur;
                    cur = g.out[cur][0];
                }
                if kind(g, cur).is_some_and(Kind::gate) {
                    self.add(v, cur, child, prev);
                }
                if cur == g.end {
                    last = Some(v);
                }
                if !visited.contains(&cur) && !queue.contains(&cur) {
                    queue.push_back(cur);
                }
            }
        }
        let Some(last) = last else {
            return false;
        };
        self.gate(ENTRY);
        self.gate(EXIT);
        self.add(ENTRY, first, first, ENTRY);
        self.add(last, EXIT, EXIT, last);
        let mut visit = Visit {
            unvisited: self.gates.clone(),
            ..Default::default()
        };
        self.explore(ENTRY, &mut visit);
        for e in visit.loops.difference(&visit.forward) {
            self.flows[*e].as_mut().unwrap().back = true;
        }
        self.normalize(g);
        self.hierarchy(g);
        self.dominators();
        true
    }
    fn explore(&self, v: usize, state: &mut Visit) -> bool {
        state.unvisited.remove(&v);
        state.visiting.insert(v);
        let (mut back, mut forward) = (false, v == EXIT);
        for e in self.outgoing(v) {
            state.flows.insert(e);
            let next = self.flows[e].as_ref().unwrap().tgt;
            let result = if state.unvisited.contains(&next) {
                Some(self.explore(next, state))
            } else if state.visiting.contains(&next) {
                Some(true)
            } else {
                state.settled.get(&next).copied()
            };
            if let Some(is_back) = result {
                if is_back {
                    back = true;
                    state.loops.insert(e);
                } else {
                    forward = true;
                    state.forward.insert(e);
                }
            }
        }
        state.visiting.remove(&v);
        let all = self.incoming(v).iter().all(|e| state.flows.contains(e));
        if all {
            state.settled.insert(v, back && !forward);
        } else {
            state.unvisited.insert(v);
        }
        back && !forward
    }
    fn normalize(&mut self, g: &mut Graph) {
        for join in sorted(g, self.gates.clone()) {
            let ins = self.incoming(join);
            if ins.len() <= 1 {
                continue;
            }
            let loops: Vec<_> = ins
                .iter()
                .copied()
                .filter(|&e| self.flows[e].as_ref().unwrap().back)
                .collect();
            if loops.is_empty() {
                continue;
            }
            if ins.len() - loops.len() > 1 {
                let lj = g.add(Kind::Xor);
                self.gate(lj);
                let srcs: BTreeSet<_> = loops
                    .iter()
                    .map(|&e| self.flows[e].as_ref().unwrap().last)
                    .collect();
                for e in self.outgoing(join) {
                    self.source(e, lj);
                }
                self.add(join, lj, lj, join);
                for e in loops {
                    self.target(e, lj);
                }
                for to in g.out[join].clone() {
                    g.unedge(join, to);
                    g.edge(lj, to);
                }
                g.edge(join, lj);
                for from in g.ins[join].clone() {
                    if srcs.contains(&from) {
                        g.unedge(from, join);
                        g.edge(from, lj);
                    }
                }
                self.loop_joins.insert(lj);
            } else {
                self.loop_joins.insert(join);
                if kind(g, join) == Some(Kind::Or) {
                    g.nodes[join].as_mut().unwrap().kind = Kind::Xor;
                }
            }
        }
    }
    fn hierarchy(&mut self, g: &Graph) {
        self.depth.insert(ENTRY, 0);
        let mut queue = VecDeque::from([ENTRY]);
        let mut seen = BTreeSet::from([ENTRY]);
        while let Some(v) = queue.pop_front() {
            let d = self.depth[&v] + 1;
            for next in sorted(g, self.successors(v)) {
                let bump = !self.depth.contains_key(&next)
                    || (self.depth[&next] < d && !self.loop_joins.contains(&next));
                if bump {
                    self.depth.insert(next, d);
                }
                if seen.contains(&next) && (self.loop_joins.contains(&next) || !bump) {
                    continue;
                }
                queue.push_back(next);
                seen.insert(next);
            }
        }
    }
    fn dominators(&mut self) {
        let mut reachable = BTreeSet::from([ENTRY]);
        let mut todo = vec![ENTRY];
        while let Some(v) = todo.pop() {
            for next in self.successors(v) {
                if reachable.insert(next) {
                    todo.push(next);
                }
            }
        }
        let mut dom: BTreeMap<_, _> = reachable
            .iter()
            .map(|&v| {
                (
                    v,
                    if v == ENTRY {
                        BTreeSet::from([v])
                    } else {
                        reachable.clone()
                    },
                )
            })
            .collect();
        loop {
            let mut changed = false;
            for &v in &reachable {
                if v == ENTRY {
                    continue;
                }
                let preds: Vec<_> = self
                    .incoming(v)
                    .iter()
                    .map(|&e| self.flows[e].as_ref().unwrap().src)
                    .filter(|p| reachable.contains(p))
                    .collect();
                let mut d = preds.iter().skip(1).fold(dom[&preds[0]].clone(), |a, p| {
                    a.intersection(&dom[p]).copied().collect()
                });
                d.insert(v);
                if d != dom[&v] {
                    dom.insert(v, d);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        for &v in &reachable {
            if v == ENTRY {
                self.dom.insert(v, v);
                continue;
            }
            if let Some(d) = dom[&v]
                .iter()
                .copied()
                .filter(|&d| d != v)
                .max_by_key(|d| dom[d].len())
            {
                self.dom.insert(v, d);
            }
        }
    }
    fn replace(&mut self, g: &mut Graph, hagen: bool) {
        let mut pending: Vec<_> = self
            .depth
            .keys()
            .copied()
            .filter(|&v| kind(g, v) == Some(Kind::Or))
            .collect();
        pending.sort_by(|&a, &b| (self.depth[&a], key(g, a)).cmp(&(self.depth[&b], key(g, b))));
        for ior in pending {
            if kind(g, ior) != Some(Kind::Or) {
                continue;
            }
            let Some(&dominator) = self.dom.get(&ior) else {
                continue;
            };
            let mut todo = BTreeMap::new();
            let mut visited = BTreeMap::new();
            let mut flows = BTreeMap::new();
            let mut looped = false;
            for e in self.incoming(ior) {
                let f = self.flows[e].as_ref().unwrap().clone();
                if f.back {
                    g.nodes[ior].as_mut().unwrap().kind = Kind::Xor;
                    looped = true;
                    break;
                }
                if !g.out.get(f.last).is_some_and(|out| out.contains(&ior)) {
                    continue;
                }
                g.unedge(f.last, ior);
                let xor = g.add(Kind::Xor);
                g.edge(f.last, xor);
                g.edge(xor, ior);
                self.gate(xor);
                self.add(xor, ior, ior, xor);
                self.target(e, xor);
                todo.insert(xor, BTreeSet::from([xor]));
                visited.insert(xor, BTreeSet::from([dominator]));
                flows.insert(xor, BTreeSet::new());
            }
            if looped {
                continue;
            }
            let depth = self.depth[&ior];
            let mut ands = BTreeSet::new();
            let mut frontier = BTreeSet::new();
            let mut injections = BTreeMap::new();
            loop {
                let mut empty = true;
                for xor in sorted(g, todo.keys().copied()) {
                    let mut next = BTreeSet::new();
                    for v in sorted(g, todo[&xor].clone()) {
                        for e in self.incoming(v) {
                            if !flows.get_mut(&xor).unwrap().insert(e) {
                                continue;
                            }
                            let src = self.flows[e].as_ref().unwrap().src;
                            if src == dominator {
                                frontier.insert(e);
                            }
                            if self.loop_joins.contains(&src) && !injections.contains_key(&src) {
                                let injecting = self
                                    .incoming(src)
                                    .into_iter()
                                    .filter(|&e| {
                                        let from = self.flows[e].as_ref().unwrap().src;
                                        self.depth.get(&from).is_none_or(|&d| d > depth)
                                    })
                                    .collect::<BTreeSet<_>>();
                                injections.insert(src, injecting);
                            }
                            if visited[&xor].contains(&src)
                                || self.depth.get(&src).is_some_and(|&d| d > depth)
                            {
                                continue;
                            }
                            visited.get_mut(&xor).unwrap().insert(src);
                            if kind(g, src) == Some(Kind::And) && self.outgoing(src).len() > 1 {
                                ands.insert(src);
                            }
                            next.insert(src);
                            empty = false;
                        }
                    }
                    todo.insert(xor, next);
                }
                if empty {
                    break;
                }
            }
            let is_xor = self.check_xor(&visited, &flows, &ands);
            if is_xor {
                g.nodes[ior].as_mut().unwrap().kind = Kind::Xor;
                continue;
            }
            if !hagen {
                g.nodes[ior].as_mut().unwrap().kind = Kind::Or;
                continue;
            }
            let mut changes = BTreeMap::<usize, BTreeSet<usize>>::new();
            for xor in sorted(g, visited.keys().copied()) {
                for v in sorted(g, visited[&xor].clone()) {
                    if kind(g, v) == Some(Kind::And) || self.outgoing(v).len() == 1 {
                        continue;
                    }
                    for e in self.outgoing(v) {
                        if flows[&xor].contains(&e) || (v == dominator && !frontier.contains(&e)) {
                            continue;
                        }
                        changes.entry(e).or_default().insert(xor);
                    }
                }
            }
            for (e, xors) in changes {
                self.token(g, e, &xors);
            }
            for join in sorted(g, injections.keys().copied()) {
                let inj = &injections[&join];
                let xors: BTreeSet<_> = visited
                    .iter()
                    .filter(|(_, v)| !v.contains(&join))
                    .map(|(&x, _)| x)
                    .collect();
                if !inj.is_empty() && !xors.is_empty() {
                    self.multiple(g, join, inj, &xors);
                }
            }
            g.nodes[ior].as_mut().unwrap().kind = Kind::And;
        }
        g.trivial();
    }
    fn check_xor(
        &self,
        visited: &BTreeMap<usize, BTreeSet<usize>>,
        flows: &BTreeMap<usize, BTreeSet<usize>>,
        ands: &BTreeSet<usize>,
    ) -> bool {
        for &and in ands {
            let out: BTreeSet<_> = self.outgoing(and).into_iter().collect();
            let sets: Vec<_> = visited
                .keys()
                .filter_map(|x| {
                    let v: BTreeSet<_> = out.intersection(&flows[x]).copied().collect();
                    if v.is_empty() {
                        None
                    } else {
                        let u = out.difference(&flows[x]).copied().collect::<BTreeSet<_>>();
                        Some((v, u))
                    }
                })
                .collect();
            for (v1, u1) in &sets {
                for (v2, u2) in &sets {
                    if u2.is_subset(u1) && (v2.is_subset(v1) || v2.is_disjoint(v1)) {
                        continue;
                    }
                    return false;
                }
            }
        }
        true
    }
    fn token(&mut self, g: &mut Graph, e: usize, xors: &BTreeSet<usize>) {
        let f = self.flows[e].as_ref().unwrap().clone();
        let and = if kind(g, f.first) == Some(Kind::And) && self.outgoing(f.first).len() > 1 {
            f.first
        } else {
            let and = g.add(Kind::And);
            g.edge(f.src, and);
            if kind(g, f.first).is_some() {
                g.edge(and, f.first);
            }
            self.gate(and);
            self.add(f.src, and, and, f.src);
            self.source(e, and);
            if g.out.get(f.src).is_some_and(|out| out.contains(&f.first)) {
                g.unedge(f.src, f.first);
            }
            and
        };
        for xor in sorted(g, xors.clone()) {
            g.edge(and, xor);
            self.add(and, xor, xor, and);
        }
    }
    fn multiple(
        &mut self,
        g: &mut Graph,
        join: usize,
        injections: &BTreeSet<usize>,
        xors: &BTreeSet<usize>,
    ) {
        let mut injections: Vec<_> = injections.iter().copied().collect();
        if injections.len() > 1 {
            let xor = g.add(Kind::Xor);
            self.gate(xor);
            let mut srcs = BTreeSet::new();
            for &e in &injections {
                srcs.insert(self.flows[e].as_ref().unwrap().last);
                self.target(e, xor);
            }
            let e = self.add(xor, join, join, xor);
            self.flows[e].as_mut().unwrap().back = true;
            injections = vec![e];
            for from in g.ins[join].clone() {
                if srcs.contains(&from) {
                    g.unedge(from, join);
                    g.edge(from, xor);
                }
            }
            g.edge(xor, join);
        }
        let e = injections[0];
        let last = self.flows[e].as_ref().unwrap().last;
        let and = if kind(g, last) == Some(Kind::And) && self.outgoing(last).len() > 1 {
            last
        } else {
            let and = g.add(Kind::And);
            self.gate(and);
            self.target(e, and);
            if g.ins[join].contains(&last) {
                g.unedge(last, join);
                g.edge(last, and);
            }
            let nf = self.add(and, join, join, and);
            self.flows[nf].as_mut().unwrap().back = true;
            g.edge(and, join);
            and
        };
        for x in sorted(g, xors.clone()) {
            g.edge(and, x);
            self.add(and, x, x, and);
        }
    }
}
pub(super) fn replace(g: &mut Graph, hagen: bool) {
    if !g
        .order
        .iter()
        .copied()
        .any(|v| kind(g, v) == Some(Kind::Or) && g.ins[v].len() > 1)
    {
        return;
    }
    let mut map = Map::new();
    if map.build(g) {
        map.replace(g, hagen);
    }
}
