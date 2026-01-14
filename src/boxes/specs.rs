//! ISO Base Media File Format box specifications.
//!
//! This module provides parsers and types for specific box types defined in
//! ISO/IEC 14496-12 and related specifications.
//!
//! Each box type provides:
//! - A zero-copy reference type (`*BoxRef`) for efficient parsing without allocation
//! - An owned type (`*Box`) for modification and serialization (requires `alloc` feature)

mod free;
mod ftyp;
mod mdat;
mod moov;
mod mvhd;
mod tkhd;
mod trak;

pub use mvhd::{
    MvhdBox, //
    MvhdFlags,
};

pub use free::FreeBoxRef;
pub use ftyp::FtypBoxRef;
pub use mdat::MdatBoxRef;
pub use moov::MoovBoxRef;
pub use trak::TrakBoxRef;

#[cfg(feature = "alloc")]
mod owned {
    pub use super::ftyp::FtypBox;
    pub use super::moov::MoovBox;
    pub use super::trak::TrakBox;
}

#[cfg(feature = "alloc")]
pub use owned::*;
