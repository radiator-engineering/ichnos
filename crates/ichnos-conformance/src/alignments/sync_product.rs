//! The synchronous product of a trace and a net, in a flat form built for
//! the search.
//!
//! pm4py builds the product as a `PetriNet` of the trace net and the model
//! (`synchronous_product.construct_cost_aware`). Here the model half is
//! prepared once per net ([`ModelPart`]) and each trace adds its own places
//! and moves ([`SyncProduct`]). Places are dense `u32` indices: the model's
//! places first, then the trace places `0..=n`.

use rustc_hash::FxHashMap;

use ichnos_model::petri::{ArcEnds, ArcKind};
use ichnos_model::{Marking, PetriNet, TransitionId};

use super::costs::ModelCosts;
use super::marking::{Packed, pack};
use super::result::Move;
use crate::error::{Error, Result};

/// A model transition with its preset and postset over dense places.
#[derive(Debug, Clone)]
struct ModelTransition {
    id: TransitionId,
    pre: Vec<(u32, u32)>,
    post: Vec<(u32, u32)>,
    model_cost: u64,
    sync_cost: u64,
    silent: bool,
}

/// The model half of every synchronous product of one net.
#[derive(Debug, Clone)]
pub(crate) struct ModelPart {
    place_count: u32,
    transitions: Vec<ModelTransition>,
    by_label: FxHashMap<Box<str>, Vec<u32>>,
    initial: Vec<Packed>,
    final_marking: Vec<Packed>,
    dense_place: Vec<u32>,
}

impl ModelPart {
    pub(crate) fn new(
        net: &PetriNet,
        initial: &Marking,
        final_marking: &Marking,
        costs: &ModelCosts,
    ) -> Result<Self> {
        if net.has_special_arcs() {
            return Err(Error::SpecialArcs);
        }
        let mut dense_place = vec![u32::MAX; net.place_index_bound()];
        for (i, p) in net.place_ids().enumerate() {
            dense_place[p.index()] = u32::try_from(i).expect("place count fits u32");
        }
        let place_count = u32::try_from(net.place_count()).expect("place count fits u32");
        let mut transitions = Vec::with_capacity(net.transition_count());
        let mut by_label: FxHashMap<Box<str>, Vec<u32>> = FxHashMap::default();
        for (id, tr) in net.transitions() {
            let mut pre = Vec::new();
            let mut post = Vec::new();
            for &a in tr.in_arcs().iter().chain(tr.out_arcs()) {
                let arc = net.arc(a);
                debug_assert_eq!(arc.kind, ArcKind::Normal);
                match arc.ends {
                    ArcEnds::PlaceToTransition(p, _) => {
                        pre.push((dense_place[p.index()], arc.weight))
                    }
                    ArcEnds::TransitionToPlace(_, p) => {
                        post.push((dense_place[p.index()], arc.weight));
                    }
                }
            }
            let index = u32::try_from(transitions.len()).expect("transition count fits u32");
            if let Some(label) = &tr.label {
                by_label
                    .entry(label.as_str().into())
                    .or_default()
                    .push(index);
            }
            transitions.push(ModelTransition {
                id,
                pre: merge_weights(pre),
                post: merge_weights(post),
                model_cost: costs.model_move(id),
                sync_cost: costs.sync_move(id),
                silent: tr.is_silent(),
            });
        }
        let encode = |m: &Marking| -> Vec<Packed> {
            m.iter()
                .filter_map(|(p, n)| {
                    let d = *dense_place.get(p.index())?;
                    (d != u32::MAX).then(|| pack(d, n))
                })
                .collect::<Vec<_>>()
        };
        let mut initial = encode(initial);
        let mut final_marking = encode(final_marking);
        initial.sort_unstable();
        final_marking.sort_unstable();
        Ok(Self {
            place_count,
            transitions,
            by_label,
            initial,
            final_marking,
            dense_place,
        })
    }

    /// The dense index of a place of the net, if it exists.
    pub(crate) fn dense_place(&self, p: ichnos_model::PlaceId) -> Option<u32> {
        self.dense_place
            .get(p.index())
            .copied()
            .filter(|&d| d != u32::MAX)
    }

    /// The number of model places.
    pub(crate) fn place_count(&self) -> u32 {
        self.place_count
    }
}

/// Sorts `(place, weight)` pairs by place and adds up repeated places.
fn merge_weights(mut v: Vec<(u32, u32)>) -> Vec<(u32, u32)> {
    v.sort_unstable();
    let mut out: Vec<(u32, u32)> = Vec::with_capacity(v.len());
    for (p, w) in v {
        match out.last_mut() {
            Some(last) if last.0 == p => last.1 += w,
            _ => out.push((p, w)),
        }
    }
    out
}

/// Which moves a search may take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MoveSet {
    /// Every move: the alignment search.
    All,
    /// Synchronous moves and model moves on silent transitions only: the
    /// prefix replay of align-ETConformance precision.
    SyncAndSilent,
}

/// The synchronous product of one trace and a net, as flat arrays.
#[derive(Debug, Clone)]
pub(crate) struct SyncProduct {
    pub(crate) place_count: usize,
    pub(crate) moves: Vec<Move>,
    pub(crate) cost: Vec<u64>,
    pre_start: Vec<u32>,
    pre: Vec<(u32, u32)>,
    delta_start: Vec<u32>,
    delta: Vec<(u32, i64)>,
    consumers_start: Vec<u32>,
    consumers: Vec<u32>,
    pub(crate) always_enabled: Vec<u32>,
    pub(crate) initial: Vec<Packed>,
    pub(crate) final_marking: Vec<Packed>,
    /// The dense index of the last trace place.
    pub(crate) trace_end: u32,
}

impl SyncProduct {
    /// Builds the product of `model` and the trace `labels`, where event `i`
    /// costs `log_cost[i]` as a log move.
    pub(crate) fn new<S: AsRef<str>>(
        model: &ModelPart,
        labels: &[S],
        log_cost: &[u64],
        moves: MoveSet,
    ) -> Self {
        debug_assert_eq!(labels.len(), log_cost.len());
        let n = labels.len();
        let trace_place = |i: usize| model.place_count + u32::try_from(i).expect("trace fits u32");
        let mut b = Builder::default();
        for mt in &model.transitions {
            if moves == MoveSet::SyncAndSilent && !mt.silent {
                continue;
            }
            b.push(
                Move::Model { transition: mt.id },
                mt.model_cost,
                &mt.pre,
                &mt.post,
            );
        }
        for (i, label) in labels.iter().enumerate() {
            let here = [(trace_place(i), 1)];
            let next = [(trace_place(i + 1), 1)];
            if moves == MoveSet::All {
                b.push(Move::Log { event: i }, log_cost[i], &here, &next);
            }
            for &j in model
                .by_label
                .get(label.as_ref())
                .map_or(&[][..], Vec::as_slice)
            {
                let mt = &model.transitions[j as usize];
                let pre = merge_weights(mt.pre.iter().copied().chain(here).collect());
                let post = merge_weights(mt.post.iter().copied().chain(next).collect());
                b.push(
                    Move::Sync {
                        event: i,
                        transition: mt.id,
                    },
                    mt.sync_cost,
                    &pre,
                    &post,
                );
            }
        }
        let place_count = model.place_count as usize + n + 1;
        let mut initial = model.initial.clone();
        initial.push(pack(trace_place(0), 1));
        let mut final_marking = model.final_marking.clone();
        final_marking.push(pack(trace_place(n), 1));
        b.finish(place_count, initial, final_marking, trace_place(n))
    }

    /// The number of moves (transitions of the product).
    pub(crate) fn len(&self) -> usize {
        self.moves.len()
    }

    /// The preset of move `t`.
    pub(crate) fn pre(&self, t: usize) -> &[(u32, u32)] {
        &self.pre[self.pre_start[t] as usize..self.pre_start[t + 1] as usize]
    }

    /// The token change of move `t`, sorted by place, without zeros.
    pub(crate) fn delta(&self, t: usize) -> &[(u32, i64)] {
        &self.delta[self.delta_start[t] as usize..self.delta_start[t + 1] as usize]
    }

    /// The moves that consume from place `p`.
    pub(crate) fn consumers(&self, p: u32) -> &[u32] {
        let p = p as usize;
        &self.consumers[self.consumers_start[p] as usize..self.consumers_start[p + 1] as usize]
    }
}

#[derive(Default)]
struct Builder {
    moves: Vec<Move>,
    cost: Vec<u64>,
    pre_start: Vec<u32>,
    pre: Vec<(u32, u32)>,
    delta_start: Vec<u32>,
    delta: Vec<(u32, i64)>,
}

impl Builder {
    fn push(&mut self, mv: Move, cost: u64, pre: &[(u32, u32)], post: &[(u32, u32)]) {
        self.moves.push(mv);
        self.cost.push(cost);
        self.pre_start.push(to_u32(self.pre.len()));
        self.pre.extend_from_slice(pre);
        self.delta_start.push(to_u32(self.delta.len()));
        // Both lists are sorted by place: merge them into the net change.
        let (mut i, mut j) = (0, 0);
        while i < pre.len() || j < post.len() {
            let (p, d) = match (pre.get(i), post.get(j)) {
                (Some(&(pp, w)), Some(&(qp, _))) if pp < qp => {
                    i += 1;
                    (pp, -i64::from(w))
                }
                (Some(&(pp, w)), Some(&(qp, v))) if pp == qp => {
                    i += 1;
                    j += 1;
                    (pp, i64::from(v) - i64::from(w))
                }
                (_, Some(&(qp, v))) => {
                    j += 1;
                    (qp, i64::from(v))
                }
                (Some(&(pp, w)), None) => {
                    i += 1;
                    (pp, -i64::from(w))
                }
                (None, None) => unreachable!(),
            };
            if d != 0 {
                self.delta.push((p, d));
            }
        }
    }

    fn finish(
        mut self,
        place_count: usize,
        initial: Vec<Packed>,
        final_marking: Vec<Packed>,
        trace_end: u32,
    ) -> SyncProduct {
        let len = self.moves.len();
        self.pre_start.push(to_u32(self.pre.len()));
        self.delta_start.push(to_u32(self.delta.len()));
        let mut counts = vec![0u32; place_count + 1];
        let mut always_enabled = Vec::new();
        for t in 0..len {
            let pre = &self.pre[self.pre_start[t] as usize..self.pre_start[t + 1] as usize];
            if pre.is_empty() {
                always_enabled.push(to_u32(t));
            }
            for &(p, _) in pre {
                counts[p as usize + 1] += 1;
            }
        }
        for p in 0..place_count {
            counts[p + 1] += counts[p];
        }
        let consumers_start = counts.clone();
        let mut consumers = vec![0u32; counts[place_count] as usize];
        for t in 0..len {
            for &(p, _) in &self.pre[self.pre_start[t] as usize..self.pre_start[t + 1] as usize] {
                let slot = &mut counts[p as usize];
                consumers[*slot as usize] = to_u32(t);
                *slot += 1;
            }
        }
        SyncProduct {
            place_count,
            moves: self.moves,
            cost: self.cost,
            pre_start: self.pre_start,
            pre: self.pre,
            delta_start: self.delta_start,
            delta: self.delta,
            consumers_start,
            consumers,
            always_enabled,
            initial,
            final_marking,
            trace_end,
        }
    }
}

fn to_u32(n: usize) -> u32 {
    u32::try_from(n).expect("synchronous product fits u32 indices")
}
