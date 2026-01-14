use crate::boxes::BoxIter;
use crate::boxes::error::*;

use crate::boxes::specs::TkhdBox;

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

    /// Returns the Track Header Box (`tkhd`) if present.
    pub fn tkhd(&self) -> Result<Option<TkhdBox>> {
        use crate::boxes::header::boxtype;

        for child in self.children() {
            let child = child?;
            if child.header.boxtype().type_field() == boxtype::TKHD {
                let tkhd = TkhdBox::parse(child.payload)?;
                return Ok(Some(tkhd));
            }
        }

        Ok(None)
    }
}

#[cfg(feature = "alloc")]
pub use owned::TrakBox;

#[cfg(feature = "alloc")]
mod owned {
    use super::*;

    /// An owned Track Box (`trak`).
    pub struct TrakBox {
        /// The Track Header Box (`tkhd`).
        pub tkhd: TkhdBox,
        // Add fields as necessary for owned representation
    }
}
