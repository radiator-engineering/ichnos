//! Incremental conformance consumers over interleaved canonical events.

use crate::{Error, MissingEventPolicy, Result};
use ichnos_core::{Event, EventKeys};

/// Shared keys and missing-field policy for streaming conformance.
#[derive(Debug, Clone, Default)]
pub struct StreamingConformanceOptions {
    /// Canonical activity and case ID keys.
    pub keys: EventKeys,
    /// Ignore incomplete events by default, or return an indexed error.
    pub missing: MissingEventPolicy,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Input {
    pub options: StreamingConformanceOptions,
    pub seen: usize,
    pub skipped: usize,
}

impl Input {
    pub fn fields(&mut self, event: &Event, extra: &[&str]) -> Result<Option<(String, String)>> {
        let index = self.seen;
        self.seen += 1;
        for key in [
            &self.options.keys.case_id[..],
            &self.options.keys.activity[..],
        ]
        .into_iter()
        .chain(extra.iter().copied())
        {
            if event.get(key).is_none() {
                if self.options.missing == MissingEventPolicy::Reject {
                    return Err(Error::MissingField {
                        key: key.into(),
                        event: index,
                    });
                }
                self.skipped += 1;
                return Ok(None);
            }
        }
        Ok(Some((
            event.get(&self.options.keys.case_id).unwrap().to_string(),
            event.get(&self.options.keys.activity).unwrap().to_string(),
        )))
    }
}
