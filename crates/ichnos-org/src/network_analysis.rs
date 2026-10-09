//! Network analysis, ported from pm4py's
//! `algo/organizational_mining/network_analysis/variants/dataframe.py`.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};

use ichnos_core::{AttributeValue, EventLog};

/// Which event of a link gives the edge its name.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EdgeReference {
    /// The source event (pm4py's `"_out"`).
    #[default]
    Source,
    /// The target event (pm4py's `"_in"`).
    Target,
}

/// Options for [`discover_network_analysis`], with the defaults of pm4py's
/// network analysis.
///
/// A column that starts with `case:` reads the trace attribute after the
/// prefix, as in a flat table.
#[derive(Debug, Clone)]
pub struct NetworkAnalysisOptions {
    /// The attribute an event sends. Default `case:concept:name`.
    pub out_column: String,
    /// The attribute an event receives. Default `case:concept:name`.
    pub in_column: String,
    /// The attribute of the source event that names the source node.
    /// Default `org:resource`.
    pub node_column_source: String,
    /// The attribute of the target event that names the target node.
    /// Default `org:resource`.
    pub node_column_target: String,
    /// The attribute that names the edge. Default `concept:name`.
    pub edge_column: String,
    /// Which event's `edge_column` names the edge. Default the source.
    pub edge_reference: EdgeReference,
    /// The attribute that orders the events. Default `time:timestamp`.
    pub sorting_column: String,
    /// The timestamp for durations. Default `time:timestamp`.
    pub timestamp_column: String,
}

impl Default for NetworkAnalysisOptions {
    fn default() -> Self {
        NetworkAnalysisOptions {
            out_column: "case:concept:name".to_owned(),
            in_column: "case:concept:name".to_owned(),
            node_column_source: "org:resource".to_owned(),
            node_column_target: "org:resource".to_owned(),
            edge_column: "concept:name".to_owned(),
            edge_reference: EdgeReference::Source,
            sorting_column: "time:timestamp".to_owned(),
            timestamp_column: "time:timestamp".to_owned(),
        }
    }
}

/// A network analysis: for each `(source node, target node)` pair, a value
/// for each edge name.
pub type NetworkAnalysis<T> = BTreeMap<(String, String), BTreeMap<String, T>>;

/// The network analysis of a log with counts, as pm4py's
/// `discover_network_analysis` computes it.
///
/// The events are ordered by `sorting_column`. Each event links to the
/// first later event whose `in_column` equals its `out_column`. A link
/// joins the source event's node to the target event's node, under the
/// name of its edge; links without a node or a name are left out. The value
/// counts the links.
///
/// Values are compared in their Python `str` form. Events without the
/// sorting attribute come last; events that tie keep their log order.
pub fn discover_network_analysis(
    log: &EventLog,
    options: &NetworkAnalysisOptions,
) -> NetworkAnalysis<u64> {
    let mut out: NetworkAnalysis<u64> = BTreeMap::new();
    for link in links(log, options) {
        *out.entry(link.nodes)
            .or_default()
            .entry(link.edge)
            .or_default() += 1;
    }
    out
}

/// The network analysis of a log with durations, as pm4py's
/// `discover_network_analysis` computes it with `performance=True`.
///
/// The links are those of [`discover_network_analysis`]. Each value lists
/// the seconds from the source event to the target event, in the order of
/// the source events; a link without both timestamps gives NaN.
pub fn discover_network_analysis_performance(
    log: &EventLog,
    options: &NetworkAnalysisOptions,
) -> NetworkAnalysis<Vec<f64>> {
    let mut out: NetworkAnalysis<Vec<f64>> = BTreeMap::new();
    for link in links(log, options) {
        out.entry(link.nodes)
            .or_default()
            .entry(link.edge)
            .or_default()
            .push(link.seconds);
    }
    out
}

/// One link between two events.
struct Link {
    nodes: (String, String),
    edge: String,
    seconds: f64,
}

/// The links of the log, in the order of their source events.
fn links(log: &EventLog, options: &NetworkAnalysisOptions) -> Vec<Link> {
    let value = |(t, e): (usize, usize), column: &str| -> Option<&AttributeValue> {
        match column.strip_prefix("case:") {
            Some(name) if log.traces[t].events[e].get(column).is_none() => {
                log.traces[t].attributes.get(name)
            }
            _ => log.traces[t].events[e].get(column),
        }
    };
    let text = |pos, column: &str| value(pos, column).map(ToString::to_string);

    let mut events: Vec<(usize, usize)> = log
        .traces
        .iter()
        .enumerate()
        .flat_map(|(t, trace)| (0..trace.events.len()).map(move |e| (t, e)))
        .collect();
    events.sort_by(|&a, &b| {
        match (
            value(a, &options.sorting_column),
            value(b, &options.sorting_column),
        ) {
            (Some(x), Some(y)) => compare(x, y),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
    });

    // The first later event that receives each value, walking backwards.
    let mut next: Vec<Option<usize>> = vec![None; events.len()];
    let mut receivers: HashMap<String, usize> = HashMap::new();
    for i in (0..events.len()).rev() {
        if let Some(sent) = text(events[i], &options.out_column) {
            next[i] = receivers.get(&sent).copied();
        }
        if let Some(received) = text(events[i], &options.in_column) {
            receivers.insert(received, i);
        }
    }

    let mut out = Vec::new();
    for (i, j) in next.iter().enumerate() {
        let Some(j) = *j else { continue };
        let (src, tgt) = (events[i], events[j]);
        let edge_event = match options.edge_reference {
            EdgeReference::Source => src,
            EdgeReference::Target => tgt,
        };
        let (Some(a), Some(b), Some(edge)) = (
            text(src, &options.node_column_source),
            text(tgt, &options.node_column_target),
            text(edge_event, &options.edge_column),
        ) else {
            continue;
        };
        let time = |pos| value(pos, &options.timestamp_column).and_then(AttributeValue::as_date);
        let seconds = match (time(src), time(tgt)) {
            (Some(x), Some(y)) => (y - x)
                .num_nanoseconds()
                .map_or_else(|| (y - x).num_seconds() as f64, |ns| ns as f64 / 1e9),
            _ => f64::NAN,
        };
        out.push(Link {
            nodes: (a, b),
            edge,
            seconds,
        });
    }
    out
}

/// Orders two values of a column: dates by instant, numbers by value, text
/// by code point. Values of different kinds order by kind.
fn compare(a: &AttributeValue, b: &AttributeValue) -> Ordering {
    let rank = |v: &AttributeValue| match v.plain() {
        AttributeValue::Date(_) => 0,
        AttributeValue::Int(_) | AttributeValue::Float(_) | AttributeValue::Bool(_) => 1,
        _ => 2,
    };
    match (a.plain(), b.plain()) {
        (AttributeValue::Date(x), AttributeValue::Date(y)) => x.cmp(y),
        _ if rank(a) == 1 && rank(b) == 1 => {
            let num = |v: &AttributeValue| match v.plain() {
                AttributeValue::Bool(x) => f64::from(u8::from(*x)),
                other => other.as_f64().unwrap_or(f64::NAN),
            };
            num(a).total_cmp(&num(b))
        }
        _ => rank(a)
            .cmp(&rank(b))
            .then_with(|| a.to_string().cmp(&b.to_string())),
    }
}
