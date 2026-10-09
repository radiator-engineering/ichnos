//! Directly-follows edge occurrences and performance observations.
use super::*;
use crate::time::BusinessHours;
use ichnos_core::chrono::{DateTime, FixedOffset, Timelike};

/// Build edge occurrences in event order, with distinct related objects in relation order.
/// Duplicate event rows are visited again; activity/type lookup uses the first row per id.
pub fn find_associations_per_edge(log: &Ocel) -> Result<EdgeAssociations> {
    let events = log.event_index();
    let objects = log.object_index();
    let mut seen = BTreeSet::new();
    let mut related: BTreeMap<&str, Vec<&Id>> = BTreeMap::new();
    for r in &log.relations {
        if !events.contains_key(r.event.as_ref()) {
            return Err(Error::MissingOcelEvent(r.event.to_string()));
        }
        if !objects.contains_key(r.object.as_ref()) {
            return Err(Error::MissingOcelObject(r.object.to_string()));
        }
        if seen.insert((&r.event, &r.object)) {
            related.entry(&r.event).or_default().push(&r.object);
        }
    }
    let mut history: BTreeMap<&Id, &Id> = BTreeMap::new();
    let mut result: EdgeAssociations = BTreeMap::new();
    for e in &log.events {
        if let Some(related) = related.get(e.id.as_ref()) {
            for &o in related {
                if let Some(previous) = history.insert(o, &e.id) {
                    let t = &log.objects[objects[o.as_ref()]].object_type;
                    let source = &log.events[events[previous.as_ref()]].activity;
                    let target = &log.events[events[e.id.as_ref()]].activity;
                    result
                        .entry(t.clone())
                        .or_default()
                        .entry((source.clone(), target.clone()))
                        .or_default()
                        .push((previous.clone(), e.id.clone(), o.clone()));
                }
            }
        }
    }
    Ok(result)
}

/// Distinct source/target event pairs for each edge.
pub fn aggregate_ev_couples(edges: &EdgeAssociations) -> EdgeMetric<BTreeSet<Association>> {
    edges
        .iter()
        .map(|(t, values)| {
            (
                t.clone(),
                values
                    .iter()
                    .map(|(edge, rows)| {
                        (
                            edge.clone(),
                            rows.iter()
                                .map(|(a, b, _)| (a.clone(), b.clone()))
                                .collect(),
                        )
                    })
                    .collect(),
            )
        })
        .collect()
}
/// Distinct object identifiers for each edge.
pub fn aggregate_unique_objects(edges: &EdgeAssociations) -> EdgeMetric<BTreeSet<Id>> {
    edges
        .iter()
        .map(|(t, values)| {
            (
                t.clone(),
                values
                    .iter()
                    .map(|(edge, rows)| {
                        (
                            edge.clone(),
                            rows.iter().map(|(_, _, o)| o.clone()).collect(),
                        )
                    })
                    .collect(),
            )
        })
        .collect()
}
/// Distinct source/target event/object triples for each edge.
pub fn aggregate_total_objects(edges: &EdgeAssociations) -> EdgeMetric<BTreeSet<EdgeOccurrence>> {
    edges
        .iter()
        .map(|(t, values)| {
            (
                t.clone(),
                values
                    .iter()
                    .map(|(edge, rows)| (edge.clone(), rows.iter().cloned().collect()))
                    .collect(),
            )
        })
        .collect()
}

/// An occurrence with source and target event ids, usable for edge performance.
pub trait EventPair {
    /// Source event identifier.
    fn source(&self) -> &str;
    /// Target event identifier.
    fn target(&self) -> &str;
}
impl EventPair for Association {
    fn source(&self) -> &str {
        &self.0
    }
    fn target(&self) -> &str {
        &self.1
    }
}
impl EventPair for EdgeOccurrence {
    fn source(&self) -> &str {
        &self.0
    }
    fn target(&self) -> &str {
        &self.1
    }
}

/// Sorted elapsed seconds for event-pair or event/object-triple aggregations.
/// Raw elapsed times floor submicrosecond fractions, and may be negative. Business hours use wall-clock dates and
/// clamp reversed intervals to zero and truncate timestamps to microseconds. Duplicate event ids use the first timestamp.
/// Triple aggregation retains separate observations for different objects.
/// An invalid schedule is rejected even when the aggregation is empty.
pub fn performance_calculation_ocel_aggregation<T: EventPair + Ord>(
    log: &Ocel,
    aggregation: &EdgeMetric<BTreeSet<T>>,
    business_hours: Option<&BusinessHours>,
) -> Result<EdgeMetric<Vec<f64>>> {
    if let Some(schedule) = business_hours {
        schedule.validate()?;
    }
    let events = log.event_index();
    let timestamp = |id: &str| {
        events
            .get(id)
            .map(|i| log.events[*i].timestamp)
            .ok_or_else(|| Error::MissingOcelEvent(id.to_owned()))
    };
    aggregation
        .iter()
        .map(|(t, edges)| {
            let values = edges
                .iter()
                .map(|(edge, rows)| {
                    let mut seconds = rows
                        .iter()
                        .map(|r| {
                            let start = timestamp(r.source())?;
                            let end = timestamp(r.target())?;
                            if let Some(schedule) = business_hours {
                                let microsecond_time = |time: DateTime<FixedOffset>| {
                                    time.with_nanosecond(time.nanosecond() / 1000 * 1000)
                                        .expect("flooring valid nanoseconds stays valid")
                                };
                                // Python business hours operates on datetime microseconds.
                                schedule
                                    .seconds_between(microsecond_time(start), microsecond_time(end))
                            } else {
                                let delta = end.signed_duration_since(start);
                                // pandas Timedelta.total_seconds floors its fractional microseconds.
                                Ok(delta.num_seconds() as f64
                                    + i64::from(delta.subsec_nanos()).div_euclid(1000) as f64 / 1e6)
                            }
                        })
                        .collect::<Result<Vec<_>>>()?;
                    seconds.sort_by(f64::total_cmp);
                    Ok((edge.clone(), seconds))
                })
                .collect::<Result<BTreeMap<_, _>>>()?;
            Ok((t.clone(), values))
        })
        .collect()
}
