//! View-based transition-system discovery, following pm4py's
//! `algo.discovery.transition_system.variants.view_based.apply`.
use crate::Result;
use ichnos_core::{EventKeys, EventLog};
use ichnos_model::transition_system::{EdgeId, StateId};
use ichnos_model::{Label, TransitionSystem};
use std::collections::{BTreeMap, BTreeSet};

/// Which side of an event boundary supplies a state's view.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TransitionDirection {
    /// The next window of activities (pm4py default).
    #[default]
    Forward,
    /// The previous window of activities.
    Backward,
}

/// State abstraction for each activity window.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TransitionAbstraction {
    /// Retain activity order and repetitions (pm4py default).
    #[default]
    Sequence,
    /// Retain only distinct labels.
    Set,
    /// Retain each label's occurrence count.
    Multiset,
}

/// Transition-system discovery options.
#[derive(Debug, Clone)]
pub struct TransitionSystemOptions {
    /// Side of each boundary supplying the activity window; defaults to forward.
    pub direction: TransitionDirection,
    /// Maximum activities in a view (default two). Zero is valid.
    pub window: usize,
    /// Activity-window abstraction; defaults to an ordered sequence.
    pub view: TransitionAbstraction,
    /// Record event positions in detailed discovery (default false).
    pub include_data: bool,
}

impl Default for TransitionSystemOptions {
    fn default() -> Self {
        Self {
            direction: TransitionDirection::Forward,
            window: 2,
            view: TransitionAbstraction::Sequence,
            include_data: false,
        }
    }
}

/// Structured state identity; no Python repr parsing or label coercion.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TransitionView {
    /// Ordered activity window.
    Sequence(Vec<Label>),
    /// Distinct labels in a window.
    Set(BTreeSet<Label>),
    /// Activity counts in a window.
    Multiset(BTreeMap<Label, usize>),
}

/// Stable Python-style state names. Sets and counters use lexical label order.
/// Labels are single-quoted with backslash, quote and control characters escaped.
/// Empty views display as `[]`, `set()` and `Counter()` respectively.
impl std::fmt::Display for TransitionView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn label(f: &mut std::fmt::Formatter<'_>, value: &Label) -> std::fmt::Result {
            write!(f, "'")?;
            for c in value.to_string().chars() {
                match c {
                    '\\' => write!(f, "\\\\")?,
                    '\'' => write!(f, "\\'")?,
                    '\n' => write!(f, "\\n")?,
                    '\r' => write!(f, "\\r")?,
                    '\t' => write!(f, "\\t")?,
                    c if c.is_control() && (c as u32) <= 255 => write!(f, "\\x{:02x}", c as u32)?,
                    c if c.is_control() => write!(f, "\\u{:04x}", c as u32)?,
                    c => write!(f, "{c}")?,
                }
            }
            write!(f, "'")
        }
        match self {
            Self::Sequence(values) => {
                write!(f, "[")?;
                for (i, value) in values.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    label(f, value)?;
                }
                write!(f, "]")
            }
            Self::Set(values) if values.is_empty() => write!(f, "set()"),
            Self::Set(values) => {
                write!(f, "{{")?;
                for (i, value) in values.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    label(f, value)?;
                }
                write!(f, "}}")
            }
            Self::Multiset(values) if values.is_empty() => write!(f, "Counter()"),
            Self::Multiset(values) => {
                write!(f, "Counter({{")?;
                for (i, (value, count)) in values.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    label(f, value)?;
                    write!(f, ": {count}")?;
                }
                write!(f, "}})")
            }
        }
    }
}

/// Position of an original event, without retaining or copying a trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TransitionEvent {
    /// Canonical trace index.
    pub trace: usize,
    /// Event index within that trace.
    pub event: usize,
}

/// Events entering and leaving one discovered state, with repetitions retained.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TransitionStateData {
    /// Events on edges entering this state.
    pub incoming: Vec<TransitionEvent>,
    /// Events on edges leaving this state.
    pub outgoing: Vec<TransitionEvent>,
}

/// Graph, structured view identities and optional event annotations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionDiscovery {
    /// Existing model transition-system graph with unique labelled edges.
    pub system: TransitionSystem,
    /// The view corresponding to each graph state.
    pub views: BTreeMap<StateId, TransitionView>,
    /// Per-state annotations, empty unless include_data is enabled.
    pub state_data: BTreeMap<StateId, TransitionStateData>,
    /// Per-edge event positions, empty unless include_data is enabled.
    pub edge_data: BTreeMap<EdgeId, Vec<TransitionEvent>>,
}

/// Discover the graph plus structured views and optional original event positions.
/// Every nonempty trace contributes one edge per event. Empty traces contribute
/// no states. Event order is retained; timestamps and case IDs are unnecessary.
/// States are interned by view before edge creation, so self-loop annotations
/// retain both endpoints rather than pm4py's detached first target object.
pub fn discover_transition_system(
    log: &EventLog,
    keys: &EventKeys,
    options: &TransitionSystemOptions,
) -> Result<TransitionDiscovery> {
    let sequences = log.activity_sequences(keys)?;
    let mut result = TransitionDiscovery {
        system: TransitionSystem::new(""),
        views: BTreeMap::new(),
        state_data: BTreeMap::new(),
        edge_data: BTreeMap::new(),
    };
    let mut states = BTreeMap::new();
    for (ti, trace) in sequences.traces.iter().enumerate() {
        if trace.is_empty() {
            continue;
        }
        let mut path = Vec::new();
        for i in 0..=trace.len() {
            let (start, end) = match options.direction {
                TransitionDirection::Forward => {
                    (i, i.saturating_add(options.window).min(trace.len()))
                }
                TransitionDirection::Backward => (i.saturating_sub(options.window), i),
            };
            let labels: Vec<_> = trace[start..end]
                .iter()
                .map(|&a| Label::from(sequences.activities.name(a)))
                .collect();
            let view = match options.view {
                TransitionAbstraction::Sequence => TransitionView::Sequence(labels),
                TransitionAbstraction::Set => TransitionView::Set(labels.into_iter().collect()),
                TransitionAbstraction::Multiset => {
                    let mut counts = BTreeMap::new();
                    for label in labels {
                        *counts.entry(label).or_default() += 1;
                    }
                    TransitionView::Multiset(counts)
                }
            };
            let state = *states.entry(view.clone()).or_insert_with(|| {
                let state = result.system.add_state(view.to_string());
                result.views.insert(state, view);
                state
            });
            path.push(state);
        }
        for (i, pair) in path.windows(2).enumerate() {
            let edge = result
                .system
                .add_edge(sequences.activities.name(trace[i]), pair[0], pair[1])
                .expect("states belong to the graph");
            if options.include_data {
                let event = TransitionEvent {
                    trace: ti,
                    event: i,
                };
                result
                    .state_data
                    .entry(pair[0])
                    .or_default()
                    .outgoing
                    .push(event);
                result
                    .state_data
                    .entry(pair[1])
                    .or_default()
                    .incoming
                    .push(event);
                result.edge_data.entry(edge).or_default().push(event);
            }
        }
    }
    Ok(result)
}

/// Discover only the existing model graph. Use [`discover_transition_system`]
/// to retain structured views and optional event annotations.
pub fn transition_system(
    log: &EventLog,
    keys: &EventKeys,
    options: &TransitionSystemOptions,
) -> Result<TransitionSystem> {
    Ok(discover_transition_system(log, keys, options)?.system)
}
