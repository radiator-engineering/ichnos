//! Synchronous product of a trace and a Petri net, ported from pm4py's
//! `petri_utils.construct_trace_net` and
//! `objects/petri_net/utils/synchronous_product.py`.

use crate::Label;
use crate::petri::{AcceptingPetriNet, Marking, PetriNet, PlaceId, TransitionId};

/// pm4py's `SKIP`, the empty side of a log or model move.
pub const SKIP: &str = ">>";

/// What a transition of a [`SynchronousProduct`] does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyncMove {
    /// Consume the trace event at this index alone.
    Log(usize),
    /// Fire this transition of the model alone.
    Model(TransitionId),
    /// Consume the trace event and fire the model transition together.
    Sync(usize, TransitionId),
}

/// The synchronous product net of a trace and an accepting Petri net
/// (pm4py's `construct_synchronous_product_net`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SynchronousProduct {
    /// The product net with its initial and final markings.
    pub net: AcceptingPetriNet,
    /// The trace's activities.
    pub trace: Vec<Label>,
    moves: Vec<Option<(SyncMove, u64)>>,
}

fn pair(a: &str, b: &str) -> String {
    format!("({a}, {b})")
}

impl SynchronousProduct {
    /// Builds the product of `trace` and `model`.
    ///
    /// The trace net has places `p_0` to `p_n` and a transition `t_<a>_<i>`
    /// labelled `a` for the event `a` at index `i`. pm4py names product
    /// places and transitions with Python pairs; here a pair is the string
    /// `"(x, y)"`, with [`SKIP`] on the empty side and `None` for a silent
    /// model transition's label. So a log move is `(t_a_0, >>)` labelled
    /// `(a, >>)`, a model move `(>>, t)` labelled `(>>, a)`, and a
    /// synchronous move `(t_a_0, t)` labelled `(a, a)`. Every product
    /// transition has a label, as in pm4py; [`SynchronousProduct::move_of`]
    /// tells the kinds apart.
    ///
    /// Places and transitions are added in this order: the trace net's,
    /// the model's (in id order), then the synchronous moves by event and
    /// model transition. Arcs keep the model's weights and kinds; pm4py
    /// copies them as normal arcs of weight 1.
    pub fn new(trace: &[Label], model: &AcceptingPetriNet) -> Self {
        let mut net = PetriNet::new("");
        let mut moves = Vec::new();
        let set_move = |moves: &mut Vec<Option<(SyncMove, u64)>>, t: TransitionId, m: SyncMove| {
            let cost = match m {
                SyncMove::Sync(..) => 0,
                SyncMove::Model(u) if model.net.transition(u).label.is_none() => 1,
                SyncMove::Log(_) | SyncMove::Model(_) => 10_000,
            };
            if moves.len() <= t.index() {
                moves.resize(t.index() + 1, None);
            }
            moves[t.index()] = Some((m, cost));
        };

        let trace_places: Vec<PlaceId> = (0..=trace.len())
            .map(|i| net.add_place(pair(&format!("p_{i}"), SKIP)))
            .collect();
        let mut trace_transitions = Vec::new();
        for (i, a) in trace.iter().enumerate() {
            let t = net.add_transition(
                pair(&format!("t_{a}_{i}"), SKIP),
                Some(pair(a.as_str(), SKIP)),
            );
            set_move(&mut moves, t, SyncMove::Log(i));
            net.add_input_arc(trace_places[i], t).expect("live ids");
            net.add_output_arc(t, trace_places[i + 1])
                .expect("live ids");
            trace_transitions.push(t);
        }

        let m = &model.net;
        let mut place_map = vec![None; m.place_index_bound()];
        for (p, place) in m.places() {
            place_map[p.index()] = Some(net.add_place(pair(SKIP, &place.name)));
        }
        let mapped = |p: PlaceId| place_map[p.index()].expect("model place");
        let copy_arcs = |net: &mut PetriNet, from: TransitionId, to: TransitionId| {
            for &a in m
                .transition(from)
                .in_arcs()
                .iter()
                .chain(m.transition(from).out_arcs())
            {
                let arc = m.arc(a);
                let ends = match arc.ends {
                    crate::petri::ArcEnds::PlaceToTransition(p, _) => {
                        crate::petri::ArcEnds::PlaceToTransition(mapped(p), to)
                    }
                    crate::petri::ArcEnds::TransitionToPlace(_, p) => {
                        crate::petri::ArcEnds::TransitionToPlace(to, mapped(p))
                    }
                };
                net.add_arc(ends, arc.weight, arc.kind).expect("live ids");
            }
        };
        for (t, tr) in m.transitions() {
            let label = tr.label.as_deref().unwrap_or("None");
            let u = net.add_transition(pair(SKIP, &tr.name), Some(pair(SKIP, label)));
            set_move(&mut moves, u, SyncMove::Model(t));
            copy_arcs(&mut net, t, u);
        }

        for (i, a) in trace.iter().enumerate() {
            for (t, tr) in m.transitions() {
                if tr.label.as_ref() != Some(a) {
                    continue;
                }
                let u = net.add_transition(
                    pair(&format!("t_{a}_{i}"), &tr.name),
                    Some(pair(a.as_str(), a.as_str())),
                );
                set_move(&mut moves, u, SyncMove::Sync(i, t));
                net.add_input_arc(trace_places[i], u).expect("live ids");
                net.add_output_arc(u, trace_places[i + 1])
                    .expect("live ids");
                copy_arcs(&mut net, t, u);
            }
        }

        let mut im = Marking::new();
        let mut fm = Marking::new();
        im.set(trace_places[0], 1);
        fm.set(trace_places[trace.len()], 1);
        for (p, n) in model.initial_marking.iter() {
            im.set(mapped(p), n);
        }
        for (p, n) in model.final_marking.iter() {
            fm.set(mapped(p), n);
        }
        Self {
            net: AcceptingPetriNet::new(net, im, fm),
            trace: trace.to_vec(),
            moves,
        }
    }

    /// The move a transition of the product stands for.
    ///
    /// # Panics
    ///
    /// If `t` is not a transition of the product.
    pub fn move_of(&self, t: TransitionId) -> SyncMove {
        self.entry(t).0
    }

    /// pm4py's standard alignment cost of a transition: 0 for a
    /// synchronous move, 1 for a silent model move, 10000 for any other
    /// log or model move.
    ///
    /// # Panics
    ///
    /// If `t` is not a transition of the product.
    pub fn standard_cost(&self, t: TransitionId) -> u64 {
        self.entry(t).1
    }

    fn entry(&self, t: TransitionId) -> (SyncMove, u64) {
        self.moves
            .get(t.index())
            .copied()
            .flatten()
            .expect("transition of the synchronous product")
    }
}
