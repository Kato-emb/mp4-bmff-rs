//! Data Information Box (`dinf`) implementation.
//!
//! The Data Information Box is a container box that specifies the location
//! of the media data. It contains a Data Reference Box (`dref`) that describes
//! where the actual media data can be found - either within this file or
//! in external files referenced by URL or URN.
//!
//! This box is required within the Media Information Box (`minf`).

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::BoxIter;

use super::dref::DrefBoxView;

/// A reference to a Data Information Box (`dinf`).
///
/// Container box that holds data reference information describing where
/// the media data is located.
///
/// # Structure
///
/// Required child boxes:
/// - `dref`: Data Reference Box - contains URL/URN references to media data.
#[derive(Debug)]
pub struct DinfBoxView<'a> {
    content: &'a [u8],
}

impl<'a> DinfBoxView<'a> {
    /// Returns an iterator over the child boxes of this `dinf` box.
    pub fn boxes(&self) -> BoxIter<'a> {
        BoxIter::new(self.content)
    }

    /// Returns the Data Reference Box (`dref`) contained in this `dinf` box.
    pub fn dref(&self) -> Result<DrefBoxView<'a>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::DREF {
                let dref = DrefBoxView::decode(b.into_payload())?;
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
        Ok(DinfBoxView { content: bytes })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use super::*;
    use crate::BoxEncode;

    use crate::codec::boxed_len;
    use crate::codec::write_box_in;
    use crate::cursor::WriteCursor;

    use crate::boxes::bmff::DrefBox;

    /// An owned Data Information Box (`dinf`).
    ///
    /// This is the owned variant of [`DinfBoxView`] that stores the
    /// contained Data Reference Box.
    ///
    /// # Structure
    ///
    /// - `dref`: Data Reference Box with URL/URN entries.
    #[derive(Debug, Clone)]
    pub struct DinfBox {
        /// The Data Reference Box contained in this `dinf` box.
        pub dref: DrefBox,
    }

    impl TryFrom<&DinfBoxView<'_>> for DinfBox {
        type Error = Error;

        fn try_from(view: &DinfBoxView<'_>) -> Result<Self> {
            let mut dref = None;

            for result in view.boxes() {
                let rawbox = result?;

                match rawbox.boxtype() {
                    BoxType::DREF => {
                        if dref.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::DREF,
                                },
                                BoxType::DINF,
                            ));
                        }
                        let dref_box = DrefBox::decode(rawbox.payload())?;
                        dref = Some(dref_box);
                    }
                    _ => continue,
                }
            }

            let dref = dref.ok_or_else(|| {
                Error::in_box(
                    ErrorKind::BoxMissing {
                        required: BoxType::DREF,
                    },
                    BoxType::DINF,
                )
            })?;

            Ok(DinfBox { dref })
        }
    }

    impl BoxCodec for DinfBox {
        fn boxtype(&self) -> BoxType {
            BoxType::DINF
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

#[cfg(feature = "alloc")]
pub use owned::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_data() -> [u8; 28] {
        [
            // dref box
            0x00, 0x00, 0x00, 0x1C, // size = 28
            b'd', b'r', b'e', b'f', // type = "dref"
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x01, // entry_count = 1
            // url box (self-contained)
            0x00, 0x00, 0x00, 0x0C, // size = 12
            b'u', b'r', b'l', b' ', // type = "url "
            0x00, // version = 0
            0x00, 0x00, 0x01, // flags = SELF_CONTAINED
        ]
    }

    #[test]
    fn test_dinf_box_view_decode() {
        let data = raw_data();
        let dinf = DinfBoxView::decode(&data).unwrap();

        assert_eq!(dinf.boxes().count(), 1);
    }

    #[test]
    fn test_dinf_box_view_dref() {
        let data = raw_data();
        let dinf = DinfBoxView::decode(&data).unwrap();
        let dref = dinf.dref().unwrap();

        assert_eq!(dref.entry_count, 1);
    }

    #[test]
    fn test_dinf_box_view_missing_dref() {
        let data: [u8; 0] = [];
        let dinf = DinfBoxView::decode(&data).unwrap();
        let result = dinf.dref();

        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_dinf_box_round_trip() {
        use crate::BoxEncode;

        let original = raw_data();
        let dinf = DinfBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; dinf.encoded_len()];
        dinf.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_dinf_box_try_from() {
        let data = raw_data();
        let view = DinfBoxView::decode(&data).unwrap();
        let owned = DinfBox::try_from(&view).unwrap();

        assert_eq!(owned.dref.entries.len(), 1);
    }
}
