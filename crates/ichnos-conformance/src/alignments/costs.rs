//! The cost model of alignments: pm4py's standard costs and per-transition
//! model costs.

use ichnos_model::{PetriNet, TransitionId};

/// Cost of a log move in pm4py's standard cost function
/// (`STD_MODEL_LOG_MOVE_COST`). Fitness divides costs by this value.
pub const STD_LOG_MOVE_COST: u64 = 10_000;

/// Cost of a model move on a visible transition in the standard cost
/// function (`STD_MODEL_LOG_MOVE_COST`).
pub const STD_MODEL_MOVE_COST: u64 = 10_000;

/// Cost of a model move on a silent transition in the standard cost function
/// (`STD_TAU_COST`).
pub const STD_SILENT_MOVE_COST: u64 = 1;

/// Cost of a synchronous move in the standard cost function
/// (`STD_SYNC_COST`).
pub const STD_SYNC_MOVE_COST: u64 = 0;

/// The cost of moving on each transition of a net: alone (a model move) and
/// together with an event of the same label (a synchronous move).
///
/// pm4py takes these as the `model_cost_function` and `sync_cost_function`
/// dictionaries. Both are indexed by [`TransitionId`] here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelCosts {
    model_move: Vec<u64>,
    sync_move: Vec<u64>,
}

impl ModelCosts {
    /// pm4py's standard costs: a model move costs [`STD_MODEL_MOVE_COST`] on
    /// a visible transition and [`STD_SILENT_MOVE_COST`] on a silent one; a
    /// synchronous move costs [`STD_SYNC_MOVE_COST`].
    pub fn standard(net: &PetriNet) -> Self {
        let mut model_move = vec![0; net.transition_index_bound()];
        for (t, tr) in net.transitions() {
            model_move[t.index()] = if tr.is_silent() {
                STD_SILENT_MOVE_COST
            } else {
                STD_MODEL_MOVE_COST
            };
        }
        Self {
            model_move,
            sync_move: vec![STD_SYNC_MOVE_COST; net.transition_index_bound()],
        }
    }

    /// The cost of a model move on `t`.
    ///
    /// # Panics
    ///
    /// Panics if `t` is not a transition of the net these costs were made for.
    pub fn model_move(&self, t: TransitionId) -> u64 {
        self.model_move[t.index()]
    }

    /// The cost of a synchronous move on `t`.
    ///
    /// # Panics
    ///
    /// Panics if `t` is not a transition of the net these costs were made for.
    pub fn sync_move(&self, t: TransitionId) -> u64 {
        self.sync_move[t.index()]
    }

    /// Sets the cost of a model move on `t`.
    ///
    /// # Panics
    ///
    /// Panics if `t` is not a transition of the net these costs were made for.
    pub fn set_model_move(&mut self, t: TransitionId, cost: u64) -> &mut Self {
        self.model_move[t.index()] = cost;
        self
    }

    /// Sets the cost of a synchronous move on `t`.
    ///
    /// # Panics
    ///
    /// Panics if `t` is not a transition of the net these costs were made for.
    pub fn set_sync_move(&mut self, t: TransitionId, cost: u64) -> &mut Self {
        self.sync_move[t.index()] = cost;
        self
    }

    pub(crate) fn covers(&self, net: &PetriNet) -> bool {
        self.model_move.len() >= net.transition_index_bound()
            && self.sync_move.len() >= net.transition_index_bound()
    }
}
