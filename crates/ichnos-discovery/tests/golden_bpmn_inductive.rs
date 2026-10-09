//! Compares `bpmn_inductive` with pm4py's `discover_bpmn_inductive`
//! (`fixtures/golden/discovery/bpmn-inductive-*.json`).
//!
//! pm4py names the BPMN nodes it creates with random UUIDs, so the diagrams
//! are compared up to isomorphism, with the graph code of `ichnos-model`'s
//! `golden_bpmn.rs`. Each diagram must also be the BPMN of the mined tree,
//! and that tree must equal pm4py's up to the order of XOR and parallel
//! children. pm4py's IMf tree can depend on Python's hash seed, so each IMf
//! golden holds one run per distinct tree; ichnos must match one of them.

mod common;

use std::collections::{BTreeMap, BTreeSet};

use common::{canonical, load_csv_log};
use ichnos_core::{EventKeys, EventLog};
use ichnos_discovery::{
    InductiveOptions, InductiveVariant, bpmn_inductive, bpmn_inductive_dfg, process_tree_inductive,
    process_tree_inductive_dfg,
};
use ichnos_golden::{Golden, golden};
use ichnos_model::bpmn::{FlowKind, GatewayDirection};
use ichnos_model::{Bpmn, Dfg, ProcessTree};
use serde_json::Value;

const LOGS: [&str; 6] = [
    "running-example-csv",
    "receipt-csv",
    "roadtraffic100traces-csv",
    "interleavings-receipt_even-csv",
    "interleavings-receipt_odd-csv",
    "reviewing-csv",
];

fn str_field<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key]
        .as_str()
        .unwrap_or_else(|| panic!("{key} is not a string"))
}

/// A directed multigraph with labelled nodes and edges.
#[derive(Debug)]
struct Graph {
    labels: Vec<String>,
    /// `(source, target) -> {edge label: count}`.
    edges: BTreeMap<(usize, usize), BTreeMap<String, usize>>,
}

impl Graph {
    fn edge_count(&self) -> usize {
        self.edges.values().flat_map(|m| m.values()).sum()
    }
}

fn node_label(class: &str, name: &str, direction: Option<&str>) -> String {
    format!("{class}|{name}|{}", direction.unwrap_or("-"))
}

fn canonical_graph(v: &Value) -> Graph {
    let mut index = BTreeMap::new();
    let mut labels = Vec::new();
    for n in v["nodes"].as_array().expect("nodes") {
        index.insert(str_field(n, "key").to_owned(), labels.len());
        labels.push(node_label(
            str_field(n, "class"),
            str_field(n, "name"),
            n["direction"].as_str(),
        ));
    }
    let mut edges: BTreeMap<(usize, usize), BTreeMap<String, usize>> = BTreeMap::new();
    for f in v["flows"].as_array().expect("flows") {
        let s = index[f[0].as_str().expect("source key")];
        let t = index[f[1].as_str().expect("target key")];
        let class = f[2].as_str().expect("flow class").to_owned();
        *edges.entry((s, t)).or_default().entry(class).or_default() += 1;
    }
    Graph { labels, edges }
}

fn flow_class(kind: FlowKind) -> &'static str {
    match kind {
        FlowKind::Sequence => "SequenceFlow",
        FlowKind::Message => "MessageFlow",
        FlowKind::Association => "Association",
    }
}

fn bpmn_graph(b: &Bpmn) -> Graph {
    let mut index = BTreeMap::new();
    let mut labels = Vec::new();
    for (id, n) in b.nodes() {
        index.insert(id, labels.len());
        labels.push(node_label(
            n.kind.class_name(),
            &n.name,
            n.kind.gateway_direction().map(GatewayDirection::as_str),
        ));
    }
    let mut edges: BTreeMap<(usize, usize), BTreeMap<String, usize>> = BTreeMap::new();
    for (_, f) in b.flows() {
        *edges
            .entry((index[&f.source()], index[&f.target()]))
            .or_default()
            .entry(flow_class(f.kind).to_owned())
            .or_default() += 1;
    }
    Graph { labels, edges }
}

/// Colour refinement on both graphs together, so colours are comparable.
fn refine(a: &Graph, b: &Graph) -> (Vec<usize>, Vec<usize>) {
    let mut colors: [Vec<String>; 2] = [a.labels.clone(), b.labels.clone()];
    let mut classes = 0;
    loop {
        let sigs: Vec<Vec<String>> = [a, b]
            .iter()
            .zip(&colors)
            .map(|(g, c)| {
                (0..g.labels.len())
                    .map(|v| {
                        let mut out: Vec<(&String, &String, usize)> = Vec::new();
                        let mut inc: Vec<(&String, &String, usize)> = Vec::new();
                        for ((s, t), m) in &g.edges {
                            for (cls, n) in m {
                                if *s == v {
                                    out.push((cls, &c[*t], *n));
                                }
                                if *t == v {
                                    inc.push((cls, &c[*s], *n));
                                }
                            }
                        }
                        out.sort();
                        inc.sort();
                        format!("{}{out:?}{inc:?}", c[v])
                    })
                    .collect()
            })
            .collect();
        let ranks: BTreeMap<&String, usize> = sigs
            .iter()
            .flatten()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .enumerate()
            .map(|(i, s)| (s, i))
            .collect();
        let new: Vec<Vec<usize>> = sigs
            .iter()
            .map(|s| s.iter().map(|x| ranks[x]).collect())
            .collect();
        if ranks.len() == classes {
            return (new[0].clone(), new[1].clone());
        }
        classes = ranks.len();
        colors = [
            new[0].iter().map(usize::to_string).collect(),
            new[1].iter().map(usize::to_string).collect(),
        ];
    }
}

/// Returns `true` if the graphs are isomorphic, labels and edge
/// multiplicities included.
fn isomorphic(a: &Graph, b: &Graph) -> bool {
    if a.labels.len() != b.labels.len() || a.edge_count() != b.edge_count() {
        return false;
    }
    let (ca, cb) = refine(a, b);
    let histogram = |c: &[usize]| {
        let mut h = BTreeMap::new();
        for &x in c {
            *h.entry(x).or_insert(0) += 1;
        }
        h
    };
    if histogram(&ca) != histogram(&cb) {
        return false;
    }
    let mut order: Vec<usize> = (0..a.labels.len()).collect();
    let size = histogram(&ca);
    order.sort_by_key(|&v| (size[&ca[v]], ca[v]));
    let mut m = Matcher {
        a,
        b,
        ca: &ca,
        cb: &cb,
        order: &order,
        map: vec![usize::MAX; a.labels.len()],
        used: vec![false; b.labels.len()],
    };
    m.extend(0)
}

/// Backtracking search for a colour-preserving bijection from `a` to `b`.
struct Matcher<'a> {
    a: &'a Graph,
    b: &'a Graph,
    ca: &'a [usize],
    cb: &'a [usize],
    order: &'a [usize],
    map: Vec<usize>,
    used: Vec<bool>,
}

impl Matcher<'_> {
    fn edge(g: &Graph, s: usize, t: usize) -> Option<&BTreeMap<String, usize>> {
        g.edges.get(&(s, t))
    }

    /// Maps `order[i..]`, given the mapping of `order[..i]`.
    fn extend(&mut self, i: usize) -> bool {
        let Some(&v) = self.order.get(i) else {
            return true;
        };
        let (a, b) = (self.a, self.b);
        for w in 0..b.labels.len() {
            if self.used[w] || self.cb[w] != self.ca[v] {
                continue;
            }
            let fits = Self::edge(a, v, v) == Self::edge(b, w, w)
                && self.order[..i].iter().all(|&u| {
                    let fu = self.map[u];
                    Self::edge(a, v, u) == Self::edge(b, w, fu)
                        && Self::edge(a, u, v) == Self::edge(b, fu, w)
                });
            if !fits {
                continue;
            }
            self.map[v] = w;
            self.used[w] = true;
            if self.extend(i + 1) {
                return true;
            }
            self.used[w] = false;
        }
        false
    }
}

#[test]
fn isomorphism_check_tells_graphs_apart() {
    let t = |s: &str| ProcessTree::parse(s).unwrap().to_bpmn().unwrap();
    let a = bpmn_graph(&t("->( 'a', X( 'b', 'c' ) )"));
    assert!(isomorphic(&a, &bpmn_graph(&t("->( 'a', X( 'c', 'b' ) )"))));
    assert!(!isomorphic(&a, &bpmn_graph(&t("->( 'a', +( 'b', 'c' ) )"))));
}

/// pm4py's tree in `run` (a JSON object), up to child order.
fn expected_tree(run: &Value) -> String {
    let tree = ProcessTree::parse(str_field(run, "tree")).expect("pm4py tree parses");
    canonical(&tree).to_string()
}

/// Checks `bpmn`, mined along with `tree`, against the golden `run`.
fn check_run(case: &str, run: &Value, tree: &ProcessTree, bpmn: &Bpmn) {
    assert_eq!(
        canonical(tree).to_string(),
        expected_tree(run),
        "{case}: tree"
    );
    assert_eq!(
        *bpmn,
        tree.to_bpmn().expect("no interleaving"),
        "{case}: the diagram is the BPMN of the tree"
    );
    let actual = bpmn_graph(bpmn);
    let expected = canonical_graph(&run["bpmn"]);
    assert!(
        isomorphic(&actual, &expected),
        "{case}: diagrams differ\nichnos: {actual:?}\npm4py: {expected:?}"
    );
}

/// [`check_run`] for a golden with `runs`: `tree` must match one of them.
fn check_any_run(g: &Golden, case: &str, tree: &ProcessTree, bpmn: &Bpmn) {
    let runs = g.expected_at("/runs").as_array().expect("runs");
    let actual = canonical(tree).to_string();
    let run = runs
        .iter()
        .find(|r| expected_tree(r) == actual)
        .unwrap_or_else(|| panic!("{case}: tree {actual} matches no pm4py run"));
    check_run(case, run, tree, bpmn);
}

fn load(g: &Golden) -> EventLog {
    load_csv_log(&g.fixture("log"))
}

#[test]
fn bpmn_im_matches_pm4py() {
    for log_id in LOGS {
        let case = format!("bpmn-inductive-im-{log_id}");
        let g = golden("discovery", &case);
        let (log, keys, options) = (load(&g), EventKeys::default(), InductiveOptions::default());
        let bpmn = bpmn_inductive(&log, &keys, &options).unwrap();
        let tree = process_tree_inductive(&log, &keys, &options).unwrap();
        check_run(&case, g.expected_at(""), &tree, &bpmn);
    }
}

#[test]
fn bpmn_imf_matches_pm4py() {
    for log_id in LOGS {
        let case = format!("bpmn-inductive-imf-{log_id}");
        let g = golden("discovery", &case);
        let (log, keys) = (load(&g), EventKeys::default());
        let options = InductiveOptions::from_noise_threshold(0.2);
        let bpmn = bpmn_inductive(&log, &keys, &options).unwrap();
        let tree = process_tree_inductive(&log, &keys, &options).unwrap();
        check_any_run(&g, &case, &tree, &bpmn);
    }
}

/// pm4py's DFG of the log: edge, start and end frequencies.
fn dfg_of(log: &EventLog) -> Dfg {
    let seqs = log.activity_sequences(&EventKeys::default()).unwrap();
    let name = |a| seqs.activities.name(a);
    let mut dfg = Dfg::new();
    for trace in &seqs.traces {
        for pair in trace.windows(2) {
            dfg.add_edge(name(pair[0]), name(pair[1]), 1);
        }
        if let (Some(&first), Some(&last)) = (trace.first(), trace.last()) {
            dfg.add_start(name(first), 1);
            dfg.add_end(name(last), 1);
        }
    }
    dfg
}

#[test]
fn bpmn_imd_matches_pm4py() {
    for log_id in LOGS {
        let case = format!("bpmn-inductive-imd-{log_id}");
        let g = golden("discovery", &case);
        let dfg = dfg_of(&load(&g));
        let options = InductiveOptions::new(InductiveVariant::Imd);
        let bpmn = bpmn_inductive_dfg(&dfg, &options);
        let tree = process_tree_inductive_dfg(&dfg, &options);
        check_run(&case, g.expected_at(""), &tree, &bpmn);
    }
}
