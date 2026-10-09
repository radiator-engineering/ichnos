//! DOT output against pm4py's `save_vis_*` (`fixtures/golden/viz`).
//!
//! Each golden holds the model pm4py drew and the graph its DOT text
//! describes, with node defaults resolved. Node names differ (pm4py uses
//! object ids), so graphs are compared up to isomorphism: nodes by their
//! attributes, edges by theirs. See `tools/golden/cases/viz.py`.

use std::collections::{BTreeMap, BTreeSet};

use ichnos_discovery::PerformanceDfg;
use ichnos_discovery::dfg::{Aggregation, BusinessHours, PerformanceSummary};
use ichnos_golden::{Golden, cases, golden};
use ichnos_model::heuristics_net::{HeuristicsEdge, HeuristicsNet, Matrix};
use ichnos_model::petri::{ArcEnds, ArcKind};
use ichnos_model::{Dfg, Label, Marking, PetriNet, PlaceId, ProcessTree};
use ichnos_viz::{
    BpmnDotOptions, DfgDotOptions, HeuristicsNetDotOptions, PerformanceDfgDotOptions,
    PetriNetDotOptions, ProcessTreeDotOptions, VizError,
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
                pos += 1;
            }
            assert_eq!(toks[pos], op("{"));
            pos += 1;
            let defaults = frames.last().expect("a frame").1.clone();
            frames.push((Attrs::new(), defaults));
            continue;
        }
        let name = id(&t).to_owned();
        if ["graph", "node", "edge"].contains(&name.as_str()) && peek(pos) == Some(&op("[")) {
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
            p.edges.push((name, target, attr_list(&mut pos)));
            continue;
        }
        let (gattrs, defaults) = frames.last().expect("a frame");
        let mut attrs = defaults.clone();
        let subgraph = (frames.len() > 1).then(|| attrs_key(gattrs));
        attrs.extend(attr_list(&mut pos));
        if let Some(s) = subgraph {
            attrs.insert("subgraph".to_owned(), s);
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
        if let Some(n) = params(&g)["max_num_edges"].as_u64() {
            options.max_num_edges = usize::try_from(n).expect("fits");
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
        let dot = ichnos_viz::process_tree_dot(&tree, &ProcessTreeDotOptions::default());
        assert_matches(&g, &dot);
    }
}

#[test]
fn bpmn_diagrams_match_pm4py() {
    for case in cases("viz").iter().filter(|c| c.starts_with("bpmn-")) {
        let g = golden("viz", case);
        let doc = ichnos_io::read_bpmn(g.fixture("model"), &ichnos_io::BpmnReadOptions::default())
            .expect("BPMN reads");
        assert_matches(
            &g,
            &ichnos_viz::bpmn_dot(&doc.model, &BpmnDotOptions::default()),
        );
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
        let dot = ichnos_viz::heuristics_net_dot(&net, &HeuristicsNetDotOptions::default());
        assert_matches(&g, &dot);
    }
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
        Ok(()) | Err(VizError::DotNotFound(_)) => {}
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
