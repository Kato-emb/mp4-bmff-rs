//! Sample table iteration utilities.
//!
//! Resolves per-sample metadata from `stbl` child boxes.

use mp4_bmff::boxes::bmff::SampleFlags;

#[cfg(feature = "alloc")]
mod builder;
mod iter;

#[cfg(feature = "alloc")]
pub use builder::StblBuilder;

pub use iter::{ResolvedSample, SampleTableExt};

/// Metadata for a single sample
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sample {
    /// Byte size of the sample.
    pub size: u32,
    /// Duration in media timescale units.
    pub duration: u32,
    /// Composition time offset (CTS - DTS).
    pub composition_time_offset: i32,
    /// Whether this sample is a sync sample (random access point).
    pub is_sync: bool,
    /// Per-sample dependency flags from `sdtp`. `None` if `sdtp` was absent.
    pub dependency: Option<SampleFlags>,
}
