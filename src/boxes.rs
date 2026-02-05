//! BMFF box type definitions and implementations.
//!
//! This module provides types for parsing and writing boxes in ISO Base Media
//! File Format (BMFF) and related specifications. All types support `no_std`
//! and `no_alloc` environments through zero-copy parsing.
//!
//! # Module Organization
//!
//! - [`bmff`]: ISO/IEC 14496-12 boxes (core BMFF specification).
//! - [`mp4`]: ISO/IEC 14496-14 boxes (MPEG-4 storage, requires `mp4` feature).
//! - [`nal`]: ISO/IEC 14496-15 boxes (NAL video like AVC/HEVC, requires `avc`/`hevc` feature).
//! - [`avc`]: Convenience re-exports of AVC types from `nal::avc`.
//! - [`sample_entry`]: Base types for sample entries used by codec-specific modules.
//!
//! # View vs Owned Types
//!
//! Most box types come in two variants:
//!
//! - **View types** (`*BoxView<'a>`): Zero-copy references into borrowed data.
//!   Available in all environments including `no_std` + `no_alloc`.
//! - **Owned types** (`*Box`): Heap-allocated types that own their data.
//!   Require the `alloc` feature.
//!
//! Fixed-size boxes (like `MvhdBox`, `TkhdBox`) are `Copy` types and don't
//! need separate View/Owned variants.

#[macro_use]
mod macros;

// ISO/IEC 14496-12
pub mod bmff;
// ISO/IEC 14496-14
#[cfg(feature = "mp4")]
pub mod mp4;
// ISO/IEC 14496-15
#[cfg(any(feature = "avc", feature = "hevc"))]
pub mod nal;

/// Re-exports of AVC (H.264) types from [`nal::avc`].
///
/// This module provides convenient access to AVC sample entry and
/// configuration box types without navigating through `nal::avc`.
#[cfg(feature = "avc")]
pub mod avc {
    use super::*;

    pub use nal::avc::AvcCBoxView;
    pub use nal::avc::M4dsBoxView;

    pub use nal::avc::Avc1SampleEntryView;
    pub use nal::avc::Avc3SampleEntryView;

    pub use nal::avc::Avc2SampleEntryView;
    pub use nal::avc::Avc4SampleEntryView;

    #[cfg(feature = "alloc")]
    mod owned_exports {
        use super::*;

        pub use nal::avc::AvcCBox;
        pub use nal::avc::M4dsBox;

        pub use nal::avc::Avc1SampleEntry;
        pub use nal::avc::Avc3SampleEntry;

        pub use nal::avc::Avc2SampleEntry;
        pub use nal::avc::Avc4SampleEntry;
    }

    #[cfg(feature = "alloc")]
    pub use owned_exports::*;
}

pub mod sample_entry;
