//! Codec-specific configuration structures (AVC, HEVC, etc.).
//!
//! This module provides codec configuration record structures as defined in
//! ISO/IEC 14496-15 (Carriage of NAL unit structured video in the ISO base
//! media file format).
//!
//! # Submodules
//!
//! - [`iter`]: Parameter set iterators for SPS, PPS, etc.
//! - [`avc`]: AVC (H.264) decoder configuration records
//!   - [`avc::AVCDecoderConfigurationRecordView`]: Zero-copy AVC config
//!   - [`avc::AVCDecoderConfigurationRecord`]: Owned AVC config (requires `alloc`)

#[cfg(any(feature = "avc", feature = "hevc"))]
pub mod iter;

#[cfg(feature = "avc")]
pub mod avc;
