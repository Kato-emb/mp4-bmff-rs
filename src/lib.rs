//! # mp4-bmff
//! A pure Rust implementation of ISO/IEC 14496-12
//! (ISO Base Media File Format, ISOBMFF).

#![cfg_attr(all(not(feature = "std"), not(test)), no_std)]
#![warn(missing_docs)]

#[cfg(not(feature = "alloc"))]
extern crate alloc;
