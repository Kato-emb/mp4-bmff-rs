//! Prelude module for convenient imports.
//!
//! This module re-exports commonly used types and traits for convenience.
//!
//! # Usage
//!
//! ```
//! use mp4_bmff::prelude::*;
//! ```

// Core traits
pub use crate::codec::BoxCodec;
pub use crate::codec::BoxDecode;

#[cfg(feature = "alloc")]
pub use crate::codec::BoxEncode;

// Core types
pub use crate::base::header::BoxHeader;
pub use crate::base::header::BoxSize;
pub use crate::base::header::BoxType;
pub use crate::base::rawbox::RawBox;
pub use crate::base::rawbox::RawBoxRef;

#[cfg(feature = "alloc")]
pub use crate::base::rawbox::RawBoxOwned;

// Error types
pub use crate::error::Error;
pub use crate::error::ErrorKind;

// Common functions
pub use crate::codec::read_box;
pub use crate::iter::iter_boxes;

#[cfg(feature = "alloc")]
pub use crate::codec::write_box;
