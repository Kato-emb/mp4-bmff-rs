//! # mp4-bmff
//!
//! A pure Rust implementation of ISO/IEC 14496-12
//! (ISO Base Media File Format, ISOBMFF).
//!
//! ## Feature Flags
//!
//! - `std` (default): Enables standard library support. Implies `alloc`.
//! - `alloc`: Enables heap allocation. Required for typed box representations.

#![cfg_attr(all(not(feature = "std"), not(test)), no_std)]
#![warn(missing_docs)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod lib {
    #[cfg(all(feature = "alloc", not(feature = "std")))]
    pub use alloc::{
        string::String, //
        vec::Vec,
    };

    #[cfg(feature = "std")]
    pub use std::{
        string::String, //
        vec::Vec,
    };
}

// =============================================================================
// Layer 0 - Byte Slice Cursors and Primitive Types
// =============================================================================

pub mod cursor;
pub mod types;

// =============================================================================
// Layer 1 - ISO BMFF Common types and Box Framing (no_std, no_alloc)
// =============================================================================

pub mod error;
pub mod header;
pub mod iter;
pub mod view;

// =============================================================================
// Layer 2 - Typed Box Representations (requires alloc)
// =============================================================================

pub mod boxes;
