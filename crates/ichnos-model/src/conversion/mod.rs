//! Conversions between model types, ported from pm4py's `objects/conversion/`.
//!
//! Each conversion is a method on the source type, for example
//! [`ProcessTree::to_petri_net`](crate::ProcessTree::to_petri_net).

mod bpmn_to_petri;
mod dfg_to_petri;
mod heuristics_to_petri;
mod petri_to_bpmn;
mod petri_to_ts;
mod tree_to_bpmn;
mod tree_to_petri;

pub use bpmn_to_petri::{BpmnPetriNet, BpmnToPetriOptions};
pub use dfg_to_petri::{ARTIFICIAL_END, ARTIFICIAL_START};
pub use petri_to_ts::EdgeNaming;
pub use tree_to_bpmn::UnsupportedOperator;

#[cfg(test)]
pub(crate) mod tests;
