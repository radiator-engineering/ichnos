//! Process tree to POWL, ported from pm4py's
//! `objects/conversion/process_tree/variants/to_powl.py`.

use crate::powl::{Powl, PowlError, StrictPartialOrder};
use crate::{Operator, ProcessTree};

impl ProcessTree {
    /// Converts the tree to a POWL model (pm4py's
    /// `process_tree.variants.to_powl.apply`).
    ///
    /// A choice and a loop map to the same POWL operator, a parallel node to
    /// a partial order with no pairs, and a sequence to a total order. pm4py
    /// orders only neighbouring children of a sequence; this order also has
    /// the pairs transitivity implies, so it is a strict partial order.
    ///
    /// # Errors
    ///
    /// - [`PowlError::UnsupportedOperator`] for an OR or interleaving node.
    ///   pm4py rejects OR too, and turns an interleaving node into a
    ///   partial order with no pairs, which allows its children to overlap.
    /// - [`PowlError::ChoiceArity`] for a choice with fewer than two
    ///   children and [`PowlError::LoopArity`] for a loop without exactly
    ///   two, which pm4py rejects too.
    pub fn to_powl(&self) -> Result<Powl, PowlError> {
        let (op, children) = match self {
            ProcessTree::Tau => return Ok(Powl::Silent),
            ProcessTree::Activity(l) => return Ok(Powl::Activity(l.clone())),
            ProcessTree::Node(op, children) => (*op, children),
        };
        let nodes = children
            .iter()
            .map(ProcessTree::to_powl)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(match op {
            Operator::Xor if nodes.len() < 2 => return Err(PowlError::ChoiceArity(nodes.len())),
            Operator::Xor => Powl::Xor(nodes),
            Operator::Loop => match <[Powl; 2]>::try_from(nodes) {
                Ok([d, r]) => Powl::looped(d, r),
                Err(nodes) => return Err(PowlError::LoopArity(nodes.len())),
            },
            Operator::Sequence => Powl::PartialOrder(StrictPartialOrder::sequence(nodes)),
            Operator::Parallel => Powl::PartialOrder(StrictPartialOrder::new(nodes)),
            Operator::Or | Operator::Interleaving => {
                return Err(PowlError::UnsupportedOperator(op));
            }
        })
    }
}
