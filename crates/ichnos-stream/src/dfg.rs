//! Online frequency discovery following pm4py streaming DFG frequency.apply.

use crate::{Error, Result, StreamSink};
use ichnos_core::{Event, EventKeys};
use ichnos_model::{Dfg, Label, dfg::ActivityCounts};
use std::collections::BTreeMap;

/// Treatment of events lacking an activity or case ID.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MissingEventPolicy {
    /// Skip the event, as pm4py does after logging a warning.
    #[default]
    Ignore,
    /// Return a typed positional error without updating the model.
    Reject,
}

/// Online DFG options; no timestamp sorting or implicit case completion.
#[derive(Debug, Clone, Default)]
pub struct StreamingDfgOptions {
    /// Activity and case keys; defaults to the standard XES keys.
    pub keys: EventKeys,
    /// Missing-field policy; defaults to Ignore.
    pub missing: MissingEventPolicy,
}

/// Snapshot of the online graph and observed activity frequencies.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StreamingDfgResult {
    /// Edge/start counts; ends count the current last activity of each case.
    pub dfg: Dfg,
    /// Frequencies of every accepted activity.
    pub activities: ActivityCounts,
}

/// Frequency DFG discovery retaining one last activity per observed case.
#[derive(Debug, Clone)]
pub struct StreamingDfgDiscovery {
    options: StreamingDfgOptions,
    result: StreamingDfgResult,
    last: BTreeMap<String, Label>,
    seen: usize,
    skipped: usize,
}

impl StreamingDfgDiscovery {
    /// Create an empty algorithm.
    pub fn new(options: StreamingDfgOptions) -> Self {
        Self {
            options,
            result: StreamingDfgResult::default(),
            last: BTreeMap::new(),
            seen: 0,
            skipped: 0,
        }
    }

    /// Process one event. Event order is retained across interleaved cases.
    /// Non-string values use core attribute display rather than Python repr.
    pub fn push(&mut self, event: &Event) -> Result<()> {
        let index = self.seen;
        self.seen += 1;
        for key in [&self.options.keys.case_id, &self.options.keys.activity] {
            if event.get(key).is_none() {
                if self.options.missing == MissingEventPolicy::Reject {
                    return Err(Error::MissingField {
                        key: key.clone(),
                        event: index,
                    });
                }
                self.skipped += 1;
                return Ok(());
            }
        }
        let case = event
            .get(&self.options.keys.case_id)
            .expect("required field checked")
            .to_string();
        let activity = Label::from(
            event
                .get(&self.options.keys.activity)
                .expect("required field checked")
                .to_string(),
        );
        if let Some(previous) = self.last.insert(case, activity.clone()) {
            self.result
                .dfg
                .add_edge(previous.clone(), activity.clone(), 1);
            let count = self
                .result
                .dfg
                .end_activities
                .get_mut(&previous)
                .expect("last activity has an end count");
            *count -= 1;
            if *count == 0 {
                self.result.dfg.end_activities.remove(&previous);
            }
        } else {
            self.result.dfg.add_start(activity.clone(), 1);
        }
        self.result.dfg.add_end(activity.clone(), 1);
        *self.result.activities.entry(activity).or_default() += 1;
        Ok(())
    }

    /// Borrow the current snapshot, including provisional end activities.
    pub fn get(&self) -> &StreamingDfgResult {
        &self.result
    }

    /// Number of missing-field events ignored by this algorithm.
    pub fn skipped(&self) -> usize {
        self.skipped
    }
}

impl Default for StreamingDfgDiscovery {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl StreamSink for StreamingDfgDiscovery {
    fn push(&mut self, event: &Event) -> Result<()> {
        Self::push(self, event)
    }
}
