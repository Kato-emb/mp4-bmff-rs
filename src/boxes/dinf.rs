use crate::error::*;
use crate::iter::BoxIter;

use super::DrefBoxView;

/// A reference to a Data Information Box (`dinf`).
#[derive(Debug)]
pub struct DinfBoxView<'a> {
    payload: &'a [u8],
}

impl<'a> DinfBoxView<'a> {
    /// Parses a `DinfBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<DinfBoxView<'a>> {
        Ok(DinfBoxView { payload })
    }

    /// Returns an iterator over the child boxes of this `DinfBoxView`.
    pub fn children(&self) -> BoxIter<'a> {
        BoxIter::new(self.payload)
    }

    /// Returns the Data Reference Box (`dref`) if present.
    pub fn dref(&self) -> Result<Option<DrefBoxView<'_>>> {
        use crate::header::boxtype;

        for child in self.children() {
            let child = child?;
            if child.header.boxtype().type_field() == boxtype::DREF {
                let dref = DrefBoxView::parse(child.payload)?;
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
    use super::*;
    use crate::header::boxtype;

    use crate::boxes::DrefBox;

    /// An owned Data Information Box (`dinf`).
    pub struct DinfBox {
        /// The Data Reference Box (`dref`).
        pub dref: DrefBox,
    }

    impl DinfBox {
        /// Creates a `DinfBox` from a `DinfBoxView`.
        pub fn from_view(view: &DinfBoxView<'_>) -> Result<DinfBox> {
            let mut dref = None;

            for child in view.children() {
                let child = child?;
                if child.header.boxtype().type_field() == boxtype::DREF {
                    let dref_view = DrefBoxView::parse(child.payload)?;
                    dref = Some(DrefBox::from_view(&dref_view));
                }
            }

            Ok(DinfBox {
                dref: dref.ok_or(Error::new(ErrorKind::BoxMissing {
                    required: boxtype::DREF,
                }))?,
            })
        }

        /// Parses a `DinfBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<DinfBox> {
            let view = DinfBoxView::parse(payload)?;
            DinfBox::from_view(&view)
        }
    }

    impl DinfBoxView<'_> {
        /// Converts this `DinfBoxView` into an owned `DinfBox`.
        pub fn to_owned(&self) -> DinfBox {
            DinfBox::from_view(self).unwrap()
        }
    }

    impl From<DinfBoxView<'_>> for DinfBox {
        fn from(view: DinfBoxView<'_>) -> Self {
            DinfBox::from_view(&view).unwrap()
        }
    }
}
