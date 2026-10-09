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
//! | Transition system | [`transition_system_dot`] | [`write_transition_system`] | `save_vis_transition_system` |
//! | Prefix tree | [`prefix_tree_dot`] | [`write_prefix_tree`] | `save_vis_prefix_tree` |
//! | Footprints | [`footprints_dot`] | [`write_footprints`] | `save_vis_footprints` (one footprint) |
//! | Footprint comparison | [`footprints_comparison_dot`] | [`write_footprints_comparison`] | `save_vis_footprints` (two footprints) |
//! | Alignments | [`alignments_dot`], [`alignment_table_dot`] | [`write_alignments`], [`write_alignment_table`] | `save_vis_alignments` |
//! | POWL | [`powl_dot`] | [`write_powl`] | `save_vis_powl` |

mod alignments;
mod bpmn;
mod dfg;
mod dot;
mod footprints;
mod heuristics_net;
mod petri_net;
mod powl;
mod prefix_tree;
mod process_tree;
mod style;
mod transition_system;
mod write;

pub use alignments::{
    AlignmentStep, AlignmentsDotOptions, VariantAlignment, alignment_table_dot, alignments_dot,
};
pub use bpmn::{BpmnDotOptions, bpmn_dot};
pub use dfg::{DfgDotOptions, PerformanceDfgDotOptions, dfg_dot, performance_dfg_dot};
pub use footprints::{FootprintsDotOptions, footprints_comparison_dot, footprints_dot};
pub use heuristics_net::{HeuristicsNetDotOptions, heuristics_net_dot};
pub use petri_net::{Decoration, PetriNetDecorations, PetriNetDotOptions, petri_net_dot};
pub use powl::{POWL_ICONS, PowlDotOptions, powl_dot, write_powl, write_powl_icons};
pub use prefix_tree::{PrefixTreeDotOptions, prefix_tree_dot};
pub use process_tree::{ProcessTreeDotOptions, process_tree_dot};
pub use transition_system::{TransitionSystemDotOptions, transition_system_dot};
pub use write::{
    VizError, write_alignment_table, write_alignments, write_bpmn, write_dfg, write_dot,
    write_footprints, write_footprints_comparison, write_heuristics_net, write_performance_dfg,
    write_petri_net, write_prefix_tree, write_process_tree, write_transition_system,
};
