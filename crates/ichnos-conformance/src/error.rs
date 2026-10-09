//! The crate-wide error type.

/// A result whose error is [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors produced by ichnos-conformance.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A conversion required for model comparison failed.
    #[error("model comparison failed: {0}")]
    ModelComparison(String),
    /// Reading the log failed, for example because an event has no activity.
    #[error(transparent)]
    Core(#[from] ichnos_core::Error),
    /// A time computation failed, for example on an invalid business
    /// schedule.
    #[error(transparent)]
    Stats(#[from] ichnos_stats::Error),
    /// Exploring the markings reachable through silent transitions hit its
    /// limit; the net may be unbounded.
    #[error(transparent)]
    Reachability(#[from] ichnos_model::petri::ReachabilityError),
    /// The net has inhibitor or reset arcs. Alignments use the classic
    /// firing rule, whose marking equation does not hold for those arcs.
    #[error("alignments need a net without inhibitor or reset arcs")]
    SpecialArcs,
    /// The final marking cannot be reached from the initial marking, so the
    /// net is not easy sound and no trace has an alignment. pm4py raises an
    /// exception in the same case.
    #[error("the final marking is not reachable from the initial marking")]
    FinalMarkingUnreachable,
    /// No path in the DFG leads from a start activity to an end activity,
    /// so no trace has an alignment.
    #[error("no path in the DFG leads from a start activity to an end activity")]
    DfgEndUnreachable,
    /// The model log of an edit-distance alignment has no traces.
    #[error("the model log has no traces")]
    EmptyModelLog,
    /// The process tree is malformed, for example a loop without exactly
    /// two children.
    #[error(transparent)]
    ProcessTree(#[from] ichnos_model::process_tree::TreeError),
    /// The process tree has an operator the method has no rules for.
    #[error("process tree operator {0} is not supported by this method")]
    UnsupportedOperator(ichnos_model::Operator),
    /// A per-event cost list does not have one entry per event.
    #[error("{actual} log move costs given for a trace of {expected} events")]
    LogMoveCostCount {
        /// The number of events in the trace.
        expected: usize,
        /// The number of costs given.
        actual: usize,
    },
    /// The base of a discounted alignment is not a finite positive number.
    #[error("the discount exponent must be finite and positive, got {0}")]
    DiscountExponent(f64),
    /// A marking puts tokens on a place that is not in the net.
    #[error("the marking puts tokens on {0:?}, which is not a place of the net")]
    UnknownPlace(ichnos_model::PlaceId),
    /// The settings of an approximate alignment method are out of range.
    #[error("invalid approximate alignment settings: {0}")]
    InvalidApproximation(&'static str),
    /// Subset alignment could not align any of the picked variants within
    /// its limits. pm4py raises an exception in the same case.
    #[error("no picked variant could be aligned against the model")]
    NoRepresentative,
    /// An object-centric event log relation names an event that is not in
    /// the events table.
    #[error("an OCEL relation names event {0:?}, which is not in the events table")]
    UnknownOcelEvent(String),
    /// An object-centric event log relation names an object that is not in
    /// the objects table. pm4py raises `KeyError`.
    #[error("an OCEL relation names object {0:?}, which is not in the objects table")]
    UnknownOcelObject(String),
    /// The normative graph and the weights give a normalisation constant of
    /// 0, so the fitness is undefined. pm4py raises `ZeroDivisionError`.
    #[error("the normative graph and the weights give a normalisation constant of 0")]
    ZeroNormalization,
    /// An ET-OT edge that both graphs share has frequency 0 in the normative
    /// graph. pm4py raises `ZeroDivisionError`.
    #[error("the normative ET-OT edge ({activity}, {object_type}) has frequency 0")]
    ZeroEdgeFrequency {
        /// The activity of the edge.
        activity: String,
        /// The object type of the edge.
        object_type: String,
    },
    /// The linear program of the state-equation heuristic failed in a way
    /// other than infeasibility.
    #[error("the state-equation linear program failed: {0}")]
    LinearProgram(String),
}
