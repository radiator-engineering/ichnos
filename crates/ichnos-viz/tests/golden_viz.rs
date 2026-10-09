//! DOT output against pm4py's `save_vis_*` (`fixtures/golden/viz`).
//!
//! Each golden holds the model pm4py drew and the graph its DOT text
//! describes, with node defaults resolved. Node names differ (pm4py uses
//! object ids), so graphs are compared up to isomorphism: nodes by their
//! attributes, edges by theirs. See `tools/golden/cases/viz.py`.
//!
//! Where pm4py's statement order does not depend on Python's set order, the
//! golden also records it, and the test checks that ichnos writes the
//! nodes and edges in the same order. Graphviz lays out in that order.

use std::collections::{BTreeMap, BTreeSet};

use ichnos_discovery::PerformanceDfg;
use ichnos_discovery::dfg::{Aggregation, BusinessHours, PerformanceSummary};
use ichnos_discovery::prefix_tree::{PrefixNode, PrefixTree};
use ichnos_golden::{Golden, cases, golden};
use ichnos_model::heuristics_net::{HeuristicsEdge, HeuristicsNet, Matrix};
use ichnos_model::petri::{ArcEnds, ArcKind};
use ichnos_model::powl::{FrequentTransition, StrictPartialOrder};
use ichnos_model::{
    Dfg, Footprints, Label, Marking, PetriNet, PlaceId, Powl, ProcessTree, TransitionSystem,
};
use ichnos_ocel::{Ocel, OcelObject};
use ichnos_viz::{
    AlignmentStep, AlignmentsDotOptions, BpmnDotOptions, Decoration, DfgDotOptions,
    DottedChartAttributes, DottedChartDotOptions, FootprintsDotOptions, HeuristicsNetDotOptions,
    NetworkAnalysisDotOptions, NetworkAnalysisEdge, ObjectGraphDotOptions, ObjectTypeNet,
    OcPetriNet, Ocdfg, OcdfgActivityMetric, OcdfgAnnotation, OcdfgCounts, OcdfgDotOptions,
    OcdfgEdge, OcdfgEdgeMetric, OcpnDiagnostics, OcpnDotOptions, PerformanceDfgDotOptions,
    PerformanceSpectrumDotOptions, PerformanceSpectrumOptions, PetriNetDotOptions,
    PlaceDiagnostics, PowlDotOptions, PrefixTreeDotOptions, ProcessTreeDotOptions,
    TransitionSystemDotOptions, VariantAlignment, VizError,
};
use serde_json::Value;

type Attrs = BTreeMap<String, String>;

/// The graph a DOT text describes, as `tools/golden/cases/viz.py` reads it.
#[derive(Debug, Default)]
struct Parsed {
    directed: bool,
    strict: bool,
    graph: Attrs,
    /// Nodes in declaration order.
    nodes: Vec<(String, Attrs)>,
    edges: Vec<(String, String, Attrs)>,
}

const PUNCT: &str = "[]{}=;,";

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Op(String),
    Id(String),
}

fn tokens(src: &str) -> Vec<Token> {
    let c: Vec<char> = src.chars().collect();
    let starts_op =
        |i: usize| i + 1 < c.len() && c[i] == '-' && (c[i + 1] == '>' || c[i + 1] == '-');
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        if c[i].is_whitespace() {
            i += 1;
        } else if starts_op(i) {
            out.push(Token::Op(c[i..i + 2].iter().collect()));
            i += 2;
        } else if PUNCT.contains(c[i]) {
            out.push(Token::Op(c[i].to_string()));
            i += 1;
        } else if c[i] == '"' {
            let mut j = i + 1;
            let mut buf = String::new();
            while c[j] != '"' {
                if c[j] == '\\' && c[j + 1] == '"' {
                    buf.push('"');
                    j += 2;
                } else {
                    buf.push(c[j]);
                    j += 1;
                }
            }
            out.push(Token::Id(buf));
            i = j + 1;
        } else if c[i] == '<' {
            let (mut depth, mut j) = (0, i);
            loop {
                match c[j] {
                    '<' => depth += 1,
                    '>' => depth -= 1,
                    _ => {}
                }
                j += 1;
                if depth == 0 {
                    break;
                }
            }
            out.push(Token::Id(c[i..j].iter().collect()));
            i = j;
        } else {
            let mut j = i;
            while j < c.len()
                && !c[j].is_whitespace()
                && !PUNCT.contains(c[j])
                && c[j] != '"'
                && c[j] != '<'
                && !starts_op(j)
            {
                j += 1;
            }
            out.push(Token::Id(c[i..j].iter().collect()));
            i = j;
        }
    }
    out
}

fn op(s: &str) -> Token {
    Token::Op(s.to_owned())
}

fn id(t: &Token) -> &str {
    match t {
        Token::Op(s) | Token::Id(s) => s,
    }
}

/// `tools/golden/cases/viz.py`'s `json.dumps(attrs, sort_keys=True)`, up
/// to formatting: both sides go through this.
fn attrs_key(a: &Attrs) -> String {
    serde_json::to_string(a).expect("attrs serialize")
}

fn parse_dot(src: &str) -> Parsed {
    let toks = tokens(src);
    let mut pos = 0;
    let peek = |pos: usize| toks.get(pos);
    let attr_list = |pos: &mut usize| {
        let mut attrs = Attrs::new();
        while toks.get(*pos) == Some(&op("[")) {
            *pos += 1;
            while toks[*pos] != op("]") {
                if toks[*pos] == op(",") || toks[*pos] == op(";") {
                    *pos += 1;
                    continue;
                }
                let k = id(&toks[*pos]).to_owned();
                assert_eq!(
                    toks[*pos + 1],
                    op("="),
                    "attribute without a value in {src}"
                );
                attrs.insert(k, id(&toks[*pos + 2]).to_owned());
                *pos += 3;
            }
            *pos += 1;
        }
        attrs
    };
    let mut p = Parsed::default();
    if peek(pos) == Some(&Token::Id("strict".to_owned())) {
        p.strict = true;
        pos += 1;
    }
    p.directed = id(&toks[pos]) == "digraph";
    pos += 1;
    if peek(pos) != Some(&op("{")) {
        pos += 1;
    }
    assert_eq!(toks[pos], op("{"));
    pos += 1;
    // One frame per open graph or subgraph: (graph attrs, node defaults).
    let mut frames: Vec<(Attrs, Attrs)> = vec![(Attrs::new(), Attrs::new())];
    // Nesting depth of each subgraph, by name.
    let mut depth: BTreeMap<String, usize> = BTreeMap::new();
    while !frames.is_empty() {
        let t = toks[pos].clone();
        pos += 1;
        if t == op("}") {
            let (attrs, _) = frames.pop().expect("a frame");
            if frames.is_empty() {
                p.graph = attrs;
            }
            continue;
        }
        if t == op(";") {
            continue;
        }
        if t == Token::Id("subgraph".to_owned()) {
            if peek(pos) != Some(&op("{")) {
                depth.insert(id(&toks[pos]).to_owned(), frames.len());
                pos += 1;
            }
            assert_eq!(toks[pos], op("{"));
            pos += 1;
            let defaults = frames.last().expect("a frame").1.clone();
            frames.push((Attrs::new(), defaults));
            continue;
        }
        let name = id(&t).to_owned();
        if ["graph", "node", "edge"].contains(&name.as_str()) {
            // `attr("node")` writes the keyword alone, which sets nothing.
            let attrs = attr_list(&mut pos);
            let frame = frames.last_mut().expect("a frame");
            match name.as_str() {
                "graph" => frame.0.extend(attrs),
                "node" => frame.1.extend(attrs),
                _ => {}
            }
            continue;
        }
        if peek(pos) == Some(&op("=")) {
            let value = id(&toks[pos + 1]).to_owned();
            pos += 2;
            frames.last_mut().expect("a frame").0.insert(name, value);
            continue;
        }
        if matches!(peek(pos), Some(Token::Op(o)) if o == "->" || o == "--") {
            let target = id(&toks[pos + 1]).to_owned();
            pos += 2;
            let mut attrs = attr_list(&mut pos);
            for k in ["lhead", "ltail"] {
                if let Some(v) = attrs.get_mut(k) {
                    *v = format!("cluster@{}", depth[v.as_str()]);
                }
            }
            p.edges.push((name, target, attrs));
            continue;
        }
        let (gattrs, defaults) = frames.last().expect("a frame");
        let mut attrs = defaults.clone();
        let subgraph = (frames.len() > 1).then(|| attrs_key(gattrs));
        attrs.extend(attr_list(&mut pos));
        if let Some(image) = attrs.get_mut("image") {
            *image = std::path::Path::new(image.as_str())
                .file_name()
                .and_then(|n| n.to_str())
                .expect("image file name")
                .to_owned();
        }
        if let Some(s) = subgraph {
            attrs.insert("subgraph".to_owned(), s);
        }
        if frames.len() > 2 {
            attrs.insert("subgraph_depth".to_owned(), (frames.len() - 1).to_string());
        }
        p.nodes.push((name, attrs));
    }
    p
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

    fn new<'a>(
        nodes: impl IntoIterator<Item = (&'a str, Attrs)>,
        edges: impl IntoIterator<Item = (&'a str, &'a str, Attrs)>,
    ) -> Graph {
        let mut index = BTreeMap::new();
        let mut labels = Vec::new();
        for (name, attrs) in nodes {
            index.insert(name, labels.len());
            labels.push(attrs_key(&attrs));
        }
        let mut out: BTreeMap<(usize, usize), BTreeMap<String, usize>> = BTreeMap::new();
        for (s, t, attrs) in edges {
            let (s, t) = (index[s], index[t]);
            *out.entry((s, t))
                .or_default()
                .entry(attrs_key(&attrs))
                .or_default() += 1;
        }
        Graph { labels, edges: out }
    }
}

fn attrs_of(v: &Value) -> Attrs {
    v.as_object()
        .expect("attributes")
        .iter()
        .map(|(k, v)| {
            let v = v.as_str().expect("attribute value");
            // Subgraph attributes are a JSON object in a string; reformat
            // them as `attrs_key` writes them.
            let v = if k == "subgraph" {
                attrs_key(&attrs_of(&serde_json::from_str(v).expect("subgraph JSON")))
            } else {
                v.to_owned()
            };
            (k.clone(), v)
        })
        .collect()
}

fn expected_graph(dot: &Value) -> Graph {
    let nodes = dot["nodes"].as_array().expect("nodes");
    let edges = dot["edges"].as_array().expect("edges");
    Graph::new(
        nodes
            .iter()
            .map(|n| (n["key"].as_str().expect("key"), attrs_of(&n["attrs"]))),
        edges.iter().map(|e| {
            (
                e[0].as_str().expect("source"),
                e[1].as_str().expect("target"),
                attrs_of(&e[2]),
            )
        }),
    )
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

/// Fails with the differing nodes and edges when the DOT text does not
/// describe the golden's graph.
fn assert_matches(g: &Golden, dot: &str) {
    let expected = g.expected_at("/dot");
    let p = parse_dot(dot);
    assert_eq!(
        p.directed,
        expected["directed"].as_bool().expect("directed"),
        "{}: directed",
        g.case
    );
    assert_eq!(
        p.strict,
        expected["strict"].as_bool().expect("strict"),
        "{}: strict",
        g.case
    );
    assert_eq!(
        p.graph,
        attrs_of(&expected["graph"]),
        "{}: graph attributes",
        g.case
    );
    let declared: BTreeSet<&str> = p.nodes.iter().map(|(n, _)| n.as_str()).collect();
    for (s, t, _) in &p.edges {
        assert!(
            declared.contains(s.as_str()) && declared.contains(t.as_str()),
            "{}: edge {s} -> {t} names an undeclared node",
            g.case
        );
    }
    let actual = Graph::new(
        p.nodes.iter().map(|(n, a)| (n.as_str(), a.clone())),
        p.edges
            .iter()
            .map(|(s, t, a)| (s.as_str(), t.as_str(), a.clone())),
    );
    let want = expected_graph(expected);
    if isomorphic(&actual, &want) {
        assert_order(g, expected, &p);
        return;
    }
    let bag = |labels: &[String]| {
        let mut m: BTreeMap<String, i64> = BTreeMap::new();
        for l in labels {
            *m.entry(l.clone()).or_default() += 1;
        }
        m
    };
    let (have, need) = (bag(&actual.labels), bag(&want.labels));
    let diff: Vec<String> = have
        .keys()
        .chain(need.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|k| have.get(*k) != need.get(*k))
        .map(|k| format!("  {k}: ichnos {:?}, pm4py {:?}", have.get(k), need.get(k)))
        .collect();
    let edge_bag = |g: &Graph| {
        let mut m: BTreeMap<(String, String, String), usize> = BTreeMap::new();
        for ((s, t), labels) in &g.edges {
            for (l, n) in labels {
                *m.entry((g.labels[*s].clone(), g.labels[*t].clone(), l.clone()))
                    .or_default() += n;
            }
        }
        m
    };
    let (he, ne) = (edge_bag(&actual), edge_bag(&want));
    let ediff: Vec<String> = he
        .keys()
        .chain(ne.keys())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter(|k| he.get(*k) != ne.get(*k))
        .take(20)
        .map(|k| format!("  {k:?}: ichnos {:?}, pm4py {:?}", he.get(k), ne.get(k)))
        .collect();
    panic!(
        "{}: the DOT graph differs from pm4py's\nnodes:\n{}\nedges:\n{}\nichnos DOT:\n{dot}",
        g.case,
        diff.join("\n"),
        ediff.join("\n")
    );
}

/// Checks the order of the nodes and edges whose labels are unique against
/// the order the golden records, if it records one.
fn assert_order(g: &Golden, expected: &Value, p: &Parsed) {
    let Some(order) = expected.get("order") else {
        return;
    };
    let label: BTreeMap<&str, String> = expected["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .map(|n| (str_field(n, "key"), attrs_key(&attrs_of(&n["attrs"]))))
        .collect();
    let golden_edges = expected["edges"].as_array().expect("edges");
    let want_nodes: Vec<String> = order["nodes"]
        .as_array()
        .expect("node order")
        .iter()
        .map(|k| label[k.as_str().expect("key")].clone())
        .collect();
    let want_edges: Vec<(String, String, String)> = order["edges"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|i| {
            let e = &golden_edges[usize::try_from(as_u64(i)).expect("index fits")];
            (
                label[e[0].as_str().expect("source")].clone(),
                label[e[1].as_str().expect("target")].clone(),
                attrs_key(&attrs_of(&e[2])),
            )
        })
        .collect();
    let node_label: BTreeMap<&str, String> = p
        .nodes
        .iter()
        .map(|(n, a)| (n.as_str(), attrs_key(a)))
        .collect();
    let have_nodes: Vec<String> = p.nodes.iter().map(|(_, a)| attrs_key(a)).collect();
    let have_edges: Vec<(String, String, String)> = p
        .edges
        .iter()
        .map(|(s, t, a)| {
            (
                node_label[s.as_str()].clone(),
                node_label[t.as_str()].clone(),
                attrs_key(a),
            )
        })
        .collect();
    fn unique<T: Ord + Clone>(want: &[T], have: &[T]) -> (Vec<T>, Vec<T>) {
        let mut count: BTreeMap<&T, usize> = BTreeMap::new();
        for x in want {
            *count.entry(x).or_default() += 1;
        }
        let keep = |v: &[T]| {
            v.iter()
                .filter(|x| count.get(x) == Some(&1))
                .cloned()
                .collect()
        };
        (keep(want), keep(have))
    }
    // Shows a node by its label attribute alone, to keep failures short.
    let short = |key: &str| -> String {
        serde_json::from_str::<Attrs>(key)
            .ok()
            .and_then(|a| a.get("label").cloned())
            .unwrap_or_else(|| key.to_owned())
    };
    let (want, have) = unique(&want_nodes, &have_nodes);
    assert!(!want.is_empty(), "{}: no node has a unique label", g.case);
    if have != want {
        let show = |v: &[String]| v.iter().map(|k| short(k)).collect::<Vec<_>>();
        panic!(
            "{}: node order\n ichnos {:?}\n pm4py  {:?}",
            g.case,
            show(&have),
            show(&want)
        );
    }
    if order.get("edges").is_none() {
        return;
    }
    let (want, have) = unique(&want_edges, &have_edges);
    if have != want {
        let show = |v: &[(String, String, String)]| {
            v.iter()
                .map(|(s, t, e)| format!("{} -> {} [{}]", short(s), short(t), short(e)))
                .collect::<Vec<_>>()
        };
        let first = have.iter().zip(&want).take_while(|(a, b)| a == b).count();
        panic!(
            "{}: edge order differs from position {first}\n ichnos {:#?}\n pm4py  {:#?}",
            g.case,
            show(&have[first..have.len().min(first + 6)]),
            show(&want[first..want.len().min(first + 6)])
        );
    }
}

fn str_field<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key]
        .as_str()
        .unwrap_or_else(|| panic!("`{key}` is not a string in {v}"))
}

fn as_u64(v: &Value) -> u64 {
    v.as_u64().unwrap_or_else(|| panic!("{v} is not a count"))
}

fn as_f64(v: &Value) -> f64 {
    match v {
        Value::String(s) if s == "NaN" => f64::NAN,
        _ => v.as_f64().unwrap_or_else(|| panic!("{v} is not a number")),
    }
}

fn counts(v: &Value) -> BTreeMap<Label, u64> {
    v.as_object()
        .expect("object of counts")
        .iter()
        .map(|(k, n)| (Label::from(k.as_str()), as_u64(n)))
        .collect()
}

fn params(g: &Golden) -> Value {
    g.meta["params"].clone()
}

fn build_net(model: &Value) -> (PetriNet, Marking, Marking) {
    let mut net = PetriNet::new("golden");
    let mut places = BTreeMap::new();
    for p in model["places"].as_array().expect("places") {
        let name = p.as_str().expect("place name");
        places.insert(name.to_owned(), net.add_place(name));
    }
    let mut transitions = BTreeMap::new();
    for t in model["transitions"].as_array().expect("transitions") {
        let name = str_field(t, "name");
        let label = t["label"].as_str().map(Label::from);
        transitions.insert(name.to_owned(), net.add_transition(name, label));
    }
    for a in model["arcs"].as_array().expect("arcs") {
        let (source, target) = (str_field(a, "source"), str_field(a, "target"));
        let ends = match (places.get(source), transitions.get(target)) {
            (Some(&p), Some(&t)) => ArcEnds::PlaceToTransition(p, t),
            _ => ArcEnds::TransitionToPlace(transitions[source], places[target]),
        };
        let kind = match str_field(a, "type") {
            "normal" => ArcKind::Normal,
            "inhibitor" => ArcKind::Inhibitor,
            "reset" => ArcKind::Reset,
            other => panic!("unknown arc type {other}"),
        };
        let weight = u32::try_from(as_u64(&a["weight"])).expect("weight fits");
        net.add_arc(ends, weight, kind).expect("valid arc");
    }
    let marking = |v: &Value, places: &BTreeMap<String, PlaceId>| -> Marking {
        v.as_object()
            .expect("a marking")
            .iter()
            .map(|(p, n)| (places[p], u32::try_from(as_u64(n)).expect("count fits")))
            .collect()
    };
    let initial = marking(&model["initial_marking"], &places);
    let fin = marking(&model["final_marking"], &places);
    (net, initial, fin)
}

fn decoration(d: &Value) -> Decoration {
    let field = |k: &str| d[k].as_str().map(str::to_owned);
    Decoration {
        label: field("label"),
        color: field("color"),
        penwidth: field("penwidth"),
    }
}

#[test]
fn guards_come_from_the_pnml_reader() {
    let g = golden("viz", "petri-net-data_petri_net");
    let doc = ichnos_io::read_pnml(g.fixture("model"), &ichnos_io::PnmlReadOptions::default())
        .expect("PNML reads");
    let net = &doc.model.net;
    let guards: BTreeMap<String, String> = doc
        .transition_data
        .iter()
        .filter_map(|(&t, d)| Some((net.transition(t).name.clone(), d.guard.clone()?)))
        .collect();
    let want: BTreeMap<String, String> = g
        .expected_at("/model/guards")
        .as_object()
        .expect("guards")
        .iter()
        .map(|(k, v)| (k.clone(), v.as_str().expect("guard").to_owned()))
        .collect();
    assert_eq!(guards, want);
}

#[test]
fn petri_nets_match_pm4py() {
    for case in cases("viz").iter().filter(|c| c.starts_with("petri-net-")) {
        let g = golden("viz", case);
        let (net, initial, fin) = build_net(g.expected_at("/model"));
        let p = params(&g);
        let mut options = PetriNetDotOptions::default();
        if let Some(v) = p["bgcolor"].as_str() {
            options.bgcolor = v.to_owned();
        }
        if let Some(v) = p["rankdir"].as_str() {
            options.rankdir = v.to_owned();
        }
        options.graph_title = p["graph_title"].as_str().map(str::to_owned);
        if let Some(n) = p["font_size"].as_u64() {
            options.font_size = u32::try_from(n).expect("fits");
        }
        options.debug = p["debug"].as_bool().unwrap_or(false);
        let transition = |name: &str| {
            net.transition_ids()
                .find(|&t| net.transition(t).name == name)
                .unwrap_or_else(|| panic!("no transition {name}"))
        };
        let place = |name: &str| {
            net.place_ids()
                .find(|&p| net.place(p).name == name)
                .unwrap_or_else(|| panic!("no place {name}"))
        };
        let deco = &p["decorations"];
        for (name, d) in deco["places"].as_object().into_iter().flatten() {
            options
                .decorations
                .places
                .insert(place(name), decoration(d));
        }
        for (name, d) in deco["transitions"].as_object().into_iter().flatten() {
            options
                .decorations
                .transitions
                .insert(transition(name), decoration(d));
        }
        for a in deco["arcs"].as_array().into_iter().flatten() {
            let (source, target) = (
                a[0].as_str().expect("source"),
                a[1].as_str().expect("target"),
            );
            let arc = net
                .arc_ids()
                .find(|&id| match net.arc(id).ends {
                    ArcEnds::PlaceToTransition(p, t) => {
                        net.place(p).name == source && net.transition(t).name == target
                    }
                    ArcEnds::TransitionToPlace(t, p) => {
                        net.transition(t).name == source && net.place(p).name == target
                    }
                })
                .expect("decorated arc");
            options.decorations.arcs.insert(arc, decoration(&a[2]));
        }
        let model = g.expected_at("/model");
        for (name, guard) in model["guards"].as_object().into_iter().flatten() {
            options
                .guards
                .insert(transition(name), guard.as_str().expect("guard").to_owned());
        }
        assert_matches(
            &g,
            &ichnos_viz::petri_net_dot(&net, &initial, &fin, &options),
        );
    }
}

#[test]
fn dfgs_match_pm4py() {
    for case in cases("viz").iter().filter(|c| c.starts_with("dfg-")) {
        let g = golden("viz", case);
        let model = g.expected_at("/model");
        let mut dfg = Dfg::new();
        for e in model["graph"].as_array().expect("edges") {
            let key = (
                Label::from(e[0].as_str().expect("activity")),
                Label::from(e[1].as_str().expect("activity")),
            );
            dfg.graph.insert(key, as_u64(&e[2]));
        }
        dfg.start_activities = counts(&model["start"]);
        dfg.end_activities = counts(&model["end"]);
        let mut options = DfgDotOptions::default();
        let p = params(&g);
        if let Some(n) = p["max_num_edges"].as_u64() {
            options.max_num_edges = usize::try_from(n).expect("fits");
        }
        if let Some(n) = p["font_size"].as_u64() {
            options.font_size = u32::try_from(n).expect("fits");
        }
        if let Some(c) = model.get("activities_count") {
            options.activities_count = Some(counts(c));
        }
        assert_matches(&g, &ichnos_viz::dfg_dot(&dfg, &options));
    }
}

#[test]
fn performance_dfgs_match_pm4py() {
    for case in cases("viz")
        .iter()
        .filter(|c| c.starts_with("performance-dfg-"))
    {
        let g = golden("viz", case);
        let model = g.expected_at("/model");
        let mut dfg = PerformanceDfg::default();
        for e in model["graph"].as_array().expect("edges") {
            let key = (
                Label::from(e[0].as_str().expect("activity")),
                Label::from(e[1].as_str().expect("activity")),
            );
            let m = &e[2];
            dfg.graph.insert(
                key,
                PerformanceSummary {
                    count: 0,
                    mean: as_f64(&m["mean"]),
                    median: as_f64(&m["median"]),
                    min: as_f64(&m["min"]),
                    max: as_f64(&m["max"]),
                    sum: as_f64(&m["sum"]),
                    stdev: as_f64(&m["stdev"]),
                    raw_values: None,
                },
            );
        }
        dfg.start_activities = counts(&model["start"]);
        dfg.end_activities = counts(&model["end"]);
        let p = params(&g);
        let mut options = PerformanceDfgDotOptions::default();
        if let Some(m) = p["aggregation_measure"].as_str() {
            options.aggregation = match m {
                "mean" => Aggregation::Mean,
                "median" => Aggregation::Median,
                "min" => Aggregation::Min,
                "max" => Aggregation::Max,
                "sum" => Aggregation::Sum,
                "stdev" => Aggregation::StandardDeviation,
                other => panic!("unknown measure {other}"),
            };
        }
        if p["business_hours"].as_bool() == Some(true) {
            options.business_hours = Some(BusinessHours::default());
        }
        if let Some(times) = model.get("serv_time") {
            options.serv_time = Some(
                times
                    .as_object()
                    .expect("service times")
                    .iter()
                    .map(|(a, v)| (Label::from(a.as_str()), as_f64(v)))
                    .collect(),
            );
        }
        assert_matches(&g, &ichnos_viz::performance_dfg_dot(&dfg, &options));
    }
}

#[test]
fn process_trees_match_pm4py() {
    for case in cases("viz")
        .iter()
        .filter(|c| c.starts_with("process-tree-"))
    {
        let g = golden("viz", case);
        let tree = ProcessTree::parse(str_field(g.expected_at("/model"), "tree")).expect("tree");
        let p = params(&g);
        let mut options = ProcessTreeDotOptions::default();
        if let Some(sort) = p["enable_deepcopy"].as_bool() {
            options.sort = sort;
        }
        if let Some(n) = p["font_size"].as_u64() {
            options.font_size = u32::try_from(n).expect("fits");
        }
        let dot = ichnos_viz::process_tree_dot(&tree, &options);
        assert_matches(&g, &dot);
    }
}

#[test]
fn bpmn_diagrams_match_pm4py() {
    for case in cases("viz").iter().filter(|c| c.starts_with("bpmn-")) {
        let g = golden("viz", case);
        let doc = ichnos_io::read_bpmn(g.fixture("model"), &ichnos_io::BpmnReadOptions::default())
            .expect("BPMN reads");
        let p = params(&g);
        let mut options = BpmnDotOptions::default();
        if let Some(v) = p["enable_swimlanes"].as_bool() {
            options.enable_swimlanes = v;
        }
        if let Some(v) = p["include_name_in_events"].as_bool() {
            options.include_name_in_events = v;
        }
        if let Some(v) = p["endpoints_shape"].as_str() {
            v.clone_into(&mut options.endpoints_shape);
        }
        if let Some(n) = p["swimlanes_margin"].as_u64() {
            options.swimlanes_margin = u32::try_from(n).expect("fits");
        }
        if let Some(n) = p["font_size"].as_u64() {
            options.font_size = u32::try_from(n).expect("fits");
        }
        assert_matches(&g, &ichnos_viz::bpmn_dot(&doc.model, &options));
    }
}

fn matrix<T>(v: &Value, value: impl Fn(&Value) -> T) -> Matrix<T> {
    let mut m: Matrix<T> = Matrix::new();
    for row in v.as_array().expect("matrix rows") {
        m.entry(Label::from(row[0].as_str().expect("activity")))
            .or_default()
            .insert(
                Label::from(row[1].as_str().expect("activity")),
                value(&row[2]),
            );
    }
    m
}

fn edges(v: &Value) -> impl Iterator<Item = (Label, HeuristicsEdge)> + '_ {
    v.as_array().expect("edges").iter().map(|e| {
        (
            Label::from(str_field(e, "target")),
            HeuristicsEdge {
                dependency: as_f64(&e["dependency"]),
                frequency: as_u64(&e["frequency"]),
            },
        )
    })
}

fn build_heuristics_net(d: &Value) -> HeuristicsNet {
    let mut h = HeuristicsNet {
        activities: d["activities"]
            .as_array()
            .expect("activities")
            .iter()
            .map(|a| Label::from(a.as_str().expect("activity")))
            .collect(),
        activity_occurrences: counts(&d["activities_occurrences"]),
        start_activities: d["start_activities"]
            .as_array()
            .expect("starts")
            .iter()
            .map(counts)
            .collect(),
        end_activities: d["end_activities"]
            .as_array()
            .expect("ends")
            .iter()
            .map(counts)
            .collect(),
        dfg_matrix: matrix(&d["dfg_matrix"], as_u64),
        dependency_matrix: matrix(&d["dependency_matrix"], as_f64),
        freq_triples_matrix: matrix(&d["freq_triples_matrix"], as_u64),
        ..HeuristicsNet::default()
    };
    for node in d["nodes"].as_array().expect("nodes") {
        let name = Label::from(str_field(node, "name"));
        h.node_mut(&name).occurrences = as_u64(&node["occurrences"]);
        for (to, e) in edges(&node["outputs"]) {
            h.add_output_connection(&name, &to, e);
        }
        for (from, e) in edges(&node["inputs"]) {
            h.add_input_connection(&name, &from, e);
        }
    }
    h
}

#[test]
fn heuristics_nets_match_pm4py() {
    for case in cases("viz")
        .iter()
        .filter(|c| c.starts_with("heuristics-net-"))
    {
        let g = golden("viz", case);
        let net = build_heuristics_net(g.expected_at("/model"));
        let mut options = HeuristicsNetDotOptions::default();
        if let Some(n) = params(&g)["min_dfg_occurrences"].as_u64() {
            options.min_dfg_occurrences = n;
        }
        let dot = ichnos_viz::heuristics_net_dot(&net, &options);
        assert_matches(&g, &dot);
    }
}

#[test]
fn transition_systems_match_pm4py() {
    for case in cases("viz")
        .iter()
        .filter(|c| c.starts_with("transition-system-"))
    {
        let g = golden("viz", case);
        let model = g.expected_at("/model");
        let mut ts = TransitionSystem::new("");
        let states: Vec<_> = model["states"]
            .as_array()
            .expect("states")
            .iter()
            .map(|s| ts.add_state(s.as_str().expect("state name")))
            .collect();
        for t in model["transitions"].as_array().expect("transitions") {
            let at = |k: usize| states[usize::try_from(as_u64(&t[k])).expect("index fits")];
            ts.add_edge(t[2].as_str().expect("transition name"), at(0), at(1))
                .expect("edge");
        }
        let dot = ichnos_viz::transition_system_dot(&ts, &TransitionSystemDotOptions::default());
        assert_matches(&g, &dot);
    }
}

#[test]
fn prefix_trees_match_pm4py() {
    for case in cases("viz")
        .iter()
        .filter(|c| c.starts_with("prefix-tree-"))
    {
        let g = golden("viz", case);
        let mut tree = PrefixTree { nodes: Vec::new() };
        for n in g.expected_at("/model/nodes").as_array().expect("nodes") {
            let label = n["label"].as_str().map(Label::from);
            let parent = n["parent"]
                .as_u64()
                .map(|p| usize::try_from(p).expect("index fits"));
            if let (Some(parent), Some(label)) = (parent, &label) {
                let index = tree.nodes.len();
                tree.nodes[parent].children.insert(label.clone(), index);
            }
            tree.nodes.push(PrefixNode {
                label,
                parent,
                children: BTreeMap::new(),
                final_node: n["final"].as_bool().expect("final"),
                depth: usize::try_from(as_u64(&n["depth"])).expect("depth fits"),
            });
        }
        let dot = ichnos_viz::prefix_tree_dot(&tree, &PrefixTreeDotOptions::default());
        assert_matches(&g, &dot);
    }
}

fn build_footprints(v: &Value) -> Footprints {
    let pairs = |key: &str| -> BTreeSet<(Label, Label)> {
        v[key]
            .as_array()
            .expect("pairs")
            .iter()
            .map(|p| {
                (
                    Label::from(p[0].as_str().expect("activity")),
                    Label::from(p[1].as_str().expect("activity")),
                )
            })
            .collect()
    };
    Footprints {
        sequence: pairs("sequence"),
        parallel: pairs("parallel"),
        ..Footprints::default()
    }
}

#[test]
fn footprints_match_pm4py() {
    for case in cases("viz").iter().filter(|c| c.starts_with("footprints-")) {
        let g = golden("viz", case);
        let model = g.expected_at("/model");
        let options = FootprintsDotOptions::default();
        let dot = if model.get("first").is_some() {
            ichnos_viz::footprints_comparison_dot(
                &build_footprints(&model["first"]),
                &build_footprints(&model["second"]),
                &options,
            )
        } else {
            ichnos_viz::footprints_dot(&build_footprints(model), &options)
        };
        assert_matches(&g, &dot);
    }
}

#[test]
fn alignment_tables_match_pm4py() {
    for case in cases("viz").iter().filter(|c| c.starts_with("alignments-")) {
        let g = golden("viz", case);
        let rows: Vec<VariantAlignment> = g
            .expected_at("/model/rows")
            .as_array()
            .expect("rows")
            .iter()
            .map(|r| VariantAlignment {
                activities: r["activities"]
                    .as_array()
                    .expect("activities")
                    .iter()
                    .map(|a| a.as_str().expect("activity").to_owned())
                    .collect(),
                count: usize::try_from(as_u64(&r["count"])).expect("count fits"),
                steps: Some(
                    r["moves"]
                        .as_array()
                        .expect("moves")
                        .iter()
                        .map(|m| match (m[0].as_str(), m[1].as_str()) {
                            (Some(">>"), model) => AlignmentStep::Model(model.map(str::to_owned)),
                            (Some(log), Some(">>")) => AlignmentStep::Log(log.to_owned()),
                            (Some(_), Some(model)) => AlignmentStep::Sync(model.to_owned()),
                            _ => panic!("unexpected move {m}"),
                        })
                        .collect(),
                ),
            })
            .collect();
        let dot = ichnos_viz::alignment_table_dot(&rows, &AlignmentsDotOptions::default());
        assert_matches(&g, &dot);
    }
}

#[test]
fn an_alignment_table_shows_variants_without_an_alignment() {
    let row = |activities: &[&str], count, steps| VariantAlignment {
        activities: activities.iter().map(|a| (*a).to_owned()).collect(),
        count,
        steps,
    };
    let dot = ichnos_viz::alignment_table_dot(
        &[
            row(&["b"], 1, None),
            row(&["a"], 1, Some(vec![AlignmentStep::Sync("a".to_owned())])),
            row(
                &["c>d"],
                2,
                Some(vec![AlignmentStep::Log("c>d".to_owned())]),
            ),
        ],
        &AlignmentsDotOptions::default(),
    );
    let p = parse_dot(&dot);
    let label = &p.nodes[0].1["label"];
    let variant = |n: usize| label.find(&format!("Variant {n} ")).expect("variant row");
    assert!(label[variant(1)..].starts_with("Variant 1 (2 occurrences)"));
    assert!(label.contains("<b>(LM)</b>c&gt;d</td>"), "{label}");
    // Ties go in activity order; a missing alignment leaves the row empty.
    assert!(
        label[variant(2)..variant(3)].contains("lightgreen\">a<"),
        "{label}"
    );
    assert!(label[variant(3)..].contains("<tr></tr>"), "{label}");
}

fn build_powl(v: &Value) -> Powl {
    let children = || -> Vec<Powl> {
        v["children"]
            .as_array()
            .expect("children")
            .iter()
            .map(build_powl)
            .collect()
    };
    match str_field(v, "kind") {
        "silent" => Powl::Silent,
        "activity" => Powl::activity(str_field(v, "label")),
        "frequent" => Powl::Frequent(FrequentTransition::new(
            str_field(v, "activity"),
            v["skippable"].as_bool().expect("skippable"),
            v["selfloop"].as_bool().expect("selfloop"),
        )),
        "xor" => Powl::xor(children()),
        "loop" => {
            let [a, b]: [Powl; 2] = children().try_into().expect("two loop children");
            Powl::looped(a, b)
        }
        "po" => {
            let mut po = StrictPartialOrder::new(children());
            for pair in v["order"].as_array().expect("order") {
                let at = |k: usize| usize::try_from(as_u64(&pair[k])).expect("index fits");
                po.add_edge(at(0), at(1));
            }
            Powl::PartialOrder(po)
        }
        other => panic!("unknown POWL kind {other}"),
    }
}

#[test]
fn powl_models_match_pm4py() {
    for case in cases("viz").iter().filter(|c| c.starts_with("powl-")) {
        let g = golden("viz", case);
        let powl = build_powl(g.expected_at("/model"));
        let p = params(&g);
        let mut options = PowlDotOptions::default();
        if let Some(v) = p["bgcolor"].as_str() {
            options.bgcolor = v.to_owned();
        }
        if let Some(v) = p["rankdir"].as_str() {
            options.rankdir = v.to_owned();
        }
        assert_matches(&g, &ichnos_viz::powl_dot(&powl, &options));
    }
}

#[test]
fn powl_icon_paths_use_the_icon_dir() {
    let powl = Powl::xor([Powl::activity("a"), Powl::activity("b")]);
    let options = PowlDotOptions {
        icon_dir: Some("icons".into()),
        ..PowlDotOptions::default()
    };
    let dot = ichnos_viz::powl_dot(&powl, &options);
    let expected = std::path::Path::new("icons").join("xor.svg");
    assert!(
        dot.contains(&format!("image=\"{}\"", expected.display())),
        "{dot}"
    );
    let dir = std::env::temp_dir().join(format!("ichnos-viz-icons-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    ichnos_viz::write_powl_icons(&dir).expect("writes icons");
    for (name, svg) in ichnos_viz::POWL_ICONS {
        assert_eq!(std::fs::read_to_string(dir.join(name)).expect("icon"), svg);
    }
    std::fs::remove_dir_all(&dir).expect("cleanup");
}

#[test]
fn empty_models_draw_empty_graphs() {
    let p = parse_dot(&ichnos_viz::powl_dot(
        &Powl::PartialOrder(StrictPartialOrder::new([])),
        &PowlDotOptions::default(),
    ));
    assert!(p.nodes.is_empty() && p.edges.is_empty(), "{p:?}");
    let p = parse_dot(&ichnos_viz::footprints_dot(
        &Footprints::default(),
        &FootprintsDotOptions::default(),
    ));
    assert_eq!(p.nodes.len(), 1, "{p:?}");
}

#[test]
fn every_golden_is_checked() {
    let prefixes = [
        "petri-net-",
        "dfg-",
        "performance-dfg-",
        "process-tree-",
        "bpmn-",
        "heuristics-net-",
        "transition-system-",
        "prefix-tree-",
        "footprints-",
        "alignments-",
        "powl-",
        "ocdfg-",
        "ocpn-",
        "object-graph-",
        "network-analysis-",
        "dotted-chart-",
        "performance-spectrum-",
    ];
    for case in cases("viz") {
        assert!(
            prefixes.iter().any(|p| case.starts_with(p)),
            "{case} has no test"
        );
    }
}

#[test]
fn isomorphism_check_tells_graphs_apart() {
    let tree = |s: &str| {
        let dot = ichnos_viz::process_tree_dot(
            &ProcessTree::parse(s).expect("tree"),
            &ProcessTreeDotOptions {
                sort: false,
                ..ProcessTreeDotOptions::default()
            },
        );
        let p = parse_dot(&dot);
        Graph::new(
            p.nodes.iter().map(|(n, a)| (n.as_str(), a.clone())),
            p.edges
                .iter()
                .map(|(s, t, a)| (s.as_str(), t.as_str(), a.clone())),
        )
    };
    let a = tree("->( 'a', X( 'b', 'c' ) )");
    assert!(isomorphic(&a, &tree("->( 'a', X( 'c', 'b' ) )")));
    assert!(!isomorphic(&a, &tree("->( 'a', +( 'b', 'c' ) )")));
    assert!(!isomorphic(&a, &tree("->( 'a', X( 'b', 'd' ) )")));
    // The drawing is undirected and does not record child order, so
    // swapping the children of a sequence gives the same graph.
    assert!(isomorphic(&a, &tree("->( X( 'b', 'c' ), 'a' )")));
}

#[test]
fn writing_dot_saves_the_text() {
    let dir = std::env::temp_dir().join(format!("ichnos-viz-golden-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let tree = ProcessTree::parse("->( 'a', 'b' )").expect("tree");
    let options = ProcessTreeDotOptions::default();
    let path = dir.join("tree.dot");
    ichnos_viz::write_process_tree(&tree, &options, &path).expect("writes");
    let text = std::fs::read_to_string(&path).expect("reads");
    assert_eq!(text, ichnos_viz::process_tree_dot(&tree, &options));
    // Rendering needs Graphviz, which may be missing.
    match ichnos_viz::write_process_tree(&tree, &options, dir.join("tree.svg")) {
        Ok(()) | Err(VizError::DotNotFound { .. }) => {}
        Err(e) => panic!("unexpected error {e}"),
    }
    std::fs::remove_dir_all(&dir).expect("cleanup");
}

#[test]
fn an_empty_dfg_draws_no_nodes() {
    // pm4py fails here, on `min()` of an empty sequence.
    let p = parse_dot(&ichnos_viz::dfg_dot(&Dfg::new(), &DfgDotOptions::default()));
    assert!(p.nodes.is_empty() && p.edges.is_empty(), "{p:?}");
    let p = parse_dot(&ichnos_viz::performance_dfg_dot(
        &PerformanceDfg::default(),
        &PerformanceDfgDotOptions::default(),
    ));
    assert!(p.nodes.is_empty() && p.edges.is_empty(), "{p:?}");
}

/// `bgcolor`, `rankdir` and `graph_title` from a case's parameters.
fn styled(p: &Value) -> (Option<String>, Option<String>, Option<String>) {
    let field = |k: &str| p[k].as_str().map(str::to_owned);
    (field("bgcolor"), field("rankdir"), field("graph_title"))
}

fn colors(model: &Value) -> BTreeMap<String, String> {
    model["colors"]
        .as_object()
        .expect("colours")
        .iter()
        .map(|(k, v)| (k.clone(), v.as_str().expect("colour").to_owned()))
        .collect()
}

fn ocdfg_counts(v: &Value) -> OcdfgCounts {
    OcdfgCounts {
        events: as_u64(&v[0]),
        unique_objects: as_u64(&v[1]),
        total_objects: as_u64(&v[2]),
    }
}

fn ocdfg_sides(v: &Value) -> BTreeMap<String, BTreeMap<String, OcdfgCounts>> {
    v.as_object()
        .expect("object types")
        .iter()
        .map(|(ot, acts)| {
            let acts = acts
                .as_object()
                .expect("activities")
                .iter()
                .map(|(a, c)| (a.clone(), ocdfg_counts(c)))
                .collect();
            (ot.clone(), acts)
        })
        .collect()
}

fn floats(v: &Value) -> Vec<f64> {
    v.as_array().expect("numbers").iter().map(as_f64).collect()
}

#[test]
fn ocdfgs_match_pm4py() {
    for case in cases("viz").iter().filter(|c| c.starts_with("ocdfg-")) {
        let g = golden("viz", case);
        let model = g.expected_at("/model");
        let mut ocdfg = Ocdfg {
            activities: model["activities"]
                .as_object()
                .expect("activities")
                .iter()
                .map(|(a, c)| (a.clone(), ocdfg_counts(c)))
                .collect(),
            start_activities: ocdfg_sides(&model["start"]),
            end_activities: ocdfg_sides(&model["end"]),
            ..Ocdfg::default()
        };
        for (ot, edges) in model["edges"].as_object().expect("edges") {
            let edges = edges
                .as_array()
                .expect("edges")
                .iter()
                .map(|e| {
                    let key = (str_value(&e[0]), str_value(&e[1]));
                    let c = ocdfg_counts(&e[2]);
                    let edge = OcdfgEdge {
                        event_couples: c.events,
                        unique_objects: c.unique_objects,
                        total_objects: c.total_objects,
                        event_couples_durations: floats(&e[3]),
                        total_objects_durations: floats(&e[4]),
                    };
                    (key, edge)
                })
                .collect();
            ocdfg.edges.insert(ot.clone(), edges);
        }
        let p = params(&g);
        let (bgcolor, rankdir, graph_title) = styled(&p);
        let mut options = OcdfgDotOptions {
            graph_title,
            object_type_colors: colors(model),
            ..OcdfgDotOptions::default()
        };
        if let Some(b) = bgcolor {
            options.bgcolor = b;
        }
        if let Some(r) = rankdir {
            options.rankdir = r;
        }
        if p["annotation"].as_str() == Some("performance") {
            options.annotation = OcdfgAnnotation::Performance;
        }
        options.act_metric = match p["act_metric"].as_str() {
            None | Some("events") => OcdfgActivityMetric::Events,
            Some("unique_objects") => OcdfgActivityMetric::UniqueObjects,
            Some("total_objects") => OcdfgActivityMetric::TotalObjects,
            Some(other) => panic!("unknown metric {other}"),
        };
        options.edge_metric = match p["edge_metric"].as_str() {
            None | Some("event_couples") => OcdfgEdgeMetric::EventCouples,
            Some("unique_objects") => OcdfgEdgeMetric::UniqueObjects,
            Some("total_objects") => OcdfgEdgeMetric::TotalObjects,
            Some(other) => panic!("unknown metric {other}"),
        };
        options.act_threshold = p["act_threshold"].as_u64().unwrap_or(0);
        options.edge_threshold = p["edge_threshold"].as_u64().unwrap_or(0);
        if p["business_hours"].as_bool() == Some(true) {
            options.business_hours = Some(BusinessHours::default());
        }
        assert_matches(&g, &ichnos_viz::ocdfg_dot(&ocdfg, &options));
    }
}

fn str_value(v: &Value) -> String {
    v.as_str()
        .unwrap_or_else(|| panic!("{v} is not a string"))
        .to_owned()
}

#[test]
fn ocpns_match_pm4py() {
    for case in cases("viz").iter().filter(|c| c.starts_with("ocpn-")) {
        let g = golden("viz", case);
        let model = g.expected_at("/model");
        let mut ocpn = OcPetriNet {
            activities: model["activities"]
                .as_array()
                .expect("activities")
                .iter()
                .map(str_value)
                .collect(),
            ..OcPetriNet::default()
        };
        for (ot, n) in model["nets"].as_object().expect("nets") {
            let (net, initial, fin) = build_net(&n["net"]);
            let diagnostics = n["diagnostics"].as_object().map(|d| OcpnDiagnostics {
                places: d["places"]
                    .as_object()
                    .expect("places")
                    .iter()
                    .map(|(name, v)| {
                        let place = net.place_by_name(name).expect("a place");
                        let diag = PlaceDiagnostics {
                            produced: as_u64(&v[0]),
                            missing: as_u64(&v[1]),
                            consumed: as_u64(&v[2]),
                            remaining: as_u64(&v[3]),
                        };
                        (place, diag)
                    })
                    .collect(),
                transitions: d["transitions"]
                    .as_object()
                    .expect("transitions")
                    .iter()
                    .map(|(name, v)| {
                        (
                            net.transition_by_name(name).expect("a transition"),
                            as_u64(v),
                        )
                    })
                    .collect(),
            });
            let double_arcs = n["double_arcs"]
                .as_object()
                .expect("double arcs")
                .iter()
                .map(|(a, v)| (a.clone(), v.as_bool().expect("a flag")))
                .collect();
            ocpn.nets.insert(
                ot.clone(),
                ObjectTypeNet {
                    net,
                    initial,
                    fin,
                    double_arcs,
                    diagnostics,
                },
            );
        }
        let (bgcolor, rankdir, graph_title) = styled(&params(&g));
        let mut options = OcpnDotOptions {
            graph_title,
            object_type_colors: colors(model),
            ..OcpnDotOptions::default()
        };
        if let Some(b) = bgcolor {
            options.bgcolor = b;
        }
        if let Some(r) = rankdir {
            options.rankdir = r;
        }
        assert_matches(&g, &ichnos_viz::ocpn_dot(&ocpn, &options));
    }
}

#[test]
fn object_graphs_match_pm4py() {
    for case in cases("viz")
        .iter()
        .filter(|c| c.starts_with("object-graph-"))
    {
        let g = golden("viz", case);
        let model = g.expected_at("/model");
        let mut ocel = Ocel::default();
        for o in model["objects"].as_array().expect("objects") {
            ocel.objects.push(OcelObject {
                id: o[0].as_str().expect("an id").into(),
                object_type: o[1].as_str().expect("a type").into(),
                attributes: Default::default(),
            });
        }
        let graph: BTreeSet<(String, String)> = model["graph"]
            .as_array()
            .expect("pairs")
            .iter()
            .map(|e| (str_value(&e[0]), str_value(&e[1])))
            .collect();
        let p = params(&g);
        let (_, rankdir, graph_title) = styled(&p);
        let mut options = ObjectGraphDotOptions {
            graph_title,
            object_type_colors: colors(model),
            directed: p["directed"].as_bool().unwrap_or(true),
            ..ObjectGraphDotOptions::default()
        };
        if let Some(r) = rankdir {
            options.rankdir = r;
        }
        assert_matches(&g, &ichnos_viz::object_graph_dot(&ocel, &graph, &options));
    }
}

#[test]
fn network_analyses_match_pm4py() {
    for case in cases("viz")
        .iter()
        .filter(|c| c.starts_with("network-analysis-"))
    {
        let g = golden("viz", case);
        let model = g.expected_at("/model");
        let p = params(&g);
        let (_, _, graph_title) = styled(&p);
        let mut options = NetworkAnalysisDotOptions {
            graph_title,
            ..NetworkAnalysisDotOptions::default()
        };
        if let Some(t) = p["activity_threshold"].as_u64() {
            options.activity_threshold = t;
        }
        if let Some(t) = p["edge_threshold"].as_u64() {
            options.edge_threshold = t;
        }
        if p["business_hours"].as_bool() == Some(true) {
            options.business_hours = Some(BusinessHours::default());
        }
        fn edges<T>(model: &Value, value: impl Fn(&Value) -> T) -> Vec<NetworkAnalysisEdge<T>> {
            model["edges"]
                .as_array()
                .expect("edges")
                .iter()
                .map(|e| NetworkAnalysisEdge {
                    source: str_value(&e[0]),
                    target: str_value(&e[1]),
                    values: e[2]
                        .as_array()
                        .expect("values")
                        .iter()
                        .map(|v| (str_value(&v[0]), value(&v[1])))
                        .collect(),
                })
                .collect()
        }
        let dot = if p["performance"].as_bool() == Some(true) {
            ichnos_viz::network_analysis_performance_dot(&edges(model, floats), &options)
        } else {
            ichnos_viz::network_analysis_dot(&edges(model, as_u64), &options)
        };
        assert_matches(&g, &dot);
    }
}

#[test]
fn dotted_charts_match_pm4py() {
    for case in cases("viz")
        .iter()
        .filter(|c| c.starts_with("dotted-chart-"))
    {
        let g = golden("viz", case);
        let model = g.expected_at("/model");
        let log = ichnos_io::read_xes(g.fixture("log"), &Default::default()).unwrap();
        let names: Vec<String> = model["attributes"]
            .as_array()
            .expect("attributes")
            .iter()
            .map(str_value)
            .collect();
        let attributes = DottedChartAttributes {
            x: names[0].clone(),
            y: names[1].clone(),
            color: names.get(2).cloned(),
        };
        let p = params(&g);
        let options = DottedChartDotOptions {
            graph_title: styled(&p).2,
            show_legend: p["show_legend"].as_bool().unwrap_or(true),
            colors: colors(model),
            ..DottedChartDotOptions::default()
        };
        let points = ichnos_viz::dotted_chart_points(&log, &attributes);
        assert_matches(
            &g,
            &ichnos_viz::dotted_chart_dot(&points, &attributes, &options),
        );
    }
}

#[test]
fn performance_spectra_match_pm4py() {
    for case in cases("viz")
        .iter()
        .filter(|c| c.starts_with("performance-spectrum-"))
    {
        let g = golden("viz", case);
        let model = g.expected_at("/model");
        let log = ichnos_io::read_xes(g.fixture("log"), &Default::default()).unwrap();
        let activities: Vec<String> = model["activities"]
            .as_array()
            .expect("activities")
            .iter()
            .map(str_value)
            .collect();
        let p = params(&g);
        let mut options = PerformanceSpectrumOptions::default();
        if let Some(n) = p["sample_size"].as_u64() {
            options.sample_size = usize::try_from(n).expect("sample size fits");
        }
        let spectrum = ichnos_viz::performance_spectrum(&log, &activities, &options);
        let expected: Vec<Vec<f64>> = model["points"]
            .as_array()
            .expect("points")
            .iter()
            .map(floats)
            .collect();
        assert_eq!(spectrum.points, expected, "{case}: points");
        let dot_options = PerformanceSpectrumDotOptions {
            graph_title: styled(&p).2,
            ..PerformanceSpectrumDotOptions::default()
        };
        assert_matches(
            &g,
            &ichnos_viz::performance_spectrum_dot(&spectrum, &dot_options),
        );
    }
}
