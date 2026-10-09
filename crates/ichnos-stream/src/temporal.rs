//! Temporal deviations in arrival order, compared against all prior case events.

use crate::{Error, Result, StreamSink, StreamingConformanceOptions, conformance::Input};
pub use ichnos_conformance::temporal_profile::{TemporalDeviation, TemporalProfile};
use ichnos_core::Event;
use std::collections::BTreeMap;

/// Streaming temporal settings; the native default uses completion timestamps
/// as starts and a six-standard-deviation tolerance.
#[derive(Debug, Clone)]
pub struct StreamingTemporalOptions {
    /// Activity/case/completion keys and missing-field policy.
    pub events: StreamingConformanceOptions,
    /// Start timestamp key, default time:timestamp.
    pub start_timestamp: String,
    /// Nonnegative finite standard-deviation multiplier, default six.
    pub zeta: f64,
}

impl Default for StreamingTemporalOptions {
    fn default() -> Self {
        Self {
            events: Default::default(),
            start_timestamp: "time:timestamp".into(),
            zeta: 6.0,
        }
    }
}

#[derive(Debug, Clone)]
struct Previous {
    activity: String,
    end: f64,
}

/// Streaming temporal profile conformance. Memory grows with case histories
/// because every arriving event must be compared with every prior event.
#[derive(Debug, Clone)]
pub struct StreamingTemporalConformance {
    input: Input,
    start_key: String,
    zeta: f64,
    profile: TemporalProfile,
    history: BTreeMap<String, Vec<Previous>>,
    deviations: BTreeMap<String, Vec<TemporalDeviation>>,
}

impl StreamingTemporalConformance {
    /// Create a checker, rejecting nonfinite/negative tolerances or profile
    /// standard deviations, and nonfinite means.
    pub fn new(profile: TemporalProfile, options: StreamingTemporalOptions) -> Result<Self> {
        if !options.zeta.is_finite() || options.zeta < 0.0 {
            return Err(Error::InvalidConformanceOption(
                "zeta must be finite and nonnegative",
            ));
        }
        if profile
            .values()
            .any(|&(mean, std)| !mean.is_finite() || !std.is_finite() || std < 0.0)
        {
            return Err(Error::InvalidConformanceOption(
                "profile bounds must be finite, with nonnegative standard deviation",
            ));
        }
        Ok(Self {
            input: Input {
                options: options.events,
                ..Default::default()
            },
            start_key: options.start_timestamp,
            zeta: options.zeta,
            profile,
            history: BTreeMap::new(),
            deviations: BTreeMap::new(),
        })
    }

    /// Check one event before storing it. Overlapping pairs are excluded;
    /// deviations are ordered by arriving event, then prior event. Invalid
    /// timestamp types return an error without changing case state.
    pub fn push(&mut self, event: &Event) -> Result<()> {
        let end_key = self.input.options.keys.timestamp.clone();
        let Some((case, activity)) = self.input.fields(event, &[&self.start_key, &end_key])? else {
            return Ok(());
        };
        let time =
            |key: &str| -> Result<f64> {
                let date = event.get(key).and_then(|v| v.as_date()).ok_or_else(|| {
                    Error::TimestampType {
                        key: key.into(),
                        event: self.input.seen - 1,
                    }
                })?;
                Ok(date.timestamp_micros() as f64 / 1e6
                    + f64::from(date.timestamp_subsec_nanos() % 1_000) / 1e9)
            };
        let (start, end) = (time(&self.start_key)?, time(&end_key)?);
        let previous = self.history.entry(case.clone()).or_default();
        for earlier in previous.iter() {
            if start < earlier.end {
                continue;
            }
            if let Some(&(mean, std)) = self
                .profile
                .get(&(earlier.activity.clone(), activity.clone()))
            {
                let seconds = start - earlier.end;
                if seconds < mean - self.zeta * std || seconds > mean + self.zeta * std {
                    self.deviations
                        .entry(case.clone())
                        .or_default()
                        .push(TemporalDeviation {
                            from: earlier.activity.clone(),
                            to: activity.clone(),
                            seconds,
                            zeta: if std > 0.0 {
                                (seconds - mean).abs() / std
                            } else {
                                f64::INFINITY
                            },
                        });
                }
            }
        }
        previous.push(Previous { activity, end });
        Ok(())
    }

    /// Deviations only, grouped by case. Empty cases are omitted.
    pub fn get(&self) -> &BTreeMap<String, Vec<TemporalDeviation>> {
        &self.deviations
    }

    /// Release a case's history and deviations; None means it was never seen.
    /// This explicit lifecycle extension bounds memory for completed cases.
    pub fn terminate(&mut self, case: &str) -> Option<Vec<TemporalDeviation>> {
        self.history.remove(case)?;
        Some(self.deviations.remove(case).unwrap_or_default())
    }

    /// Count of incomplete events ignored by the configured policy.
    pub fn skipped(&self) -> usize {
        self.input.skipped
    }
}

impl StreamSink for StreamingTemporalConformance {
    fn push(&mut self, event: &Event) -> Result<()> {
        Self::push(self, event)
    }
}
