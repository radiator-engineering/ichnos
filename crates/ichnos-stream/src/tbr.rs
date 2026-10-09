//! Incremental classic token replay with per-case markings.

use crate::{Error, Result, StreamSink, StreamingConformanceOptions, conformance::Input};
use ichnos_core::Event;
use ichnos_model::{Marking, PetriNet, PlaceId, TransitionId, petri::ArcKind};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Settings for native-style shortest-path silent replay.
#[derive(Debug, Clone)]
pub struct StreamingTbrOptions {
    /// Activity/case keys and missing-field policy.
    pub events: StreamingConformanceOptions,
    /// Iteration limit including the attempt that fires the visible event.
    /// Default ten; zero skips silent exploration and inserts missing tokens.
    pub maximum_iterations_invisibles: usize,
}

impl Default for StreamingTbrOptions {
    fn default() -> Self {
        Self {
            events: Default::default(),
            maximum_iterations_invisibles: 10,
        }
    }
}

/// Diagnostics of an open case, before final-marking checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamingTbrStatus {
    /// Current marking using typed place IDs.
    pub marking: Marking,
    /// Tokens inserted while executing known activities.
    pub missing: u64,
}

impl StreamingTbrStatus {
    /// Prefix fitness considers missing tokens only.
    pub fn is_fit(&self) -> bool {
        self.missing == 0
    }
}

/// Final diagnostics, following the native streaming signed final differences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamingTbrTermination {
    /// Last replay marking, without an implicit silent completion walk.
    pub marking: Marking,
    /// Prefix missing tokens plus signed final-place shortfalls. A surplus
    /// in a final place can make this value negative, as in native streaming.
    pub missing: i64,
    /// Signed surplus over the final marking across all places.
    pub remaining: i64,
    /// True exactly when both native diagnostic counts are zero.
    pub is_fit: bool,
}

/// Streaming TBR using the shared Petri net firing rules. It stores a marking,
/// not an event history, per case. Unknown activities are counted and ignored.
/// Only normal arcs are accepted; nonnormal classic semantics are ambiguous.
#[derive(Debug, Clone)]
pub struct StreamingTbrConformance {
    input: Input,
    net: PetriNet,
    initial: Marking,
    final_marking: Marking,
    maximum_iterations: usize,
    transitions: Vec<TransitionId>,
    places: Vec<PlaceId>,
    paths: BTreeMap<(PlaceId, TransitionId), Vec<TransitionId>>,
    cases: BTreeMap<String, StreamingTbrStatus>,
    unknown: usize,
}

impl StreamingTbrConformance {
    /// Prepare a net, validating marking membership and normal arcs. Node
    /// name/ID order breaks shortest-path and duplicate-label ties deterministically.
    pub fn new(
        net: PetriNet,
        initial: Marking,
        final_marking: Marking,
        options: StreamingTbrOptions,
    ) -> Result<Self> {
        if initial
            .iter()
            .chain(final_marking.iter())
            .any(|(p, _)| !net.contains_place(p))
        {
            return Err(Error::InvalidConformanceOption(
                "marking contains a place outside the net",
            ));
        }
        if net.transitions().any(|(_, t)| {
            t.in_arcs()
                .iter()
                .chain(t.out_arcs())
                .any(|&a| net.arc(a).kind != ArcKind::Normal)
        }) {
            return Err(Error::InvalidConformanceOption(
                "streaming TBR requires normal arcs",
            ));
        }
        let mut transitions: Vec<_> = net.transition_ids().collect();
        transitions.sort_by_key(|&t| (net.transition(t).name.clone(), t));
        let mut places: Vec<_> = net.place_ids().collect();
        places.sort_by_key(|&p| (net.place(p).name.clone(), p));
        let mut paths = BTreeMap::new();
        for &source in &places {
            let mut seen = BTreeSet::from([source]);
            let mut targets = BTreeSet::new();
            let mut queue = VecDeque::from([(source, Vec::<TransitionId>::new())]);
            while let Some((place, path)) = queue.pop_front() {
                let mut after: Vec<_> = net.place_postset(place).collect();
                after.sort_by_key(|&t| (net.transition(t).name.clone(), t));
                for t in after {
                    if net.transition(t).label.is_some() {
                        if targets.insert(t) && !path.is_empty() {
                            paths.insert((source, t), path.clone());
                        }
                    } else {
                        let mut next_path = path.clone();
                        next_path.push(t);
                        let mut after: Vec<_> = net
                            .transition(t)
                            .out_arcs()
                            .iter()
                            .map(|&a| net.arc(a).place())
                            .collect();
                        after.sort_by_key(|&p| (net.place(p).name.clone(), p));
                        for p in after {
                            if seen.insert(p) {
                                queue.push_back((p, next_path.clone()));
                            }
                        }
                    }
                }
            }
        }
        Ok(Self {
            input: Input {
                options: options.events,
                ..Default::default()
            },
            net,
            initial,
            final_marking,
            maximum_iterations: options.maximum_iterations_invisibles,
            transitions,
            places,
            paths,
            cases: BTreeMap::new(),
            unknown: 0,
        })
    }

    /// Consume an event, trying shortest silent paths before inserting tokens.
    /// If a selected silent path fails, fallback uses the original marking
    /// rather than the native null-marking exception; no partial path is committed.
    pub fn push(&mut self, event: &Event) -> Result<()> {
        let Some((case, activity)) = self.input.fields(event, &[])? else {
            return Ok(());
        };
        let matching: Vec<_> = self
            .transitions
            .iter()
            .copied()
            .filter(|&t| {
                self.net
                    .transition(t)
                    .label
                    .as_ref()
                    .is_some_and(|l| l.as_str() == activity)
            })
            .collect();
        if matching.is_empty() {
            self.unknown += 1;
            return Ok(());
        }
        let mut status = self
            .cases
            .get(&case)
            .cloned()
            .unwrap_or_else(|| StreamingTbrStatus {
                marking: self.initial.clone(),
                missing: 0,
            });
        let original = status.marking.clone();
        let mut current = original.clone();
        let mut fired = false;
        for _ in 0..self.maximum_iterations {
            if let Some(&t) = matching.iter().find(|&&t| self.net.is_enabled(t, &current)) {
                status.marking = self.net.weak_fire(t, &current);
                fired = true;
                break;
            }
            let Some(next) = self.enable_with_invisibles(&current, &matching) else {
                break;
            };
            if next == current {
                break;
            }
            current = next;
        }
        if !fired {
            let t = matching[0];
            let mut marking = original;
            for &a in self.net.transition(t).in_arcs() {
                let arc = self.net.arc(a);
                let missing = arc.weight.saturating_sub(marking.get(arc.place()));
                status.missing += u64::from(missing);
                marking.add(arc.place(), missing);
            }
            status.marking = self.net.weak_fire(t, &marking);
        }
        self.cases.insert(case, status);
        Ok(())
    }

    fn enable_with_invisibles(
        &self,
        marking: &Marking,
        matching: &[TransitionId],
    ) -> Option<Marking> {
        let mut best: Option<&Vec<TransitionId>> = None;
        for &p in &self.places {
            if marking.get(p) == 0 {
                continue;
            }
            for &t in matching {
                if let Some(path) = self.paths.get(&(p, t))
                    && best.is_none_or(|known| path.len() < known.len())
                {
                    best = Some(path);
                }
            }
        }
        let mut next = marking.clone();
        for &t in best? {
            next = self.net.fire(t, &next).ok()?;
        }
        Some(next)
    }

    /// Open-case marking snapshots, in lexical case order.
    pub fn get(&self) -> &BTreeMap<String, StreamingTbrStatus> {
        &self.cases
    }

    /// Status of one open case; absent and unknown-only cases return None.
    pub fn get_status(&self, case: &str) -> Option<&StreamingTbrStatus> {
        self.cases.get(case)
    }

    /// Apply native streaming signed final diagnostics and release the case.
    /// No silent completion occurs: native terminate passes an encoded marking
    /// to its place-keyed final search and cannot follow a final silent path.
    pub fn terminate(&mut self, case: &str) -> Option<StreamingTbrTermination> {
        let status = self.cases.remove(case)?;
        let mut missing = status.missing as i64;
        let mut remaining = 0;
        if status.marking != self.final_marking {
            for (p, n) in self.final_marking.iter() {
                missing += i64::from(n) - i64::from(status.marking.get(p));
            }
            remaining = self
                .places
                .iter()
                .map(|&p| i64::from(status.marking.get(p)) - i64::from(self.final_marking.get(p)))
                .sum();
        }
        Some(StreamingTbrTermination {
            marking: status.marking,
            missing,
            remaining,
            is_fit: missing == 0 && remaining == 0,
        })
    }

    /// Terminate every case and return all final diagnostics.
    pub fn terminate_all(&mut self) -> BTreeMap<String, StreamingTbrTermination> {
        let keys: Vec<_> = self.cases.keys().cloned().collect();
        keys.into_iter()
            .map(|case| {
                let result = self.terminate(&case).unwrap();
                (case, result)
            })
            .collect()
    }

    /// The owned model, for interpreting typed marking IDs.
    pub fn net(&self) -> &PetriNet {
        &self.net
    }

    /// Count of incomplete events ignored by the configured policy.
    pub fn skipped(&self) -> usize {
        self.input.skipped
    }

    /// Count of events with an activity absent from the model.
    pub fn unknown_activities(&self) -> usize {
        self.unknown
    }
}

impl StreamSink for StreamingTbrConformance {
    fn push(&mut self, event: &Event) -> Result<()> {
        Self::push(self, event)
    }
}
