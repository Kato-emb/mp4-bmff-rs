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
// Internal - Byte Slice Cursor Module
// =============================================================================
pub(crate) mod cursor;

// =============================================================================
// Layer 0 - Primitive Types
// =============================================================================
pub mod types;

// =============================================================================
// Layer 1 - ISO BMFF base structures (no_std)
// =============================================================================
pub mod base;
pub mod codec;
pub mod error;
pub mod iter;

// Re-export base structures
pub use base::header::{
    BoxHeader, //
    BoxSize,
    BoxType,
};
pub use base::rawbox::{
    RawBox, //
    RawBoxRef,
};

// Re-export error types
pub use error::{
    Error, //
    ErrorKind,
    Result,
};

// Re-export codec traits
pub use codec::{
    BoxCodec, //
    BoxDecode,
    BoxEncode,
};

// Re-export codec helpers
pub use codec::read_box;
pub use codec::write_box;
pub use iter::iter_boxes;

// =============================================================================
// Layer 2 - Typed Box Representations (requires alloc)
// =============================================================================
pub mod boxes;
pub mod descriptor;

// Re-export owned box types
#[cfg(feature = "alloc")]
pub use base::rawbox::RawBoxOwned;

// =============================================================================
// Layer 3 -  I/O (std)
// =============================================================================
#[cfg(feature = "std")]
pub mod io;
