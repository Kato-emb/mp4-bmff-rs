//! ISO/IEC 14496-14 (MP4) Box Structures.
//!
//! This module provides types for working with boxes defined in ISO/IEC 14496-14,
//! which specifies the storage of MPEG-4 content in the ISO Base Media File Format.
//!
//! # Sample Entry Types
//!
//! - [`Mp4aSampleEntry`]/[`Mp4aSampleEntryView`]: MPEG-4 Audio Sample Entry (`mp4a`).
//!   Used for AAC and other MPEG-4 audio codecs.
//! - [`Mp4vSampleEntry`]/[`Mp4vSampleEntryView`]: MPEG-4 Visual Sample Entry (`mp4v`).
//!   Used for MPEG-4 Part 2 video.
//! - [`MpegSampleEntry`]/[`MpegSampleEntryView`]: MPEG-4 Systems Sample Entry (`mp4s`).
//!   Used for BIFS, OD, and other MPEG-4 Systems streams.
//!
//! # Configuration Boxes
//!
//! - [`EsdsBox`]/[`EsdsBoxView`]: Elementary Stream Descriptor Box (`esds`).
//!   Contains the ES Descriptor with codec configuration (decoder config,
//!   buffer requirements, bitrate info, and decoder-specific data like
//!   AudioSpecificConfig for AAC).
//!
//! # Features
//!
//! This module requires the `mp4` feature to be enabled.

mod esds;
mod mp4a;
mod mp4s;
mod mp4v;

pub use esds::EsdsBoxView;
pub use esds::EsdsFlags;

pub use mp4a::Mp4aSampleEntryView;
pub use mp4s::MpegSampleEntryView;
pub use mp4v::Mp4vSampleEntryView;

#[cfg(feature = "alloc")]
mod owned_exports {
    use super::*;

    pub use esds::EsdsBox;
    pub use mp4a::Mp4aSampleEntry;
    pub use mp4s::MpegSampleEntry;
    pub use mp4v::Mp4vSampleEntry;
}

#[cfg(feature = "alloc")]
pub use owned_exports::*;

define_box_types!(
    /// ES Descriptor Box
    ESDS = b"esds",
    /// MPEG-4 Visual Sample Entry Box
    MP4V = b"mp4v",
    /// MPEG-4 Audio Sample Entry Box
    MP4A = b"mp4a",
    /// MPEG-4 Sample Entry Box
    MP4S = b"mp4s",
);
