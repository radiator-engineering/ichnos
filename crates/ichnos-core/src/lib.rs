//! Event log data model: logs, traces, events, typed attributes and an Arrow columnar view.
//!
//! [`EventLog`] is the one in-memory type that ichnos algorithms take, as
//! `&EventLog`. It holds [`Trace`]s of [`Event`]s, each with typed
//! [`Attributes`], plus the XES log metadata (extensions, globals,
//! classifiers) so that XES files round-trip. [`EventKeys`] names the
//! attributes that algorithms read. [`EventLog::to_arrow`] and
//! [`EventLog::from_arrow`] convert to and from an Arrow `RecordBatch` for
//! I/O and interchange. [`EventLog::activity_sequences`] and
//! [`EventLog::variants`] give the interned activity view most miners use.
//!
//! ```
//! use ichnos_core::{EventKeys, EventLog};
//!
//! let keys = EventKeys::default();
//! let log = EventLog::from_trace_strings(["A,B,C", "A,C", "A,B,C"], ",", &keys);
//! let variants = log.variants(&keys).unwrap();
//! assert_eq!(variants.len(), 2);
//! assert_eq!(variants.variants[0].count(), 2);
//!
//! let batch = log.to_arrow(&keys).unwrap();
//! let back = EventLog::from_arrow(&batch, &keys).unwrap();
//! assert_eq!(back.traces, log.traces);
//! ```

pub mod activity;
pub mod artificial;
pub mod attribute;
pub mod columnar;
pub mod error;
pub mod format;
pub mod graph;
pub mod hof;
pub mod keys;
pub mod lifecycle;
pub mod log;
pub mod sample;
pub mod sort;

pub use arrow;
pub use chrono;
pub use petgraph;

pub use activity::{ActivityId, ActivityIndex, ActivitySequences, Variant, Variants};
pub use attribute::{AttributeValue, Attributes, MetaValue};
pub use error::{Error, Position, Result};
pub use format::format_batch;
pub use graph::{LogEdge, LogGraph, LogNode};
pub use keys::EventKeys;
pub use log::{Classifier, Event, EventLog, EventStream, Extension, Globals, Trace, XesExtension};
pub use sort::SortOrder;
