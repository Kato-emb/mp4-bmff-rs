use super::DrefBoxRef;

use crate::boxes::BoxIter;
use crate::boxes::error::*;

/// A reference to a Data Information Box (`dinf`).
#[derive(Debug)]
pub struct DinfBoxRef<'a> {
    payload: &'a [u8],
}

impl<'a> DinfBoxRef<'a> {
    /// Parses a `DinfBoxRef` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<DinfBoxRef<'a>> {
        Ok(DinfBoxRef { payload })
    }

    /// Returns an iterator over the child boxes of this `DinfBoxRef`.
    pub fn children(&self) -> BoxIter<'a> {
        BoxIter::new(self.payload)
    }

    /// Returns the Data Reference Box (`dref`) if present.
    pub fn dref(&self) -> Result<Option<DrefBoxRef<'_>>> {
        use crate::boxes::header::boxtype;

        for child in self.children() {
            let child = child?;
            if child.header.boxtype().type_field() == boxtype::DREF {
                let dref = DrefBoxRef::parse(child.payload)?;
                return Ok(Some(dref));
            }
        }

        Ok(None)
    }
}

#[cfg(feature = "alloc")]
pub use owned::DinfBox;

#[cfg(feature = "alloc")]
mod owned {
    // use super::*;
    use crate::boxes::specs::DrefBox;

    /// An owned Data Information Box (`dinf`).
    pub struct DinfBox {
        /// The Data Reference Box (`dref`).
        pub dref: DrefBox,
        // Add fields as necessary for owned representation
    }
}
