//! Object-centric directly-follows graphs (pm4py's `discover_ocdfg`,
//! `algo/discovery/ocel/ocdfg`).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt;
use std::sync::Arc;

use chrono::{DateTime, FixedOffset};

use crate::Ocel;

/// A function that gives the seconds from one timestamp to a later one.
pub type SecondsBetween = dyn Fn(DateTime<FixedOffset>, DateTime<FixedOffset>) -> f64 + Send + Sync;

/// How [`discover_ocdfg`] measures the time each edge takes.
#[derive(Clone, Default)]
pub enum OcdfgDurations {
    /// The elapsed seconds from the source event to the target event. The
    /// default. A target event earlier than its source gives a negative
    /// time, as in pm4py.
    #[default]
    Elapsed,
    /// The seconds a caller's function gives. pm4py's `business_hours=True`
    /// is `ichnos_stats::time::BusinessHours::seconds_between` here.
    Custom(Arc<SecondsBetween>),
    /// No durations (pm4py's `compute_edges_performance=False`).
    Skip,
}

impl fmt::Debug for OcdfgDurations {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OcdfgDurations::Elapsed => f.write_str("Elapsed"),
            OcdfgDurations::Custom(_) => f.write_str("Custom(..)"),
            OcdfgDurations::Skip => f.write_str("Skip"),
        }
    }
}

/// Options for [`discover_ocdfg`], with pm4py's defaults.
#[derive(Debug, Clone, Default)]
pub struct OcdfgOptions {
    /// How to measure edge durations. Default [`OcdfgDurations::Elapsed`].
    pub durations: OcdfgDurations,
}

/// Why [`discover_ocdfg`] failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OcdfgError {
    /// A relation names an event that is not in the events table.
    #[error("a relation names unknown event {0}")]
    UnknownEvent(String),
    /// A relation names an object that is not in the objects table.
    #[error("a relation names unknown object {0}")]
    UnknownObject(String),
}

/// The events and objects behind an activity of an [`Ocdfg`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OcdfgActivity {
    /// The events (pm4py's `events` metric).
    pub events: BTreeSet<Arc<str>>,
    /// The distinct objects (pm4py's `unique_objects`).
    pub unique_objects: BTreeSet<Arc<str>>,
    /// The `(event, object)` pairs (pm4py's `total_objects`).
    pub total_objects: BTreeSet<(Arc<str>, Arc<str>)>,
}

impl OcdfgActivity {
    fn add(&mut self, event: &Arc<str>, object: &Arc<str>) {
        self.events.insert(event.clone());
        self.unique_objects.insert(object.clone());
        self.total_objects.insert((event.clone(), object.clone()));
    }
}

/// The occurrences of one edge of an [`Ocdfg`]: an object goes from an
/// event of the source activity straight to an event of the target
/// activity.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OcdfgEdge {
    /// The `(source event, target event)` pairs (pm4py's `event_couples`).
    pub event_couples: BTreeSet<(Arc<str>, Arc<str>)>,
    /// The distinct objects (pm4py's `unique_objects`).
    pub unique_objects: BTreeSet<Arc<str>>,
    /// The `(source event, target event, object)` triples (pm4py's
    /// `total_objects`).
    pub total_objects: BTreeSet<(Arc<str>, Arc<str>, Arc<str>)>,
    /// The duration of each event couple in seconds, sorted (pm4py's
    /// `edges_performance["event_couples"]`). Empty with
    /// [`OcdfgDurations::Skip`].
    pub event_couples_durations: Vec<f64>,
    /// The duration of each triple in seconds, sorted (pm4py's
    /// `edges_performance["total_objects"]`). Empty with
    /// [`OcdfgDurations::Skip`].
    pub total_objects_durations: Vec<f64>,
}

/// Activities, each with its [`OcdfgActivity`].
pub type OcdfgActivities = BTreeMap<Arc<str>, OcdfgActivity>;

/// `(source, target)` activity edges, each with its [`OcdfgEdge`].
pub type OcdfgEdges = BTreeMap<(Arc<str>, Arc<str>), OcdfgEdge>;

/// An object-centric directly-follows graph (pm4py's `discover_ocdfg`
/// dictionary).
///
/// pm4py keys each part by metric first. Here each part is keyed by object
/// type and activity, and holds all the metrics.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Ocdfg {
    /// The activities of the events table.
    pub activities: BTreeSet<Arc<str>>,
    /// The object types of the objects table.
    pub object_types: BTreeSet<Arc<str>>,
    /// Each activity over all object types (pm4py's `activities_indep`).
    /// An activity without related objects is left out.
    pub activities_indep: OcdfgActivities,
    /// The activities of each object type (pm4py's `activities_ot`).
    pub activities_ot: BTreeMap<Arc<str>, OcdfgActivities>,
    /// The activity of each object's first event, by object type (pm4py's
    /// `start_activities`).
    pub start_activities: BTreeMap<Arc<str>, OcdfgActivities>,
    /// The activity of each object's last event, by object type (pm4py's
    /// `end_activities`).
    pub end_activities: BTreeMap<Arc<str>, OcdfgActivities>,
    /// The `(source, target)` activity edges of each object type (pm4py's
    /// `edges` and `edges_performance`).
    pub edges: BTreeMap<Arc<str>, OcdfgEdges>,
}

/// Discovers the object-centric directly-follows graph of a log, as pm4py's
/// `discover_ocdfg` does with its `classic` variant.
///
/// Each event–object relation counts once, however often it repeats. An
/// object's events are those of its relations: its first and last relation
/// give its start and end activity, and its edges follow the events in log
/// order. Where an event or object id repeats, its first row gives the
/// activity, timestamp or object type.
///
/// # Errors
///
/// [`OcdfgError::UnknownEvent`] or [`OcdfgError::UnknownObject`] when a
/// relation names an event or object that the log does not have.
pub fn discover_ocdfg(ocel: &Ocel, options: &OcdfgOptions) -> Result<Ocdfg, OcdfgError> {
    let event_index = ocel.event_index();
    let object_index = ocel.object_index();
    // Each relation once, as (event row, object row), in relation order.
    let mut seen = HashSet::new();
    let mut relations: Vec<(usize, usize)> = Vec::new();
    for r in &ocel.relations {
        let e = *event_index
            .get(&*r.event)
            .ok_or_else(|| OcdfgError::UnknownEvent(r.event.to_string()))?;
        let o = *object_index
            .get(&*r.object)
            .ok_or_else(|| OcdfgError::UnknownObject(r.object.to_string()))?;
        if seen.insert((e, o)) {
            relations.push((e, o));
        }
    }

    let mut ocdfg = Ocdfg {
        activities: ocel.events.iter().map(|e| e.activity.clone()).collect(),
        object_types: ocel.objects.iter().map(|o| o.object_type.clone()).collect(),
        ..Ocdfg::default()
    };
    let add = |part: &mut BTreeMap<Arc<str>, OcdfgActivities>, e: usize, o: usize| {
        let (event, object) = (&ocel.events[e], &ocel.objects[o]);
        part.entry(object.object_type.clone())
            .or_default()
            .entry(event.activity.clone())
            .or_default()
            .add(&event.id, &object.id);
    };
    let mut first: HashMap<usize, usize> = HashMap::new();
    let mut last: HashMap<usize, usize> = HashMap::new();
    // The objects of each event row, in relation order.
    let mut objects_of: HashMap<usize, Vec<usize>> = HashMap::new();
    for &(e, o) in &relations {
        let event = &ocel.events[e];
        ocdfg
            .activities_indep
            .entry(event.activity.clone())
            .or_default()
            .add(&event.id, &ocel.objects[o].id);
        add(&mut ocdfg.activities_ot, e, o);
        first.entry(o).or_insert(e);
        last.insert(o, e);
        objects_of.entry(e).or_default().push(o);
    }
    for (o, e) in first {
        add(&mut ocdfg.start_activities, e, o);
    }
    for (o, e) in last {
        add(&mut ocdfg.end_activities, e, o);
    }

    // The last event row of each object so far. A repeated event row is
    // visited again, as in pm4py.
    let mut history: HashMap<usize, usize> = HashMap::new();
    for row in &ocel.events {
        let e = event_index[&*row.id];
        let Some(objects) = objects_of.get(&e) else {
            continue;
        };
        for &o in objects {
            let Some(prev) = history.insert(o, e) else {
                continue;
            };
            let (source, target, object) = (&ocel.events[prev], &ocel.events[e], &ocel.objects[o]);
            let edge = ocdfg
                .edges
                .entry(object.object_type.clone())
                .or_default()
                .entry((source.activity.clone(), target.activity.clone()))
                .or_default();
            edge.event_couples
                .insert((source.id.clone(), target.id.clone()));
            edge.unique_objects.insert(object.id.clone());
            edge.total_objects
                .insert((source.id.clone(), target.id.clone(), object.id.clone()));
        }
    }

    let seconds: &SecondsBetween = match &options.durations {
        OcdfgDurations::Skip => return Ok(ocdfg),
        OcdfgDurations::Elapsed => &elapsed_seconds,
        OcdfgDurations::Custom(f) => f.as_ref(),
    };
    let time = |id: &str| ocel.events[event_index[id]].timestamp;
    for edges in ocdfg.edges.values_mut() {
        for edge in edges.values_mut() {
            edge.event_couples_durations = sorted(
                edge.event_couples
                    .iter()
                    .map(|(a, b)| seconds(time(a), time(b))),
            );
            edge.total_objects_durations = sorted(
                edge.total_objects
                    .iter()
                    .map(|(a, b, _)| seconds(time(a), time(b))),
            );
        }
    }
    Ok(ocdfg)
}

/// The elapsed seconds from `from` to `to`, with the fraction floored to
/// whole microseconds as pandas' `Timedelta.total_seconds` does.
fn elapsed_seconds(from: DateTime<FixedOffset>, to: DateTime<FixedOffset>) -> f64 {
    let delta = to.signed_duration_since(from);
    delta.num_seconds() as f64 + i64::from(delta.subsec_nanos()).div_euclid(1000) as f64 / 1e6
}

fn sorted(values: impl Iterator<Item = f64>) -> Vec<f64> {
    let mut values: Vec<f64> = values.collect();
    values.sort_by(f64::total_cmp);
    values
}
