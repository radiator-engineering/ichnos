//! Target vectors for prediction, ported from pm4py's
//! `algo/transformation/log_to_target`.

use std::collections::BTreeSet;

use ichnos_core::EventLog;

use crate::error::Result;
use crate::util::{event_seconds, event_str};

/// What [`extract_target_vector`] predicts for each event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetVariant {
    /// A one-hot vector of the next activity; all zeros for the last event.
    NextActivity,
    /// Seconds until the next event; 0 for the last event.
    NextTime,
    /// Seconds until the last event of the trace.
    RemainingTime,
}

/// Options for [`extract_target_vector`], with pm4py's defaults.
#[derive(Clone, Debug)]
pub struct TargetOptions {
    /// The activity attribute. Default `concept:name`.
    pub activity_key: String,
    /// The timestamp attribute. Default `time:timestamp`.
    pub timestamp_key: String,
    /// The activities of the one-hot vectors, in order. Default: every
    /// activity of the log, sorted. A next activity outside the list gives
    /// all zeros.
    pub activities: Option<Vec<String>>,
    /// Pads each trace with zero targets to this many events. pm4py's
    /// `enable_padding` with `pad_size`; `Some(None)` pads to the longest
    /// trace. Default no padding.
    pub padding: Option<Option<usize>>,
}

impl Default for TargetOptions {
    fn default() -> Self {
        TargetOptions {
            activity_key: "concept:name".to_owned(),
            timestamp_key: "time:timestamp".to_owned(),
            activities: None,
            padding: None,
        }
    }
}

/// The targets of each event, trace by trace.
#[derive(Clone, Debug, PartialEq)]
pub enum TargetVector {
    /// One one-hot vector per event ([`TargetVariant::NextActivity`]).
    Activities(Vec<Vec<Vec<f64>>>),
    /// One number per event ([`TargetVariant::NextTime`] and
    /// [`TargetVariant::RemainingTime`]).
    Times(Vec<Vec<f64>>),
}

/// The target of each event and the names of its parts, as pm4py's
/// `extract_target_vector` computes them.
///
/// The names are the activities for [`TargetVariant::NextActivity`],
/// `@@next_time` and `@@remaining_time` for the others. pm4py fails on a log
/// without traces; this returns empty targets instead.
pub fn extract_target_vector(
    log: &EventLog,
    variant: TargetVariant,
    options: &TargetOptions,
) -> Result<(TargetVector, Vec<String>)> {
    let longest = log.traces.iter().map(|t| t.events.len()).max().unwrap_or(0);
    let pad = options.padding.map(|p| p.unwrap_or(longest));
    match variant {
        TargetVariant::NextActivity => {
            let activities = match &options.activities {
                Some(a) => a.clone(),
                None => {
                    let mut set = BTreeSet::new();
                    for (t, trace) in log.traces.iter().enumerate() {
                        for (e, event) in trace.events.iter().enumerate() {
                            set.insert(event_str(event, &options.activity_key, t, e)?);
                        }
                    }
                    set.into_iter().collect()
                }
            };
            let mut target = Vec::with_capacity(log.traces.len());
            for (t, trace) in log.traces.iter().enumerate() {
                let mut rows = Vec::with_capacity(trace.events.len());
                for i in 0..trace.events.len() {
                    let mut row = vec![0.0; activities.len()];
                    if i + 1 < trace.events.len() {
                        let next =
                            event_str(&trace.events[i + 1], &options.activity_key, t, i + 1)?;
                        if let Some(k) = activities.iter().position(|a| *a == next) {
                            row[k] = 1.0;
                        }
                    }
                    rows.push(row);
                }
                if let Some(pad) = pad {
                    while rows.len() < pad {
                        rows.push(vec![0.0; activities.len()]);
                    }
                }
                target.push(rows);
            }
            Ok((TargetVector::Activities(target), activities))
        }
        TargetVariant::NextTime | TargetVariant::RemainingTime => {
            let mut target = Vec::with_capacity(log.traces.len());
            for (t, trace) in log.traces.iter().enumerate() {
                let times = trace
                    .events
                    .iter()
                    .enumerate()
                    .map(|(e, event)| event_seconds(event, &options.timestamp_key, t, e))
                    .collect::<Result<Vec<f64>>>()?;
                let mut rows: Vec<f64> = (0..times.len())
                    .map(|i| match variant {
                        TargetVariant::NextTime => times.get(i + 1).unwrap_or(&times[i]) - times[i],
                        _ => times[times.len() - 1] - times[i],
                    })
                    .collect();
                if let Some(pad) = pad {
                    rows.resize(rows.len().max(pad), 0.0);
                }
                target.push(rows);
            }
            let name = if variant == TargetVariant::NextTime {
                "@@next_time"
            } else {
                "@@remaining_time"
            };
            Ok((TargetVector::Times(target), vec![name.to_owned()]))
        }
    }
}
