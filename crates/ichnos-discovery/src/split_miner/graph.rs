use super::{Edge, pair, rpst};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(super) enum Kind {
    Task,
    Start,
    End,
    Xor,
    And,
    Or,
}
impl Kind {
    pub(super) fn gate(self) -> bool {
        matches!(self, Self::Xor | Self::And | Self::Or)
    }
    pub(super) fn text(self) -> &'static str {
        match self {
            Self::Xor => "xor",
            Self::And => "and",
            Self::Or => "or",
            Self::Task => "task",
            Self::Start => "start",
            Self::End => "end",
        }
    }
}
#[derive(Clone)]
pub(super) struct Node {
    pub key: String,
    pub kind: Kind,
}
pub(super) struct Graph {
    pub nodes: Vec<Option<Node>>,
    pub order: Vec<usize>,
    pub ins: Vec<Vec<usize>>,
    pub out: Vec<Vec<usize>>,
    pub concurrent: BTreeSet<Edge>,
    pub start: usize,
    pub end: usize,
    counter: usize,
}
#[derive(Clone)]
pub(super) struct Item {
    past: BTreeSet<usize>,
    future: BTreeSet<usize>,
    children: Vec<Item>,
    kind: Kind,
}
type Signature = (Vec<usize>, Vec<usize>);
impl Item {
    fn sig(&self) -> Signature {
        (
            self.past.iter().copied().collect(),
            self.future.iter().copied().collect(),
        )
    }
    fn union(&self) -> BTreeSet<usize> {
        self.past.union(&self.future).copied().collect()
    }
    fn merged(children: Vec<Self>, kind: Kind, forced: bool) -> Self {
        let past = children
            .iter()
            .flat_map(|c| c.past.iter().copied())
            .collect();
        let future = if kind == Kind::Xor {
            children[0].future.clone()
        } else if forced {
            children
                .iter()
                .flat_map(|c| c.future.iter().copied())
                .filter(|v| !BTreeSet::<usize>::contains(&past, v))
                .collect()
        } else {
            children
                .iter()
                .skip(1)
                .fold(children[0].future.clone(), |a, c| {
                    a.intersection(&c.future).copied().collect()
                })
        };
        Self {
            past,
            future,
            children,
            kind,
        }
    }
}
impl Graph {
    pub fn new(labels: Vec<(String, Kind)>) -> Self {
        let start = labels.iter().position(|x| x.1 == Kind::Start).unwrap();
        let end = labels.iter().position(|x| x.1 == Kind::End).unwrap();
        let n = labels.len();
        Self {
            nodes: labels
                .into_iter()
                .map(|(key, kind)| Some(Node { key, kind }))
                .collect(),
            order: (0..n).rev().collect(),
            ins: vec![Vec::new(); n],
            out: vec![Vec::new(); n],
            concurrent: BTreeSet::new(),
            start,
            end,
            counter: 0,
        }
    }
    pub fn kind(&self, id: usize) -> Kind {
        self.nodes[id].as_ref().unwrap().kind
    }
    pub fn keys(&self, mut ids: Vec<usize>) -> Vec<usize> {
        ids.sort_by(|&a, &b| {
            self.nodes[a]
                .as_ref()
                .unwrap()
                .key
                .cmp(&self.nodes[b].as_ref().unwrap().key)
        });
        ids
    }
    pub fn add(&mut self, kind: Kind) -> usize {
        self.counter += 1;
        let i = self.nodes.len();
        self.nodes.push(Some(Node {
            key: format!("{}_{}", kind.text(), self.counter),
            kind,
        }));
        self.ins.push(Vec::new());
        self.out.push(Vec::new());
        self.order.push(i);
        i
    }
    pub fn edge(&mut self, a: usize, b: usize) {
        if !self.out[a].contains(&b) {
            self.out[a].push(b);
            self.ins[b].push(a);
        }
    }
    pub fn unedge(&mut self, a: usize, b: usize) {
        self.out[a].retain(|&v| v != b);
        self.ins[b].retain(|&v| v != a);
    }
    pub fn remove(&mut self, v: usize) {
        for a in self.ins[v].clone() {
            self.unedge(a, v);
        }
        for b in self.out[v].clone() {
            self.unedge(v, b);
        }
        self.nodes[v] = None;
    }
    pub fn trivial(&mut self) {
        while let Some(v) = self.order.iter().copied().find(|&v| {
            self.nodes[v].is_some()
                && self.kind(v).gate()
                && self.ins[v].len() == 1
                && self.out[v].len() == 1
        }) {
            let a = self.ins[v][0];
            let b = self.out[v][0];
            self.remove(v);
            if a != b {
                self.edge(a, b);
            }
        }
    }
    pub fn splits(&mut self) {
        let mut shared = BTreeMap::new();
        let mut queue = VecDeque::from([self.start]);
        let mut queued = BTreeSet::from([self.start]);
        while let Some(entry) = queue.pop_front() {
            if entry == self.end {
                continue;
            }
            let succs = self.out[entry].clone();
            if succs.len() > 1 {
                let mut items: Vec<_> = self
                    .keys(succs.clone())
                    .into_iter()
                    .map(|s| Item {
                        past: BTreeSet::from([s]),
                        future: succs
                            .iter()
                            .copied()
                            .filter(|&b| b != s && self.concurrent.contains(&pair(s, b)))
                            .collect(),
                        children: Vec::new(),
                        kind: Kind::Task,
                    })
                    .collect();
                while items.len() > 1 {
                    let mut merged = false;
                    loop {
                        if !reduce_group(&mut items, Kind::Xor) {
                            break;
                        }
                        merged = true;
                    }
                    if reduce_group(&mut items, Kind::And) {
                        merged = true;
                    }
                    if merged {
                        continue;
                    }
                    let mut ordered: Vec<_> = (0..items.len()).collect();
                    ordered.sort_by_key(|&i| items[i].sig());
                    let mut best = None;
                    for (i, &a) in ordered.iter().enumerate() {
                        for &b in &ordered[i + 1..] {
                            let d = items[a]
                                .union()
                                .symmetric_difference(&items[b].union())
                                .count();
                            if best.is_none_or(|(_, _, old)| d < old) {
                                best = Some((a, b, d));
                            }
                        }
                    }
                    let (a, b, _) = best.unwrap();
                    let child = vec![items[a].clone(), items[b].clone()];
                    let mut i = 0;
                    items.retain(|_| {
                        let keep = i != a && i != b;
                        i += 1;
                        keep
                    });
                    items.push(Item::merged(child, Kind::And, true));
                }
                for &s in &succs {
                    self.unedge(entry, s);
                }
                self.render(entry, items.pop().unwrap(), &mut shared);
            }
            for s in succs {
                if queued.insert(s) {
                    queue.push_back(s);
                }
            }
        }
    }
    fn render(&mut self, entry: usize, mut item: Item, shared: &mut BTreeMap<Signature, usize>) {
        let sig = item.sig();
        if let Some(&g) = shared.get(&sig) {
            self.edge(entry, g);
            return;
        }
        if item.children.is_empty() {
            if let Some(&a) = item.past.first() {
                self.edge(entry, a);
            }
            return;
        }
        let gate = self.add(item.kind);
        self.edge(entry, gate);
        item.children.sort_by_key(Item::sig);
        for c in item.children {
            self.render(gate, c, shared);
        }
        shared.insert(sig, gate);
    }
    pub fn joins(&mut self) {
        for _ in 0..self.nodes.len() + 5 {
            let edges: Vec<_> = self
                .order
                .iter()
                .flat_map(|&a| self.out[a].iter().map(move |&b| (a, b)))
                .collect();
            let Some(fragments) = rpst::fragments(&edges, self.nodes.len()) else {
                break;
            };
            let mut changed = BTreeSet::new();
            for f in fragments {
                if f.kind != 'B' && f.kind != 'R' {
                    continue;
                }
                let (entry, exit) = (f.entry, f.exit);
                let (merge, matching, is_loop) = if !self.kind(exit).gate() {
                    (exit, entry, false)
                } else if !self.kind(entry).gate() {
                    (entry, exit, true)
                } else {
                    continue;
                };
                if changed.contains(&merge) {
                    continue;
                }
                let kind = if f.kind == 'R' {
                    Kind::Or
                } else {
                    let k = self.kind(matching);
                    if !k.gate() {
                        continue;
                    }
                    k
                };
                let preds: BTreeSet<_> = f
                    .edges
                    .iter()
                    .filter(|e| e.1 == merge)
                    .map(|e| e.0)
                    .collect();
                let incoming = self.ins[merge].clone();
                let gate = self.add(kind);
                self.edge(gate, merge);
                for p in incoming {
                    if is_loop || preds.contains(&p) {
                        self.unedge(p, merge);
                        self.edge(p, gate);
                    }
                }
                changed.insert(merge);
            }
            if changed.is_empty() {
                break;
            }
        }
        for v in self.order.clone() {
            if self.nodes[v].is_none() || self.kind(v).gate() || self.ins[v].len() <= 1 {
                continue;
            }
            let gate = self.add(Kind::Or);
            for p in self.ins[v].clone() {
                self.unedge(p, v);
                self.edge(p, gate);
            }
            self.edge(gate, v);
        }
    }
}
fn reduce_group(items: &mut Vec<Item>, kind: Kind) -> bool {
    let mut ordered: Vec<_> = (0..items.len()).collect();
    ordered.sort_by_key(|&i| items[i].sig());
    for a in ordered {
        let mut group: Vec<_> = (0..items.len())
            .filter(|&b| {
                b != a
                    && if kind == Kind::Xor {
                        items[a].future == items[b].future
                    } else {
                        items[a].union() == items[b].union()
                    }
            })
            .collect();
        if group.is_empty() {
            continue;
        }
        group.push(a);
        let children = group.iter().map(|&i| items[i].clone()).collect();
        let mut i = 0;
        items.retain(|_| {
            let keep = !group.contains(&i);
            i += 1;
            keep
        });
        items.push(Item::merged(children, kind, false));
        return true;
    }
    false
}
