//! State-based streaming checks for the eighteen typed DECLARE templates.

use crate::{Error, Result, StreamSink};
use ichnos_core::{AttributeValue, Event, EventKeys};
pub use ichnos_discovery::declare::{DeclareActivities, DeclareModel, DeclareTemplate};
use ichnos_model::Label;
use std::collections::BTreeMap;

/// Handling of incomplete Declare events.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DeclareMissingPolicy {
    /// Native defaults: missing case uses "undefined_case", missing activity
    /// advances each monitor with no matching label.
    #[default]
    NativeDefaults,
    /// Count and skip events missing case or activity.
    Ignore,
    /// Return an indexed error before changing monitoring state.
    Reject,
}

/// Event keys and incomplete-event policy for Declare monitoring.
#[derive(Debug, Clone, Default)]
pub struct StreamingDeclareOptions {
    /// Canonical activity/case/timestamp keys. Native hardcodes these defaults;
    /// custom keys are a Rust extension.
    pub keys: EventKeys,
    /// Defaults to the native missing-field behavior.
    pub missing: DeclareMissingPolicy,
}

/// Typed identity of one Declare constraint.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeclareConstraint {
    /// Supported template.
    pub template: DeclareTemplate,
    /// Unary label or ordered binary labels.
    pub activities: DeclareActivities,
}

/// The current native automaton state name, without serialized tuple keys.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DeclareAutomatonState {
    /// Initial/pending monitor state.
    #[default]
    Initial,
    /// Existence has observed its activity.
    Seen,
    /// Exactly-one has observed one occurrence.
    SeenOnce,
    /// Init was satisfied by the first event.
    Ok,
    /// An immediate violation occurred; later events cannot add another.
    Violated,
}

impl DeclareAutomatonState {
    /// Name used by the pinned native monitor.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Initial => "init",
            Self::Seen => "seen",
            Self::SeenOnce => "seen_once",
            Self::Ok => "ok",
            Self::Violated => "violated",
        }
    }
}

/// Timestamp or event-number fallback recorded for a consumed event.
#[derive(Debug, Clone, PartialEq)]
pub enum DeclareEventTime {
    /// Original typed timestamp attribute, without coercion or validation.
    Attribute(AttributeValue),
    /// One-based global processed-event index when no timestamp is present.
    EventIndex(u64),
}

/// Immediate violation count at one input event.
#[derive(Debug, Clone, PartialEq)]
pub struct DeclareDeviation {
    /// Timestamp or processed-event fallback.
    pub time: DeclareEventTime,
    /// Number of constraints first entering their absorbing violation state.
    pub deviations: u64,
}

/// Prefix diagnostics for one case. Pending obligations are not violations.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StreamingDeclareCase {
    /// Processed events for this case.
    pub events: u64,
    /// Count of first immediate constraint violations.
    pub deviations: u64,
    /// Each typed constraint's current automaton state.
    pub constraints_state: BTreeMap<DeclareConstraint, DeclareAutomatonState>,
}

/// Global Declare monitoring snapshot.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StreamingDeclareResult {
    /// Number of consumed events across cases, excluding Ignore skips.
    pub total_events_processed: u64,
    /// Number of first immediate violations across all cases.
    pub total_deviations: u64,
    /// One entry per consumed event, including zero-deviation events.
    pub deviations_per_time: Vec<DeclareDeviation>,
    /// Per-case prefix diagnostics, ordered by displayed case ID.
    pub cases: BTreeMap<String, StreamingDeclareCase>,
}

#[derive(Debug, Clone, Default)]
struct Monitor {
    state: DeclareAutomatonState,
    seen_a: bool,
    seen_b: bool,
    pending: u64,
    waiting_a: bool,
    waiting_b: bool,
    last: Option<Label>,
}

impl Monitor {
    fn new(template: DeclareTemplate) -> Self {
        Self {
            waiting_a: matches!(
                template,
                DeclareTemplate::AlternatePrecedence | DeclareTemplate::AlternateSuccession
            ),
            ..Default::default()
        }
    }

    fn advance(&mut self, constraint: &DeclareConstraint, activity: Option<&str>) -> bool {
        use DeclareAutomatonState as S;
        use DeclareTemplate as T;
        if self.state == S::Violated {
            return false;
        }
        let (a, b) = match &constraint.activities {
            DeclareActivities::Unary(a) => (a.as_str(), None),
            DeclareActivities::Binary(a, b) => (a.as_str(), Some(b.as_str())),
        };
        let is_a = activity == Some(a);
        let is_b = b.is_some() && activity == b;
        let violated = match constraint.template {
            T::Existence => {
                if is_a {
                    self.state = S::Seen;
                }
                false
            }
            T::Absence => is_a,
            T::ExactlyOne => {
                if is_a {
                    if self.state == S::SeenOnce {
                        true
                    } else {
                        self.state = S::SeenOnce;
                        false
                    }
                } else {
                    false
                }
            }
            T::Init => {
                if self.state == S::Initial {
                    self.state = S::Ok;
                    !is_a
                } else {
                    false
                }
            }
            T::RespondedExistence | T::Coexistence => {
                self.seen_a |= is_a;
                self.seen_b |= is_b;
                false
            }
            T::Response => {
                if is_a {
                    self.pending += 1;
                }
                if is_b && self.pending > 0 {
                    self.pending -= 1;
                }
                false
            }
            T::Precedence => {
                if is_b && !self.seen_a {
                    true
                } else {
                    self.seen_a |= is_a;
                    false
                }
            }
            T::Succession => {
                if is_b && !self.seen_a {
                    true
                } else {
                    if is_a {
                        self.seen_a = true;
                        self.pending += 1;
                    } else if is_b && self.pending > 0 {
                        self.pending -= 1;
                    }
                    false
                }
            }
            T::AlternateResponse => {
                if is_a && self.waiting_b {
                    true
                } else {
                    if is_a {
                        self.waiting_b = true;
                    }
                    if is_b {
                        self.waiting_b = false;
                    }
                    false
                }
            }
            T::AlternatePrecedence => {
                if is_b && self.waiting_a {
                    true
                } else {
                    if is_b {
                        self.waiting_a = true;
                    }
                    if is_a {
                        self.waiting_a = false;
                    }
                    false
                }
            }
            T::AlternateSuccession => {
                if is_a {
                    if self.waiting_b {
                        true
                    } else {
                        self.waiting_b = true;
                        self.waiting_a = false;
                        false
                    }
                } else if is_b {
                    if self.waiting_a {
                        true
                    } else {
                        self.waiting_a = true;
                        self.waiting_b = false;
                        false
                    }
                } else {
                    false
                }
            }
            T::ChainResponse => {
                if self.waiting_b && !is_b {
                    true
                } else {
                    self.waiting_b = is_a;
                    false
                }
            }
            T::ChainPrecedence => {
                if is_b && self.last.as_deref() != Some(a) {
                    true
                } else {
                    self.last = activity.map(Label::from);
                    false
                }
            }
            T::ChainSuccession => {
                if (self.waiting_b && !is_b) || (is_b && self.last.as_deref() != Some(a)) {
                    true
                } else {
                    self.waiting_b = is_a;
                    self.last = activity.map(Label::from);
                    false
                }
            }
            T::NonCoexistence => {
                self.seen_a |= is_a;
                self.seen_b |= is_b;
                self.seen_a && self.seen_b
            }
            T::NonSuccession => {
                self.seen_a |= is_a;
                is_b && self.seen_a
            }
            T::NonChainSuccession => {
                if self.waiting_b && is_b {
                    true
                } else {
                    self.waiting_b = is_a;
                    false
                }
            }
        };
        if violated {
            self.state = S::Violated;
        }
        violated
    }
}

#[derive(Debug, Clone, Default)]
struct Case {
    monitors: Vec<Monitor>,
    events: u64,
    deviations: u64,
}

/// Streaming automata for the merged typed Declare model.
///
/// This is prefix monitoring, not completed-trace Declare conformance. Native
/// streaming semantics perform no end-of-case checks: pending responses and
/// unseen existence requirements do not contribute violations. Every monitor
/// reports at most one immediate violation per case and remains absorbing.
/// Discovery support/confidence counts are ignored. No event trace is stored;
/// per-case monitor memory is proportional to the number of constraints.
/// Global deviation history grows with consumed events until explicitly cleared.
#[derive(Debug, Clone)]
pub struct StreamingDeclareConformance {
    options: StreamingDeclareOptions,
    constraints: Vec<DeclareConstraint>,
    cases: BTreeMap<String, Case>,
    total_events: u64,
    total_deviations: u64,
    history: Vec<DeclareDeviation>,
    seen: usize,
    skipped: usize,
}

impl StreamingDeclareConformance {
    /// Prepare monitors for the model's rules. Incorrect unary/binary arity
    /// returns a typed error; support/confidence metadata is unused.
    pub fn new(model: &DeclareModel, options: StreamingDeclareOptions) -> Result<Self> {
        let mut constraints = Vec::new();
        for (&template, rules) in &model.rules {
            for activities in rules.keys() {
                if template.is_unary() != matches!(activities, DeclareActivities::Unary(_)) {
                    return Err(Error::InvalidConformanceOption(
                        "Declare rule arity does not match template",
                    ));
                }
                constraints.push(DeclareConstraint {
                    template,
                    activities: activities.clone(),
                });
            }
        }
        Ok(Self {
            options,
            constraints,
            cases: BTreeMap::new(),
            total_events: 0,
            total_deviations: 0,
            history: Vec::new(),
            seen: 0,
            skipped: 0,
        })
    }

    /// Consume one event, reporting violations in the typed result history.
    /// Does not log, print or perform deferred/end-of-case checks.
    pub fn push(&mut self, event: &Event) -> Result<()> {
        let index = self.seen;
        self.seen += 1;
        let case = event.get(&self.options.keys.case_id);
        let activity = event.get(&self.options.keys.activity);
        if self.options.missing != DeclareMissingPolicy::NativeDefaults {
            for (key, present) in [
                (&self.options.keys.case_id, case.is_some()),
                (&self.options.keys.activity, activity.is_some()),
            ] {
                if !present {
                    if self.options.missing == DeclareMissingPolicy::Ignore {
                        self.skipped += 1;
                        return Ok(());
                    }
                    return Err(Error::MissingField {
                        key: key.to_string(),
                        event: index,
                    });
                }
            }
        }
        let case_id = case.map_or_else(|| "undefined_case".into(), ToString::to_string);
        let activity = activity.map(ToString::to_string);
        let data = self.cases.entry(case_id).or_insert_with(|| Case {
            monitors: self
                .constraints
                .iter()
                .map(|c| Monitor::new(c.template))
                .collect(),
            ..Default::default()
        });
        self.total_events += 1;
        data.events += 1;
        let deviations = self
            .constraints
            .iter()
            .zip(&mut data.monitors)
            .filter_map(|(c, m)| m.advance(c, activity.as_deref()).then_some(1_u64))
            .sum::<u64>();
        data.deviations += deviations;
        self.total_deviations += deviations;
        let time = event
            .get(&self.options.keys.timestamp)
            .map_or(DeclareEventTime::EventIndex(self.total_events), |v| {
                DeclareEventTime::Attribute(v.clone())
            });
        self.history.push(DeclareDeviation { time, deviations });
        Ok(())
    }

    /// Snapshot global totals, full event-level history and all case states.
    pub fn get(&self) -> StreamingDeclareResult {
        StreamingDeclareResult {
            total_events_processed: self.total_events,
            total_deviations: self.total_deviations,
            deviations_per_time: self.history.clone(),
            cases: self
                .cases
                .iter()
                .map(|(c, data)| (c.clone(), self.case_result(data)))
                .collect(),
        }
    }

    /// Prefix state for an open case; None if no such case is tracked.
    pub fn get_status(&self, case: &str) -> Option<StreamingDeclareCase> {
        self.cases.get(case).map(|data| self.case_result(data))
    }

    /// Number of incomplete events skipped by Ignore policy.
    pub fn skipped_events(&self) -> usize {
        self.skipped
    }

    /// Release case monitors and allow fresh reuse of the ID. This extension
    /// performs no end checks and does not erase global totals/history.
    pub fn remove_case(&mut self, case: &str) -> bool {
        self.cases.remove(case).is_some()
    }

    /// Release event-level history, preserving totals and per-case monitors.
    /// An explicit memory-management extension to native streaming Declare.
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    fn case_result(&self, data: &Case) -> StreamingDeclareCase {
        StreamingDeclareCase {
            events: data.events,
            deviations: data.deviations,
            constraints_state: self
                .constraints
                .iter()
                .cloned()
                .zip(data.monitors.iter().map(|m| m.state))
                .collect(),
        }
    }
}

impl StreamSink for StreamingDeclareConformance {
    fn push(&mut self, event: &Event) -> Result<()> {
        self.push(event)
    }
}
