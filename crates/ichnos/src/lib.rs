//! Process mining in Rust. Re-exports the public API of every ichnos crate.

/// Process discovery: the inductive miner family and the temporal profile.
/// See [`ichnos_discovery`].
pub use ichnos_discovery as discovery;

pub use ichnos_core::{
    ActivityId, ActivityIndex, ActivitySequences, AttributeValue, Attributes, Classifier, Event,
    EventKeys, EventLog, EventStream, Extension, Globals, LogEdge, LogGraph, LogNode, MetaValue,
    Position, SortOrder, Trace, Variant, Variants, XesExtension, format_batch,
};

/// Process models: Petri nets, process trees, DFGs, transition systems and
/// their conversions. See [`ichnos_model`].
pub use ichnos_model as model;

/// Conformance checking: alignments, fitness and precision. See
/// [`ichnos_conformance`].
pub use ichnos_conformance as conformance;
