//! Conversions between model types, ported from pm4py's `objects/conversion/`.
//!
//! Each conversion is a method on the source type, for example
//! [`ProcessTree::to_petri_net`](crate::ProcessTree::to_petri_net).

mod tree_to_petri;

#[cfg(test)]
mod tests;
