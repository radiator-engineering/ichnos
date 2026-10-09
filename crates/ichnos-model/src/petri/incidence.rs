//! Incidence matrix, ported from pm4py's `petri_net/utils/incidence_matrix.py`.

use super::{ArcEnds, ArcKind, Marking, PetriNet, PlaceId, TransitionId};

/// The incidence matrix of a Petri net: one row per place, one column per
/// transition, each cell the net token change when the transition fires.
///
/// Rows and columns follow place and transition id order, skipping removed
/// elements. pm4py sorts them by name instead and counts every arc as weight
/// 1; this matrix uses the arc weights. Inhibitor and reset arcs are left
/// out, since their effect is not a constant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncidenceMatrix {
    rows: Vec<Vec<i64>>,
    places: Vec<PlaceId>,
    transitions: Vec<TransitionId>,
    place_row: Vec<Option<usize>>,
}

impl IncidenceMatrix {
    /// Builds the matrix for `net`.
    pub fn new(net: &PetriNet) -> Self {
        let places: Vec<PlaceId> = net.place_ids().collect();
        let transitions: Vec<TransitionId> = net.transition_ids().collect();
        let mut place_row = vec![None; net.place_index_bound()];
        for (i, p) in places.iter().enumerate() {
            place_row[p.index()] = Some(i);
        }
        let mut column = vec![0usize; net.transition_index_bound()];
        for (i, t) in transitions.iter().enumerate() {
            column[t.index()] = i;
        }
        let mut rows = vec![vec![0i64; transitions.len()]; places.len()];
        for (_, arc) in net.arcs() {
            if arc.kind != ArcKind::Normal {
                continue;
            }
            let w = i64::from(arc.weight);
            let (p, t, delta) = match arc.ends {
                ArcEnds::PlaceToTransition(p, t) => (p, t, -w),
                ArcEnds::TransitionToPlace(t, p) => (p, t, w),
            };
            let r = place_row[p.index()].expect("arcs only touch live places");
            rows[r][column[t.index()]] += delta;
        }
        Self {
            rows,
            places,
            transitions,
            place_row,
        }
    }

    /// The matrix rows, indexed `[row][column]`.
    pub fn rows(&self) -> &[Vec<i64>] {
        &self.rows
    }

    /// The place of each row.
    pub fn places(&self) -> &[PlaceId] {
        &self.places
    }

    /// The transition of each column.
    pub fn transitions(&self) -> &[TransitionId] {
        &self.transitions
    }

    /// Encodes a marking as a vector with one entry per row.
    pub fn encode_marking(&self, m: &Marking) -> Vec<i64> {
        let mut v = vec![0i64; self.rows.len()];
        for (p, n) in m.iter() {
            if let Some(Some(r)) = self.place_row.get(p.index()) {
                v[*r] = i64::from(n);
            }
        }
        v
    }
}
