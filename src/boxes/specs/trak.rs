use crate::boxes::BoxIter;
use crate::boxes::error::*;

/// A reference to a Track Box (`trak`).
#[derive(Debug)]
pub struct TrakBoxRef<'a> {
    payload: &'a [u8],
}

impl<'a> TrakBoxRef<'a> {
    /// Parses a `TrakBoxRef` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<TrakBoxRef<'a>> {
        Ok(TrakBoxRef { payload })
    }

    /// Returns an iterator over the child boxes of this `TrakBoxRef`.
    pub fn children(&self) -> BoxIter<'a> {
        BoxIter::new(self.payload)
    }
}

#[cfg(feature = "alloc")]
pub use owned::TrakBox;

#[cfg(feature = "alloc")]
mod owned {
    /// An owned Track Box (`trak`).
    pub struct TrakBox {
        // Add fields as necessary for owned representation
    }
}
