//! # mp4-bmff
//!
//! A pure Rust implementation of ISO/IEC 14496-12
//! (ISO Base Media File Format, ISOBMFF).
//!
//! ## Feature Flags
//!
//! - `std` (default): Enables standard library support. Implies `alloc`.
//! - `alloc`: Enables heap allocation. Required for typed box representations.
//!
//! ## Module Organization
//!
//! The crate is organized into layers with different allocation requirements:
//!
//! ### Core Layer (no_std, no_alloc)
//!
//! These modules work in bare-metal environments without heap allocation:
//!
//! - [`cursor`]: Zero-copy read/write cursors for byte slices
//! - [`types`]: Primitive BMFF types (FourCC, UUID, fixed-point numbers)
//! - [`boxes`]: Box header parsing, view-based box iteration
//!
//! ### Typed Layer (requires `alloc`)
//!
//! These modules require heap allocation for owned representations:
//!
//! - TODO: Add typed box representations and parsing
//!
//! ## Usage
//!
//! ### no_alloc: View-based parsing
//!
//! ```
//! use mp4_bmff::boxes::BoxIter;
//!
//! let data = [
//!     0x00, 0x00, 0x00, 0x0C, b'f', b't', b'y', b'p',
//!     b'i', b's', b'o', b'm',
//! ];
//!
//! for result in BoxIter::new(&data) {
//!     let view = result.unwrap();
//!     // Access header and raw payload without allocation
//!     let _boxtype = view.header.boxtype();
//!     let _payload = view.payload;
//! }
//! ```

#![cfg_attr(all(not(feature = "std"), not(test)), no_std)]
#![warn(missing_docs)]

#[cfg(feature = "alloc")]
extern crate alloc;

// =============================================================================
// Core Layer - no_std, no_alloc compatible
// =============================================================================

pub mod boxes;
pub mod cursor;
pub mod types;

// =============================================================================
// Typed Layer - requires alloc feature
// =============================================================================

// #[cfg(feature = "alloc")]
// pub mod typed;
