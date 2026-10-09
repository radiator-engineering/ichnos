//! Conversions between model types, ported from pm4py's `objects/conversion/`.
//!
//! Each conversion is a method on the source type, for example
//! [`ProcessTree::to_petri_net`](crate::ProcessTree::to_petri_net).

mod bpmn_to_petri;
mod dfg_to_petri;
mod heuristics_to_petri;
mod petri_to_bpmn;
mod petri_to_graph;
mod petri_to_ts;
mod powl_to_petri;
mod powl_to_tree;
mod tree_to_bpmn;
mod tree_to_petri;
mod tree_to_powl;
mod wf_net_to_powl;
mod wf_net_to_tree;

pub use bpmn_to_petri::{BpmnPetriNet, BpmnToPetriOptions};
pub use dfg_to_petri::{ARTIFICIAL_END, ARTIFICIAL_START};
pub use petri_to_graph::{PetriGraph, PetriGraphArc, PetriGraphNode};
pub use petri_to_ts::EdgeNaming;
pub use tree_to_bpmn::UnsupportedOperator;
pub use wf_net_to_powl::WfNetToPowlError;
pub use wf_net_to_tree::WfNetToTreeError;

#[cfg(test)]
pub(crate) mod tests;
