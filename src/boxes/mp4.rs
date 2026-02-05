//! ISO/IEC 14496-14 Box Structures

mod esds;
mod mp4a;
mod mp4v;

pub use esds::EsdsBoxView;
pub use esds::EsdsFlags;

pub use mp4a::Mp4aSampleEntryView;
pub use mp4v::Mp4vSampleEntryView;

#[cfg(feature = "alloc")]
mod owned_exports {
    use super::*;

    pub use esds::EsdsBox;
    pub use mp4a::Mp4aSampleEntry;
    pub use mp4v::Mp4vSampleEntry;
}

#[cfg(feature = "alloc")]
pub use owned_exports::*;

define_box_types!(
    /// ES Descriptor Box
    ESDS = b"esds",
    /// MPEG-4 Visual Sample Entry Box
    MP4V = b"mp4v",
    /// MPEG-4 Audio Sample Entry Box
    MP4A = b"mp4a",
);
