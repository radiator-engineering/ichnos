//! Streaming start, directly-follows and end footprint checks.

use crate::{Result, StreamSink, StreamingConformanceOptions, conformance::Input};
use ichnos_core::Event;
use ichnos_model::{Footprints, Label};
use std::collections::{BTreeMap, BTreeSet};

/// An open case's footprint diagnostics; end constraints wait for termination.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StreamingFootprintsStatus {
    /// Last activity that belongs to the model, if any.
    pub last_activity: Option<Label>,
    /// Number of unknown activities or violated start/edge constraints.
    pub deviations: u64,
}

impl StreamingFootprintsStatus {
    /// Whether no deviation has occurred so far.
    pub fn is_fit(&self) -> bool {
        self.deviations == 0
    }
}

/// Incremental footprints conformance, retaining one accepted activity per case.
#[derive(Debug, Clone)]
pub struct StreamingFootprintsConformance {
    input: Input,
    footprints: Footprints,
    ends: BTreeSet<Label>,
    cases: BTreeMap<String, StreamingFootprintsStatus>,
}

impl StreamingFootprintsConformance {
    /// Create a checker. End activities are separate from shared model footprints.
    pub fn new(
        footprints: Footprints,
        end_activities: BTreeSet<Label>,
        options: StreamingConformanceOptions,
    ) -> Self {
        Self {
            input: Input {
                options,
                ..Default::default()
            },
            footprints,
            ends: end_activities,
            cases: BTreeMap::new(),
        }
    }

    /// Consume one event. Unknown activities count but do not replace the last
    /// known activity. Their deviations survive subsequent events, including
    /// when an unknown activity starts a case.
    pub fn push(&mut self, event: &Event) -> Result<()> {
        let Some((case, activity)) = self.input.fields(event, &[])? else {
            return Ok(());
        };
        let activity = Label::from(activity);
        let status = self.cases.entry(case).or_default();
        if !self.footprints.activities.contains(&activity) {
            status.deviations += 1;
            return Ok(());
        }
        let fits = match &status.last_activity {
            None => self.footprints.start_activities.contains(&activity),
            Some(previous) => {
                let pair = (previous.clone(), activity.clone());
                self.footprints.sequence.contains(&pair) || self.footprints.parallel.contains(&pair)
            }
        };
        status.deviations += u64::from(!fits);
        status.last_activity = Some(activity);
        Ok(())
    }

    /// Open-case snapshot in lexical case order, including unknown-only cases.
    pub fn get(&self) -> &BTreeMap<String, StreamingFootprintsStatus> {
        &self.cases
    }

    /// Diagnostics for an open case; None means no open case is tracked. Unknown-only cases return Some.
    pub fn get_status(&self, case: &str) -> Option<&StreamingFootprintsStatus> {
        self.cases.get(case)
    }

    /// Check the end constraint and remove this case. A reused ID starts fresh.
    pub fn terminate(&mut self, case: &str) -> Option<bool> {
        let status = self.cases.remove(case)?;
        Some(
            status.is_fit()
                && status
                    .last_activity
                    .as_ref()
                    .is_some_and(|a| self.ends.contains(a)),
        )
    }

    /// Terminate all open cases and return their end-aware fitness.
    pub fn terminate_all(&mut self) -> BTreeMap<String, bool> {
        let keys: Vec<_> = self.cases.keys().cloned().collect();
        keys.into_iter()
            .map(|case| {
                let fit = self
                    .terminate(&case)
                    .expect("case came from the open-case map");
                (case, fit)
            })
            .collect()
    }

    /// Count of incomplete events ignored by the configured policy.
    pub fn skipped(&self) -> usize {
        self.input.skipped
    }
}

impl StreamSink for StreamingFootprintsConformance {
    fn push(&mut self, event: &Event) -> Result<()> {
        Self::push(self, event)
    }
}
