//! Canonical row/Arrow counterpart of pm4py streaming from_pandas.apply.

use crate::{Error, Result, StreamSink};
use arrow::record_batch::RecordBatch;
use ichnos_core::{AttributeValue, Event, EventKeys, EventStream, Trace};
use std::{cmp::Ordering, collections::BTreeMap};

/// Resettable traces projected from a canonical event table.
///
/// Cases sort by typed value (numbers numerically, strings lexically); events retain input order within
/// each case. Only activity/timestamp columns are emitted, under standard XES
/// keys, as in pm4py. Interleaved rows are grouped correctly rather than using
/// pm4py's contiguous-slice assumption. The input table is not modified.
#[derive(Debug, Clone)]
pub struct TraceIterator {
    traces: Vec<Trace>,
    position: usize,
}

impl TraceIterator {
    /// Group canonical event rows using the supplied column keys.
    pub fn from_event_stream(stream: &EventStream, keys: &EventKeys) -> Result<Self> {
        let mut cases = BTreeMap::<CaseKey, Trace>::new();
        for (i, input) in stream.events.iter().enumerate() {
            let field = |key: &str| {
                input.get(key).ok_or_else(|| Error::MissingField {
                    key: key.to_owned(),
                    event: i,
                })
            };
            let case_value = field(&keys.case_id)?;
            let case = CaseKey(case_value.clone());
            let mut event = Event::new();
            event.insert("concept:name", field(&keys.activity)?.clone());
            event.insert("time:timestamp", field(&keys.timestamp)?.clone());
            cases
                .entry(case)
                .or_insert_with(|| {
                    let mut trace = Trace::new();
                    trace.attributes.insert("concept:name", case_value.clone());
                    trace
                })
                .events
                .push(event);
        }
        Ok(Self {
            traces: cases.into_values().collect(),
            position: 0,
        })
    }

    /// Convert an Arrow batch through core's canonical event conversion.
    pub fn from_record_batch(batch: &RecordBatch, keys: &EventKeys) -> Result<Self> {
        Self::from_event_stream(&EventStream::from_arrow(batch)?, keys)
    }

    /// Rewind to the first trace without rebuilding the projection.
    pub fn reset(&mut self) {
        self.position = 0;
    }

    /// Read the next trace, or None after the projection is exhausted.
    pub fn read_trace(&mut self) -> Option<Trace> {
        self.next()
    }

    /// Deliver the remaining traces to a push-based consumer.
    pub fn to_trace_stream(&mut self, sink: &mut impl StreamSink<Trace>) -> Result<usize> {
        let mut count = 0;
        for trace in self {
            sink.push(&trace)?;
            count += 1;
        }
        Ok(count)
    }
}

impl Iterator for TraceIterator {
    type Item = Trace;
    fn next(&mut self) -> Option<Trace> {
        let trace = self.traces.get(self.position)?.clone();
        self.position += 1;
        Some(trace)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.traces.len() - self.position;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for TraceIterator {}
impl std::iter::FusedIterator for TraceIterator {}

// Typed identity avoids display collisions and follows np.unique ordering for
// homogeneous columns. Mixed core variants retain distinct identities.
#[derive(Debug, Clone)]
struct CaseKey(AttributeValue);

impl PartialEq for CaseKey {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}
impl Eq for CaseKey {}
impl PartialOrd for CaseKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for CaseKey {
    fn cmp(&self, other: &Self) -> Ordering {
        compare_value(&self.0, &other.0)
    }
}

fn compare_float(a: f64, b: f64) -> Ordering {
    match (a.is_nan(), b.is_nan()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => a.partial_cmp(&b).expect("non-NaN floats"),
    }
}

// Compare without converting i64 to f64: that would merge adjacent IDs above
// 2^53. Cast only after checking the float lies within the integer range.
fn compare_int_float(a: i64, b: f64) -> Ordering {
    if b.is_nan() || b >= i64::MAX as f64 {
        return Ordering::Less;
    }
    if b < i64::MIN as f64 {
        return Ordering::Greater;
    }
    a.cmp(&(b as i64))
        .then_with(|| compare_float(0.0, b.fract()))
}

fn compare_attributes(a: &ichnos_core::Attributes, b: &ichnos_core::Attributes) -> Ordering {
    let mut a: Vec<_> = a.iter().collect();
    let mut b: Vec<_> = b.iter().collect();
    a.sort_by_key(|(key, _)| *key);
    b.sort_by_key(|(key, _)| *key);
    for ((ak, av), (bk, bv)) in a.iter().zip(&b) {
        let ordering = ak.cmp(bk).then_with(|| compare_value(av, bv));
        if !ordering.is_eq() {
            return ordering;
        }
    }
    a.len().cmp(&b.len())
}

fn compare_value(a: &AttributeValue, b: &AttributeValue) -> Ordering {
    use AttributeValue::*;
    let rank = |value: &AttributeValue| match value {
        Bool(_) => 0,
        Int(_) | Float(_) => 1,
        String(_) => 2,
        Id(_) => 3,
        Date(_) => 4,
        List(_) => 5,
        Container(_) => 6,
        Meta(_) => 7,
    };
    let order = rank(a).cmp(&rank(b));
    if !order.is_eq() {
        return order;
    }
    match (a, b) {
        (Bool(a), Bool(b)) => a.cmp(b),
        (Int(a), Int(b)) => a.cmp(b),
        (Float(a), Float(b)) => compare_float(*a, *b),
        (Int(a), Float(b)) => compare_int_float(*a, *b).then(Ordering::Less),
        (Float(a), Int(b)) => compare_int_float(*b, *a).reverse().then(Ordering::Greater),
        (String(a), String(b)) | (Id(a), Id(b)) => a.cmp(b),
        (Date(a), Date(b)) => a.cmp(b),
        (List(a), List(b)) => {
            for ((ak, av), (bk, bv)) in a.iter().zip(b) {
                let order = ak.cmp(bk).then_with(|| compare_value(av, bv));
                if !order.is_eq() {
                    return order;
                }
            }
            a.len().cmp(&b.len())
        }
        (Container(a), Container(b)) => compare_attributes(a, b),
        (Meta(a), Meta(b)) => {
            compare_value(&a.value, &b.value).then_with(|| compare_attributes(&a.meta, &b.meta))
        }
        _ => unreachable!("equal ranks have matching variants except numbers"),
    }
}
