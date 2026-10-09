//! Feature extraction and machine-learning utilities for event logs.
//!
//! Ports pm4py's `pm4py.ml` module:
//!
//! - [`split_train_test`] and [`get_prefixes_from_log`] cut a log into
//!   training data;
//! - [`extract_features_dataframe`] and [`trace_features`] give one row of
//!   features per case;
//! - [`extract_outcome_enriched_dataframe`] adds case timing columns and
//!   case features to each event;
//! - [`extract_temporal_features_dataframe`] gives one row per time bin;
//! - [`extract_target_vector`] gives the targets of next-activity,
//!   next-time and remaining-time prediction;
//! - [`extract_ocel_features`] gives one row of features per object of an
//!   object-centric event log;
//! - [`profiles`] gives trace profiles and deterministic Lloyd clustering.

mod error;
mod features;
mod ocel_features;
mod outcome;
pub mod profiles;
mod split;
mod target;
mod temporal;
mod trace_features;
mod util;

pub use error::{Error, Result};
pub use features::{FeatureOptions, FeatureTable, NumericAggregation, extract_features_dataframe};
pub use ocel_features::{ObjectFeatures, OcelFeatureOptions, extract_ocel_features};
pub use outcome::{CaseTimes, OutcomeEnriched, OutcomeOptions, extract_outcome_enriched_dataframe};
pub use split::{get_prefixes_from_log, split_train_test};
pub use target::{TargetOptions, TargetVariant, TargetVector, extract_target_vector};
pub use temporal::{
    GrouperFreq, TemporalOptions, TemporalRow, extract_temporal_features_dataframe,
};
pub use trace_features::{TraceExtras, TraceFeatureOptions, trace_features};
