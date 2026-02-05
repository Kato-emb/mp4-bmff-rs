//! MPEG-4 Systems descriptor structures (ISO/IEC 14496-1).
//!
//! This module provides types for parsing and creating MPEG-4 Systems descriptors
//! as defined in ISO/IEC 14496-1. These descriptors are used in the `esds` box
//! to describe elementary stream configuration.
//!
//! # Descriptor Structure
//!
//! Each descriptor consists of:
//! - **Tag** (1 byte): Identifies the descriptor type
//! - **Size** (1-4 bytes): Variable-length encoded size of the instance data
//! - **Instance data**: Descriptor-specific payload
//!
//! # Common Descriptors
//!
//! | Tag | Name | Description |
//! |-----|------|-------------|
//! | 0x03 | ES_Descriptor | Elementary Stream Descriptor |
//! | 0x04 | DecoderConfigDescriptor | Decoder configuration |
//! | 0x05 | DecoderSpecificInfo | Codec-specific data |
//! | 0x06 | SLConfigDescriptor | Sync Layer configuration |
//!
//! # Example
//!
//! ```
//! use mp4_bmff::formats::mpeg4::systems::descriptor::{Tag, iter_descriptors};
//!
//! // Parse descriptors from a byte slice
//! let data = [0x05, 0x02, 0x11, 0x90]; // DecoderSpecificInfo with 2 bytes
//! for result in iter_descriptors(&data) {
//!     let descr = result.unwrap();
//!     if descr.tag() == Tag::DECODER_SPECIFIC_INFO_TAG {
//!         println!("Found decoder specific info: {:?}", descr.instance());
//!     }
//! }
//! ```

mod base;
pub mod iter;

mod dec;
mod es;

pub use base::raw::{
    RawDescriptor, //
    RawDescriptorRef,
};
pub use base::size::SizeOfInstance;
pub use base::tag::Tag;

pub use iter::iter_descriptors;

pub use dec::{
    DecoderConfigDescriptorView, //
    ObjectTypeIndication,
    StreamType,
};
pub use es::EsDescriptorView;

#[cfg(feature = "alloc")]
mod owned_exports {
    use super::*;
    pub use base::raw::RawDescriptorOwned;

    pub use dec::DecoderConfigDescriptor;
    pub use es::EsDescriptor;
}

#[cfg(feature = "alloc")]
pub use owned_exports::*;
