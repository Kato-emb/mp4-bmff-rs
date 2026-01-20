//! BMFF box header types and parsing.

pub mod boxsize;
pub mod boxtype;
pub mod fullbox;

pub use boxsize::BoxSize;
pub use boxtype::BoxType;
pub use fullbox::FullBoxFlags;
