//! Common BMFF data types shared across boxes and fields.
//!
//! This module provides strongly typed wrappers for primitive values used
//! throughout BMFF structures. These types ensure type safety and provide
//! convenient conversion methods.
//!
//! # Fixed-Point Numbers
//!
//! BMFF uses fixed-point numbers for precise representation of fractional
//! values without floating-point hardware:
//!
//! - [`I8F8`]: Signed 8.8 fixed-point (16-bit storage).
//! - [`U8F8`]: Unsigned 8.8 fixed-point (16-bit storage).
//! - [`I16F16`]: Signed 16.16 fixed-point (32-bit storage).
//! - [`U16F16`]: Unsigned 16.16 fixed-point (32-bit storage).
//! - [`I2F30`]: Signed 2.30 fixed-point (32-bit storage, for matrix projective terms).
//! - [`Fixed`]: Generic fixed-point type with configurable storage and precision.
//!
//! # Identifier Types
//!
//! - [`FourCC`]: 4-byte identifier code used for box types and brands.
//! - [`Uuid`]: 16-byte UUID for extended box types.
//! - [`LanguageCode`]: ISO-639-2/T 3-letter language code.
//!
//! # Fraction Types
//!
//! - [`Fraction`]: Generic fraction (numerator/denominator pair).
//! - [`Fraction32`]: Unsigned 32-bit fraction alias.
//!
//! # Composite Types
//!
//! - [`Matrix`]: 3×3 affine transformation matrix for video tracks.
//! - [`QuickTimeDateTime`]: Timestamp as seconds since 1904-01-01 (QuickTime epoch).
//!
//! # Example
//!
//! ```
//! use mp4_bmff::types::{FourCC, U16F16, Matrix};
//!
//! // Create a FourCC identifier
//! let brand = FourCC::new(*b"isom");
//!
//! // Create a fixed-point number (72.0 dpi)
//! let dpi = U16F16::from_raw(0x00480000);
//! assert_eq!(dpi.integer(), 72);
//!
//! // Identity transformation matrix
//! let transform = Matrix::identity();
//! ```

mod fixed;
mod fourcc;
mod fraction;
mod language;
mod matrix;
mod time;
mod uuid;

pub use fixed::{
    Fixed, //
    I2F30,
    I8F8,
    I16F16,
    U8F8,
    U16F16,
};
pub use fourcc::FourCC;
pub use fraction::{Fraction, FractionU32};
pub use language::LanguageCode;
pub use matrix::Matrix;
pub use time::QuickTimeDateTime;
pub use uuid::Uuid;
