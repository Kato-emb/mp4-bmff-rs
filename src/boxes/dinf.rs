use crate::cursor::ReadCursor;

use crate::BoxIter;
use crate::BoxType;
use crate::error::*;

use super::DrefBoxView;

/// A reference to a Data Information Box (`dinf`).
#[derive(Debug)]
pub struct DinfBoxView<'a> {
    payload: &'a [u8],
}

impl<'a> DinfBoxView<'a> {
    /// Returns an iterator over the child boxes of this `DinfBoxView`.
    pub fn children(&self) -> BoxIter<'a> {
        BoxIter::new(self.payload)
    }

    /// Returns the Data Reference Box (`dref`) if present.
    pub fn dref(&self) -> Result<DrefBoxView<'a>> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::DREF {
                let dref = DrefBoxView::parse(child.payload())?;
                return Ok(dref);
            }
        }

        Err(Error::in_box(
            ErrorKind::BoxMissing {
                required: BoxType::DREF,
            },
            BoxType::DINF,
        ))
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<DinfBoxView<'a>> {
        let payload = cur.take(cur.remaining())?;
        Ok(DinfBoxView { payload })
    }

    /// Parses a `DinfBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<DinfBoxView<'a>> {
        let mut cursor = ReadCursor::new(payload);
        let this = DinfBoxView::parse_in(&mut cursor)?;

        Ok(this)
    }
}

#[cfg(feature = "alloc")]
pub use owned::DinfBox;

#[cfg(feature = "alloc")]
mod owned {
    use super::*;

    use crate::cursor::WriteCursor;

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
                if child.boxtype() == BoxType::DREF {
                    let dref_view = DrefBoxView::parse(child.payload())?;
                    dref = Some(DrefBox::from_view(&dref_view)?);
                }
            }

            Ok(DinfBox {
                dref: dref.ok_or(Error::in_box(
                    ErrorKind::BoxMissing {
                        required: BoxType::DREF,
                    },
                    BoxType::DINF,
                ))?,
            })
        }

        /// Parses a `DinfBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<DinfBox> {
            let view = DinfBoxView::parse(payload)?;
            DinfBox::from_view(&view)
        }

        /// Returns the size of this `DinfBox`.
        #[inline]
        pub fn size(&self) -> usize {
            self.dref.size() // Box header + dref box
        }

        pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
            // Write dref box
            self.dref.write_in(cur)?;
            Ok(())
        }

        /// Writes this `DinfBox` into the given payload.
        pub fn write(&self, payload: &mut [u8]) -> Result<()> {
            let mut cursor = WriteCursor::new(payload);
            self.write_in(&mut cursor)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_box(fourcc: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let size = (8 + payload.len()) as u32;
        let mut data = Vec::new();
        data.extend_from_slice(&size.to_be_bytes());
        data.extend_from_slice(fourcc);
        data.extend_from_slice(payload);
        data
    }

    fn make_dref_payload_self_contained() -> Vec<u8> {
        // version=0, flags=0, entry_count=1
        let mut dref_payload = vec![0, 0, 0, 0, 0, 0, 0, 1];
        // url box (self-contained): size=12, 'url ', version=0, flags=0x000001
        dref_payload.extend_from_slice(&[0, 0, 0, 12, b'u', b'r', b'l', b' ', 0, 0, 0, 1]);
        dref_payload
    }

    #[test]
    fn parse_dinf_with_dref() {
        let dref_box = make_box(b"dref", &make_dref_payload_self_contained());
        let dinf_payload = dref_box;

        let dinf = DinfBoxView::parse(&dinf_payload).unwrap();
        let dref = dinf.dref().unwrap();

        assert_eq!(dref.entry_count, 1);
    }

    #[test]
    fn parse_dinf_missing_dref() {
        // dinf with no children
        let dinf_payload: Vec<u8> = vec![];

        let dinf = DinfBoxView::parse(&dinf_payload).unwrap();
        let result = dinf.dref();

        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn dinf_box_round_trip() {
        use crate::BoxFrameMut;
        use crate::boxes::{DrefBox, DrefEntry, UrlBox, UrlFlags};

        let dinf_box = DinfBox {
            dref: DrefBox {
                version: 0,
                flags: crate::boxes::DrefFlags::empty(),
                entries: vec![DrefEntry::Url(UrlBox {
                    version: 0,
                    flags: UrlFlags::SELF_CONTAINED,
                    location: None,
                })],
            },
        };

        // Write dinf (note: dinf.size() returns dref payload size, need frame)
        let dref_payload_size = dinf_box.dref.size();
        let dref_frame_size = BoxFrameMut::required_len(BoxType::DREF, dref_payload_size);
        let mut buf = vec![0u8; dref_frame_size];

        // Write dref as a framed box
        let mut frame = BoxFrameMut::new(&mut buf, BoxType::DREF, dref_payload_size).unwrap();
        dinf_box.dref.write(frame.payload_mut()).unwrap();

        // Parse back as dinf payload
        let reparsed = DinfBoxView::parse(&buf).unwrap();
        let dref = reparsed.dref().unwrap();

        assert_eq!(dref.entry_count, 1);
    }
}
