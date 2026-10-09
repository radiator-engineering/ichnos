//! Log-skeleton conformance checking, ported from pm4py's
//! `algo/conformance/log_skeleton/variants/classic.py`.

use super::{LogSkeleton, SkeletonRelation};
use crate::Result;
use ichnos_core::{EventKeys, EventLog};
use ichnos_model::Label;
use std::collections::{BTreeMap, BTreeSet};

/// One of the six constraints of a log skeleton.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SkeletonConstraint {
    /// Pairs of activities that occur equally often.
    Equivalence,
    /// The second activity occurs after the first.
    AlwaysAfter,
    /// The second activity occurs before the first.
    AlwaysBefore,
    /// The two activities do not occur in the same trace.
    NeverTogether,
    /// The second activity directly follows the first.
    DirectlyFollows,
    /// The number of occurrences of an activity.
    ActivityFrequency,
}

impl SkeletonConstraint {
    /// All six constraints, in pm4py's order; also the default set.
    pub const ALL: [Self; 6] = [
        Self::Equivalence,
        Self::AlwaysAfter,
        Self::AlwaysBefore,
        Self::NeverTogether,
        Self::DirectlyFollows,
        Self::ActivityFrequency,
    ];

    /// pm4py's name of the constraint.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Equivalence => "equivalence",
            Self::AlwaysAfter => "always_after",
            Self::AlwaysBefore => "always_before",
            Self::NeverTogether => "never_together",
            Self::DirectlyFollows => "directly_follows",
            Self::ActivityFrequency => "activ_freq",
        }
    }
}

/// A deviation of a trace from a log skeleton.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkeletonDeviation {
    /// The pairs of a relation that the trace breaks, in pair order.
    Relation {
        /// The relation.
        constraint: SkeletonConstraint,
        /// The broken pairs.
        pairs: Vec<(Label, Label)>,
    },
    /// An activity whose number of occurrences is not allowed. The count is
    /// 0 for an activity that the model does not know, as in pm4py, and for
    /// a required activity that the trace lacks.
    ActivityFrequency {
        /// The activity.
        activity: Label,
        /// Its number of occurrences, or 0 as above.
        count: u64,
    },
}

impl SkeletonDeviation {
    /// The constraint the deviation breaks.
    pub fn constraint(&self) -> SkeletonConstraint {
        match self {
            Self::Relation { constraint, .. } => *constraint,
            Self::ActivityFrequency { .. } => SkeletonConstraint::ActivityFrequency,
        }
    }
}

/// The log-skeleton conformance of one trace, with pm4py's field names.
#[derive(Debug, Clone, PartialEq)]
pub struct SkeletonTraceConformance {
    /// The deviations, sorted by pm4py's constraint name, then by activity
    /// and count.
    pub deviations: Vec<SkeletonDeviation>,
    /// The number of broken constraints.
    pub no_dev_total: usize,
    /// The number of constraints that apply to the trace.
    pub no_constr_total: usize,
    /// One minus the share of broken constraints; 1 when none apply.
    pub dev_fitness: f64,
    /// Whether the trace has no deviation.
    pub is_fit: bool,
}

/// Options for [`conformance_log_skeleton`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkeletonConformanceOptions {
    /// The constraints to check (default all six).
    pub considered_constraints: BTreeSet<SkeletonConstraint>,
}

impl Default for SkeletonConformanceOptions {
    fn default() -> Self {
        Self {
            considered_constraints: SkeletonConstraint::ALL.into_iter().collect(),
        }
    }
}

/// Checks each trace of the log against a log skeleton, as pm4py's
/// `conformance_log_skeleton` does.
///
/// A relation pair applies to a trace when its first activity occurs in the
/// trace. Every activity of the trace, and every activity the model requires
/// at least once, applies to the activity frequencies. Returns one result
/// per trace, in log order.
///
/// # Errors
///
/// An event without the activity attribute.
pub fn conformance_log_skeleton(
    log: &EventLog,
    model: &LogSkeleton,
    keys: &EventKeys,
    options: &SkeletonConformanceOptions,
) -> Result<Vec<SkeletonTraceConformance>> {
    let sequences = log.activity_sequences(keys)?;
    let mut cache = BTreeMap::new();
    let mut out = Vec::with_capacity(sequences.traces.len());
    for t in &sequences.traces {
        let result = cache.entry(t.as_slice()).or_insert_with(|| {
            let trace: Vec<Label> = t
                .iter()
                .map(|&a| Label::from(sequences.activities.name(a)))
                .collect();
            check_trace(&trace, model, options)
        });
        out.push(result.clone());
    }
    Ok(out)
}

fn check_trace(
    trace: &[Label],
    model: &LogSkeleton,
    options: &SkeletonConformanceOptions,
) -> SkeletonTraceConformance {
    let mut positions: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (i, a) in trace.iter().enumerate() {
        positions.entry(a.as_str()).or_default().push(i);
    }
    let count = |a: &Label| positions.get(a.as_str()).map_or(0, Vec::len);
    let first = |a: &Label| positions[a.as_str()][0];
    let last = |a: &Label| positions[a.as_str()][positions[a.as_str()].len() - 1];
    let adjacent: BTreeSet<(&Label, &Label)> = trace.windows(2).map(|w| (&w[0], &w[1])).collect();

    let mut deviations = Vec::new();
    let mut dev_total = 0;
    let mut constr_total = 0;
    for constraint in SkeletonConstraint::ALL {
        if !options.considered_constraints.contains(&constraint) {
            continue;
        }
        let relation: &SkeletonRelation = match constraint {
            SkeletonConstraint::Equivalence => &model.equivalence,
            SkeletonConstraint::AlwaysAfter => &model.always_after,
            SkeletonConstraint::AlwaysBefore => &model.always_before,
            SkeletonConstraint::NeverTogether => &model.never_together,
            SkeletonConstraint::DirectlyFollows => &model.directly_follows,
            SkeletonConstraint::ActivityFrequency => {
                let allowed = &model.activity_frequencies;
                for (&a, at) in &positions {
                    constr_total += 1;
                    let n = at.len() as u64;
                    let broken = match allowed.get(a) {
                        Some(counts) if counts.contains(&n) => None,
                        Some(_) => Some(n),
                        None => Some(0),
                    };
                    if let Some(count) = broken {
                        dev_total += 1;
                        deviations.push(SkeletonDeviation::ActivityFrequency {
                            activity: Label::from(a),
                            count,
                        });
                    }
                }
                for (a, counts) in allowed {
                    if counts.first().is_some_and(|&m| m > 0) && !positions.contains_key(a.as_str())
                    {
                        constr_total += 1;
                        dev_total += 1;
                        deviations.push(SkeletonDeviation::ActivityFrequency {
                            activity: a.clone(),
                            count: 0,
                        });
                    }
                }
                continue;
            }
        };
        let mut broken = Vec::new();
        for (a, b) in relation
            .iter()
            .filter(|(a, _)| positions.contains_key(a.as_str()))
        {
            constr_total += 1;
            let present = count(b) > 0;
            let holds = match constraint {
                SkeletonConstraint::Equivalence => a != b && count(a) == count(b),
                SkeletonConstraint::AlwaysAfter => present && first(a) < last(b),
                SkeletonConstraint::AlwaysBefore => present && first(b) < last(a),
                // A pair holds here when both activities occur: that is the
                // deviation.
                SkeletonConstraint::NeverTogether => !(a != b && present),
                SkeletonConstraint::DirectlyFollows => adjacent.contains(&(a, b)),
                SkeletonConstraint::ActivityFrequency => unreachable!(),
            };
            if !holds {
                broken.push((a.clone(), b.clone()));
            }
        }
        if !broken.is_empty() {
            dev_total += broken.len();
            deviations.push(SkeletonDeviation::Relation {
                constraint,
                pairs: broken,
            });
        }
    }
    deviations.sort_by(|x, y| {
        x.constraint()
            .as_str()
            .cmp(y.constraint().as_str())
            .then_with(|| match (x, y) {
                (
                    SkeletonDeviation::ActivityFrequency {
                        activity: a,
                        count: m,
                    },
                    SkeletonDeviation::ActivityFrequency {
                        activity: b,
                        count: n,
                    },
                ) => (a, m).cmp(&(b, n)),
                _ => std::cmp::Ordering::Equal,
            })
    });
    SkeletonTraceConformance {
        is_fit: deviations.is_empty(),
        deviations,
        no_dev_total: dev_total,
        no_constr_total: constr_total,
        dev_fitness: if constr_total > 0 {
            1.0 - dev_total as f64 / constr_total as f64
        } else {
            1.0
        },
    }
}
