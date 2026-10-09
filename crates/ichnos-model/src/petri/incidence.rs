//! Incidence matrix, ported from pm4py's `petri_net/utils/incidence_matrix.py`.

use super::{ArcEnds, ArcKind, Marking, PetriNet};

/// The incidence matrix of a Petri net: one row per place, one column per
/// transition, each cell the net token change when the transition fires.
///
/// Rows and columns follow place and transition ids. pm4py sorts them by
/// name instead and counts every arc as weight 1; this matrix uses the arc
/// weights. Inhibitor and reset arcs are left out, since their effect is not
/// a constant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncidenceMatrix {
    rows: Vec<Vec<i64>>,
}

impl IncidenceMatrix {
    /// Builds the matrix for `net`.
    pub fn new(net: &PetriNet) -> Self {
        let mut rows = vec![vec![0i64; net.transition_count()]; net.place_count()];
        for (_, arc) in net.arcs() {
            if arc.kind != ArcKind::Normal {
                continue;
            }
            let w = i64::from(arc.weight);
            match arc.ends {
                ArcEnds::PlaceToTransition(p, t) => rows[p.index()][t.index()] -= w,
                ArcEnds::TransitionToPlace(t, p) => rows[p.index()][t.index()] += w,
            }
        }
        Self { rows }
    }

    /// The matrix rows, indexed `[place][transition]`.
    pub fn rows(&self) -> &[Vec<i64>] {
        &self.rows
    }

    /// Encodes a marking as a vector indexed by place id.
    pub fn encode_marking(&self, m: &Marking) -> Vec<i64> {
        let mut v = vec![0i64; self.rows.len()];
        for (p, n) in m.iter() {
            v[p.index()] = i64::from(n);
        }
        v
    }
}
