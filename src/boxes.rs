//! BMFF box core infrastructure.
//!
//! This module provides the core types for parsing and writing BMFF boxes.
//! All types in this module are designed to work in `no_std` and `no_alloc`
//! environments, using zero-copy parsing with borrowed data.
//!
//! # Module Structure
//!
//! - [`error`]: Error types for box operations
//! - [`boxsize`]: Type-safe box size representation
//! - [`boxtype`]: Type-safe box type representation (FourCC, UUID)
//! - [`header`]: Box header parsing and writing
//! - [`view`]: Zero-copy view into a box (header + payload slice)
//! - [`iter`]: Iterator over consecutive boxes
//!
//! # Example
//!
//! ```
//! use mp4_bmff::boxes::{BoxIter, BoxView};
//! use mp4_bmff::cursor::ReadCursor;
//!
//! // Parse boxes without heap allocation
//! let data = [
//!     0x00, 0x00, 0x00, 0x08, b'f', b'r', b'e', b'e',
//! ];
//!
//! let mut iter = BoxIter::new(&data);
//! if let Some(Ok(view)) = iter.next() {
//!     assert_eq!(view.header.boxsize().value(), Some(8));
//! }
//! ```

// =============================================================================
// Core Layer - no_std, no_alloc compatible
// =============================================================================

pub mod error;
pub mod header;

// Re-exports for convenience
pub use error::{
    Error, //
    ErrorKind,
    Result,
};
pub use header::BoxHeader;
pub use header::FullBoxHeader;

// =============================================================================
// Framing Layer - no_std, no_alloc compatible
// =============================================================================

pub mod iter;
pub mod view;

pub use iter::BoxIter;
pub use view::BoxView;

// =============================================================================
// Box type Layer -  alloc feature
// =============================================================================

pub mod specs;
