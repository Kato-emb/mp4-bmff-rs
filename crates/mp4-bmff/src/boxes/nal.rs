//! ISO/IEC 14496-15 NAL Unit Structured Video.
//!
//! This module provides types for working with boxes defined in ISO/IEC 14496-15,
//! which specifies the storage of NAL unit structured video (AVC/H.264, HEVC/H.265)
//! in the ISO Base Media File Format.
//!
//! # AVC/H.264 Types (requires `avc` feature)
//!
//! ## Sample Entry Types
//! - [`avc::Avc1SampleEntry`]/[`avc::Avc1SampleEntryView`]: AVC Sample Entry (`avc1`).
//!   Standard entry where parameter sets are stored in `avcC`.
//! - [`avc::Avc3SampleEntry`]/[`avc::Avc3SampleEntryView`]: AVC Sample Entry (`avc3`).
//!   Entry where parameter sets may appear in-band within samples.
//! - [`avc::Avc2SampleEntry`]/[`avc::Avc2SampleEntryView`]: AVC Sample Entry (`avc2`).
//! - [`avc::Avc4SampleEntry`]/[`avc::Avc4SampleEntryView`]: AVC Sample Entry (`avc4`).
//!
//! ## Configuration Boxes
//! - [`avc::AvcCBox`]/[`avc::AvcCBoxView`]: AVC Configuration Box (`avcC`).
//!   Contains the AVC Decoder Configuration Record with profile, level,
//!   and Sequence/Picture Parameter Sets (SPS/PPS).
//! - [`avc::M4dsBox`]/[`avc::M4dsBoxView`]: MPEG-4 Extension Descriptors Box (`m4ds`).
//!   Optional box containing additional MPEG-4 descriptors.
//!
//! # Features
//!
//! - `avc`: Enables AVC/H.264 support.
//! - `hevc`: Enables HEVC/H.265 support (not yet implemented).

#[cfg(feature = "avc")]
pub mod avc;

define_box_types!(
    /// AVC Configuration Box
    AVCC = b"avcC",
    /// MPEG-4 Extension Descriptors Box
    M4DS = b"m4ds",
    /// AVC Sample Entry Box
    AVC1 = b"avc1",
    /// AVC3 Sample Entry Box
    AVC3 = b"avc3",
    /// AVC2 Sample Entry Box
    AVC2 = b"avc2",
    /// AVC4 Sample Entry Box
    AVC4 = b"avc4",
);
