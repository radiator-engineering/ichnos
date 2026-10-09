//! SaCoFa trace-variant query and PRIPEL contextual enrichment.
//!
//! Options follow the pm4py entry point; random draws come from the caller.
//! A seed is useful for experiments. The tests compare mechanisms and output
//! invariants; they do not establish a privacy guarantee for the composed release.
mod context;
mod matching;
mod mechanisms;
mod sacofa;
use ichnos_core::EventLog;
use rand::Rng;
use std::collections::BTreeSet;

/// Parameters of the SaCoFa + PRIPEL pipeline.
#[derive(Debug, Clone)]
pub struct PrivacyOptions {
    /// Positive finite epsilon, supplied to both phases as in pm4py.
    pub epsilon: f64,
    /// Maximum prefix depth, including the completion marker (default 10).
    pub max_prefix_length: usize,
    /// Minimum count of an unfinished prefix (default 20).
    pub pruning_count: u64,
    /// Attributes to exclude from contextual enrichment.
    pub blocklist: BTreeSet<String>,
    /// Maximum number of expanded prefixes across all levels.
    pub max_prefixes: usize,
    /// Maximum number of generated traces.
    pub max_traces: usize,
    /// Maximum size of the assignment cost matrix.
    pub max_matching_cells: usize,
}
impl Default for PrivacyOptions {
    fn default() -> Self {
        Self {
            epsilon: 1.,
            max_prefix_length: 10,
            pruning_count: 20,
            blocklist: BTreeSet::new(),
            max_prefixes: 1_000_000,
            max_traces: 10_000,
            max_matching_cells: 2_000_000,
        }
    }
}
/// Invalid input, empty query or resource exhaustion.
#[derive(Debug, thiserror::Error)]
pub enum PrivacyError {
    /// Invalid options.
    #[error("invalid privacy options: {0}")]
    Options(&'static str),
    /// Input events must have activity labels and timestamps.
    #[error("invalid privacy input: {0}")]
    Input(String),
    /// The trace-variant query produced no nonempty traces.
    #[error("trace-variant query is empty; adjust prefix depth or pruning count")]
    EmptyQuery,
    /// No partial release is returned on resource exhaustion.
    #[error("privacy pipeline exceeded {0}")]
    Limit(&'static str),
}
pub(crate) fn validate(o: &PrivacyOptions) -> Result<(), PrivacyError> {
    if !o.epsilon.is_finite()
        || o.epsilon <= 0.
        || !o.epsilon.recip().is_finite()
        || o.max_prefix_length == 0
        || o.max_prefix_length > 100
        || o.max_traces == 0
    {
        return Err(PrivacyError::Options(
            "positive finite epsilon, depth 1..=100 and positive trace bound required",
        ));
    }
    Ok(())
}
/// Generates noisy variants, optimally matches them to source traces, samples
/// missing context, randomizes scalar attributes and shifts timestamps.
///
/// Original trace attributes are discarded. The input log is never mutated.
pub fn anonymize_differential_privacy<R: Rng + ?Sized>(
    log: &EventLog,
    o: &PrivacyOptions,
    rng: &mut R,
) -> Result<EventLog, PrivacyError> {
    validate(o)?;
    let variants = sacofa::query(log, o, rng)?;
    context::enrich(log, &variants, o, rng)
}
/// Runs only the SaCoFa control-flow phase, returning nonempty activity traces.
pub fn trace_variant_query<R: Rng + ?Sized>(
    log: &EventLog,
    o: &PrivacyOptions,
    rng: &mut R,
) -> Result<Vec<Vec<String>>, PrivacyError> {
    validate(o)?;
    sacofa::query(log, o, rng)
}
