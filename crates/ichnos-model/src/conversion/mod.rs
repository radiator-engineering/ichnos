//! Conversions between model types, ported from pm4py's `objects/conversion/`.
//!
//! Each conversion is a method on the source type, for example
//! [`ProcessTree::to_petri_net`](crate::ProcessTree::to_petri_net).

mod dfg_to_petri;
mod petri_to_ts;
mod tree_to_petri;

pub use dfg_to_petri::{ARTIFICIAL_END, ARTIFICIAL_START};
pub use petri_to_ts::EdgeNaming;

#[cfg(test)]
mod tests;
