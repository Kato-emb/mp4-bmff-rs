use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
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
    pub fn dref(&self) -> Result<DrefBoxView<'a>> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::DREF {
                let dref = DrefBoxView::decode(child.into_payload())?;
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
}

impl BoxCodec for DinfBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::DINF
    }
}

impl<'de> BoxDecode<'de> for DinfBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        Ok(DinfBoxView { payload: bytes })
    }
}

impl<'a> TryFrom<&'a [u8]> for DinfBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        DinfBoxView::decode(value)
    }
}

#[cfg(feature = "alloc")]
pub use owned::DinfBox;

#[cfg(feature = "alloc")]
mod owned {
    use super::*;
    use crate::BoxEncode;
    use crate::codec::boxed_len;
    use crate::codec::write_box_in;

    use crate::boxes::DrefBox;

    use crate::cursor::WriteCursor;

    /// An owned Data Information Box (`dinf`).
    pub struct DinfBox {
        /// The Data Reference Box (`dref`).
        pub dref: DrefBox,
    }

    impl BoxCodec for DinfBox {
        fn boxtype(&self) -> BoxType {
            BoxType::DINF
        }
    }

    impl TryFrom<&DinfBoxView<'_>> for DinfBox {
        type Error = Error;

        fn try_from(view: &DinfBoxView<'_>) -> Result<Self> {
            let mut dref = None;

            for child in view.children() {
                let child = child?;
                if child.boxtype() == BoxType::DREF {
                    let dref_view = DrefBoxView::decode(child.payload())?;
                    dref = Some(DrefBox::try_from(&dref_view)?);
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
    }

    impl BoxDecode<'_> for DinfBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = DinfBoxView::decode(bytes)?;
            DinfBox::try_from(&view)
        }
    }

    impl BoxEncode for DinfBox {
        fn encoded_len(&self) -> usize {
            boxed_len(&self.dref)
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);
            write_box_in(&mut cur, &self.dref)?;
            Ok(cur.position())
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

        let dinf = DinfBoxView::decode(&dinf_payload).unwrap();
        let dref = dinf.dref().unwrap();

        assert_eq!(dref.entry_count, 1);
    }

    #[test]
    fn parse_dinf_missing_dref() {
        // dinf with no children
        let dinf_payload: Vec<u8> = vec![];

        let dinf = DinfBoxView::decode(&dinf_payload).unwrap();
        let result = dinf.dref();

        assert!(result.is_err());
    }
}
