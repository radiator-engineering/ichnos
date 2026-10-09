//! Object features of an object-centric event log, ported from pm4py's
//! `extract_ocel_features` (`algo/transformation/ocel/features/objects` and
//! the object graphs of `algo/transformation/ocel/graphs`).

use std::collections::{BTreeSet, HashMap, HashSet};

use ichnos_ocel::Ocel;

use crate::error::{Error, Result};
use crate::util::seconds;

/// Options for [`extract_ocel_features`], with pm4py's defaults.
#[derive(Clone, Debug)]
pub struct OcelFeatureOptions {
    /// Count the directly-following activity pairs of each object
    /// (`@@ocel_lif_path_<a>##<b>`). Default `true`.
    pub lifecycle_paths: bool,
    /// Count the objects whose lifecycle overlaps each object's
    /// (`@@object_wip`). Default `false`.
    pub work_in_progress: bool,
    /// String object attributes: one feature
    /// `@@object_attr_value_<attr>_<value>` per value.
    pub str_attributes: Vec<String>,
    /// Numeric object attributes: the feature `@@event_num_<attr>`.
    pub num_attributes: Vec<String>,
}

impl Default for OcelFeatureOptions {
    fn default() -> Self {
        OcelFeatureOptions {
            lifecycle_paths: true,
            work_in_progress: false,
            str_attributes: Vec::new(),
            num_attributes: Vec::new(),
        }
    }
}

/// One row of features per object.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ObjectFeatures {
    /// The object of each row.
    pub object_ids: Vec<String>,
    /// The feature names, one per column.
    pub names: Vec<String>,
    /// The rows.
    pub rows: Vec<Vec<f64>>,
}

impl ObjectFeatures {
    /// The values of the feature `name`, one per row.
    pub fn column(&self, name: &str) -> Option<Vec<f64>> {
        let k = self.names.iter().position(|n| n == name)?;
        Some(self.rows.iter().map(|r| r[k]).collect())
    }
}

/// The features of the objects of type `object_type`, as pm4py's
/// `extract_ocel_features` computes them.
///
/// Features are computed over all objects, then the rows of the type are
/// kept, in the order of [`Ocel::objects`]. In order, the features are:
///
/// - `@@object_lifecycle_length`: the number of related events;
/// - `@@object_lifecycle_duration`, `@@object_lifecycle_start_timestamp` and
///   `@@object_lifecycle_end_timestamp`, in seconds, from the first and last
///   relation;
/// - `@@object_degree_centrality` in the object interaction graph, whose
///   edges join objects that share an event;
/// - `@@object_general_interaction_graph`: the number of neighbours there;
/// - `@@object_general_descendants_graph_ascendants` and `_descendants`:
///   edges from objects already seen to objects first seen in a later event;
/// - `@@object_general_inheritance_graph_ascendants` and `_descendants`:
///   edges from objects that end in an event to objects that start in it,
///   without pairs that go both ways;
/// - `@@object_cobirth` and `@@object_codeath`: the objects that start, or
///   end, in the same event;
/// - `@@ocel_lif_activity_<act>`: how often each activity occurs in the
///   object's lifecycle, activities in order of first occurrence;
/// - the string and numeric attribute features of the options;
/// - `@@object_interaction_graph_<type>`: neighbours by object type, types
///   in order of first occurrence;
/// - `@@object_wip` when enabled;
/// - `@@object_lifecycle_unq_act`: the number of distinct activities;
/// - the lifecycle paths when enabled, sorted.
///
/// pm4py orders the values of a string attribute by Python's set order;
/// this sorts them. An object type without objects gives no rows and no
/// features, as pm4py drops the columns of an empty frame. An event without
/// related objects is an error, as in pm4py.
pub fn extract_ocel_features(
    ocel: &Ocel,
    object_type: &str,
    options: &OcelFeatureOptions,
) -> Result<ObjectFeatures> {
    let n = ocel.objects.len();
    let object_index: HashMap<&str, usize> = ocel
        .objects
        .iter()
        .enumerate()
        .map(|(i, o)| (&*o.id, i))
        .collect();
    let event_index: HashMap<&str, usize> = ocel
        .events
        .iter()
        .enumerate()
        .map(|(i, e)| (&*e.id, i))
        .collect();

    // Each object's related events and each event's related objects, in
    // relation order.
    let mut lifecycle: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut event_objects: Vec<Vec<usize>> = vec![Vec::new(); ocel.events.len()];
    for r in &ocel.relations {
        if let (Some(&o), Some(&e)) = (object_index.get(&*r.object), event_index.get(&*r.event)) {
            lifecycle[o].push(e);
            event_objects[e].push(o);
        }
    }
    if let Some(e) = event_objects.iter().position(Vec::is_empty) {
        return Err(Error::EventWithoutObjects(ocel.events[e].id.to_string()));
    }
    let ids: Vec<&str> = ocel.objects.iter().map(|o| &*o.id).collect();

    let mut names: Vec<String> = Vec::new();
    let mut columns: Vec<Vec<f64>> = Vec::new();
    let mut push = |name: String, values: Vec<f64>| {
        names.push(name);
        columns.push(values);
    };

    push(
        "@@object_lifecycle_length".to_owned(),
        lifecycle.iter().map(|l| l.len() as f64).collect(),
    );
    let span: Vec<(f64, f64)> = lifecycle
        .iter()
        .map(|l| match (l.first(), l.last()) {
            (Some(&f), Some(&e)) => (
                seconds(&ocel.events[f].timestamp),
                seconds(&ocel.events[e].timestamp),
            ),
            _ => (0.0, 0.0),
        })
        .collect();
    push(
        "@@object_lifecycle_duration".to_owned(),
        span.iter().map(|(s, e)| e - s).collect(),
    );
    push(
        "@@object_lifecycle_start_timestamp".to_owned(),
        span.iter().map(|s| s.0).collect(),
    );
    push(
        "@@object_lifecycle_end_timestamp".to_owned(),
        span.iter().map(|s| s.1).collect(),
    );

    // The interaction graph: objects that share an event.
    let mut interaction: Vec<HashSet<usize>> = vec![HashSet::new(); n];
    for objs in &event_objects {
        for &a in objs {
            for &b in objs {
                if ids[a] < ids[b] {
                    interaction[a].insert(b);
                    interaction[b].insert(a);
                }
            }
        }
    }
    let nodes = interaction.iter().filter(|s| !s.is_empty()).count();
    push(
        "@@object_degree_centrality".to_owned(),
        interaction
            .iter()
            .map(|s| match nodes {
                _ if s.is_empty() => 0.0,
                1 => 1.0,
                _ => s.len() as f64 / (nodes - 1) as f64,
            })
            .collect(),
    );
    push(
        "@@object_general_interaction_graph".to_owned(),
        interaction.iter().map(|s| s.len() as f64).collect(),
    );

    let directed = |edges: &HashSet<(usize, usize)>,
                    names: [&str; 2],
                    push: &mut dyn FnMut(String, Vec<f64>)| {
        let mut asc = vec![0.0; n];
        let mut desc = vec![0.0; n];
        for &(a, b) in edges {
            desc[a] += 1.0;
            asc[b] += 1.0;
        }
        push(names[0].to_owned(), asc);
        push(names[1].to_owned(), desc);
    };
    directed(
        &descendants(&event_objects),
        [
            "@@object_general_descendants_graph_ascendants",
            "@@object_general_descendants_graph_descendants",
        ],
        &mut push,
    );
    directed(
        &inheritance(&event_objects, n),
        [
            "@@object_general_inheritance_graph_ascendants",
            "@@object_general_inheritance_graph_descendants",
        ],
        &mut push,
    );
    push(
        "@@object_cobirth".to_owned(),
        births(event_objects.iter(), &ids),
    );
    push(
        "@@object_codeath".to_owned(),
        births(event_objects.iter().rev(), &ids),
    );

    let mut activities: Vec<&str> = Vec::new();
    let mut seen_activities: HashSet<&str> = HashSet::new();
    for e in &ocel.events {
        if seen_activities.insert(&e.activity) {
            activities.push(&e.activity);
        }
    }
    let activity = |e: usize| &*ocel.events[e].activity;
    for act in &activities {
        push(
            format!("@@ocel_lif_activity_{act}"),
            lifecycle
                .iter()
                .map(|l| l.iter().filter(|&&e| activity(e) == *act).count() as f64)
                .collect(),
        );
    }

    for attr in &options.str_attributes {
        let values: Vec<Option<String>> = ocel
            .objects
            .iter()
            .map(|o| o.attributes.get(attr).map(ToString::to_string))
            .collect();
        let distinct: BTreeSet<&String> = values.iter().flatten().collect();
        for value in distinct {
            push(
                format!("@@object_attr_value_{attr}_{value}"),
                values
                    .iter()
                    .map(|v| f64::from(u8::from(v.as_ref() == Some(value))))
                    .collect(),
            );
        }
    }
    for attr in &options.num_attributes {
        let mut values = Vec::with_capacity(n);
        for o in &ocel.objects {
            values.push(match o.attributes.get(attr) {
                None => 0.0,
                Some(v) => v
                    .as_f64()
                    .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
                    .ok_or_else(|| Error::NotNumeric(attr.clone()))?,
            });
        }
        push(format!("@@event_num_{attr}"), values);
    }

    let mut types: Vec<&str> = Vec::new();
    for o in &ocel.objects {
        if !types.contains(&&*o.object_type) {
            types.push(&o.object_type);
        }
    }
    for ot in &types {
        push(
            format!("@@object_interaction_graph_{ot}"),
            interaction
                .iter()
                .map(|s| {
                    s.iter()
                        .filter(|&&b| &*ocel.objects[b].object_type == *ot)
                        .count() as f64
                })
                .collect(),
        );
    }

    if options.work_in_progress {
        let mut order: Vec<usize> = (0..n).collect();
        order.sort_by(|&a, &b| {
            span[a]
                .0
                .total_cmp(&span[b].0)
                .then(span[a].1.total_cmp(&span[b].1))
        });
        let mut wip = vec![0.0; n];
        for i in 0..n {
            let ct = span[order[i]].1;
            let j = (i + 1..n).find(|&j| span[order[j]].0 > ct).unwrap_or(n);
            wip[order[i]] = (j - i) as f64;
        }
        push("@@object_wip".to_owned(), wip);
    }

    push(
        "@@object_lifecycle_unq_act".to_owned(),
        lifecycle
            .iter()
            .map(|l| l.iter().map(|&e| activity(e)).collect::<HashSet<_>>().len() as f64)
            .collect(),
    );

    if options.lifecycle_paths {
        let paths: Vec<Vec<String>> = lifecycle
            .iter()
            .map(|l| {
                l.windows(2)
                    .map(|w| format!("{}##{}", activity(w[0]), activity(w[1])))
                    .collect()
            })
            .collect();
        let all: BTreeSet<&String> = paths.iter().flatten().collect();
        for p in all {
            push(
                format!("@@ocel_lif_path_{p}"),
                paths
                    .iter()
                    .map(|l| l.iter().filter(|x| *x == p).count() as f64)
                    .collect(),
            );
        }
    }

    let keep: Vec<usize> = (0..n)
        .filter(|&o| &*ocel.objects[o].object_type == object_type)
        .collect();
    if keep.is_empty() {
        return Ok(ObjectFeatures::default());
    }
    Ok(ObjectFeatures {
        object_ids: keep.iter().map(|&o| ids[o].to_owned()).collect(),
        rows: keep
            .iter()
            .map(|&o| columns.iter().map(|c| c[o]).collect())
            .collect(),
        names,
    })
}

/// Splits an event's objects into those seen before and those new, in
/// relation order without repeats.
fn split_seen(objs: &[usize], seen: &HashSet<usize>) -> (Vec<usize>, Vec<usize>) {
    let mut old = Vec::new();
    let mut new = Vec::new();
    for &o in objs {
        let side = if seen.contains(&o) {
            &mut old
        } else {
            &mut new
        };
        if !side.contains(&o) {
            side.push(o);
        }
    }
    (old, new)
}

/// pm4py's `object_descendants_graph`.
fn descendants(event_objects: &[Vec<usize>]) -> HashSet<(usize, usize)> {
    let mut graph = HashSet::new();
    let mut seen = HashSet::new();
    for objs in event_objects {
        let (old, new) = split_seen(objs, &seen);
        for &a in &old {
            for &b in &new {
                graph.insert((a, b));
            }
        }
        seen.extend(new);
    }
    graph
}

/// pm4py's `object_inheritance_graph`.
fn inheritance(event_objects: &[Vec<usize>], n: usize) -> HashSet<(usize, usize)> {
    let mut last = vec![usize::MAX; n];
    for (e, objs) in event_objects.iter().enumerate() {
        for &o in objs {
            last[o] = e;
        }
    }
    let mut graph = HashSet::new();
    let mut seen = HashSet::new();
    for (e, objs) in event_objects.iter().enumerate() {
        let (_, new) = split_seen(objs, &seen);
        for &b in &new {
            for &a in objs {
                if last[a] == e && a != b {
                    graph.insert((a, b));
                }
            }
            seen.insert(b);
        }
    }
    graph
        .iter()
        .filter(|&&(a, b)| !graph.contains(&(b, a)))
        .copied()
        .collect()
}

/// pm4py's `object_cobirth_graph` (events in order) and
/// `object_codeath_graph` (events in reverse): for each object, the number
/// of objects first seen in the same event.
fn births<'a>(events: impl Iterator<Item = &'a Vec<usize>>, ids: &[&str]) -> Vec<f64> {
    let mut conn: Vec<HashSet<usize>> = vec![HashSet::new(); ids.len()];
    let mut seen = HashSet::new();
    for objs in events {
        let (_, new) = split_seen(objs, &seen);
        for &a in &new {
            for &b in &new {
                if ids[a] < ids[b] {
                    conn[a].insert(b);
                    conn[b].insert(a);
                }
            }
        }
        seen.extend(new);
    }
    conn.iter().map(|s| s.len() as f64).collect()
}
