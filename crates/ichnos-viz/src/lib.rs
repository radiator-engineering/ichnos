//! Graphviz dot and SVG export of process models, ported from pm4py's
//! `visualization` package and its `save_vis_*` functions.
//!
//! Each `*_dot` function returns the DOT text that pm4py's matching
//! visualizer builds: the same nodes and edges with the same labels,
//! shapes, colours and pen widths. Node names differ, since pm4py uses
//! Python object ids. Each `write_*` function saves that text, as `.dot` or
//! `.gv`, or renders it with the Graphviz `dot` program to any format
//! Graphviz knows, such as `.svg` or `.png` ([`write_dot`]).
//!
//! pm4py's `view_*` functions render the same graph and open it in a
//! viewer. This crate does not open viewers; call the `*_dot` or
//! `write_*` function instead.
//!
//! | Model | DOT | Save | pm4py |
//! | --- | --- | --- | --- |
//! | Petri net | [`petri_net_dot`] | [`write_petri_net`] | `save_vis_petri_net` |
//! | Frequency DFG | [`dfg_dot`] | [`write_dfg`] | `save_vis_dfg` |
//! | Performance DFG | [`performance_dfg_dot`] | [`write_performance_dfg`] | `save_vis_performance_dfg` |
//! | Process tree | [`process_tree_dot`] | [`write_process_tree`] | `save_vis_process_tree` |
//! | BPMN | [`bpmn_dot`] | [`write_bpmn`] | `save_vis_bpmn` |
//! | Heuristics net | [`heuristics_net_dot`] | [`write_heuristics_net`] | `save_vis_heuristics_net` |

mod bpmn;
mod dfg;
mod dot;
mod heuristics_net;
mod petri_net;
mod process_tree;
mod style;
mod write;

pub use bpmn::{BpmnDotOptions, bpmn_dot};
pub use dfg::{DfgDotOptions, PerformanceDfgDotOptions, dfg_dot, performance_dfg_dot};
pub use heuristics_net::{HeuristicsNetDotOptions, heuristics_net_dot};
pub use petri_net::{Decoration, PetriNetDecorations, PetriNetDotOptions, petri_net_dot};
pub use process_tree::{ProcessTreeDotOptions, process_tree_dot};
pub use write::{
    VizError, write_bpmn, write_dfg, write_dot, write_heuristics_net, write_performance_dfg,
    write_petri_net, write_process_tree,
};
