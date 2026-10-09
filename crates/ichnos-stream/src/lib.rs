//! Incremental process mining over canonical events and traces.
//!
//! Live streams deliver synchronously in FIFO order. File readers are lazy
//! iterators; online algorithms retain their state and expose typed snapshots.

//!
//! ```
//! use ichnos_core::Event;
//! use ichnos_stream::{LiveEventStream, StreamingDfgDiscovery};
//! use std::{cell::RefCell, rc::Rc};
//!
//! let algorithm = Rc::new(RefCell::new(StreamingDfgDiscovery::default()));
//! let mut stream = LiveEventStream::new();
//! stream.register(algorithm.clone());
//! stream.start()?;
//! let event: Event = [("case:concept:name", "c"), ("concept:name", "a")].into_iter().collect();
//! stream.append(event)?;
//! stream.stop()?;
//! assert_eq!(algorithm.borrow().get().activities["a"], 1);
//! # Ok::<(), ichnos_stream::Error>(())
//! ```

mod alignments;
mod conformance;
mod conversion;
mod dfg;
mod error;
mod footprints;
mod live;
mod ocel;
mod reader;
mod tbr;
mod temporal;

pub use conversion::TraceIterator;
pub use dfg::{MissingEventPolicy, StreamingDfgDiscovery, StreamingDfgOptions, StreamingDfgResult};
pub use error::{Error, Result};
pub use live::{
    Collector, LiveEventStream, LiveStream, LiveTraceStream, ObserverId, StreamSink, StreamState,
};
pub use reader::{
    CsvEventReader, CsvStreamOptions, XesEventReader, XesStreamOptions, XesTraceReader, feed,
};

pub use conformance::StreamingConformanceOptions;
pub use footprints::{StreamingFootprintsConformance, StreamingFootprintsStatus};
pub use tbr::{
    StreamingTbrConformance, StreamingTbrOptions, StreamingTbrStatus, StreamingTbrTermination,
};
pub use temporal::{
    StreamingTemporalConformance, StreamingTemporalOptions, TemporalDeviation, TemporalProfile,
};

pub use alignments::{
    StreamingAlignmentOptions, StreamingAlignmentResult, StreamingAlignmentStep,
    StreamingAlignments,
};
pub use ocel::{OcelDistributorOptions, OcelFlatteningDistributor};
