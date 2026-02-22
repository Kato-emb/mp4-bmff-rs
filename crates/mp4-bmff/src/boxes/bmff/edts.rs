//! Edit Box (`edts`) implementation.
//!
//! The Edit Box is an optional container that maps the timeline of a track
//! (the presentation time) to the media time within that track. This allows
//! for operations like trimming, looping, or inserting empty time.

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::BoxIter;

use super::ElstBoxView;

/// A reference to an Edit Box (`edts`).
///
/// The Edit Box contains an edit list that defines how to map the track
/// timeline to the actual media samples. Each edit segment specifies
/// a portion of the media to play and at what rate.
///
/// # Structure
///
/// - `elst`: Edit List Box (optional) - the actual edit list entries.
#[derive(Debug)]
pub struct EdtsBoxView<'a> {
    content: &'a [u8],
}

impl<'a> EdtsBoxView<'a> {
    /// Returns an iterator over the child boxes of this `edts` box.
    pub fn boxes(&self) -> BoxIter<'a> {
        BoxIter::new(self.content)
    }

    /// Returns the Edit List Box (`elst`) contained in this `edts` box if present.
    ///
    /// # Errors
    ///
    /// Returns an error if there are multiple `elst` boxes. The `elst` box is optional, but if present, there must be only one.
    pub fn elst(&self) -> Result<Option<ElstBoxView<'a>>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::ELST {
                let elst = ElstBoxView::decode(b.into_payload())?;
                return Ok(Some(elst));
            }
        }

        Ok(None)
    }
}

impl BoxCodec for EdtsBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::EDTS
    }
}

impl<'de> BoxDecode<'de> for EdtsBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        Ok(EdtsBoxView { content: bytes })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use super::*;
    use crate::BoxEncode;

    use crate::codec::boxed_len;
    use crate::codec::write_box_in;
    use crate::cursor::WriteCursor;

    use crate::boxes::bmff::ElstBox;

    /// An owned Edit Box (`edts`).
    ///
    /// This is the owned variant of [`EdtsBoxView`] that stores the edit list
    /// in a heap-allocated structure.
    ///
    /// # Structure
    ///
    /// - `elst`: Edit List Box containing the timeline mapping entries.
    #[derive(Debug, Clone)]
    pub struct EdtsBox {
        /// Edit List Box defining how track time maps to media time.
        pub elst: Option<ElstBox>,
    }

    impl TryFrom<&EdtsBoxView<'_>> for EdtsBox {
        type Error = Error;

        fn try_from(view: &EdtsBoxView<'_>) -> Result<Self> {
            let mut elst = None;

            for result in view.boxes() {
                let rawbox = result?;

                match rawbox.boxtype() {
                    BoxType::ELST => {
                        if elst.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::ELST,
                                },
                                BoxType::EDTS,
                            ));
                        }
                        let elst_box = ElstBox::try_from(&ElstBoxView::decode(rawbox.payload())?)?;
                        elst = Some(elst_box);
                    }
                    _ => continue,
                }
            }

            Ok(EdtsBox { elst })
        }
    }

    impl BoxCodec for EdtsBox {
        fn boxtype(&self) -> BoxType {
            BoxType::EDTS
        }
    }

    impl BoxDecode<'_> for EdtsBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = EdtsBoxView::decode(bytes)?;
            EdtsBox::try_from(&view)
        }
    }

    impl BoxEncode for EdtsBox {
        #[inline]
        fn encoded_len(&self) -> usize {
            let mut len = 0;
            if let Some(elst) = &self.elst {
                len += boxed_len(elst);
            }
            len
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            if let Some(elst) = &self.elst {
                write_box_in(&mut cur, elst)?;
            }

            Ok(cur.position())
        }
    }
}

#[cfg(feature = "alloc")]
pub use owned::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_data_empty() -> [u8; 0] {
        []
    }

    fn raw_data_with_elst() -> [u8; 28] {
        [
            // elst box
            0x00, 0x00, 0x00, 0x1C, // size = 28
            b'e', b'l', b's', b't', // type = "elst"
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x01, // entry_count = 1
            // entry (v0: segment_duration=4, media_time=4, rate=4 = 12 bytes)
            0x00, 0x00, 0x03, 0xE8, // segment_duration = 1000
            0x00, 0x00, 0x00, 0x00, // media_time = 0
            0x00, 0x01, 0x00, 0x00, // media_rate = 1.0 (16.16 fixed point)
        ]
    }

    #[test]
    fn test_edts_box_view_decode_empty() {
        let data = raw_data_empty();
        let edts = EdtsBoxView::decode(&data).unwrap();

        assert_eq!(edts.boxes().count(), 0);
        assert!(edts.elst().unwrap().is_none());
    }

    #[test]
    fn test_edts_box_view_decode_with_elst() {
        let data = raw_data_with_elst();
        let edts = EdtsBoxView::decode(&data).unwrap();

        let elst = edts.elst().unwrap();
        assert!(elst.is_some());

        let elst = elst.unwrap();
        assert_eq!(elst.entry_count, 1);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_edts_box_round_trip() {
        use crate::BoxEncode;

        let original = raw_data_with_elst();
        let edts = EdtsBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; edts.encoded_len()];
        let len = edts.encode_into(&mut encoded).unwrap();

        assert_eq!(len, original.len());
        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_edts_box_try_from() {
        let data = raw_data_with_elst();
        let view = EdtsBoxView::decode(&data).unwrap();
        let owned = EdtsBox::try_from(&view).unwrap();

        assert!(owned.elst.is_some());
    }
}
