//! Metadata-preserving filters over event logs. Activities follow core validation rules.
mod attributes;
mod dfg;
mod sequence;
mod temporal;
pub use attributes::*;
pub use dfg::*;
pub use sequence::*;
pub use temporal::*;

use crate::Result;
use ichnos_core::{EventLog, Trace};

/// Whether matching cases or events are retained.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Retention {
    /// Retain matches (the default).
    #[default]
    Retain,
    /// Remove matches.
    Exclude,
}
impl Retention {
    fn accepts(self, matched: bool) -> bool {
        matched == (self == Self::Retain)
    }
}
fn select(
    log: &EventLog,
    mut predicate: impl FnMut(usize, &Trace) -> Result<bool>,
) -> Result<EventLog> {
    let mut result = log.filter_traces(|_| false);
    for (i, trace) in log.traces.iter().enumerate() {
        if predicate(i, trace)? {
            result.traces.push(trace.clone());
        }
    }
    Ok(result)
}
fn slice(trace: &Trace, range: std::ops::Range<usize>) -> Trace {
    Trace {
        attributes: trace.attributes.clone(),
        events: trace.events[range].to_vec(),
    }
}
