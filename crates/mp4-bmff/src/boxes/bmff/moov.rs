//! Movie Box (`moov`) implementation.
//!
//! The Movie Box contains all the metadata needed to present the media.
//! It includes the Movie Header Box (`mvhd`) with global information,
//! and one or more Track Boxes (`trak`) containing per-track metadata.
//! For fragmented movies, it may also contain a Movie Extends Box (`mvex`).

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::BoxIter;

use super::MvexBoxView;
use super::MvhdBox;
use super::TrakBoxView;

/// A reference to a Movie Box (`moov`).
///
/// The Movie Box is a container for all the metadata describing the media
/// presentation. It is required in every ISO Base Media File and must appear
/// exactly once. The `moov` box can appear before or after the Media Data Box
/// (`mdat`); placing it before enables "fast start" streaming.
///
/// # Structure
///
/// - `mvhd`: Movie Header Box (required, exactly one) - global presentation info.
/// - `trak`: Track Box (required, one or more) - per-track metadata.
/// - `mvex`: Movie Extends Box (optional) - present when file uses movie fragments.
#[derive(Debug)]
pub struct MoovBoxView<'a> {
    content: &'a [u8],
}

impl<'a> MoovBoxView<'a> {
    /// Returns an iterator over the child boxes of this `moov` box.
    pub fn boxes(&self) -> BoxIter<'a> {
        BoxIter::new(self.content)
    }

    /// Returns the Movie Header Box (`mvhd`) contained in this `moov` box.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is malformed or too short.
    pub fn mvhd(&self) -> Result<MvhdBox> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::MVHD {
                let mvhd = MvhdBox::decode(b.payload())?;
                return Ok(mvhd);
            }
        }

        Err(Error::in_box(
            ErrorKind::BoxMissing {
                required: BoxType::MVHD,
            },
            BoxType::MOOV,
        ))
    }

    /// Returns an iterator over the Track Boxes (`trak`) contained in this `moov` box.
    pub fn traks(&self) -> impl Iterator<Item = Result<TrakBoxView<'a>>> + 'a {
        self.boxes().filter_map(|result| match result {
            Ok(rawbox) if rawbox.boxtype() == BoxType::TRAK => {
                Some(TrakBoxView::decode(rawbox.into_payload()))
            }
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
    }

    /// Returns the Movie Extends Box (`mvex`) contained in this `moov` box if present.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is malformed or too short.
    pub fn mvex(&self) -> Result<Option<MvexBoxView<'a>>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::MVEX {
                let mvex = MvexBoxView::decode(b.into_payload())?;
                return Ok(Some(mvex));
            }
        }

        Ok(None)
    }
}

impl BoxCodec for MoovBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::MOOV
    }
}

impl<'de> BoxDecode<'de> for MoovBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        Ok(MoovBoxView { content: bytes })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use super::*;
    use crate::BoxEncode;

    use crate::codec::boxed_len;
    use crate::codec::write_box_in;
    use crate::cursor::WriteCursor;

    use super::super::MvexBox;
    use super::super::TrakBox;

    /// An owned Movie Box (`moov`).
    ///
    /// This is the owned variant of [`MoovBoxView`] that stores child boxes
    /// in heap-allocated structures.
    ///
    /// # Example
    ///
    /// ```
    /// use mp4_bmff::boxes::bmff::{MoovBox, MvhdBox, TrakBox};
    /// use mp4_bmff::BoxDecode;
    ///
    /// // The moov box contains mvhd (movie header) and trak (track) boxes
    /// // In practice, you would decode from actual file data
    /// ```
    #[derive(Debug, Clone)]
    pub struct MoovBox {
        /// Movie Header Box - contains global presentation information.
        pub mvhd: MvhdBox,
        /// Track Boxes - one per media track (audio, video, etc.).
        pub traks: Vec<TrakBox>,
        /// Movie Extends Box - present when the file uses movie fragments.
        pub mvex: Option<MvexBox>,
    }

    impl TryFrom<&MoovBoxView<'_>> for MoovBox {
        type Error = Error;

        fn try_from(view: &MoovBoxView<'_>) -> Result<Self> {
            let mut mvhd = None;
            let mut traks = Vec::new();
            let mut mvex = None;

            for result in view.boxes() {
                let rawbox = result?;
                match rawbox.boxtype() {
                    BoxType::MVHD => {
                        if mvhd.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::MVHD,
                                },
                                BoxType::MOOV,
                            ));
                        }
                        let mvhd_box = MvhdBox::decode(rawbox.payload())?;
                        mvhd = Some(mvhd_box);
                    }
                    BoxType::TRAK => {
                        let trak_box = TrakBox::decode(rawbox.payload())?;
                        traks.push(trak_box);
                    }
                    BoxType::MVEX => {
                        if mvex.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::MVEX,
                                },
                                BoxType::MOOV,
                            ));
                        }
                        let mvex_box = MvexBox::decode(rawbox.payload())?;
                        mvex = Some(mvex_box);
                    }
                    _ => continue,
                }
            }

            let mvhd = mvhd.ok_or_else(|| {
                Error::in_box(
                    ErrorKind::BoxMissing {
                        required: BoxType::MVHD,
                    },
                    BoxType::MOOV,
                )
            })?;

            Ok(MoovBox { mvhd, traks, mvex })
        }
    }

    impl BoxCodec for MoovBox {
        fn boxtype(&self) -> BoxType {
            BoxType::MOOV
        }
    }

    impl BoxDecode<'_> for MoovBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = MoovBoxView::decode(bytes)?;
            MoovBox::try_from(&view)
        }
    }

    impl BoxEncode for MoovBox {
        #[inline]
        fn encoded_len(&self) -> usize {
            let mut len = boxed_len(&self.mvhd);
            for trak in &self.traks {
                len += boxed_len(trak);
            }
            if let Some(mvex) = &self.mvex {
                len += boxed_len(mvex);
            }
            len
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            write_box_in(&mut cur, &self.mvhd)?;

            for trak in &self.traks {
                write_box_in(&mut cur, trak)?;
            }

            if let Some(mvex) = &self.mvex {
                write_box_in(&mut cur, mvex)?;
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

    fn raw_data_mvhd_only() -> [u8; 108] {
        [
            // mvhd box (version 0)
            0x00, 0x00, 0x00, 0x6C, // size = 108
            b'm', b'v', b'h', b'd', // type = "mvhd"
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x00, // creation_time
            0x00, 0x00, 0x00, 0x00, // modification_time
            0x00, 0x00, 0x03, 0xE8, // timescale = 1000
            0x00, 0x00, 0x10, 0x00, // duration = 4096
            0x00, 0x01, 0x00, 0x00, // rate = 1.0 (16.16 fixed point)
            0x01, 0x00, // volume = 1.0 (8.8 fixed point)
            0x00, 0x00, // reserved
            0x00, 0x00, 0x00, 0x00, // reserved[0]
            0x00, 0x00, 0x00, 0x00, // reserved[1]
            // matrix (9 * 4 = 36 bytes)
            0x00, 0x01, 0x00, 0x00, // matrix[0] = 1.0
            0x00, 0x00, 0x00, 0x00, // matrix[1]
            0x00, 0x00, 0x00, 0x00, // matrix[2]
            0x00, 0x00, 0x00, 0x00, // matrix[3]
            0x00, 0x01, 0x00, 0x00, // matrix[4] = 1.0
            0x00, 0x00, 0x00, 0x00, // matrix[5]
            0x00, 0x00, 0x00, 0x00, // matrix[6]
            0x00, 0x00, 0x00, 0x00, // matrix[7]
            0x40, 0x00, 0x00, 0x00, // matrix[8] = 1.0 (16.16)
            // pre_defined (6 * 4 = 24 bytes)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x02, // next_track_id = 2
        ]
    }

    #[test]
    fn test_moov_box_view_decode_empty() {
        let data = raw_data_empty();
        let moov = MoovBoxView::decode(&data).unwrap();

        assert_eq!(moov.boxes().count(), 0);
    }

    #[test]
    fn test_moov_box_view_missing_required_mvhd() {
        let data = raw_data_empty();
        let moov = MoovBoxView::decode(&data).unwrap();

        let result = moov.mvhd();
        assert!(result.is_err());
    }

    #[test]
    fn test_moov_box_view_mvhd_present() {
        let data = raw_data_mvhd_only();
        let moov = MoovBoxView::decode(&data).unwrap();

        let mvhd = moov.mvhd().unwrap();
        assert_eq!(mvhd.timescale, 1000);
        assert_eq!(mvhd.next_track_id, 2);
    }

    #[test]
    fn test_moov_box_view_no_traks() {
        let data = raw_data_mvhd_only();
        let moov = MoovBoxView::decode(&data).unwrap();

        assert_eq!(moov.traks().count(), 0);
    }

    #[test]
    fn test_moov_box_view_mvex_not_present() {
        let data = raw_data_mvhd_only();
        let moov = MoovBoxView::decode(&data).unwrap();

        assert!(moov.mvex().unwrap().is_none());
    }
}
