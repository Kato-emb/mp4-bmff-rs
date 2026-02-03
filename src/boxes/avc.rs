//! ISO/IEC 14496-15 AVC Boxes

mod avcc;

pub use avcc::AvcCBoxView;
pub use avcc::AvcSampleEntryView;
pub use avcc::NalUnitIter;

#[cfg(feature = "alloc")]
mod owned_exports {
    use super::*;

    pub use avcc::Avc1Box;
    pub use avcc::Avc3Box;
    pub use avcc::AvcCBox;
}

#[cfg(feature = "alloc")]
pub use owned_exports::*;
