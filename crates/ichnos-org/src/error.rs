//! The crate-wide error type.

use ichnos_core::{EventLog, Position};

/// Errors from organizational mining.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A core log error, such as an event without a needed attribute.
    #[error(transparent)]
    Core(#[from] ichnos_core::Error),
    /// An option is outside its valid range.
    #[error("invalid option: {0}")]
    InvalidOption(&'static str),
}

/// A result whose error is [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The Python `str` of an event attribute, or an error naming the event.
pub(crate) fn event_str(log: &EventLog, trace: usize, event: usize, key: &str) -> Result<String> {
    log.traces[trace].events[event]
        .get(key)
        .map(ToString::to_string)
        .ok_or_else(|| {
            ichnos_core::Error::MissingAttribute {
                key: key.to_owned(),
                position: Position::Event { trace, event },
            }
            .into()
        })
}
