//! Canonical row/Arrow counterpart of pm4py streaming from_pandas.apply.

use crate::{Error, Result, StreamSink};
use arrow::record_batch::RecordBatch;
use ichnos_core::{Event, EventKeys, EventStream, Trace};
use std::collections::BTreeMap;

/// Resettable traces projected from a canonical event table.
///
/// Cases have lexical core-display order; events retain input order within
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
        let mut cases = BTreeMap::<String, Trace>::new();
        for (i, input) in stream.events.iter().enumerate() {
            let field = |key: &str| {
                input.get(key).ok_or_else(|| Error::MissingField {
                    key: key.to_owned(),
                    event: i,
                })
            };
            let case_value = field(&keys.case_id)?;
            let case = case_value.to_string();
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
