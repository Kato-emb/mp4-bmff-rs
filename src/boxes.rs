//! BMFF box core infrastructure.
//!
//! This module provides the core types for parsing and writing BMFF boxes.
//! All types in this module are designed to work in `no_std` and `no_alloc`
//! environments, using zero-copy parsing with borrowed data.

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

/// re-exports of `AVC` structures in `nal` (ISO/IEC 14496-15)
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
