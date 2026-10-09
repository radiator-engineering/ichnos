//! Replay of one trace (pm4py's `token_replay.apply_trace`).

use std::collections::{BTreeSet, VecDeque};

use ichnos_model::TransitionId;
use rustc_hash::FxHashSet;

use super::net::{ReplayNet, Tokens};
use super::{TokenReplayer, TraceReplay};
use crate::error::Result;

/// pm4py's `MAX_REC_DEPTH_HIDTRANSENABL`.
const MAX_HIDDEN_DEPTH: usize = 2;
/// pm4py's `MAX_IT_FINAL1` and `MAX_IT_FINAL2`.
const FINAL_ROUNDS: usize = 5;
/// pm4py's bound on the rounds of `enable_hidden_transitions`.
const ENABLE_ROUNDS: usize = 10_000_000;

/// The running state of one replay.
struct State<'a> {
    net: &'a ReplayNet,
    exhaustive: bool,
    marking: Tokens,
    activated: Vec<TransitionId>,
    consumed: u64,
    produced: u64,
}

impl State<'_> {
    /// Fires `t` and counts its tokens.
    fn fire_counted(&mut self, t: TransitionId) {
        self.net.fire(t, &mut self.marking);
        self.activated.push(t);
        self.consumed += self.net.consumed[t.index()];
        self.produced += self.net.produced[t.index()];
    }

    /// Places where `t` lacks tokens, in name order.
    fn missing_places(&self, t: TransitionId) -> Vec<usize> {
        let mut places: Vec<usize> = self.net.pre[t.index()]
            .iter()
            .filter(|&&(p, w)| self.marking[p] < w)
            .map(|&(p, _)| p)
            .collect();
        places.sort_by_key(|&p| self.net.place_rank[p]);
        places.dedup();
        places
    }

    fn marked(&self) -> Vec<usize> {
        self.net.marked_by_name(&self.marking).collect()
    }

    /// pm4py's `apply_hidden_trans`: fires silent transitions on shortest
    /// paths towards the places where `t` lacks tokens.
    ///
    /// Silent transitions fired here are added to `activated` but not yet
    /// counted; the caller counts them.
    fn apply_hidden(
        &mut self,
        t: TransitionId,
        depth: usize,
        visited: &mut FxHashSet<TransitionId>,
    ) {
        if depth >= MAX_HIDDEN_DEPTH || visited.contains(&t) {
            return;
        }
        visited.insert(t);
        let net = self.net;
        let at_start = self.marking.clone();
        let missing = self.missing_places(t);
        let groups = net.paths_between(&self.marked(), &missing);
        if groups.is_empty() {
            return;
        }
        // In pm4py's exhaustive search a successful search hands back a new
        // set of visited transitions, so later additions in this call do not
        // reach the caller's set.
        let mut own_visited;
        let visited: &mut FxHashSet<TransitionId> = if self.exhaustive {
            match self.enable_exhaustive(t, &groups, visited) {
                Some(new_visited) => {
                    own_visited = new_visited;
                    &mut own_visited
                }
                None => visited,
            }
        } else {
            self.enable_in_rounds(t, &groups, visited);
            visited
        };
        if !net.is_enabled(t, &self.marking) {
            let groups = net.paths_between(&self.marked(), &missing);
            for group in groups {
                for &t4 in group {
                    if t4 == t || visited.contains(&t4) {
                        continue;
                    }
                    if !net.is_enabled(t4, &self.marking) {
                        self.apply_hidden(t4, depth + 1, visited);
                    }
                    if net.is_enabled(t4, &self.marking) {
                        net.fire(t4, &mut self.marking);
                        self.activated.push(t4);
                        visited.insert(t4);
                    }
                }
            }
        }
        if !net.is_enabled(t, &self.marking) && at_start != self.marking {
            self.apply_hidden(t, depth + 1, visited);
        }
    }

    /// pm4py's `enable_hidden_transitions` without exhaustive exploration:
    /// walks the groups in turn and fires what is enabled, until `t` is
    /// enabled or a group fires nothing.
    fn enable_in_rounds(
        &mut self,
        t: TransitionId,
        groups: &[&[TransitionId]],
        visited: &mut FxHashSet<TransitionId>,
    ) {
        let net = self.net;
        let mut next = vec![0usize; groups.len()];
        for z in 0..ENABLE_ROUNDS {
            let g = z % groups.len();
            let mut changed = false;
            while next[g] < groups[g].len() {
                let t3 = groups[g][next[g]];
                if t3 != t && net.is_enabled(t3, &self.marking) && !visited.contains(&t3) {
                    net.fire(t3, &mut self.marking);
                    self.activated.push(t3);
                    visited.insert(t3);
                    changed = true;
                }
                next[g] += 1;
                if net.is_enabled(t, &self.marking) {
                    break;
                }
            }
            if net.is_enabled(t, &self.marking) || !changed {
                break;
            }
        }
    }

    /// pm4py's `enable_hidden_transitions` with exhaustive exploration: a
    /// breadth-first search over firings of the groups' transitions for a
    /// marking that enables `t`. pm4py skips a marking when one with the
    /// same marked places was expanded before.
    ///
    /// On success, updates the marking and activated transitions and
    /// returns the visited transitions of the path found. Returns `None`
    /// when no such marking is found, leaving the state as it was.
    fn enable_exhaustive(
        &mut self,
        t: TransitionId,
        groups: &[&[TransitionId]],
        visited: &FxHashSet<TransitionId>,
    ) -> Option<FxHashSet<TransitionId>> {
        let net = self.net;
        let mut queue = VecDeque::new();
        queue.push_back((self.marking.clone(), Vec::new(), visited.clone()));
        let mut seen: FxHashSet<Vec<usize>> = FxHashSet::default();
        let mut first = true;
        while let Some((m, fired, vis)) = queue.pop_front() {
            if net.is_enabled(t, &m) {
                if first {
                    return None;
                }
                self.marking = m;
                self.activated.extend(fired);
                return Some(vis);
            }
            first = false;
            let support: Vec<usize> = (0..m.len()).filter(|&p| m[p] > 0).collect();
            if !seen.insert(support) {
                continue;
            }
            for group in groups {
                for &t3 in *group {
                    if t3 != t && !vis.contains(&t3) && net.is_enabled(t3, &m) {
                        let mut m2 = m.clone();
                        net.fire(t3, &mut m2);
                        let mut fired2 = fired.clone();
                        fired2.push(t3);
                        let mut vis2 = vis.clone();
                        vis2.insert(t3);
                        queue.push_back((m2, fired2, vis2));
                    }
                }
            }
        }
        None
    }

    /// pm4py's `break_condition_final_marking`: every place of the final
    /// marking holds a token.
    fn covers_final_places(&self, fm: &[usize]) -> bool {
        fm.iter().all(|&p| self.marking[p] > 0)
    }
}

impl TokenReplayer {
    /// pm4py's `apply_trace` with caches off, as pm4py runs it outside a
    /// reduction.
    pub(crate) fn replay_trace<S: AsRef<str>>(&self, trace: &[S]) -> Result<TraceReplay> {
        let net = &self.net;
        let opts = &self.options;
        let mut state = State {
            net,
            exhaustive: opts.exhaustive_invisible_exploration,
            marking: self.initial.clone(),
            activated: Vec::new(),
            consumed: 0,
            produced: self.initial.iter().map(|&n| u64::from(n)).sum(),
        };
        let mut missing: u64 = 0;
        let mut problems = Vec::new();
        let mut flooded: u64 = 0;
        let mut not_in_model = false;

        for activity in trace {
            let activity = activity.as_ref();
            let Some(&mapped) = self.by_label.get(activity) else {
                not_in_model = true;
                continue;
            };
            // An enabled transition with the label wins; otherwise always
            // the same one, so that duplicate labels replay the same way.
            let t = self.labelled[activity]
                .iter()
                .copied()
                .find(|&t| net.is_enabled(t, &state.marking))
                .unwrap_or(mapped);
            if opts.walk_through_hidden_transitions && !net.is_enabled(t, &state.marking) {
                let before = state.activated.len();
                let mut visited = FxHashSet::default();
                state.apply_hidden(t, 0, &mut visited);
                for &h in &state.activated[before..] {
                    state.consumed += net.consumed[h.index()];
                    state.produced += net.produced[h.index()];
                }
            }
            let marked_before: Vec<bool> = state.marking.iter().map(|&n| n > 0).collect();
            let enabled = net.is_enabled(t, &state.marking);
            if !enabled {
                problems.push(t);
                if opts.stop_immediately_unfit {
                    missing += 1;
                    break;
                }
                // pm4py adds the arc weight, not the shortfall, so a
                // weighted arc can leave extra tokens behind.
                for &(p, w) in &net.pre[t.index()] {
                    if state.marking[p] < w {
                        missing += u64::from(w - state.marking[p]);
                        state.marking[p] += w;
                    }
                }
            }
            state.fire_counted(t);
            if !enabled && opts.cleaning_token_flood {
                flooded += self.clean_token_flood(&mut state.marking, &marked_before);
            }
        }

        if opts.try_to_reach_final_marking_through_hidden {
            self.reach_final_marking(&mut state);
        }

        let reached = state.marking;
        let mut missing_final: u64 = 0;
        let mut remaining: u64 = flooded;
        for (p, &n) in reached.iter().enumerate() {
            let target = self.final_tokens[p];
            if target > n {
                missing_final += u64::from(target - n);
            }
            remaining += u64::from(n.saturating_sub(target));
        }
        let mut is_fit = if opts.consider_remaining_in_fitness {
            missing == 0 && remaining == 0
        } else {
            missing == 0
        };
        if opts.consider_activities_not_in_model_in_fitness && not_in_model {
            is_fit = false;
        }
        let consumed =
            state.consumed + self.final_tokens.iter().map(|&n| u64::from(n)).sum::<u64>();
        let missing = missing + missing_final;
        let produced = state.produced;
        let fitness = if consumed > 0 && produced > 0 {
            0.5 * (1.0 - missing as f64 / consumed as f64)
                + 0.5 * (1.0 - remaining as f64 / produced as f64)
        } else {
            1.0
        };
        let enabled_transitions_in_marking = self.eventually_enabled(&reached)?;
        Ok(TraceReplay {
            is_fit,
            fitness,
            activated_transitions: state.activated,
            transitions_with_problems: problems,
            reached_marking: net.marking(&reached),
            enabled_transitions_in_marking,
            missing_tokens: missing,
            consumed_tokens: consumed,
            remaining_tokens: remaining,
            produced_tokens: produced,
        })
    }

    /// The end of pm4py's `apply_trace`: fire silent transitions on the
    /// shortest paths towards the final marking, then, for a final marking
    /// on one place, along the paths to that place.
    fn reach_final_marking(&self, state: &mut State<'_>) {
        let net = &self.net;
        let fm = &self.final_places;
        for _ in 0..FINAL_ROUNDS {
            if state.covers_final_places(fm) {
                break;
            }
            let groups = net.paths_between(&state.marked(), fm);
            for group in groups {
                for &t in group {
                    if net.is_enabled(t, &state.marking) {
                        state.fire_counted(t);
                    }
                }
                if state.covers_final_places(fm) {
                    break;
                }
            }
        }
        if state.covers_final_places(fm) {
            return;
        }
        let [sink] = fm[..] else {
            return;
        };
        let mut connections: Vec<&[TransitionId]> = state
            .marked()
            .into_iter()
            .filter_map(|p| net.paths[p].get(&sink).map(Vec::as_slice))
            .collect();
        connections.sort_by_key(|path| path.len());
        for _ in 0..FINAL_ROUNDS {
            for path in &connections {
                for &t in *path {
                    if !net.is_enabled(t, &state.marking) {
                        break;
                    }
                    state.fire_counted(t);
                }
            }
        }
    }

    /// pm4py's token-flood cleaning after a transition fired with missing
    /// tokens: a place that held tokens before and shares an S-component
    /// with a place that just got its first token is emptied. Returns the
    /// number of places emptied; pm4py counts one remaining token for each,
    /// whatever the place held.
    fn clean_token_flood(&self, marking: &mut [u32], marked_before: &[bool]) -> u64 {
        let net = &self.net;
        let now: Vec<usize> = net.marked_by_name(marking).collect();
        let fresh: Vec<usize> = now.iter().copied().filter(|&p| !marked_before[p]).collect();
        let kept: Vec<usize> = now.iter().copied().filter(|&p| marked_before[p]).collect();
        let mut cleaned = 0;
        for p1 in kept {
            let shares = fresh.iter().any(|&p2| {
                self.s_components
                    .iter()
                    .any(|c| c.contains(&p1) && c.contains(&p2))
            });
            if shares {
                marking[p1] = 0;
                cleaned += 1;
            }
        }
        cleaned
    }

    /// pm4py's `get_visible_transitions_eventually_enabled_by_marking`
    /// (see [`ReplayNet::eventually_enabled`]).
    pub(crate) fn eventually_enabled(&self, marking: &[u32]) -> Result<BTreeSet<TransitionId>> {
        self.net.eventually_enabled(marking)
    }
}
