//! Test support: loads pm4py golden outputs and compares them with tolerances. Not published.
//!
//! pm4py is the oracle for ichnos. `tools/golden/generate.py` runs pm4py on the
//! fixture logs in `fixtures/logs/` and writes one JSON file per case to
//! `fixtures/golden/<area>/<case>.json`. This crate finds those files, loads
//! them, and compares ichnos output against them.
//!
//! ```no_run
//! use ichnos_golden::{fixture_path, golden, log::LogSummary};
//!
//! let g = golden("log", "running-example-xes");
//! let expected: LogSummary = g.expected_as();
//! let input = fixture_path("running-example.xes");
//! # let _ = (expected, input);
//! ```
//!
//! # Tolerances
//!
//! [`Tolerance`] accepts a value when `|actual - expected| <= abs` or
//! `|actual - expected| <= rel * max(|actual|, |expected|)`. NaN equals NaN and
//! an infinity equals the same infinity. The presets are:
//!
//! | Preset | `abs` | `rel` | Use for |
//! |---|---|---|---|
//! | [`Tolerance::EXACT`] | 0 | 0 | values that must match bit for bit |
//! | [`Tolerance::COUNT`] | 1e-9 | 0 | counts that pass through floats |
//! | [`Tolerance::METRIC`] | 1e-12 | 1e-6 | computed metrics (fitness, precision, durations) |
//!
//! [`JsonCompare::default`] uses [`Tolerance::METRIC`] for floats. Integers in
//! JSON always compare exactly.
//!
//! # Non-finite floats
//!
//! JSON has no NaN or infinity, so the generator writes them as the strings
//! `"NaN"`, `"Infinity"` and `"-Infinity"`. [`as_f64`] reads both forms, and
//! [`compare_json`] treats such a string as equal to the matching float.

mod compare;
mod error;
mod golden;
pub mod log;
mod paths;

pub use compare::{
    JsonCompare, Mismatch, Tolerance, as_f64, assert_close, assert_json_eq, assert_map_close,
    assert_map_eq, assert_multiset_eq, compare_close, compare_json, compare_map_close,
    compare_map_eq, compare_multiset_eq,
};
pub use error::GoldenError;
pub use golden::{Golden, Meta, areas, cases, golden, try_golden};
pub use paths::{fixture_path, fixtures_dir, golden_dir, golden_path, workspace_root};
