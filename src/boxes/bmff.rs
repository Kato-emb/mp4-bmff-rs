//! ISO/IEC 14496-12 Box Structures

mod free;
mod ftyp;
mod mdat;
mod pdin;

// Variable-size boxes - View types
pub use free::FreeBoxView;
pub use ftyp::FtypBoxView;
pub use mdat::MdatBoxView;
pub use pdin::PdinBoxView;

// Re-export entry structs
pub use pdin::PdinEntry;

#[cfg(feature = "alloc")]
mod owned_exports {
    use super::*;

    // Variable-size boxes - Owned types
    pub use free::FreeBox;
    pub use ftyp::FtypBox;
    pub use mdat::MdatBox;
    pub use pdin::PdinBox;
}

#[cfg(feature = "alloc")]
pub use owned_exports::*;
