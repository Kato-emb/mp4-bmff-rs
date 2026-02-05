//! ISO Base Media File Format (BMFF) base structures.
//!
//! This module provides the fundamental building blocks for parsing and writing
//! BMFF boxes. All types support `no_std` and `no_alloc` environments.
//!
//! # Module Organization
//!
//! - [`header`]: Box header structures including size and type fields.
//!   - [`BoxHeader`]: Complete box header with size and type.
//!   - [`BoxSize`]: Type-safe box size (compact, extended, or EOF).
//!   - [`BoxType`]: Type-safe box type (FourCC or UUID).
//! - [`rawbox`]: Raw box representation with header and uninterpreted payload.
//!   - [`RawBox`]: Generic raw box with configurable payload storage.
//!   - [`RawBoxRef`]: Zero-copy reference to a raw box.
//!   - `RawBoxOwned`: Owned raw box with heap-allocated payload (requires `alloc`).
//!
//! # Box Structure
//!
//! Every BMFF box consists of:
//! ```text
//! +--------+--------+------------------+
//! |  size  |  type  |     payload      |
//! +--------+--------+------------------+
//!  4 bytes  4 bytes   (size - 8) bytes
//! ```
//!
//! Extended boxes may have additional fields for large sizes (64-bit) or
//! UUID-based types (16-byte user type).
//!
//! [`BoxHeader`]: header::BoxHeader
//! [`BoxSize`]: header::BoxSize
//! [`BoxType`]: header::BoxType
//! [`RawBox`]: rawbox::RawBox
//! [`RawBoxRef`]: rawbox::RawBoxRef

pub mod header;
pub mod rawbox;
