//! Media Box (`mdia`) implementation.
//!
//! The Media Box contains all objects that define information about the media
//! data within a track. The Media Box is required within every Track Box (`trak`)
//! and provides the complete description of the media format and timing.

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::BoxIter;

use super::ElngBoxView;
use super::HdlrBoxView;
use super::MdhdBox;
use super::MinfBoxView;

/// A reference to a Media Box (`mdia`).
///
/// The Media Box declares the overall information about the media data within
/// a track. It contains the handler that identifies the media type, the media
/// header with timing information, and the media information container.
///
/// # Structure
///
/// - `mdhd`: Media Header Box (required) - timescale, duration, and language.
/// - `hdlr`: Handler Reference Box (required) - identifies the media handler.
/// - `minf`: Media Information Box (required) - contains media-specific data.
/// - `elng`: Extended Language Tag Box (optional) - RFC 4646 language tag.
#[derive(Debug)]
pub struct MdiaBoxView<'a> {
    content: &'a [u8],
}

impl<'a> MdiaBoxView<'a> {
    /// Returns an iterator over the child boxes of this `mdia` box.
    pub fn boxes(&self) -> BoxIter<'a> {
        BoxIter::new(self.content)
    }

    /// Returns the Media Header Box (`mdhd`) contained in this `mdia` box.
    pub fn mdhd(&self) -> Result<MdhdBox> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::MDHD {
                let mdhd = MdhdBox::decode(b.payload())?;
                return Ok(mdhd);
            }
        }

        Err(Error::in_box(
            ErrorKind::BoxMissing {
                required: BoxType::MDHD,
            },
            BoxType::MDIA,
        ))
    }

    /// Returns the Handler Reference Box (`hdlr`) contained in this `mdia` box.
    pub fn hdlr(&self) -> Result<HdlrBoxView<'a>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::HDLR {
                let hdlr = HdlrBoxView::decode(b.into_payload())?;
                return Ok(hdlr);
            }
        }

        Err(Error::in_box(
            ErrorKind::BoxMissing {
                required: BoxType::HDLR,
            },
            BoxType::MDIA,
        ))
    }

    /// Returns the Media Information Box (`minf`) contained in this `mdia` box.
    pub fn minf(&self) -> Result<MinfBoxView<'a>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::MINF {
                let minf = MinfBoxView::decode(b.into_payload())?;
                return Ok(minf);
            }
        }

        Err(Error::in_box(
            ErrorKind::BoxMissing {
                required: BoxType::MINF,
            },
            BoxType::MDIA,
        ))
    }

    /// Returns the Extended language tag Box (`elng`) contained in this `mdia` box, if any.
    pub fn elng(&self) -> Result<Option<ElngBoxView<'a>>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::ELNG {
                let elng = ElngBoxView::decode(b.into_payload())?;
                return Ok(Some(elng));
            }
        }

        Ok(None)
    }
}

impl BoxCodec for MdiaBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::MDIA
    }
}

impl<'de> BoxDecode<'de> for MdiaBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        Ok(MdiaBoxView { content: bytes })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use super::*;
    use crate::BoxEncode;

    use crate::codec::boxed_len;
    use crate::codec::write_box_in;
    use crate::cursor::WriteCursor;

    use crate::boxes::bmff::ElngBox;
    use crate::boxes::bmff::HdlrBox;
    use crate::boxes::bmff::MinfBox;

    /// An owned Media Box (`mdia`).
    ///
    /// This is the owned variant of [`MdiaBoxView`] that stores child boxes
    /// in heap-allocated structures.
    ///
    /// # Structure
    ///
    /// - `mdhd`: Media Header Box - media timescale, duration, and language.
    /// - `hdlr`: Handler Reference Box - identifies the media type (video/audio/etc.).
    /// - `minf`: Media Information Box - contains sample description and sample table.
    /// - `elng`: Extended Language Tag Box (optional) - RFC 4646 language tag.
    #[derive(Debug, Clone)]
    pub struct MdiaBox {
        /// Media Header Box with timescale and duration specific to this media.
        pub mdhd: MdhdBox,
        /// Handler Reference Box identifying the media type (e.g., "vide", "soun").
        pub hdlr: HdlrBox,
        /// Media Information Box containing sample table and data location info.
        pub minf: MinfBox,
        /// Extended Language Tag Box for RFC 4646 language tags, if present.
        pub elng: Option<ElngBox>,
    }

    impl TryFrom<&MdiaBoxView<'_>> for MdiaBox {
        type Error = Error;

        fn try_from(view: &MdiaBoxView<'_>) -> Result<Self> {
            let mut mdhd = None;
            let mut hdlr = None;
            let mut minf = None;
            let mut elng = None;

            for result in view.boxes() {
                let rawbox = result?;

                match rawbox.boxtype() {
                    BoxType::MDHD => {
                        if mdhd.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::MDHD,
                                },
                                BoxType::MDIA,
                            ));
                        }
                        let mdhd_box = MdhdBox::decode(rawbox.payload())?;
                        mdhd = Some(mdhd_box);
                    }
                    BoxType::HDLR => {
                        if hdlr.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::HDLR,
                                },
                                BoxType::MDIA,
                            ));
                        }
                        let hdlr_box = HdlrBox::decode(rawbox.payload())?;
                        hdlr = Some(hdlr_box);
                    }
                    BoxType::MINF => {
                        if minf.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::MINF,
                                },
                                BoxType::MDIA,
                            ));
                        }
                        let minf_box = MinfBox::decode(rawbox.payload())?;
                        minf = Some(minf_box);
                    }
                    BoxType::ELNG => {
                        if elng.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::ELNG,
                                },
                                BoxType::MDIA,
                            ));
                        }
                        let elng_box = ElngBox::decode(rawbox.payload())?;
                        elng = Some(elng_box);
                    }
                    _ => continue,
                }
            }

            let mdhd = mdhd.ok_or_else(|| {
                Error::in_box(
                    ErrorKind::BoxMissing {
                        required: BoxType::MDHD,
                    },
                    BoxType::MDIA,
                )
            })?;

            let hdlr = hdlr.ok_or_else(|| {
                Error::in_box(
                    ErrorKind::BoxMissing {
                        required: BoxType::HDLR,
                    },
                    BoxType::MDIA,
                )
            })?;

            let minf = minf.ok_or_else(|| {
                Error::in_box(
                    ErrorKind::BoxMissing {
                        required: BoxType::MINF,
                    },
                    BoxType::MDIA,
                )
            })?;

            Ok(MdiaBox {
                mdhd,
                hdlr,
                minf,
                elng,
            })
        }
    }

    impl BoxCodec for MdiaBox {
        fn boxtype(&self) -> BoxType {
            BoxType::MDIA
        }
    }

    impl BoxDecode<'_> for MdiaBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = MdiaBoxView::decode(bytes)?;
            MdiaBox::try_from(&view)
        }
    }

    impl BoxEncode for MdiaBox {
        fn encoded_len(&self) -> usize {
            let mut len = 0;
            len += boxed_len(&self.mdhd);
            len += boxed_len(&self.hdlr);
            len += boxed_len(&self.minf);
            if let Some(elng) = &self.elng {
                len += boxed_len(elng);
            }
            len
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            write_box_in(&mut cur, &self.mdhd)?;
            write_box_in(&mut cur, &self.hdlr)?;
            write_box_in(&mut cur, &self.minf)?;

            if let Some(elng) = &self.elng {
                write_box_in(&mut cur, elng)?;
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

    fn raw_data_mdhd_only() -> [u8; 32] {
        [
            // mdhd box (version 0)
            0x00, 0x00, 0x00, 0x20, // size = 32
            b'm', b'd', b'h', b'd', // type = "mdhd"
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x00, // creation_time
            0x00, 0x00, 0x00, 0x00, // modification_time
            0x00, 0x00, 0xAC, 0x44, // timescale = 44100
            0x00, 0x00, 0x10, 0x00, // duration = 4096
            0x55, 0xC4, // language = "und"
            0x00, 0x00, // pre_defined
        ]
    }

    #[test]
    fn test_mdia_box_view_decode_empty() {
        let data = raw_data_empty();
        let mdia = MdiaBoxView::decode(&data).unwrap();

        assert_eq!(mdia.boxes().count(), 0);
    }

    #[test]
    fn test_mdia_box_view_missing_required_mdhd() {
        let data = raw_data_empty();
        let mdia = MdiaBoxView::decode(&data).unwrap();

        let result = mdia.mdhd();
        assert!(result.is_err());
    }

    #[test]
    fn test_mdia_box_view_missing_required_hdlr() {
        let data = raw_data_mdhd_only();
        let mdia = MdiaBoxView::decode(&data).unwrap();

        let result = mdia.hdlr();
        assert!(result.is_err());
    }

    #[test]
    fn test_mdia_box_view_missing_required_minf() {
        let data = raw_data_mdhd_only();
        let mdia = MdiaBoxView::decode(&data).unwrap();

        let result = mdia.minf();
        assert!(result.is_err());
    }

    #[test]
    fn test_mdia_box_view_elng_not_present() {
        let data = raw_data_mdhd_only();
        let mdia = MdiaBoxView::decode(&data).unwrap();

        assert!(mdia.elng().unwrap().is_none());
    }
}
