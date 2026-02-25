//! Movie Extends Box (`mvex`) implementation.
//!
//! The Movie Extends Box signals that the movie may contain movie fragments
//! (fragmented MP4/fMP4). It provides default values for track fragments and
//! optionally declares the total duration of all fragments.
//!
//! This box is required for fragmented MP4 files and resides within the
//! Movie Box (`moov`).

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::BoxIter;

use super::MehdBox;
use super::TrexBox;

/// A reference to a Movie Extends Box (`mvex`).
///
/// Container for fragmented movie metadata. Signals that the file contains
/// movie fragments and provides default sample properties for each track.
///
/// # Structure
///
/// Optional child boxes:
/// - `mehd`: Movie Extends Header Box - total fragment duration.
///
/// Required child boxes (one per track):
/// - `trex`: Track Extends Box - default sample properties per track.
#[derive(Debug)]
pub struct MvexBoxView<'a> {
    content: &'a [u8],
}

impl<'a> MvexBoxView<'a> {
    /// Returns an iterator over the child boxes of this `mvex` box.
    pub fn boxes(&self) -> BoxIter<'a> {
        BoxIter::new(self.content)
    }

    /// Returns the Movie Extends Header Box (`mehd`) contained in this `mvex` box.
    ///
    /// # Errors
    ///
    /// Returns an error if there are multiple `mehd` boxes. It's valid for the `mehd` box to be missing, in which case this returns `Ok(None)`.
    pub fn mehd(&self) -> Result<Option<MehdBox>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::MEHD {
                let mehd = MehdBox::decode(b.payload())?;
                return Ok(Some(mehd));
            }
        }

        Ok(None)
    }

    /// Returns an iterator over the Track Extends Defaults Boxes (`trex`) contained in this `mvex` box.
    ///
    /// # Errors
    ///
    /// Returns an error if any of the `trex` boxes are invalid.
    pub fn trexs(&self) -> impl Iterator<Item = Result<TrexBox>> + 'a {
        self.boxes().filter_map(|result| match result {
            Ok(rawbox) if rawbox.boxtype() == BoxType::TREX => {
                Some(TrexBox::decode(rawbox.payload()))
            }
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
    }
}

impl BoxCodec for MvexBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::MVEX
    }
}

impl<'de> BoxDecode<'de> for MvexBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        Ok(MvexBoxView { content: bytes })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use alloc::vec::Vec;

    use super::*;
    use crate::BoxEncode;

    use crate::codec::boxed_len;
    use crate::codec::write_box_in;
    use crate::cursor::WriteCursor;

    /// An owned Movie Extends Box (`mvex`).
    ///
    /// This is the owned variant of [`MvexBoxView`] that stores child boxes
    /// in heap-allocated memory.
    ///
    /// # Structure
    ///
    /// - `mehd`: Optional Movie Extends Header with fragment duration.
    /// - `trexs`: Track Extends boxes (one per track) with default properties.
    #[derive(Debug, Clone)]
    pub struct MvexBox {
        /// Movie Extends Header Box (`mehd`), if present.
        pub mehd: Option<MehdBox>,
        /// Track Extends Defaults Boxes (`trex`), one per track.
        pub trexs: Vec<TrexBox>,
    }

    impl TryFrom<&MvexBoxView<'_>> for MvexBox {
        type Error = Error;

        fn try_from(view: &MvexBoxView<'_>) -> Result<Self> {
            let mut mehd = None;
            let mut trexs = Vec::new();

            for result in view.boxes() {
                let rawbox = result?;

                match rawbox.boxtype() {
                    BoxType::MEHD => {
                        if mehd.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::MEHD,
                                },
                                BoxType::MVEX,
                            ));
                        }
                        let mehd_box = MehdBox::decode(rawbox.payload())?;
                        mehd = Some(mehd_box);
                    }
                    BoxType::TREX => {
                        let trex_box = TrexBox::decode(rawbox.payload())?;
                        trexs.push(trex_box);
                    }
                    _ => {}
                }
            }

            Ok(MvexBox { mehd, trexs })
        }
    }

    impl BoxCodec for MvexBox {
        fn boxtype(&self) -> BoxType {
            BoxType::MVEX
        }
    }

    impl BoxDecode<'_> for MvexBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = MvexBoxView::decode(bytes)?;
            MvexBox::try_from(&view)
        }
    }

    impl BoxEncode for MvexBox {
        #[inline]
        fn encoded_len(&self) -> usize {
            let mut len = 0;
            if let Some(mehd) = &self.mehd {
                len += boxed_len(mehd);
            }
            for trex in &self.trexs {
                len += boxed_len(trex);
            }
            len
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            if let Some(mehd) = &self.mehd {
                write_box_in(&mut cur, mehd)?;
            }
            for trex in &self.trexs {
                write_box_in(&mut cur, trex)?;
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

    fn raw_data_with_trex() -> [u8; 32] {
        [
            // trex box
            0x00, 0x00, 0x00, 0x20, // size = 32
            b't', b'r', b'e', b'x', // type = "trex"
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x01, // track_id = 1
            0x00, 0x00, 0x00, 0x01, // default_sample_description_index = 1
            0x00, 0x00, 0x03, 0xE8, // default_sample_duration = 1000
            0x00, 0x00, 0x00, 0x00, // default_sample_size = 0
            0x00, 0x00, 0x00, 0x00, // default_sample_flags = 0
        ]
    }

    fn raw_data_with_mehd_and_trex() -> [u8; 48] {
        [
            // mehd box (version 0)
            0x00, 0x00, 0x00, 0x10, // size = 16
            b'm', b'e', b'h', b'd', // type = "mehd"
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x10, 0x00, // fragment_duration = 4096
            // trex box
            0x00, 0x00, 0x00, 0x20, // size = 32
            b't', b'r', b'e', b'x', // type = "trex"
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x01, // track_id = 1
            0x00, 0x00, 0x00, 0x01, // default_sample_description_index = 1
            0x00, 0x00, 0x03, 0xE8, // default_sample_duration = 1000
            0x00, 0x00, 0x00, 0x00, // default_sample_size = 0
            0x00, 0x00, 0x00, 0x00, // default_sample_flags = 0
        ]
    }

    #[test]
    fn test_mvex_box_view_decode_empty() {
        let data = raw_data_empty();
        let mvex = MvexBoxView::decode(&data).unwrap();

        assert_eq!(mvex.boxes().count(), 0);
        assert!(mvex.mehd().unwrap().is_none());
        assert_eq!(mvex.trexs().count(), 0);
    }

    #[test]
    fn test_mvex_box_view_with_trex() {
        let data = raw_data_with_trex();
        let mvex = MvexBoxView::decode(&data).unwrap();

        assert!(mvex.mehd().unwrap().is_none());

        let trexs: Vec<_> = mvex.trexs().collect();
        assert_eq!(trexs.len(), 1);

        let trex = trexs[0].as_ref().unwrap();
        assert_eq!(trex.track_id, 1);
    }

    #[test]
    fn test_mvex_box_view_with_mehd_and_trex() {
        let data = raw_data_with_mehd_and_trex();
        let mvex = MvexBoxView::decode(&data).unwrap();

        let mehd = mvex.mehd().unwrap();
        assert!(mehd.is_some());
        assert_eq!(mehd.unwrap().fragment_duration, 4096);

        let trexs: Vec<_> = mvex.trexs().collect();
        assert_eq!(trexs.len(), 1);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_mvex_box_try_from() {
        let data = raw_data_with_mehd_and_trex();
        let view = MvexBoxView::decode(&data).unwrap();
        let owned = MvexBox::try_from(&view).unwrap();

        assert!(owned.mehd.is_some());
        assert_eq!(owned.trexs.len(), 1);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_mvex_box_round_trip() {
        use crate::BoxEncode;

        let original = raw_data_with_mehd_and_trex();
        let mvex = MvexBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; mvex.encoded_len()];
        let len = mvex.encode_into(&mut encoded).unwrap();

        assert_eq!(len, original.len());
        assert_eq!(&encoded[..], &original[..]);
    }
}
