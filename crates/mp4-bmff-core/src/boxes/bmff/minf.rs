//! Media Information Box (`minf`) implementation.
//!
//! The Media Information Box contains all objects necessary for defining the
//! characteristics of media in a track, and for maintaining media-specific
//! information. It includes a media header specific to the media type, data
//! information, and the sample table.

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::BoxIter;

use super::DinfBoxView;
use super::NmhdBox;
use super::SmhdBox;
use super::StblBoxView;
use super::VmhdBox;

/// A reference to a Media Information Box (`minf`).
///
/// The Media Information Box is a container for all information required to
/// access and decode media samples within a track. It contains a media-specific
/// header box, the sample table, and data location information.
///
/// # Structure
///
/// - Media header (one of the following is required):
///   - `vmhd`: Video Media Header Box - for video tracks.
///   - `smhd`: Sound Media Header Box - for audio tracks.
///   - `nmhd`: Null Media Header Box - for streams with no media-specific header.
/// - `dinf`: Data Information Box (required) - declares location of media data.
/// - `stbl`: Sample Table Box (required) - sample timing and location info.
#[derive(Debug)]
pub struct MinfBoxView<'a> {
    content: &'a [u8],
}

impl<'a> MinfBoxView<'a> {
    /// Returns an iterator over the child boxes of this `minf` box.
    pub fn boxes(&self) -> BoxIter<'a> {
        BoxIter::new(self.content)
    }

    /// Returns the Null Media Header Box (`nmhd`) contained in this `minf` box.
    pub fn nmhd(&self) -> Result<Option<NmhdBox>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::NMHD {
                let nmhd = NmhdBox::decode(b.payload())?;
                return Ok(Some(nmhd));
            }
        }

        Ok(None)
    }

    /// Returns the Sample Table Box (`stbl`) contained in this `minf` box.
    pub fn stbl(&self) -> Result<StblBoxView<'a>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::STBL {
                let stbl = StblBoxView::decode(b.into_payload())?;
                return Ok(stbl);
            }
        }

        Err(Error::in_box(
            ErrorKind::BoxMissing {
                required: BoxType::STBL,
            },
            BoxType::MINF,
        ))
    }

    /// Returns the Data Information Box (`dinf`) contained in this `minf` box.
    pub fn dinf(&self) -> Result<DinfBoxView<'a>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::DINF {
                let dinf = DinfBoxView::decode(b.into_payload())?;
                return Ok(dinf);
            }
        }

        Err(Error::in_box(
            ErrorKind::BoxMissing {
                required: BoxType::DINF,
            },
            BoxType::MINF,
        ))
    }

    /// Returns the Video Media Header Box (`vmhd`) contained in this `minf` box, if any.
    pub fn vmhd(&self) -> Result<Option<VmhdBox>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::VMHD {
                let vmhd = VmhdBox::decode(b.payload())?;
                return Ok(Some(vmhd));
            }
        }

        Ok(None)
    }

    /// Returns the Sound Media Header Box (`smhd`) contained in this `minf` box, if any.
    pub fn smhd(&self) -> Result<Option<SmhdBox>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::SMHD {
                let smhd = SmhdBox::decode(b.payload())?;
                return Ok(Some(smhd));
            }
        }

        Ok(None)
    }
}

impl BoxCodec for MinfBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::MINF
    }
}

impl<'a> BoxDecode<'a> for MinfBoxView<'a> {
    fn decode(bytes: &'a [u8]) -> Result<Self> {
        Ok(MinfBoxView { content: bytes })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use super::*;
    use crate::BoxEncode;

    use crate::codec::boxed_len;
    use crate::codec::write_box_in;
    use crate::cursor::WriteCursor;

    use crate::boxes::bmff::DinfBox;
    use crate::boxes::bmff::StblBox;

    /// An owned Media Header Box.
    ///
    /// The media header varies depending on the type of media in the track.
    /// Exactly one media header box must be present in the Media Information Box.
    #[derive(Debug, Clone)]
    pub enum MediaHeaderBox {
        /// Null Media Header Box (`nmhd`) - for generic or non-standard media.
        Nmhd(NmhdBox),
        /// Video Media Header Box (`vmhd`) - for video tracks.
        Vmhd(VmhdBox),
        /// Sound Media Header Box (`smhd`) - for audio tracks.
        Smhd(SmhdBox),
        // Hmhd - Hint Media Header Box (not yet implemented)
    }

    /// An owned Media Information Box (`minf`).
    ///
    /// This is the owned variant of [`MinfBoxView`] that stores child boxes
    /// in heap-allocated structures.
    ///
    /// # Structure
    ///
    /// - `media_header`: Type-specific media header (vmhd/smhd/nmhd).
    /// - `stbl`: Sample Table Box with timing and location information.
    /// - `dinf`: Data Information Box declaring where media data is stored.
    #[derive(Debug, Clone)]
    pub struct MinfBox {
        /// Media-type-specific header (video, audio, or null).
        pub media_header: MediaHeaderBox,
        /// Sample Table Box containing sample timing and location info.
        pub stbl: StblBox,
        /// Data Information Box declaring media data location.
        pub dinf: DinfBox,
    }

    impl TryFrom<&MinfBoxView<'_>> for MinfBox {
        type Error = Error;

        fn try_from(view: &MinfBoxView<'_>) -> Result<Self> {
            let mut media_header = None;
            let mut stbl = None;
            let mut dinf = None;

            for result in view.boxes() {
                let rawbox = result?;

                match rawbox.boxtype() {
                    BoxType::NMHD => {
                        if media_header.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::NMHD,
                                },
                                BoxType::MINF,
                            ));
                        }
                        let nmhd_box = NmhdBox::decode(rawbox.payload())?;
                        media_header = Some(MediaHeaderBox::Nmhd(nmhd_box));
                    }
                    BoxType::VMHD => {
                        if media_header.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::VMHD,
                                },
                                BoxType::MINF,
                            ));
                        }
                        let vmhd_box = VmhdBox::decode(rawbox.payload())?;
                        media_header = Some(MediaHeaderBox::Vmhd(vmhd_box));
                    }
                    BoxType::SMHD => {
                        if media_header.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::SMHD,
                                },
                                BoxType::MINF,
                            ));
                        }
                        let smhd_box = SmhdBox::decode(rawbox.payload())?;
                        media_header = Some(MediaHeaderBox::Smhd(smhd_box));
                    }
                    BoxType::STBL => {
                        if stbl.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::STBL,
                                },
                                BoxType::MINF,
                            ));
                        }
                        let stbl_box = StblBox::decode(rawbox.payload())?;
                        stbl = Some(stbl_box);
                    }
                    BoxType::DINF => {
                        if dinf.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::DINF,
                                },
                                BoxType::MINF,
                            ));
                        }
                        let dinf_box = DinfBox::decode(rawbox.payload())?;
                        dinf = Some(dinf_box);
                    }
                    _ => continue,
                }
            }

            let media_header = media_header.ok_or_else(|| {
                Error::in_box(
                    ErrorKind::BoxMissing {
                        required: BoxType::NMHD,
                    },
                    BoxType::MINF,
                )
            })?;

            let stbl = stbl.ok_or_else(|| {
                Error::in_box(
                    ErrorKind::BoxMissing {
                        required: BoxType::STBL,
                    },
                    BoxType::MINF,
                )
            })?;

            let dinf = dinf.ok_or_else(|| {
                Error::in_box(
                    ErrorKind::BoxMissing {
                        required: BoxType::DINF,
                    },
                    BoxType::MINF,
                )
            })?;

            Ok(MinfBox {
                media_header,
                stbl,
                dinf,
            })
        }
    }

    impl BoxCodec for MinfBox {
        fn boxtype(&self) -> BoxType {
            BoxType::MINF
        }
    }

    impl BoxDecode<'_> for MinfBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = MinfBoxView::decode(bytes)?;
            MinfBox::try_from(&view)
        }
    }

    impl BoxEncode for MinfBox {
        #[inline]
        fn encoded_len(&self) -> usize {
            (match &self.media_header {
                MediaHeaderBox::Nmhd(nmhd) => boxed_len(nmhd),
                MediaHeaderBox::Vmhd(vmhd) => boxed_len(vmhd),
                MediaHeaderBox::Smhd(smhd) => boxed_len(smhd),
            }) + boxed_len(&self.stbl)
                + boxed_len(&self.dinf)
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            match &self.media_header {
                MediaHeaderBox::Nmhd(nmhd) => {
                    write_box_in(&mut cur, nmhd)?;
                }
                MediaHeaderBox::Vmhd(vmhd) => {
                    write_box_in(&mut cur, vmhd)?;
                }
                MediaHeaderBox::Smhd(smhd) => {
                    write_box_in(&mut cur, smhd)?;
                }
            }

            write_box_in(&mut cur, &self.stbl)?;
            write_box_in(&mut cur, &self.dinf)?;

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

    fn raw_data_vmhd_only() -> [u8; 20] {
        [
            // vmhd box
            0x00, 0x00, 0x00, 0x14, // size = 20
            b'v', b'm', b'h', b'd', // type = "vmhd"
            0x00, // version = 0
            0x00, 0x00, 0x01, // flags = 1
            0x00, 0x00, // graphics_mode = 0
            0x00, 0x00, // opcolor[0]
            0x00, 0x00, // opcolor[1]
            0x00, 0x00, // opcolor[2]
        ]
    }

    #[test]
    fn test_minf_box_view_decode_empty() {
        let data = raw_data_empty();
        let minf = MinfBoxView::decode(&data).unwrap();

        assert_eq!(minf.boxes().count(), 0);
    }

    #[test]
    fn test_minf_box_view_missing_required_stbl() {
        let data = raw_data_vmhd_only();
        let minf = MinfBoxView::decode(&data).unwrap();

        let result = minf.stbl();
        assert!(result.is_err());
    }

    #[test]
    fn test_minf_box_view_missing_required_dinf() {
        let data = raw_data_vmhd_only();
        let minf = MinfBoxView::decode(&data).unwrap();

        let result = minf.dinf();
        assert!(result.is_err());
    }

    #[test]
    fn test_minf_box_view_vmhd_present() {
        let data = raw_data_vmhd_only();
        let minf = MinfBoxView::decode(&data).unwrap();

        let vmhd = minf.vmhd().unwrap();
        assert!(vmhd.is_some());
        assert!(minf.smhd().unwrap().is_none());
        assert!(minf.nmhd().unwrap().is_none());
    }
}
