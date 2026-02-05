//! Media format specific structures and types.
//!
//! This module contains format-specific data structures that are used within
//! BMFF container boxes but are not part of the BMFF specification itself.
//! These structures are defined in other ISO/IEC specifications and are
//! embedded in various BMFF boxes.
//!
//! # Submodules
//!
//! - [`mpeg4`]: MPEG-4 (ISO/IEC 14496) specific structures including:
//!   - Elementary Stream Descriptors (ES_Descriptor)
//!   - Decoder Configuration Descriptors
//!   - AVC (H.264) decoder configuration records

pub mod mpeg4;
