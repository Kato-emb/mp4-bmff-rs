//! Utility extensions for the `mp4-bmff` crate.

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "io")]
pub mod io;

pub mod mux;

pub mod sample;
pub mod track;
