//! The crate-wide error type.

use crate::bpmn::{BpmnError, NodeNotEnabled};
use crate::conversion::UnsupportedOperator;
use crate::dfg::DfgError;
use crate::petri::{NotEnabled, PetriNetError, ReachabilityError};
use crate::process_tree::{ParseError, TreeError};
use crate::transition_system::TsError;

/// Any error raised by this crate.
///
/// Each module returns its own error type. Use this one to handle them
/// together with `?`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Building or editing a Petri net failed.
    #[error(transparent)]
    PetriNet(#[from] PetriNetError),
    /// A transition was fired in a marking that does not enable it.
    #[error(transparent)]
    NotEnabled(#[from] NotEnabled),
    /// A state-space exploration hit its limit.
    #[error(transparent)]
    Reachability(#[from] ReachabilityError),
    /// A process tree is malformed.
    #[error(transparent)]
    Tree(#[from] TreeError),
    /// A process tree string could not be parsed.
    #[error(transparent)]
    Parse(#[from] ParseError),
    /// A DFG operation named an activity the graph does not have.
    #[error(transparent)]
    Dfg(#[from] DfgError),
    /// A transition system operation failed.
    #[error(transparent)]
    TransitionSystem(#[from] TsError),
    /// Building or editing a BPMN diagram failed.
    #[error(transparent)]
    Bpmn(#[from] BpmnError),
    /// A BPMN node was fired in a marking that does not enable it.
    #[error(transparent)]
    BpmnNotEnabled(#[from] NodeNotEnabled),
    /// A conversion met an operator it does not support.
    #[error(transparent)]
    UnsupportedOperator(#[from] UnsupportedOperator),
}
