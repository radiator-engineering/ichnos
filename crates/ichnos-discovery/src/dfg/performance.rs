use super::{Aggregation, BusinessHours, stats_error};
use crate::{Error, Result};
use ichnos_core::{
    Event, EventKeys, EventLog, Position,
    chrono::{DateTime, FixedOffset},
};
use ichnos_model::{Label, dfg::ActivityCounts};
use std::collections::BTreeMap;

/// Options for performance DFG discovery.
#[derive(Clone, Debug, Default)]
pub struct PerformanceDfgOptions {
    /// Measure source completion to target start instead of target completion.
    pub use_start_timestamp: bool,
    /// Optional weekly wall-clock schedule. Dates in `non_working_dates` are
    /// excluded; arbitrary Python work-calendar objects are not accepted.
    pub business_hours: Option<BusinessHours>,
    /// Retain each observed duration in original event order (pm4py raw_values).
    pub keep_raw_values: bool,
}

/// All six aggregates of the durations observed on a DFG edge, in seconds.
#[derive(Clone, Debug, PartialEq)]
pub struct PerformanceSummary {
    /// Number of edge observations.
    pub count: usize,
    /// Arithmetic mean.
    pub mean: f64,
    /// Median, averaging the middle pair for an even sample size.
    pub median: f64,
    /// Minimum duration.
    pub min: f64,
    /// Maximum duration.
    pub max: f64,
    /// Sum of durations.
    pub sum: f64,
    /// Sample standard deviation; zero for a single observation.
    pub stdev: f64,
    /// Observed values, only when requested by the options.
    pub raw_values: Option<Vec<f64>>,
}
impl PerformanceSummary {
    /// Select one aggregate, using the shared statistics aggregation enum.
    pub fn aggregate(&self, aggregation: Aggregation) -> f64 {
        match aggregation {
            Aggregation::Mean => self.mean,
            Aggregation::Median => self.median,
            Aggregation::Min => self.min,
            Aggregation::Max => self.max,
            Aggregation::Sum => self.sum,
            Aggregation::StandardDeviation => self.stdev,
        }
    }
    fn new(values: Vec<f64>, keep_raw: bool) -> Self {
        let count = values.len();
        let sum = values.iter().sum::<f64>();
        let mean = sum / count as f64;
        let stdev = if count == 1 {
            0.0
        } else {
            (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (count - 1) as f64).sqrt()
        };
        let mut sorted = values.clone();
        sorted.sort_by(f64::total_cmp);
        Self {
            count,
            mean,
            stdev,
            sum,
            median: (sorted[(count - 1) / 2] + sorted[count / 2]) / 2.0,
            min: sorted[0],
            max: sorted[count - 1],
            raw_values: keep_raw.then_some(values),
        }
    }
}

/// Performance summaries and actual trace-boundary frequencies.
#[derive(Clone, Debug, Default)]
pub struct PerformanceDfg {
    /// Source/target activity pair to its duration summary.
    pub graph: BTreeMap<(Label, Label), PerformanceSummary>,
    /// Frequencies of trace start activities.
    pub start_activities: ActivityCounts,
    /// Frequencies of trace end activities.
    pub end_activities: ActivityCounts,
    /// Schedule retained for downstream duration interpretation.
    pub business_hours: Option<BusinessHours>,
}

fn date(event: &Event, key: &str, position: Position) -> Result<DateTime<FixedOffset>> {
    let value = event
        .get(key)
        .ok_or_else(|| ichnos_core::Error::MissingAttribute {
            key: key.into(),
            position,
        })?;
    value.as_date().ok_or_else(|| {
        Error::Core(ichnos_core::Error::AttributeType {
            key: key.into(),
            position,
            expected: "date",
            found: value.type_name(),
        })
    })
}

/// Discover all duration aggregates on adjacent event pairs without sorting.
/// Negative waiting times are clipped to zero, including overlapping intervals.
/// Missing or non-date timestamps produce a positional core error. Single-event
/// traces need no timestamps because they have no duration observations.
pub fn performance_dfg(
    log: &EventLog,
    keys: &EventKeys,
    options: &PerformanceDfgOptions,
) -> Result<PerformanceDfg> {
    if let Some(schedule) = &options.business_hours {
        schedule.validate().map_err(stats_error)?;
    }
    let sequences = log.activity_sequences(keys)?;
    let mut start_activities = ActivityCounts::new();
    let mut end_activities = ActivityCounts::new();
    for trace in &sequences.traces {
        if let (Some(&first), Some(&last)) = (trace.first(), trace.last()) {
            *start_activities
                .entry(Label::from(sequences.activities.name(first)))
                .or_default() += 1;
            *end_activities
                .entry(Label::from(sequences.activities.name(last)))
                .or_default() += 1;
        }
    }
    let mut observations = BTreeMap::<(Label, Label), Vec<f64>>::new();
    let start_key = if options.use_start_timestamp {
        &keys.start_timestamp
    } else {
        &keys.timestamp
    };
    for (ti, trace) in log.traces.iter().enumerate() {
        for ei in 1..trace.len() {
            let source = date(
                &trace.events[ei - 1],
                &keys.timestamp,
                Position::Event {
                    trace: ti,
                    event: ei - 1,
                },
            )?;
            let target = date(
                &trace.events[ei],
                start_key,
                Position::Event {
                    trace: ti,
                    event: ei,
                },
            )?;
            let seconds = if let Some(schedule) = &options.business_hours {
                schedule
                    .seconds_between(source, target)
                    .map_err(stats_error)?
            } else {
                let duration = target.signed_duration_since(source);
                duration.num_seconds() as f64 + duration.subsec_nanos() as f64 / 1e9
            }
            .max(0.0);
            observations
                .entry((
                    Label::from(sequences.activities.name(sequences.traces[ti][ei - 1])),
                    Label::from(sequences.activities.name(sequences.traces[ti][ei])),
                ))
                .or_default()
                .push(seconds);
        }
    }
    Ok(PerformanceDfg {
        graph: observations
            .into_iter()
            .map(|(edge, values)| {
                (
                    edge,
                    PerformanceSummary::new(values, options.keep_raw_values),
                )
            })
            .collect(),
        start_activities,
        end_activities,
        business_hours: options.business_hours.clone(),
    })
}
