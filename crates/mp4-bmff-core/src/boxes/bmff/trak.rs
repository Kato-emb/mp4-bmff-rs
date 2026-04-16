//! Track Box (`trak`) implementation.
//!
//! The Track Box contains all information for a single track of the presentation.
//! Each track is independent and represents a timed sequence of media data
//! (video frames, audio samples, etc.). A movie typically has multiple tracks
//! (e.g., one video track and one or more audio tracks).

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::BoxIter;

use super::EdtsBoxView;
use super::MdiaBoxView;
use super::TkhdBox;
use super::TrefBoxView;
use super::TrgrBoxView;

/// A reference to a Track Box (`trak`).
///
/// The Track Box is a container for a single track of the presentation.
/// Each track is independent and has its own timeline and media data.
///
/// # Structure
///
/// - `tkhd`: Track Header Box (required) - track-level metadata.
/// - `mdia`: Media Box (required) - contains media-specific information.
/// - `edts`: Edit Box (optional) - maps timeline to media time.
/// - `tref`: Track Reference Box (optional) - references to other tracks.
/// - `trgr`: Track Group Box (optional) - track grouping information.
#[derive(Debug)]
pub struct TrakBoxView<'a> {
    content: &'a [u8],
}

impl<'a> TrakBoxView<'a> {
    /// Returns an iterator over the child boxes of this `trak` box.
    pub fn boxes(&self) -> BoxIter<'a> {
        BoxIter::new(self.content)
    }

    /// Returns the Track Header Box (`tkhd`) contained in this `trak` box.
    pub fn tkhd(&self) -> Result<TkhdBox> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::TKHD {
                let tkhd = TkhdBox::decode(b.payload())?;
                return Ok(tkhd);
            }
        }

        Err(Error::in_box(
            ErrorKind::BoxMissing {
                required: BoxType::TKHD,
            },
            BoxType::TRAK,
        ))
    }

    /// Returns the Track Reference Box (`tref`) contained in this `trak` box, if any.
    pub fn tref(&self) -> Result<Option<TrefBoxView<'a>>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::TREF {
                let tref = TrefBoxView::decode(b.into_payload())?;
                return Ok(Some(tref));
            }
        }

        Ok(None)
    }

    /// Returns the Track Group Box (`trgr`) contained in this `trak` box, if any.
    pub fn trgr(&self) -> Result<Option<TrgrBoxView<'a>>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::TRGR {
                let trgr = TrgrBoxView::decode(b.into_payload())?;
                return Ok(Some(trgr));
            }
        }

        Ok(None)
    }

    /// Returns the Media Box (`mdia`) contained in this `trak` box.
    pub fn mdia(&self) -> Result<MdiaBoxView<'a>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::MDIA {
                let mdia = MdiaBoxView::decode(b.into_payload())?;
                return Ok(mdia);
            }
        }

        Err(Error::in_box(
            ErrorKind::BoxMissing {
                required: BoxType::MDIA,
            },
            BoxType::TRAK,
        ))
    }

    /// Returns the Edit Box (`edts`) contained in this `trak` box, if any.
    pub fn edts(&self) -> Result<Option<EdtsBoxView<'a>>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::EDTS {
                let edts = EdtsBoxView::decode(b.into_payload())?;
                return Ok(Some(edts));
            }
        }

        Ok(None)
    }
}

impl BoxCodec for TrakBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::TRAK
    }
}

impl<'de> BoxDecode<'de> for TrakBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        Ok(TrakBoxView { content: bytes })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use super::*;
    use crate::BoxEncode;

    use crate::codec::boxed_len;
    use crate::codec::write_box_in;
    use crate::cursor::WriteCursor;

    use crate::boxes::bmff::EdtsBox;
    use crate::boxes::bmff::MdiaBox;
    use crate::boxes::bmff::TrefBox;
    use crate::boxes::bmff::TrgrBox;

    /// An owned Track Box (`trak`).
    ///
    /// This is the owned variant of [`TrakBoxView`] that stores child boxes
    /// in heap-allocated structures.
    ///
    /// # Structure
    ///
    /// - `tkhd`: Track Header Box (required) - track-level metadata.
    /// - `mdia`: Media Box (required) - media handler and sample information.
    /// - `edts`: Edit Box (optional) - timeline-to-media-time mapping.
    /// - `tref`: Track Reference Box (optional) - inter-track references.
    /// - `trgr`: Track Group Box (optional) - track grouping.
    #[derive(Debug, Clone)]
    pub struct TrakBox {
        /// Track Header Box - contains track-level metadata (ID, duration, dimensions).
        pub tkhd: TkhdBox,
        /// Track Reference Box - references to related tracks (e.g., hint tracks).
        pub tref: Option<TrefBox>,
        /// Track Group Box - groups tracks with similar characteristics.
        pub trgr: Option<TrgrBox>,
        /// Media Box - contains media handler and sample table information.
        pub mdia: MdiaBox,
        /// Edit Box - defines how to map the track timeline to media samples.
        pub edts: Option<EdtsBox>,
    }

    impl TryFrom<&TrakBoxView<'_>> for TrakBox {
        type Error = Error;

        fn try_from(view: &TrakBoxView<'_>) -> Result<Self> {
            let mut tkhd = None;
            let mut tref = None;
            let mut trgr = None;
            let mut mdia = None;
            let mut edts = None;

            for result in view.boxes() {
                let rawbox = result?;

                match rawbox.boxtype() {
                    BoxType::TKHD => {
                        if tkhd.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::TKHD,
                                },
                                BoxType::TRAK,
                            ));
                        }
                        let tkhd_box = TkhdBox::decode(rawbox.payload())?;
                        tkhd = Some(tkhd_box);
                    }
                    BoxType::TREF => {
                        if tref.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::TREF,
                                },
                                BoxType::TRAK,
                            ));
                        }
                        let tref_box = TrefBox::try_from(&TrefBoxView::decode(rawbox.payload())?)?;
                        tref = Some(tref_box);
                    }
                    BoxType::TRGR => {
                        if trgr.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::TRGR,
                                },
                                BoxType::TRAK,
                            ));
                        }
                        let trgr_box = TrgrBox::try_from(&TrgrBoxView::decode(rawbox.payload())?)?;
                        trgr = Some(trgr_box);
                    }
                    BoxType::MDIA => {
                        if mdia.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::MDIA,
                                },
                                BoxType::TRAK,
                            ));
                        }
                        let mdia_box = MdiaBox::try_from(&MdiaBoxView::decode(rawbox.payload())?)?;
                        mdia = Some(mdia_box);
                    }
                    BoxType::EDTS => {
                        if edts.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::EDTS,
                                },
                                BoxType::TRAK,
                            ));
                        }
                        let edts_box = EdtsBox::try_from(&EdtsBoxView::decode(rawbox.payload())?)?;
                        edts = Some(edts_box);
                    }
                    _ => continue,
                }
            }

            let tkhd = tkhd.ok_or_else(|| {
                Error::in_box(
                    ErrorKind::BoxMissing {
                        required: BoxType::TKHD,
                    },
                    BoxType::TRAK,
                )
            })?;

            let mdia = mdia.ok_or_else(|| {
                Error::in_box(
                    ErrorKind::BoxMissing {
                        required: BoxType::MDIA,
                    },
                    BoxType::TRAK,
                )
            })?;

            Ok(TrakBox {
                tkhd,
                tref,
                trgr,
                mdia,
                edts,
            })
        }
    }

    impl BoxCodec for TrakBox {
        fn boxtype(&self) -> BoxType {
            BoxType::TRAK
        }
    }

    impl BoxDecode<'_> for TrakBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = TrakBoxView::decode(bytes)?;
            TrakBox::try_from(&view)
        }
    }

    impl BoxEncode for TrakBox {
        #[inline]
        fn encoded_len(&self) -> usize {
            let mut len = boxed_len(&self.tkhd);
            if let Some(tref) = &self.tref {
                len += boxed_len(tref);
            }
            if let Some(trgr) = &self.trgr {
                len += boxed_len(trgr);
            }
            len += boxed_len(&self.mdia);
            if let Some(edts) = &self.edts {
                len += boxed_len(edts);
            }
            len
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            write_box_in(&mut cur, &self.tkhd)?;

            if let Some(tref) = &self.tref {
                write_box_in(&mut cur, tref)?;
            }

            if let Some(trgr) = &self.trgr {
                write_box_in(&mut cur, trgr)?;
            }

            write_box_in(&mut cur, &self.mdia)?;

            if let Some(edts) = &self.edts {
                write_box_in(&mut cur, edts)?;
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

    fn raw_data_tkhd_only() -> [u8; 92] {
        [
            // tkhd box (version 0)
            0x00, 0x00, 0x00, 0x5C, // size = 92
            b't', b'k', b'h', b'd', // type = "tkhd"
            0x00, // version = 0
            0x00, 0x00, 0x03, // flags = enabled | in_movie
            0x00, 0x00, 0x00, 0x00, // creation_time
            0x00, 0x00, 0x00, 0x00, // modification_time
            0x00, 0x00, 0x00, 0x01, // track_id = 1
            0x00, 0x00, 0x00, 0x00, // reserved
            0x00, 0x00, 0x10, 0x00, // duration = 4096
            0x00, 0x00, 0x00, 0x00, // reserved[0]
            0x00, 0x00, 0x00, 0x00, // reserved[1]
            0x00, 0x00, // layer = 0
            0x00, 0x00, // alternate_group = 0
            0x01, 0x00, // volume = 1.0 (8.8 fixed point)
            0x00, 0x00, // reserved
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
            0x01, 0x40, 0x00, 0x00, // width = 320.0 (16.16 fixed point)
            0x00, 0xF0, 0x00, 0x00, // height = 240.0 (16.16 fixed point)
        ]
    }

    #[test]
    fn test_trak_box_view_decode_empty() {
        let data = raw_data_empty();
        let trak = TrakBoxView::decode(&data).unwrap();

        assert_eq!(trak.boxes().count(), 0);
    }

    #[test]
    fn test_trak_box_view_missing_required_tkhd() {
        let data = raw_data_empty();
        let trak = TrakBoxView::decode(&data).unwrap();

        let result = trak.tkhd();
        assert!(result.is_err());
    }

    #[test]
    fn test_trak_box_view_missing_required_mdia() {
        let data = raw_data_tkhd_only();
        let trak = TrakBoxView::decode(&data).unwrap();

        let result = trak.mdia();
        assert!(result.is_err());
    }

    #[test]
    fn test_trak_box_view_optional_boxes_not_present() {
        let data = raw_data_tkhd_only();
        let trak = TrakBoxView::decode(&data).unwrap();

        assert!(trak.tref().unwrap().is_none());
        assert!(trak.trgr().unwrap().is_none());
        assert!(trak.edts().unwrap().is_none());
    }

    #[test]
    fn test_trak_box_view_tkhd_present() {
        let data = raw_data_tkhd_only();
        let trak = TrakBoxView::decode(&data).unwrap();

        let tkhd = trak.tkhd().unwrap();
        assert_eq!(tkhd.track_id, 1);
    }
}
