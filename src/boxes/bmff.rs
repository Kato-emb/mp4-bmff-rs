//! ISO/IEC 14496-12 Box Structures

mod free;
mod ftyp;
mod mdat;

pub use free::FreeBoxView;
pub use ftyp::FtypBoxView;
pub use mdat::MdatBoxView;

#[cfg(feature = "alloc")]
mod owned_exports {
    use super::*;

    // Variable-size boxes - Owned types
    pub use free::FreeBox;
    pub use ftyp::FtypBox;
    pub use mdat::MdatBox;
}

#[cfg(feature = "alloc")]
pub use owned_exports::*;
