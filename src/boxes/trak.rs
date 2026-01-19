use crate::cursor::ReadCursor;

use crate::BoxIter;
use crate::BoxType;
use crate::error::*;

use super::tkhd::TkhdBox;

/// A reference to a Track Box (`trak`).
#[derive(Debug)]
pub struct TrakBoxView<'a> {
    payload: &'a [u8],
}

impl<'a> TrakBoxView<'a> {
    /// Returns an iterator over the child boxes of this `TrakBoxView`.
    pub fn children(&self) -> BoxIter<'a> {
        BoxIter::new(self.payload)
    }

    /// Returns the Track Header Box (`tkhd`) if present.
    pub fn tkhd(&self) -> Option<Result<TkhdBox>> {
        for child in self.children() {
            match child {
                Ok(c) if c.header.boxtype() == BoxType::TKHD => {
                    return Some(TkhdBox::parse(c.payload));
                }
                Ok(_) => continue,
                Err(e) => return Some(Err(e)),
            }
        }

        None
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<TrakBoxView<'a>> {
        let payload = cur.take(cur.remaining())?;
        Ok(TrakBoxView { payload })
    }

    /// Parses a `TrakBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<TrakBoxView<'a>> {
        let mut cursor = ReadCursor::new(payload);
        let this = TrakBoxView::parse_in(&mut cursor)?;

        Ok(this)
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
