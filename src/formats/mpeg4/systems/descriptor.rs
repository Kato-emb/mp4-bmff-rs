//!

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
