//! Batch detection by activity/resource, ported from pm4py's
//! `algo.discovery.batches.utils.detection` interval merge rules.

use crate::{Error, Result};
use ichnos_core::{Event, EventKeys, EventLog, Position};
use ichnos_model::Label;
use std::collections::BTreeMap;

/// Category assigned in precedence order: simultaneous, start, end, sequential, concurrent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BatchType {
    /// Equal start and completion times.
    Simultaneous,
    /// Equal start times.
    Start,
    /// Equal completion times.
    End,
    /// Each completion equals the next start.
    Sequential,
    /// All other merged groups, including groups separated by small gaps.
    Concurrent,
}

impl BatchType {
    /// Category name returned by pm4py batch detection.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Simultaneous => "Simultaneous",
            Self::Start => "Batching on Start",
            Self::End => "Batching on End",
            Self::Sequential => "Sequential batching",
            Self::Concurrent => "Concurrent batching",
        }
    }
}

/// An event identity in a batch: timestamps and trace case label.
#[derive(Debug, Clone, PartialEq)]
pub struct BatchEvent {
    /// Start, in floating epoch seconds as in pm4py.
    pub start: f64,
    /// Completion, in floating epoch seconds.
    pub end: f64,
    /// Case identifier from the trace attribute configured in the options.
    pub case: Label,
}

/// One merged interval with deduplicated, sorted event identities.
#[derive(Debug, Clone, PartialEq)]
pub struct Batch {
    /// Earliest start.
    pub start: f64,
    /// Latest completion.
    pub end: f64,
    /// Distinct `(start, end, case)` events; identical observations collapse.
    pub events: Vec<BatchEvent>,
}

/// Batches for an activity/resource pair. Groups are returned in descending count/pair order.
#[derive(Debug, Clone, PartialEq)]
pub struct BatchGroup {
    /// Activity label.
    pub activity: Label,
    /// Resource label (string or ID).
    pub resource: Label,
    /// Categories with at least one retained batch.
    pub batches: BTreeMap<BatchType, Vec<Batch>>,
}

impl BatchGroup {
    /// Total number of retained batches across categories.
    pub fn count(&self) -> usize {
        self.batches.values().map(Vec::len).sum()
    }
}

/// Batch discovery options.
#[derive(Debug, Clone)]
pub struct BatchOptions {
    /// Maximum gap in seconds (default 900); must be finite and nonnegative.
    pub merge_distance: f64,
    /// Minimum distinct event count (default 2); must be positive.
    pub min_batch_size: usize,
    /// Read `keys.start_timestamp` instead of completion as start.
    pub use_start_timestamp: bool,
    /// Trace attribute holding case identity (default `concept:name`).
    pub case_attribute: String,
}

impl Default for BatchOptions {
    fn default() -> Self {
        Self {
            merge_distance: 900.0,
            min_batch_size: 2,
            use_start_timestamp: false,
            case_attribute: "concept:name".into(),
        }
    }
}

pub(crate) fn timestamp(event: &Event, key: &str, position: Position) -> Result<f64> {
    let value = event
        .get(key)
        .ok_or_else(|| ichnos_core::Error::MissingAttribute {
            key: key.into(),
            position,
        })?;
    let date = value
        .as_date()
        .ok_or_else(|| ichnos_core::Error::AttributeType {
            key: key.into(),
            position,
            expected: "date",
            found: value.type_name(),
        })?;
    Ok(ichnos_stats::time::datetime_timestamp(date))
}

fn text(
    value: Option<&ichnos_core::AttributeValue>,
    key: &str,
    position: Position,
) -> Result<Label> {
    let value = value.ok_or_else(|| ichnos_core::Error::MissingAttribute {
        key: key.into(),
        position,
    })?;
    Ok(Label::from(value.as_str().ok_or_else(|| {
        ichnos_core::Error::AttributeType {
            key: key.into(),
            position,
            expected: "string or id",
            found: value.type_name(),
        }
    })?))
}

fn event_cmp(a: &BatchEvent, b: &BatchEvent) -> std::cmp::Ordering {
    a.start
        .total_cmp(&b.start)
        .then(a.end.total_cmp(&b.end))
        .then(a.case.cmp(&b.case))
}

// Python tuple comparison reaches set comparison only when endpoints tie.
fn less(a: &Batch, b: &Batch) -> bool {
    if a.start != b.start {
        return a.start < b.start;
    }
    if a.end != b.end {
        return a.end < b.end;
    }
    a.events.len() < b.events.len() && a.events.iter().all(|e| b.events.contains(e))
}

fn sift_down(heap: &mut [Batch], start: usize, mut pos: usize) {
    let item = heap[pos].clone();
    while pos > start {
        let parent = (pos - 1) / 2;
        if !less(&item, &heap[parent]) {
            break;
        }
        heap[pos] = heap[parent].clone();
        pos = parent;
    }
    heap[pos] = item;
}

fn sift_up(heap: &mut [Batch], mut pos: usize) {
    let start = pos;
    let item = heap[pos].clone();
    let mut child = 2 * pos + 1;
    while child < heap.len() {
        let right = child + 1;
        if right < heap.len() && !less(&heap[child], &heap[right]) {
            child = right;
        }
        heap[pos] = heap[child].clone();
        pos = child;
        child = 2 * pos + 1;
    }
    heap[pos] = item;
    sift_down(heap, start, pos);
}

fn merge(a: Batch, b: Batch) -> Batch {
    let mut events = a.events;
    events.extend(b.events);
    events.sort_by(event_cmp);
    events.dedup();
    Batch {
        start: a.start.min(b.start),
        end: a.end.max(b.end),
        events,
    }
}

fn category(batch: &Batch) -> BatchType {
    let events = &batch.events;
    let first = &events[0];
    let same_start = events.iter().all(|e| e.start == first.start);
    let same_end = events.iter().all(|e| e.end == first.end);
    if same_start && same_end {
        BatchType::Simultaneous
    } else if same_start {
        BatchType::Start
    } else if same_end {
        BatchType::End
    } else if events.windows(2).all(|w| w[0].end == w[1].start) {
        BatchType::Sequential
    } else {
        BatchType::Concurrent
    }
}

/// Detect batches without altering input. Uses completion times as starts by default.
/// Missing resource/case/timestamp fields return positional core errors. The
/// pm4py heap merge order and event identity deduplication are preserved.
pub fn discover_batches(
    log: &EventLog,
    keys: &EventKeys,
    options: &BatchOptions,
) -> Result<Vec<BatchGroup>> {
    if !options.merge_distance.is_finite()
        || options.merge_distance < 0.0
        || options.min_batch_size == 0
    {
        return Err(Error::InvalidOption(
            "batch distance must be finite and nonnegative, and size positive",
        ));
    }
    let sequences = log.activity_sequences(keys)?;
    let mut groups = BTreeMap::<(Label, Label), Vec<Batch>>::new();
    for (ti, trace) in log.traces.iter().enumerate() {
        if trace.is_empty() {
            continue;
        }
        let case = trace
            .attributes
            .get(&options.case_attribute)
            .and_then(ichnos_core::AttributeValue::as_str)
            .map(Label::from)
            .ok_or_else(|| Error::BatchCase {
                trace: ti,
                key: options.case_attribute.clone(),
            })?;
        for (ei, event) in trace.events.iter().enumerate() {
            let position = Position::Event {
                trace: ti,
                event: ei,
            };
            let resource = text(event.get(&keys.resource), &keys.resource, position)?;
            let end = timestamp(event, &keys.timestamp, position)?;
            let start = if options.use_start_timestamp {
                timestamp(event, &keys.start_timestamp, position)?
            } else {
                end
            };
            let activity = Label::from(sequences.activities.name(sequences.traces[ti][ei]));
            groups.entry((activity, resource)).or_default().push(Batch {
                start,
                end,
                events: vec![BatchEvent {
                    start,
                    end,
                    case: case.clone(),
                }],
            });
        }
    }
    let mut result = Vec::new();
    for ((activity, resource), mut heap) in groups {
        for i in (0..heap.len() / 2).rev() {
            sift_up(&mut heap, i);
        }
        while let Some(i) =
            (0..heap.len().saturating_sub(1)).find(|&i| heap[i].end > heap[i + 1].start)
        {
            let b = heap.remove(i + 1);
            let a = heap.remove(i);
            heap.push(merge(a, b));
            // Equal positive-length intervals overlap regardless of tie order.
            // Use a total order here; the heap retains Python's set comparison.
            heap.sort_by(|a, b| {
                a.start
                    .total_cmp(&b.start)
                    .then(a.end.total_cmp(&b.end))
                    .then_with(|| {
                        a.events
                            .iter()
                            .zip(&b.events)
                            .map(|(a, b)| event_cmp(a, b))
                            .find(|order| !order.is_eq())
                            .unwrap_or_else(|| a.events.len().cmp(&b.events.len()))
                    })
            });
        }
        loop {
            let mut changed = false;
            let mut i = 0;
            while i + 1 < heap.len() {
                if heap[i + 1].start - heap[i].end <= options.merge_distance {
                    let b = heap.remove(i + 1);
                    let a = heap.remove(i);
                    heap.push(merge(a, b));
                    let last = heap.len() - 1;
                    sift_down(&mut heap, 0, last);
                    changed = true;
                } else {
                    i += 1;
                }
            }
            if !changed {
                break;
            }
        }
        let mut batches = BTreeMap::<BatchType, Vec<Batch>>::new();
        for mut batch in heap {
            if batch.events.len() >= options.min_batch_size {
                batch.events.sort_by(event_cmp);
                batches.entry(category(&batch)).or_default().push(batch);
            }
        }
        if !batches.is_empty() {
            result.push(BatchGroup {
                activity,
                resource,
                batches,
            });
        }
    }
    result.sort_by(|a, b| {
        (b.count(), &b.activity, &b.resource).cmp(&(a.count(), &a.activity, &a.resource))
    });
    Ok(result)
}
