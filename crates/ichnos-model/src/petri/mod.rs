//! Place/transition Petri nets, with optional inhibitor and reset arcs.
//!
//! A [`PetriNet`] stores its places, transitions and arcs in arenas. Elements
//! are addressed by the typed indices [`PlaceId`], [`TransitionId`] and
//! [`ArcId`]. An id is only meaningful for the net that created it.
//!
//! Arcs always connect a place and a transition, so the bipartite rule holds
//! by construction. Inhibitor and reset arcs go from a place to a transition,
//! as in pm4py's `InhibitorNet` and `ResetNet`.

mod incidence;
mod marking;
mod reachability;
mod reduction;
mod semantics;

pub use incidence::IncidenceMatrix;
pub use marking::Marking;
pub use reachability::{ReachabilityError, ReachabilityGraph, ReachabilityOptions};
pub use semantics::NotEnabled;

use crate::Label;

/// Index of a place in a [`PetriNet`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlaceId(pub(crate) u32);

/// Index of a transition in a [`PetriNet`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransitionId(pub(crate) u32);

/// Index of an arc in a [`PetriNet`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArcId(pub(crate) u32);

macro_rules! impl_index {
    ($t:ty) => {
        impl $t {
            /// Returns the position of this element in its arena.
            pub fn index(self) -> usize {
                self.0 as usize
            }
        }
    };
}
impl_index!(PlaceId);
impl_index!(TransitionId);
impl_index!(ArcId);

fn to_u32(n: usize) -> u32 {
    u32::try_from(n).expect("a Petri net holds at most u32::MAX elements of each kind")
}

/// A place of a Petri net.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    /// The place name. pm4py uses it as the place id in PNML.
    pub name: String,
    in_arcs: Vec<ArcId>,
    out_arcs: Vec<ArcId>,
}

impl Place {
    /// Arcs that end in this place (from transitions).
    pub fn in_arcs(&self) -> &[ArcId] {
        &self.in_arcs
    }

    /// Arcs that start in this place (to transitions).
    pub fn out_arcs(&self) -> &[ArcId] {
        &self.out_arcs
    }
}

/// A transition of a Petri net.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transition {
    /// The transition name (its id in PNML).
    pub name: String,
    /// The activity label, or `None` for a silent (tau) transition.
    pub label: Option<Label>,
    in_arcs: Vec<ArcId>,
    out_arcs: Vec<ArcId>,
}

impl Transition {
    /// Arcs that end in this transition (from places).
    pub fn in_arcs(&self) -> &[ArcId] {
        &self.in_arcs
    }

    /// Arcs that start in this transition (to places).
    pub fn out_arcs(&self) -> &[ArcId] {
        &self.out_arcs
    }

    /// Returns `true` if the transition has no label.
    pub fn is_silent(&self) -> bool {
        self.label.is_none()
    }
}

/// The kind of an arc from a place to a transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArcKind {
    /// An ordinary arc: the transition consumes `weight` tokens.
    Normal,
    /// An inhibitor arc: the transition is enabled only while the place is
    /// empty. It consumes nothing. Its weight is kept but not used.
    Inhibitor,
    /// A reset arc: firing empties the place. It does not affect enabling.
    Reset,
}

/// The two ends of an arc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArcEnds {
    /// From a place to a transition.
    PlaceToTransition(PlaceId, TransitionId),
    /// From a transition to a place.
    TransitionToPlace(TransitionId, PlaceId),
}

/// An arc of a Petri net.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Arc {
    /// The source and target of the arc.
    pub ends: ArcEnds,
    /// The arc weight, at least 1.
    pub weight: u32,
    /// The arc kind. Arcs from a transition to a place are always
    /// [`ArcKind::Normal`].
    pub kind: ArcKind,
}

impl Arc {
    /// Returns the place this arc touches.
    pub fn place(&self) -> PlaceId {
        match self.ends {
            ArcEnds::PlaceToTransition(p, _) | ArcEnds::TransitionToPlace(_, p) => p,
        }
    }

    /// Returns the transition this arc touches.
    pub fn transition(&self) -> TransitionId {
        match self.ends {
            ArcEnds::PlaceToTransition(_, t) | ArcEnds::TransitionToPlace(t, _) => t,
        }
    }
}

/// Errors raised when building a Petri net.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PetriNetError {
    /// An arc weight was zero.
    #[error("arc weight must be at least 1")]
    ZeroWeight,
    /// A place id does not belong to this net.
    #[error("place {0:?} does not exist in this net")]
    UnknownPlace(PlaceId),
    /// A transition id does not belong to this net.
    #[error("transition {0:?} does not exist in this net")]
    UnknownTransition(TransitionId),
}

/// A Petri net stored in arenas.
///
/// Removing an element leaves a hole in its arena, so the ids of the other
/// elements stay valid. [`PetriNet::compact`] closes the holes and returns
/// the old-to-new id mapping.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PetriNet {
    /// The net name.
    pub name: String,
    places: Vec<Option<Place>>,
    transitions: Vec<Option<Transition>>,
    arcs: Vec<Option<Arc>>,
    live_places: usize,
    live_transitions: usize,
    live_arcs: usize,
}

/// The id mapping produced by [`PetriNet::compact`]. Removed elements map to
/// `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compaction {
    places: Vec<Option<PlaceId>>,
    transitions: Vec<Option<TransitionId>>,
}

impl Compaction {
    /// The new id of a place, or `None` if it was removed.
    pub fn place(&self, old: PlaceId) -> Option<PlaceId> {
        self.places.get(old.index()).copied().flatten()
    }

    /// The new id of a transition, or `None` if it was removed.
    pub fn transition(&self, old: TransitionId) -> Option<TransitionId> {
        self.transitions.get(old.index()).copied().flatten()
    }

    /// Rewrites a marking to the new place ids, dropping removed places.
    pub fn marking(&self, m: &Marking) -> Marking {
        m.iter()
            .filter_map(|(p, n)| self.place(p).map(|q| (q, n)))
            .collect()
    }
}

impl PetriNet {
    /// Creates an empty net with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }

    /// Adds a place and returns its id.
    pub fn add_place(&mut self, name: impl Into<String>) -> PlaceId {
        let id = PlaceId(to_u32(self.places.len()));
        self.places.push(Some(Place {
            name: name.into(),
            in_arcs: Vec::new(),
            out_arcs: Vec::new(),
        }));
        self.live_places += 1;
        id
    }

    /// Adds a transition and returns its id. Pass `None` as the label for a
    /// silent transition.
    pub fn add_transition(
        &mut self,
        name: impl Into<String>,
        label: Option<impl Into<Label>>,
    ) -> TransitionId {
        let id = TransitionId(to_u32(self.transitions.len()));
        self.transitions.push(Some(Transition {
            name: name.into(),
            label: label.map(Into::into),
            in_arcs: Vec::new(),
            out_arcs: Vec::new(),
        }));
        self.live_transitions += 1;
        id
    }

    /// Adds a normal arc of weight 1 from a place to a transition.
    pub fn add_input_arc(
        &mut self,
        place: PlaceId,
        transition: TransitionId,
    ) -> Result<ArcId, PetriNetError> {
        self.add_arc(
            ArcEnds::PlaceToTransition(place, transition),
            1,
            ArcKind::Normal,
        )
    }

    /// Adds a normal arc of weight 1 from a transition to a place.
    pub fn add_output_arc(
        &mut self,
        transition: TransitionId,
        place: PlaceId,
    ) -> Result<ArcId, PetriNetError> {
        self.add_arc(
            ArcEnds::TransitionToPlace(transition, place),
            1,
            ArcKind::Normal,
        )
    }

    /// Adds an arc with the given ends, weight and kind.
    ///
    /// An arc from a transition to a place is always stored as
    /// [`ArcKind::Normal`], whatever `kind` says.
    pub fn add_arc(
        &mut self,
        ends: ArcEnds,
        weight: u32,
        kind: ArcKind,
    ) -> Result<ArcId, PetriNetError> {
        if weight == 0 {
            return Err(PetriNetError::ZeroWeight);
        }
        let (place, transition) = match ends {
            ArcEnds::PlaceToTransition(p, t) | ArcEnds::TransitionToPlace(t, p) => (p, t),
        };
        if !self.contains_place(place) {
            return Err(PetriNetError::UnknownPlace(place));
        }
        if !self.contains_transition(transition) {
            return Err(PetriNetError::UnknownTransition(transition));
        }
        let id = ArcId(to_u32(self.arcs.len()));
        let kind = match ends {
            ArcEnds::PlaceToTransition(..) => kind,
            ArcEnds::TransitionToPlace(..) => ArcKind::Normal,
        };
        match ends {
            ArcEnds::PlaceToTransition(p, t) => {
                self.place_entry(p).out_arcs.push(id);
                self.transition_mut(t).in_arcs.push(id);
            }
            ArcEnds::TransitionToPlace(t, p) => {
                self.transition_mut(t).out_arcs.push(id);
                self.place_entry(p).in_arcs.push(id);
            }
        }
        self.arcs.push(Some(Arc { ends, weight, kind }));
        self.live_arcs += 1;
        Ok(id)
    }

    /// Removes an arc. Does nothing if it was already removed.
    pub fn remove_arc(&mut self, id: ArcId) {
        let Some(arc) = self.arcs.get_mut(id.index()).and_then(Option::take) else {
            return;
        };
        self.live_arcs -= 1;
        let (p, t) = (arc.place(), arc.transition());
        let (place_list, transition_list) = match arc.ends {
            ArcEnds::PlaceToTransition(..) => (
                &mut self.places[p.index()]
                    .as_mut()
                    .expect("arc ends exist")
                    .out_arcs,
                true,
            ),
            ArcEnds::TransitionToPlace(..) => (
                &mut self.places[p.index()]
                    .as_mut()
                    .expect("arc ends exist")
                    .in_arcs,
                false,
            ),
        };
        place_list.retain(|&a| a != id);
        let tr = self.transition_mut(t);
        if transition_list {
            tr.in_arcs.retain(|&a| a != id);
        } else {
            tr.out_arcs.retain(|&a| a != id);
        }
    }

    /// Removes a transition and its arcs (pm4py's
    /// `petri_utils.remove_transition`). Does nothing if it was already
    /// removed.
    pub fn remove_transition(&mut self, id: TransitionId) {
        if !self.contains_transition(id) {
            return;
        }
        let tr = self.transition(id);
        let arcs: Vec<ArcId> = tr.in_arcs.iter().chain(&tr.out_arcs).copied().collect();
        for a in arcs {
            self.remove_arc(a);
        }
        self.transitions[id.index()] = None;
        self.live_transitions -= 1;
    }

    /// Removes a place and its arcs (pm4py's `petri_utils.remove_place`).
    /// Does nothing if it was already removed.
    pub fn remove_place(&mut self, id: PlaceId) {
        if !self.contains_place(id) {
            return;
        }
        let pl = self.place(id);
        let arcs: Vec<ArcId> = pl.in_arcs.iter().chain(&pl.out_arcs).copied().collect();
        for a in arcs {
            self.remove_arc(a);
        }
        self.places[id.index()] = None;
        self.live_places -= 1;
    }

    /// Renumbers the remaining elements densely, in their current order, and
    /// returns the mapping from old to new ids.
    pub fn compact(&mut self) -> Compaction {
        fn remap<T, I: Copy>(items: &[Option<T>], make: impl Fn(u32) -> I) -> Vec<Option<I>> {
            let mut next = 0u32;
            items
                .iter()
                .map(|x| {
                    x.as_ref().map(|_| {
                        let id = make(next);
                        next += 1;
                        id
                    })
                })
                .collect()
        }
        let map = Compaction {
            places: remap(&self.places, PlaceId),
            transitions: remap(&self.transitions, TransitionId),
        };
        let name = std::mem::take(&mut self.name);
        let old = std::mem::replace(self, PetriNet::new(name));
        for p in old.places.into_iter().flatten() {
            self.add_place(p.name);
        }
        for t in old.transitions.into_iter().flatten() {
            self.add_transition(t.name, t.label);
        }
        for arc in old.arcs.into_iter().flatten() {
            let ends = match arc.ends {
                ArcEnds::PlaceToTransition(p, t) => ArcEnds::PlaceToTransition(
                    map.place(p).expect("live arc"),
                    map.transition(t).expect("live arc"),
                ),
                ArcEnds::TransitionToPlace(t, p) => ArcEnds::TransitionToPlace(
                    map.transition(t).expect("live arc"),
                    map.place(p).expect("live arc"),
                ),
            };
            self.add_arc(ends, arc.weight, arc.kind)
                .expect("compaction keeps arcs valid");
        }
        map
    }

    /// Returns `true` if the place exists and was not removed.
    pub fn contains_place(&self, id: PlaceId) -> bool {
        matches!(self.places.get(id.index()), Some(Some(_)))
    }

    /// Returns `true` if the transition exists and was not removed.
    pub fn contains_transition(&self, id: TransitionId) -> bool {
        matches!(self.transitions.get(id.index()), Some(Some(_)))
    }

    fn place_entry(&mut self, id: PlaceId) -> &mut Place {
        self.places[id.index()].as_mut().expect("place was removed")
    }

    /// Returns the place with the given id.
    ///
    /// # Panics
    ///
    /// Panics if the id does not belong to this net or the place was removed.
    pub fn place(&self, id: PlaceId) -> &Place {
        self.places[id.index()].as_ref().expect("place was removed")
    }

    /// Returns the transition with the given id.
    ///
    /// # Panics
    ///
    /// Panics if the id does not belong to this net or the transition was
    /// removed.
    pub fn transition(&self, id: TransitionId) -> &Transition {
        self.transitions[id.index()]
            .as_ref()
            .expect("transition was removed")
    }

    /// Returns a mutable reference to the transition with the given id, for
    /// example to relabel it.
    ///
    /// # Panics
    ///
    /// Panics if the id does not belong to this net or the transition was
    /// removed.
    pub fn transition_mut(&mut self, id: TransitionId) -> &mut Transition {
        self.transitions[id.index()]
            .as_mut()
            .expect("transition was removed")
    }

    /// Returns the arc with the given id.
    ///
    /// # Panics
    ///
    /// Panics if the id does not belong to this net or the arc was removed.
    pub fn arc(&self, id: ArcId) -> &Arc {
        self.arcs[id.index()].as_ref().expect("arc was removed")
    }

    /// Number of places.
    pub fn place_count(&self) -> usize {
        self.live_places
    }

    /// Number of transitions.
    pub fn transition_count(&self) -> usize {
        self.live_transitions
    }

    /// Number of arcs.
    pub fn arc_count(&self) -> usize {
        self.live_arcs
    }

    /// One more than the largest place index ever used. Vectors indexed by
    /// [`PlaceId::index`] need this length.
    pub fn place_index_bound(&self) -> usize {
        self.places.len()
    }

    /// One more than the largest transition index ever used. Vectors indexed
    /// by [`TransitionId::index`] need this length.
    pub fn transition_index_bound(&self) -> usize {
        self.transitions.len()
    }

    /// Iterates over the place ids in insertion order.
    pub fn place_ids(&self) -> impl Iterator<Item = PlaceId> + '_ {
        self.places().map(|(id, _)| id)
    }

    /// Iterates over the transition ids in insertion order.
    pub fn transition_ids(&self) -> impl Iterator<Item = TransitionId> + '_ {
        self.transitions().map(|(id, _)| id)
    }

    /// Iterates over the arc ids in insertion order.
    pub fn arc_ids(&self) -> impl Iterator<Item = ArcId> + '_ {
        self.arcs().map(|(id, _)| id)
    }

    /// Iterates over `(id, place)` pairs.
    pub fn places(&self) -> impl Iterator<Item = (PlaceId, &Place)> {
        self.places
            .iter()
            .enumerate()
            .filter_map(|(i, p)| p.as_ref().map(|p| (PlaceId(to_u32(i)), p)))
    }

    /// Iterates over `(id, transition)` pairs.
    pub fn transitions(&self) -> impl Iterator<Item = (TransitionId, &Transition)> {
        self.transitions
            .iter()
            .enumerate()
            .filter_map(|(i, t)| t.as_ref().map(|t| (TransitionId(to_u32(i)), t)))
    }

    /// Iterates over `(id, arc)` pairs.
    pub fn arcs(&self) -> impl Iterator<Item = (ArcId, &Arc)> {
        self.arcs
            .iter()
            .enumerate()
            .filter_map(|(i, a)| a.as_ref().map(|a| (ArcId(to_u32(i)), a)))
    }

    /// Finds the first place with the given name.
    pub fn place_by_name(&self, name: &str) -> Option<PlaceId> {
        self.places()
            .find(|(_, p)| p.name == name)
            .map(|(id, _)| id)
    }

    /// Finds the first transition with the given name.
    pub fn transition_by_name(&self, name: &str) -> Option<TransitionId> {
        self.transitions()
            .find(|(_, t)| t.name == name)
            .map(|(id, _)| id)
    }

    /// Places that feed the transition through any kind of arc (its preset).
    pub fn preset(&self, t: TransitionId) -> impl Iterator<Item = PlaceId> + '_ {
        self.transition(t)
            .in_arcs
            .iter()
            .map(|&a| self.arc(a).place())
    }

    /// Places the transition produces into (its postset).
    pub fn postset(&self, t: TransitionId) -> impl Iterator<Item = PlaceId> + '_ {
        self.transition(t)
            .out_arcs
            .iter()
            .map(|&a| self.arc(a).place())
    }

    /// Transitions that produce into the place.
    pub fn place_preset(&self, p: PlaceId) -> impl Iterator<Item = TransitionId> + '_ {
        self.place(p)
            .in_arcs
            .iter()
            .map(|&a| self.arc(a).transition())
    }

    /// Transitions that consume from (or test) the place.
    pub fn place_postset(&self, p: PlaceId) -> impl Iterator<Item = TransitionId> + '_ {
        self.place(p)
            .out_arcs
            .iter()
            .map(|&a| self.arc(a).transition())
    }

    /// Returns `true` if any arc is an inhibitor or reset arc.
    pub fn has_special_arcs(&self) -> bool {
        self.arcs().any(|(_, a)| a.kind != ArcKind::Normal)
    }

    /// The marking with one token in every place that has no input arc.
    ///
    /// Matches pm4py's `initial_marking.discover_initial_marking`.
    pub fn discover_initial_marking(&self) -> Marking {
        self.places()
            .filter(|(_, p)| p.in_arcs.is_empty())
            .map(|(id, _)| (id, 1))
            .collect()
    }

    /// The marking with one token in every place that has no output arc.
    ///
    /// Matches pm4py's `final_marking.discover_final_marking`.
    pub fn discover_final_marking(&self) -> Marking {
        self.places()
            .filter(|(_, p)| p.out_arcs.is_empty())
            .map(|(id, _)| (id, 1))
            .collect()
    }
}

/// A Petri net with an initial and a final marking.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AcceptingPetriNet {
    /// The net.
    pub net: PetriNet,
    /// The marking the process starts in.
    pub initial_marking: Marking,
    /// The marking a complete run ends in.
    pub final_marking: Marking,
}

impl AcceptingPetriNet {
    /// Bundles a net with its initial and final markings.
    pub fn new(net: PetriNet, initial_marking: Marking, final_marking: Marking) -> Self {
        Self {
            net,
            initial_marking,
            final_marking,
        }
    }
}

#[cfg(test)]
mod tests;
