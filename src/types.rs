//! Common BMFF data types shared across boxes and fields.
//!
//! This module hosts strongly typed wrappers such as fixed-point numbers,
//! matrices, and string helpers to provide a well-defined, documented API
//! for BMFF structures.

mod fixed;
mod fourcc;
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
pub use language::LanguageCode;
pub use matrix::Matrix;
pub use time::QuickTimeDateTime;
pub use uuid::Uuid;
