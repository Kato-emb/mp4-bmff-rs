//!

use crate::BoxType;
use crate::error::Result;

/// A trait for BMFF box codecs.
pub trait BoxCodec {
    /// Returns the box type.
    fn boxtype(&self) -> BoxType;
}

/// A trait for decoding BMFF boxes from byte slices.
pub trait BoxDecode<'de>: Sized {
    /// Decodes the box from the given byte slice.
    fn decode(bytes: &'de [u8]) -> Result<Self>;
}

/// A trait for encoding BMFF boxes into byte slices.
pub trait BoxEncode {
    /// Encodes the box into the given byte slice.
    fn encode(&self, bytes: &mut [u8]) -> Result<usize>;
}
