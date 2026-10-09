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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PetriNet {
    /// The net name.
    pub name: String,
    places: Vec<Place>,
    transitions: Vec<Transition>,
    arcs: Vec<Arc>,
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
        self.places.push(Place {
            name: name.into(),
            in_arcs: Vec::new(),
            out_arcs: Vec::new(),
        });
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
        self.transitions.push(Transition {
            name: name.into(),
            label: label.map(Into::into),
            in_arcs: Vec::new(),
            out_arcs: Vec::new(),
        });
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
        if place.index() >= self.places.len() {
            return Err(PetriNetError::UnknownPlace(place));
        }
        if transition.index() >= self.transitions.len() {
            return Err(PetriNetError::UnknownTransition(transition));
        }
        let id = ArcId(to_u32(self.arcs.len()));
        let kind = match ends {
            ArcEnds::PlaceToTransition(..) => kind,
            ArcEnds::TransitionToPlace(..) => ArcKind::Normal,
        };
        match ends {
            ArcEnds::PlaceToTransition(p, t) => {
                self.places[p.index()].out_arcs.push(id);
                self.transitions[t.index()].in_arcs.push(id);
            }
            ArcEnds::TransitionToPlace(t, p) => {
                self.transitions[t.index()].out_arcs.push(id);
                self.places[p.index()].in_arcs.push(id);
            }
        }
        self.arcs.push(Arc { ends, weight, kind });
        Ok(id)
    }

    /// Returns the place with the given id.
    ///
    /// # Panics
    ///
    /// Panics if the id does not belong to this net.
    pub fn place(&self, id: PlaceId) -> &Place {
        &self.places[id.index()]
    }

    /// Returns the transition with the given id.
    ///
    /// # Panics
    ///
    /// Panics if the id does not belong to this net.
    pub fn transition(&self, id: TransitionId) -> &Transition {
        &self.transitions[id.index()]
    }

    /// Returns a mutable reference to the transition with the given id, for
    /// example to relabel it.
    ///
    /// # Panics
    ///
    /// Panics if the id does not belong to this net.
    pub fn transition_mut(&mut self, id: TransitionId) -> &mut Transition {
        &mut self.transitions[id.index()]
    }

    /// Returns the arc with the given id.
    ///
    /// # Panics
    ///
    /// Panics if the id does not belong to this net.
    pub fn arc(&self, id: ArcId) -> &Arc {
        &self.arcs[id.index()]
    }

    /// Number of places.
    pub fn place_count(&self) -> usize {
        self.places.len()
    }

    /// Number of transitions.
    pub fn transition_count(&self) -> usize {
        self.transitions.len()
    }

    /// Number of arcs.
    pub fn arc_count(&self) -> usize {
        self.arcs.len()
    }

    /// Iterates over all place ids in insertion order.
    pub fn place_ids(&self) -> impl ExactSizeIterator<Item = PlaceId> + use<> {
        (0..to_u32(self.places.len())).map(PlaceId)
    }

    /// Iterates over all transition ids in insertion order.
    pub fn transition_ids(&self) -> impl ExactSizeIterator<Item = TransitionId> + use<> {
        (0..to_u32(self.transitions.len())).map(TransitionId)
    }

    /// Iterates over all arc ids in insertion order.
    pub fn arc_ids(&self) -> impl ExactSizeIterator<Item = ArcId> + use<> {
        (0..to_u32(self.arcs.len())).map(ArcId)
    }

    /// Iterates over `(id, place)` pairs.
    pub fn places(&self) -> impl ExactSizeIterator<Item = (PlaceId, &Place)> {
        self.place_ids().zip(&self.places)
    }

    /// Iterates over `(id, transition)` pairs.
    pub fn transitions(&self) -> impl ExactSizeIterator<Item = (TransitionId, &Transition)> {
        self.transition_ids().zip(&self.transitions)
    }

    /// Iterates over `(id, arc)` pairs.
    pub fn arcs(&self) -> impl ExactSizeIterator<Item = (ArcId, &Arc)> {
        self.arc_ids().zip(&self.arcs)
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
        self.arcs.iter().any(|a| a.kind != ArcKind::Normal)
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
