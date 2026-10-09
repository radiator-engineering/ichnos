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
//! | Object-centric DFG | [`ocdfg_dot`] | [`write_ocdfg`] | `save_vis_ocdfg` |
//! | Object-centric Petri net | [`ocpn_dot`] | [`write_ocpn`] | `save_vis_ocpn` |
//! | Object graph | [`object_graph_dot`] | [`write_object_graph`] | `save_vis_object_graph` |
//! | Network analysis | [`network_analysis_dot`], [`network_analysis_performance_dot`] | [`write_network_analysis`], [`write_network_analysis_performance`] | `save_vis_network_analysis` |
//! | Dotted chart | [`dotted_chart_dot`] | [`write_dotted_chart`] | `save_vis_dotted_chart` |
//! | Performance spectrum | [`performance_spectrum_dot`] | [`write_performance_spectrum`] | `save_vis_performance_spectrum` |
//!
//! The dotted chart and the performance spectrum fix each node's position,
//! so their `write_*` functions render with `neato -n1` ([`write_neato`]).
//! Object types get colours from the options or [`object_type_color`];
//! pm4py derives them from Python's string hash, which changes from one
//! process to the next.

mod alignments;
mod bpmn;
mod dfg;
mod dot;
mod dotted_chart;
mod footprints;
mod heuristics_net;
mod network_analysis;
mod object_graph;
mod ocdfg;
mod ocpn;
mod performance_spectrum;
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
pub use dotted_chart::{
    ChartValue, DottedChartAttributes, DottedChartDotOptions, DottedChartPoint, dotted_chart_dot,
    dotted_chart_points,
};
pub use footprints::{FootprintsDotOptions, footprints_comparison_dot, footprints_dot};
pub use heuristics_net::{HeuristicsNetDotOptions, heuristics_net_dot};
pub use network_analysis::{
    NetworkAnalysisDotOptions, NetworkAnalysisEdge, network_analysis_dot,
    network_analysis_performance_dot,
};
pub use object_graph::{ObjectGraphDotOptions, object_graph_dot};
pub use ocdfg::{
    Ocdfg, OcdfgActivityMetric, OcdfgAnnotation, OcdfgCounts, OcdfgDotOptions, OcdfgEdge,
    OcdfgEdgeMetric, object_type_color, ocdfg_dot,
};
pub use ocpn::{
    ObjectTypeNet, OcPetriNet, OcpnDiagnostics, OcpnDotOptions, PlaceDiagnostics, ocpn_dot,
};
pub use performance_spectrum::{
    PerformanceSpectrum, PerformanceSpectrumDotOptions, PerformanceSpectrumOptions,
    performance_spectrum, performance_spectrum_dot,
};
pub use petri_net::{Decoration, PetriNetDecorations, PetriNetDotOptions, petri_net_dot};
pub use powl::{POWL_ICONS, PowlDotOptions, powl_dot, write_powl, write_powl_icons};
pub use prefix_tree::{PrefixTreeDotOptions, prefix_tree_dot};
pub use process_tree::{ProcessTreeDotOptions, process_tree_dot};
pub use transition_system::{TransitionSystemDotOptions, transition_system_dot};
pub use write::{
    VizError, write_alignment_table, write_alignments, write_bpmn, write_dfg, write_dot,
    write_dotted_chart, write_footprints, write_footprints_comparison, write_heuristics_net,
    write_neato, write_network_analysis, write_network_analysis_performance, write_object_graph,
    write_ocdfg, write_ocpn, write_performance_dfg, write_performance_spectrum, write_petri_net,
    write_prefix_tree, write_process_tree, write_transition_system,
};
