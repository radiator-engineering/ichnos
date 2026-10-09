//! The summary of an object-centric event log (pm4py's `OCEL.get_summary`).

use std::cmp::Reverse;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::sync::Arc;

use ichnos_core::python::repr;

use crate::ocel::Ocel;

/// Counts that describe an object-centric event log (pm4py's
/// `OCEL.get_summary`).
///
/// Its [`Display`](fmt::Display) gives pm4py's text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcelSummary {
    /// The number of events.
    pub events: usize,
    /// The number of objects.
    pub objects: usize,
    /// The number of distinct activities.
    pub activities: usize,
    /// The number of distinct object types.
    pub object_types: usize,
    /// The number of event-to-object relations.
    pub relations: usize,
    /// The number of events of each activity, most frequent first. Ties
    /// keep the order in which the activities first appear.
    pub activity_counts: Vec<(Arc<str>, usize)>,
    /// The number of objects of each type, most frequent first. Ties keep
    /// the order in which the types first appear.
    pub object_type_counts: Vec<(Arc<str>, usize)>,
    /// The number of distinct activities related to objects of each type,
    /// highest first. Ties are in type order.
    pub activities_per_object_type: Vec<(Arc<str>, usize)>,
}

impl Ocel {
    /// Counts that describe the log (pm4py's `OCEL.get_summary`).
    ///
    /// The activities per object type come from the relations whose event
    /// and object are both in the log.
    pub fn summary(&self) -> OcelSummary {
        let activity_counts = counts(self.events.iter().map(|e| &e.activity));
        let object_type_counts = counts(self.objects.iter().map(|o| &o.object_type));
        let events = self.event_index();
        let objects = self.object_index();
        let mut per_type: BTreeMap<&Arc<str>, HashSet<&Arc<str>>> = BTreeMap::new();
        for r in &self.relations {
            if let (Some(&e), Some(&o)) = (events.get(&*r.event), objects.get(&*r.object)) {
                per_type
                    .entry(&self.objects[o].object_type)
                    .or_default()
                    .insert(&self.events[e].activity);
            }
        }
        let mut activities_per_object_type: Vec<(Arc<str>, usize)> = per_type
            .into_iter()
            .map(|(t, activities)| (t.clone(), activities.len()))
            .collect();
        activities_per_object_type.sort_by_key(|(_, n)| Reverse(*n));
        OcelSummary {
            events: self.events.len(),
            objects: self.objects.len(),
            activities: activity_counts.len(),
            object_types: object_type_counts.len(),
            relations: self.relations.len(),
            activity_counts,
            object_type_counts,
            activities_per_object_type,
        }
    }
}

/// How often each value occurs, most frequent first, with ties in order of
/// first appearance (pandas' `value_counts`).
fn counts<'a>(values: impl Iterator<Item = &'a Arc<str>>) -> Vec<(Arc<str>, usize)> {
    let mut index: HashMap<&Arc<str>, usize> = HashMap::new();
    let mut result: Vec<(Arc<str>, usize)> = Vec::new();
    for v in values {
        let i = *index.entry(v).or_insert_with(|| {
            result.push((v.clone(), 0));
            result.len() - 1
        });
        result[i].1 += 1;
    }
    result.sort_by_key(|(_, n)| Reverse(*n));
    result
}

impl fmt::Display for OcelSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Object-Centric Event Log (number of events: {}, number of objects: {}, \
             number of activities: {}, number of object types: {}, \
             events-objects relationships: {})",
            self.events, self.objects, self.activities, self.object_types, self.relations
        )?;
        write!(f, "Activities occurrences: ")?;
        counter(f, &self.activity_counts)?;
        write!(f, "\nObject types occurrences (number of objects): ")?;
        counter(f, &self.object_type_counts)?;
        write!(f, "\nUnique activities per object type: ")?;
        counter(f, &self.activities_per_object_type)?;
        write!(
            f,
            "\nPlease use <THIS>.get_extended_table() to get a dataframe \
             representation of the events related to the objects."
        )
    }
}

/// The summary text, as pm4py's `OCEL.__str__` and `__repr__` give it.
impl fmt::Display for Ocel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.summary().fmt(f)
    }
}

/// Writes counts as Python prints a `Counter`.
fn counter(f: &mut fmt::Formatter<'_>, counts: &[(Arc<str>, usize)]) -> fmt::Result {
    if counts.is_empty() {
        return write!(f, "Counter()");
    }
    write!(f, "Counter({{")?;
    for (i, (key, count)) in counts.iter().enumerate() {
        if i > 0 {
            write!(f, ", ")?;
        }
        write!(f, "{}: {count}", repr(key))?;
    }
    write!(f, "}})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_log() {
        let text = Ocel::new().summary().to_string();
        assert!(text.contains("Activities occurrences: Counter()\n"));
        assert_eq!(Ocel::new().to_string(), text);
    }
}
