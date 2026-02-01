//! ISO/IEC 14496-14 Box Structures

pub mod descriptor;

mod esds;
mod mp4a;

pub use esds::EsdsBoxView;
pub use esds::EsdsFlags;

pub use mp4a::Mp4aBoxView;

#[cfg(feature = "alloc")]
mod owned_exports {
    use super::*;

    pub use esds::EsdsBox;
    pub use mp4a::Mp4aBox;
}

#[cfg(feature = "alloc")]
pub use owned_exports::*;

define_box_types!(
    /// ES Descriptor Box
    ESDS = b"esds",
    /// MPEG-4 Audio Sample Entry Box
    MP4A = b"mp4a",
);
