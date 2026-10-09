//! Interned activity sequences and variants: the cheap projection most miners
//! work on.

use std::borrow::Cow;
use std::ops::Index;
use std::sync::Arc;

use rustc_hash::FxHashMap;

use crate::error::{Error, Position, Result};
use crate::keys::EventKeys;
use crate::log::EventLog;

/// A dense activity ID: an index into an [`ActivityIndex`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ActivityId(pub u32);

impl ActivityId {
    /// The ID as a `usize`, for indexing vectors.
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// An interner that maps activity names to dense [`ActivityId`]s, numbered in
/// order of first appearance.
#[derive(Debug, Clone, Default)]
pub struct ActivityIndex {
    names: Vec<Arc<str>>,
    ids: FxHashMap<Arc<str>, ActivityId>,
}

impl ActivityIndex {
    /// An empty index.
    pub fn new() -> Self {
        Self::default()
    }

    /// The ID of `name`, adding it if it is new.
    pub fn intern(&mut self, name: &str) -> ActivityId {
        if let Some(&id) = self.ids.get(name) {
            return id;
        }
        let id = ActivityId(u32::try_from(self.names.len()).expect("fewer than 2^32 activities"));
        let name: Arc<str> = name.into();
        self.names.push(name.clone());
        self.ids.insert(name, id);
        id
    }

    /// The ID of `name`, if interned.
    pub fn get(&self, name: &str) -> Option<ActivityId> {
        self.ids.get(name).copied()
    }

    /// The name of `id`. Panics if `id` is not from this index.
    pub fn name(&self, id: ActivityId) -> &str {
        &self.names[id.index()]
    }

    /// The number of distinct activities.
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// Iterates over `(id, name)` in ID order.
    pub fn iter(&self) -> impl Iterator<Item = (ActivityId, &str)> {
        self.names
            .iter()
            .enumerate()
            .map(|(i, n)| (ActivityId(i as u32), &**n))
    }
}

impl Index<ActivityId> for ActivityIndex {
    type Output = str;

    fn index(&self, id: ActivityId) -> &str {
        self.name(id)
    }
}

/// Every trace of a log as a sequence of activity IDs.
#[derive(Debug, Clone, Default)]
pub struct ActivitySequences {
    /// The activity names.
    pub activities: ActivityIndex,
    /// One sequence per trace, in log order.
    pub traces: Vec<Vec<ActivityId>>,
}

/// One variant: a distinct activity sequence and the traces that follow it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    /// The activity sequence.
    pub activities: Vec<ActivityId>,
    /// Indices of the traces with this sequence, ascending.
    pub traces: Vec<usize>,
}

impl Variant {
    /// The number of traces with this sequence.
    pub fn count(&self) -> usize {
        self.traces.len()
    }
}

/// The variants of a log, in order of first appearance.
#[derive(Debug, Clone, Default)]
pub struct Variants {
    /// The activity names.
    pub activities: ActivityIndex,
    /// The variants, in order of their first trace.
    pub variants: Vec<Variant>,
}

impl Variants {
    /// Groups activity sequences into variants.
    pub fn from_sequences(sequences: ActivitySequences) -> Self {
        let mut index: FxHashMap<Vec<ActivityId>, usize> = FxHashMap::default();
        let mut variants: Vec<Variant> = Vec::new();
        for (t, seq) in sequences.traces.into_iter().enumerate() {
            match index.get(&seq) {
                Some(&v) => variants[v].traces.push(t),
                None => {
                    index.insert(seq.clone(), variants.len());
                    variants.push(Variant {
                        activities: seq,
                        traces: vec![t],
                    });
                }
            }
        }
        Self {
            activities: sequences.activities,
            variants,
        }
    }

    /// The number of variants.
    pub fn len(&self) -> usize {
        self.variants.len()
    }

    /// Whether there are no variants.
    pub fn is_empty(&self) -> bool {
        self.variants.is_empty()
    }

    /// Iterates over the variants.
    pub fn iter(&self) -> std::slice::Iter<'_, Variant> {
        self.variants.iter()
    }

    /// The activity names of `variant`.
    pub fn names<'a>(&'a self, variant: &'a Variant) -> impl Iterator<Item = &'a str> {
        variant
            .activities
            .iter()
            .map(|&id| self.activities.name(id))
    }
}

impl EventLog {
    /// Every trace as a sequence of interned activity IDs.
    ///
    /// Reads the activity from `keys.activity`. A non-string activity is
    /// formatted as Python's `str()` would. Fails if an event has no activity.
    pub fn activity_sequences(&self, keys: &EventKeys) -> Result<ActivitySequences> {
        let mut activities = ActivityIndex::new();
        let traces =
            self.traces
                .iter()
                .enumerate()
                .map(|(t, trace)| {
                    trace
                        .events
                        .iter()
                        .enumerate()
                        .map(|(e, event)| {
                            let value = event.get(&keys.activity).ok_or_else(|| {
                                Error::MissingAttribute {
                                    key: keys.activity.clone(),
                                    position: Position::Event { trace: t, event: e },
                                }
                            })?;
                            let name = match value.as_str() {
                                Some(s) => Cow::Borrowed(s),
                                None => Cow::Owned(value.to_string()),
                            };
                            Ok(activities.intern(&name))
                        })
                        .collect::<Result<Vec<_>>>()
                })
                .collect::<Result<Vec<_>>>()?;
        Ok(ActivitySequences { activities, traces })
    }

    /// The variants of the log, keyed by interned activity sequence, in order
    /// of first appearance.
    pub fn variants(&self, keys: &EventKeys) -> Result<Variants> {
        Ok(Variants::from_sequences(self.activity_sequences(keys)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::attribute::AttributeValue;

    #[test]
    fn interning_is_dense_and_stable() {
        let mut index = ActivityIndex::new();
        let a = index.intern("a");
        let b = index.intern("b");
        assert_eq!(index.intern("a"), a);
        assert_eq!((a.0, b.0), (0, 1));
        assert_eq!(&index[b], "b");
        assert_eq!(index.get("c"), None);
        assert_eq!(index.len(), 2);
    }

    #[test]
    fn variants_count_traces() {
        let keys = EventKeys::default();
        let log =
            EventLog::from_trace_strings(["A,B,C", "A,C", "A,B,C", "A,C", "A,B,C"], ",", &keys);
        let variants = log.variants(&keys).unwrap();
        assert_eq!(variants.len(), 2);
        assert_eq!(variants.variants[0].count(), 3);
        assert_eq!(variants.variants[0].traces, [0, 2, 4]);
        let names: Vec<&str> = variants.names(&variants.variants[1]).collect();
        assert_eq!(names, ["A", "C"]);
    }

    #[test]
    fn non_string_activities_are_formatted() {
        let keys = EventKeys::default();
        let mut log = EventLog::from_trace_strings(["A"], ",", &keys);
        log.traces[0].events[0].insert("concept:name", AttributeValue::from(7));
        let seqs = log.activity_sequences(&keys).unwrap();
        assert_eq!(seqs.activities.name(seqs.traces[0][0]), "7");
    }

    #[test]
    fn missing_activity_is_an_error() {
        let keys = EventKeys::default().with_activity("task");
        let log = EventLog::from_trace_strings(["A"], ",", &EventKeys::default());
        let err = log.activity_sequences(&keys).unwrap_err();
        assert!(matches!(err, Error::MissingAttribute { ref key, .. } if key == "task"));
    }
}
