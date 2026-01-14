use crate::boxes::BoxIter;
use crate::boxes::error::*;
use crate::boxes::specs::MvhdBox;

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
        for child in self.children() {
            let child = child?;
            if child.header.boxtype().type_field().as_ascii() == Some("mvhd") {
                let mvhd = MvhdBox::parse(child.payload)?;
                return Ok(Some(mvhd));
            }
        }

        Ok(None)
    }

    // pub fn traks(&self) -> impl Iterator<Item = Result<TrakBoxRef<'a>>>
}

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use super::*;
    /// An owned Movie Box (`moov`).
    pub struct MoovBox {
        pub mvhd: MvhdBox,
        // pub mvex: Option<MvexBox>,
        // pub traks: Vec<TrakBox>,
    }
}
