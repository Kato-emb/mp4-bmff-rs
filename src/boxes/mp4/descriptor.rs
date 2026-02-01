//! Descriptor types and utilities.
//! ISO/IEC 14496-1 (MPEG-4 Part 1) Descriptors
//! and ISO/IEC 14496-3 (MPEG-4 Part 3) AudioSpecificConfig.

mod dec;
mod es;
mod iter;
mod pli;
mod size;
mod tag;
mod view;

pub use iter::DescrptorIter;
pub use size::SizeOfInstance;
pub use tag::Tag;
pub use view::DescriptorView;

pub use dec::DecoderConfigDescriptorView;
pub use dec::ObjectTypeIndication;
pub use dec::StreamType;

pub use pli::ExtensionProfileLevelDescriptor;
pub use pli::ProfileLevelIndicationIndexDescriptor;

pub use es::EsDescriptorView;

#[cfg(feature = "alloc")]
mod owned_exports {
    use super::*;

    pub use view::DescriptorOwned;

    pub use dec::DecoderConfigDescriptor;
    pub use es::EsDescriptor;
}

#[cfg(feature = "alloc")]
pub use owned_exports::*;
