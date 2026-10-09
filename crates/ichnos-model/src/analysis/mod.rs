//! Petri net analysis, ported from pm4py's `analysis` module: workflow-net
//! and soundness checks (Woflan), simplicity, maximal decomposition,
//! implicit-place removal, synchronous products and marking equations.
//!
//! Linear and integer programs are solved with the pure-Rust `microlp`
//! solver through `good_lp`. pm4py uses scipy, CVXOPT or PuLP; the results
//! the functions return do not depend on the solver.

mod decomposition;
mod implicit_places;
mod lp;
mod marking_equation;
mod simplicity;
mod sync_product;
mod woflan;
mod workflow;

pub use decomposition::DecompositionPart;
pub use simplicity::SimplicityVariant;
pub use sync_product::{SKIP, SyncMove, SynchronousProduct};
pub use woflan::SoundnessReport;

use crate::petri::{Marking, PetriNet, ReachabilityError};

/// Errors raised by the analysis functions.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum AnalysisError {
    /// The linear-program solver failed for a reason other than
    /// infeasibility.
    #[error("linear program solver failed: {0}")]
    LinearProgram(String),
    /// The state space has more markings than the limit.
    #[error("the state space has more than {0} markings")]
    TooManyMarkings(usize),
    /// The reachability graph could not be built.
    #[error(transparent)]
    Reachability(#[from] ReachabilityError),
    /// No place has this name.
    #[error("no place named {0:?}")]
    UnknownPlace(String),
}

impl PetriNet {
    /// A marking given by place names and token counts (pm4py's
    /// `generate_marking`). A name that comes twice takes the last count.
    ///
    /// # Errors
    ///
    /// [`AnalysisError::UnknownPlace`] when no place has one of the names.
    /// If several places share a name, the one with the highest id is
    /// used, as pm4py's name dictionary keeps the last.
    pub fn marking_from_names<'a>(
        &self,
        counts: impl IntoIterator<Item = (&'a str, u32)>,
    ) -> Result<Marking, AnalysisError> {
        let mut m = Marking::new();
        for (name, n) in counts {
            let p = self
                .places()
                .filter(|(_, p)| p.name == name)
                .map(|(id, _)| id)
                .last()
                .ok_or_else(|| AnalysisError::UnknownPlace(name.to_owned()))?;
            m.set(p, n);
        }
        Ok(m)
    }
}
