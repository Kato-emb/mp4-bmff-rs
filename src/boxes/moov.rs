use crate::cursor::ReadCursor;

use crate::BoxType;
use crate::error::*;
use crate::iter::BoxIter;

use super::mvhd::MvhdBox;
use super::trak::TrakBoxView;

/// A reference to a Movie Box (`moov`).
pub struct MoovBoxView<'a> {
    payload: &'a [u8],
}

impl<'a> MoovBoxView<'a> {
    /// Returns an iterator over the child boxes of this `MoovBoxView`.
    pub fn children(&self) -> BoxIter<'a> {
        BoxIter::new(self.payload)
    }

    /// Returns the Movie Header Box (`mvhd`) if present.
    pub fn mvhd(&self) -> Option<Result<MvhdBox>> {
        for child in self.children() {
            match child {
                Ok(c) if c.header.boxtype() == BoxType::MVHD => {
                    return Some(MvhdBox::parse(c.payload));
                }
                Ok(_) => continue,
                Err(e) => return Some(Err(e)),
            }
        }

        None
    }

    /// Returns an iterator over the Track Boxes (`trak`) contained in this `MoovBoxView`.
    pub fn traks(&self) -> impl Iterator<Item = Result<TrakBoxView<'a>>> + 'a {
        self.children().filter_map(|child| match child {
            Ok(view) if view.header.boxtype() == BoxType::TRAK => {
                Some(TrakBoxView::parse(view.payload))
            }
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<MoovBoxView<'a>> {
        let payload = cur.take(cur.remaining())?;
        Ok(MoovBoxView { payload })
    }

    /// Parses a `MoovBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<MoovBoxView<'a>> {
        let mut cursor = ReadCursor::new(payload);
        let this = MoovBoxView::parse_in(&mut cursor)?;

        Ok(this)
    }
}

#[cfg(feature = "alloc")]
pub use owned::MoovBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use super::*;

    use crate::boxes::TrakBox;

    /// An owned Movie Box (`moov`).
    pub struct MoovBox {
        /// The Movie Header Box (`mvhd`).
        pub mvhd: MvhdBox,
        // pub mvex: Option<MvexBox>,
        /// The Track Boxes (`trak`).
        pub traks: Vec<TrakBox>,
    }
}
