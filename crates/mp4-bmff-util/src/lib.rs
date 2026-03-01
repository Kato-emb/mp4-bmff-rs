//! Utility extensions for the `mp4-bmff` crate.

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "io")]
pub mod io;

#[cfg(feature = "time")]
pub mod time;

#[cfg(feature = "sample")]
pub mod sample;

#[cfg(feature = "remux")]
pub mod remux;
