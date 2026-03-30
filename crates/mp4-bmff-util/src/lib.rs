//! Utility extensions for the `mp4-bmff` crate.

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "io")]
pub mod io;

#[cfg(any(feature = "mux", feature = "demux"))]
pub mod multiplex;
