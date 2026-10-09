//! Readers and writers for XES event logs.

mod error;
pub mod xes;

pub use error::{Error, Result};
pub use xes::{
    XesReadOptions, XesWriteOptions, read_xes, read_xes_from_reader, write_xes, write_xes_to_writer,
};
