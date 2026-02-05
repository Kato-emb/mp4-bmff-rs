//! Codec-specific configuration structures (AVC, HEVC, etc.).

#[cfg(any(feature = "avc", feature = "hevc"))]
pub mod iter;

#[cfg(feature = "avc")]
pub mod avc;
