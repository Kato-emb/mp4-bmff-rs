use crate::boxes::BoxIter;
use crate::boxes::error::*;
use crate::boxes::specs::MvhdBox;
use crate::boxes::specs::TrakBoxRef;

/// A reference to a Movie Box (`moov`).
pub struct MoovBoxRef<'a> {
    payload: &'a [u8],
}

impl<'a> MoovBoxRef<'a> {
    /// Parses a `MoovBoxRef` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<MoovBoxRef<'a>> {
        Ok(MoovBoxRef { payload })
    }

    /// Returns an iterator over the child boxes of this `MoovBoxRef`.
    pub fn children(&self) -> BoxIter<'a> {
        BoxIter::new(self.payload)
    }

    /// Returns the Movie Header Box (`mvhd`) if present.
    pub fn mvhd(&self) -> Result<Option<MvhdBox>> {
        use crate::boxes::header::boxtype;

        for child in self.children() {
            let child = child?;
            if child.header.boxtype().type_field() == boxtype::MVHD {
                let mvhd = MvhdBox::parse(child.payload)?;
                return Ok(Some(mvhd));
            }
        }

        Ok(None)
    }

    /// Returns an iterator over the Track Boxes (`trak`) contained in this `MoovBoxRef`.
    pub fn traks(&self) -> impl Iterator<Item = Result<TrakBoxRef<'a>>> {
        use crate::boxes::header::boxtype;

        self.children().filter_map(|child| {
            let child = match child {
                Ok(c) => c,
                Err(e) => return Some(Err(e)),
            };
            if child.header.boxtype().type_field() == boxtype::TRAK {
                match TrakBoxRef::parse(child.payload) {
                    Ok(trak) => Some(Ok(trak)),
                    Err(e) => Some(Err(e)),
                }
            } else {
                None
            }
        })
    }
}

#[cfg(feature = "alloc")]
pub use owned::MoovBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use super::*;

    /// An owned Movie Box (`moov`).
    pub struct MoovBox {
        /// The Movie Header Box (`mvhd`).
        pub mvhd: MvhdBox,
        // pub mvex: Option<MvexBox>,
        // pub traks: Vec<TrakBox>,
    }
}
