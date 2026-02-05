//! MPEG-4 (ISO/IEC 14496) specific structures and types.
//!
//! This module provides structures defined in the MPEG-4 specification suite,
//! which are embedded within BMFF boxes but follow their own encoding rules.
//!
//! # Submodules
//!
//! - `systems`: MPEG-4 Systems (ISO/IEC 14496-1) descriptor structures
//!   - Elementary Stream Descriptors
//!   - Decoder Configuration Descriptors
//!   - Requires the `systems` feature
//!
//! - `codecs`: Codec-specific configuration records
//!   - AVC (H.264) decoder configuration (ISO/IEC 14496-15)
//!   - Requires `mp4`, `avc`, or `hevc` features

#[cfg(feature = "systems")]
pub mod systems;

#[cfg(any(feature = "mp4", feature = "avc", feature = "hevc"))]
pub mod codecs;
