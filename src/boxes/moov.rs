use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxIter;
use crate::BoxType;
use crate::error::*;

use crate::boxes::MvexBoxView;
use crate::boxes::MvhdBox;
use crate::boxes::TrakBoxView;

/// A reference to a Movie Box (`moov`).
#[derive(Debug)]
pub struct MoovBoxView<'a> {
    payload: &'a [u8],
}

impl<'a> MoovBoxView<'a> {
    /// Returns an iterator over the child boxes of this `MoovBoxView`.
    pub fn children(&self) -> BoxIter<'a> {
        BoxIter::new(self.payload)
    }

    /// Returns the Movie Header Box (`mvhd`).
    pub fn mvhd(&self) -> Result<MvhdBox> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::MVHD {
                let mvhd = MvhdBox::decode(child.into_payload())?;
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

    /// Returns an iterator over the Track Boxes (`trak`) contained in this `MoovBoxView`.
    pub fn traks(&self) -> impl Iterator<Item = Result<TrakBoxView<'a>>> + 'a {
        self.children().filter_map(|child| match child {
            Ok(view) if view.boxtype() == BoxType::TRAK => {
                Some(TrakBoxView::decode(view.into_payload()))
            }
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
    }

    /// Returns the Movie Extends Box (`mvex`) if present.
    pub fn mvex(&self) -> Result<Option<MvexBoxView<'a>>> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::MVEX {
                let mvex = MvexBoxView::decode(child.into_payload())?;
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
        Ok(MoovBoxView { payload: bytes })
    }
}

impl<'a> TryFrom<&'a [u8]> for MoovBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        MoovBoxView::decode(value)
    }
}

#[cfg(feature = "alloc")]
pub use owned::MoovBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use crate::cursor::WriteCursor;

    use crate::base::writer::write_box_in;

    use super::*;
    use crate::BoxEncode;
    use crate::boxes::MvexBox;
    use crate::boxes::TrakBox;

    /// An owned Movie Box (`moov`).
    pub struct MoovBox {
        /// The Movie Header Box (`mvhd`).
        pub mvhd: MvhdBox,
        /// The Movie Extends Box (`mvex`), if present.
        pub mvex: Option<MvexBox>,
        /// The Track Boxes (`trak`).
        pub traks: Vec<TrakBox>,
    }

    impl TryFrom<&MoovBoxView<'_>> for MoovBox {
        type Error = Error;

        fn try_from(view: &MoovBoxView<'_>) -> Result<Self> {
            let mut mvhd = None;
            let mut mvex = None;
            let mut traks = Vec::new();

            for child in view.children() {
                let child = child?;

                match child.boxtype() {
                    BoxType::MVHD if mvhd.is_none() => {
                        mvhd = Some(MvhdBox::decode(child.payload())?);
                    }
                    BoxType::MVHD => {
                        return Err(Error::in_box(
                            ErrorKind::InvalidBoxField {
                                field: "Movie Header Box",
                                reason: "multiple mvhd boxes found",
                            },
                            BoxType::MOOV,
                        ));
                    }
                    BoxType::MVEX if mvex.is_none() => {
                        let mvex_view = MvexBoxView::decode(child.payload())?;
                        mvex = Some(MvexBox::try_from(&mvex_view)?);
                    }
                    BoxType::MVEX => {
                        return Err(Error::in_box(
                            ErrorKind::InvalidBoxField {
                                field: "Movie Extends Box",
                                reason: "multiple mvex boxes found",
                            },
                            BoxType::MOOV,
                        ));
                    }
                    BoxType::TRAK => {
                        let trak_view = TrakBoxView::decode(child.payload())?;
                        traks.push(TrakBox::try_from(&trak_view)?);
                    }
                    _ => continue,
                }
            }

            Ok(MoovBox {
                mvhd: mvhd.ok_or(Error::in_box(
                    ErrorKind::BoxMissing {
                        required: BoxType::MVHD,
                    },
                    BoxType::MOOV,
                ))?,
                mvex,
                traks,
            })
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
        fn encode(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            write_box_in(&mut cur, &self.mvhd)?;

            if let Some(ref mvex) = self.mvex {
                write_box_in(&mut cur, mvex)?;
            }

            for trak in &self.traks {
                write_box_in(&mut cur, trak)?;
            }

            Ok(cur.position())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::LanguageCode;

    fn make_box(boxtype: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let size = (8 + payload.len()) as u32;
        let mut data = Vec::new();
        data.extend_from_slice(&size.to_be_bytes());
        data.extend_from_slice(boxtype);
        data.extend_from_slice(payload);
        data
    }

    fn make_mvhd_payload() -> Vec<u8> {
        let mut data = Vec::new();
        // FullBoxHeader: version=0, flags=0
        data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        // creation_time, modification_time
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());
        // timescale
        data.extend_from_slice(&1000u32.to_be_bytes());
        // duration
        data.extend_from_slice(&5000u32.to_be_bytes());
        // rate (1.0)
        data.extend_from_slice(&0x00010000i32.to_be_bytes());
        // volume (1.0)
        data.extend_from_slice(&0x0100u16.to_be_bytes());
        // reserved (10 bytes)
        data.extend_from_slice(&[0u8; 10]);
        // matrix (identity)
        data.extend_from_slice(&0x00010000i32.to_be_bytes());
        data.extend_from_slice(&0i32.to_be_bytes());
        data.extend_from_slice(&0i32.to_be_bytes());
        data.extend_from_slice(&0i32.to_be_bytes());
        data.extend_from_slice(&0x00010000i32.to_be_bytes());
        data.extend_from_slice(&0i32.to_be_bytes());
        data.extend_from_slice(&0i32.to_be_bytes());
        data.extend_from_slice(&0i32.to_be_bytes());
        data.extend_from_slice(&0x40000000i32.to_be_bytes());
        // pre_defined (24 bytes)
        data.extend_from_slice(&[0u8; 24]);
        // next_track_id
        data.extend_from_slice(&2u32.to_be_bytes());
        data
    }

    fn make_tkhd_payload(track_id: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(0);
        data.extend_from_slice(&[0, 0, 3]);
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&track_id.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&1000u32.to_be_bytes());
        data.extend_from_slice(&[0u8; 8]);
        data.extend_from_slice(&[0u8; 4]);
        data.extend_from_slice(&[0x01, 0x00]);
        data.extend_from_slice(&[0u8; 2]);
        data.extend_from_slice(&0x00010000i32.to_be_bytes());
        data.extend_from_slice(&0i32.to_be_bytes());
        data.extend_from_slice(&0i32.to_be_bytes());
        data.extend_from_slice(&0i32.to_be_bytes());
        data.extend_from_slice(&0x00010000i32.to_be_bytes());
        data.extend_from_slice(&0i32.to_be_bytes());
        data.extend_from_slice(&0i32.to_be_bytes());
        data.extend_from_slice(&0i32.to_be_bytes());
        data.extend_from_slice(&0x40000000i32.to_be_bytes());
        data.extend_from_slice(&(1920u32 << 16).to_be_bytes());
        data.extend_from_slice(&(1080u32 << 16).to_be_bytes());
        data
    }

    fn make_mdia_payload() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&make_box(b"mdhd", &make_mdhd_payload()));
        data.extend_from_slice(&make_box(b"hdlr", &make_hdlr_payload()));
        data.extend_from_slice(&make_box(b"minf", &make_minf_payload()));
        data
    }

    fn make_mdhd_payload() -> Vec<u8> {
        let mut data = Vec::new();
        data.push(0);
        data.extend_from_slice(&[0, 0, 0]);
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&1000u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&LanguageCode::UNDETERMINED.to_packed().to_be_bytes());
        data.extend_from_slice(&[0, 0]);
        data
    }

    fn make_hdlr_payload() -> Vec<u8> {
        let mut data = Vec::new();
        data.push(0);
        data.extend_from_slice(&[0, 0, 0]);
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(b"vide");
        data.extend_from_slice(&[0u8; 12]);
        data.push(0);
        data
    }

    fn make_minf_payload() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&make_box(b"vmhd", &make_vmhd_payload()));
        data.extend_from_slice(&make_box(b"dinf", &make_dinf_payload()));
        data.extend_from_slice(&make_box(b"stbl", &make_stbl_payload()));
        data
    }

    fn make_vmhd_payload() -> Vec<u8> {
        let mut data = Vec::new();
        data.push(0);
        data.extend_from_slice(&[0, 0, 1]);
        data.extend_from_slice(&[0, 0]);
        data.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
        data
    }

    fn make_dinf_payload() -> Vec<u8> {
        make_box(b"dref", &make_dref_payload())
    }

    fn make_dref_payload() -> Vec<u8> {
        let mut data = Vec::new();
        data.push(0);
        data.extend_from_slice(&[0, 0, 0]);
        data.extend_from_slice(&1u32.to_be_bytes());
        data.extend_from_slice(&make_box(b"url ", &make_url_payload()));
        data
    }

    fn make_url_payload() -> Vec<u8> {
        let mut data = Vec::new();
        data.push(0);
        data.extend_from_slice(&[0, 0, 1]);
        data
    }

    fn make_stbl_payload() -> Vec<u8> {
        let mut data = Vec::new();

        let mut stsd = Vec::new();
        stsd.push(0);
        stsd.extend_from_slice(&[0, 0, 0]);
        stsd.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&make_box(b"stsd", &stsd));

        let mut stts = Vec::new();
        stts.push(0);
        stts.extend_from_slice(&[0, 0, 0]);
        stts.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&make_box(b"stts", &stts));

        let mut stsc = Vec::new();
        stsc.push(0);
        stsc.extend_from_slice(&[0, 0, 0]);
        stsc.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&make_box(b"stsc", &stsc));

        let mut stco = Vec::new();
        stco.push(0);
        stco.extend_from_slice(&[0, 0, 0]);
        stco.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&make_box(b"stco", &stco));

        data
    }

    fn make_trak_payload(track_id: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&make_box(b"tkhd", &make_tkhd_payload(track_id)));
        data.extend_from_slice(&make_box(b"mdia", &make_mdia_payload()));
        data
    }

    fn make_moov_payload(num_tracks: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&make_box(b"mvhd", &make_mvhd_payload()));
        for i in 1..=num_tracks {
            data.extend_from_slice(&make_box(b"trak", &make_trak_payload(i)));
        }
        data
    }

    #[test]
    fn parse_moov_view() {
        let payload = make_moov_payload(2);
        let moov = MoovBoxView::decode(&payload).unwrap();

        let mvhd = moov.mvhd().unwrap();
        assert_eq!(mvhd.timescale, 1000);
        assert_eq!(mvhd.duration, 5000);
        assert_eq!(mvhd.next_track_id, 2);

        let traks: Vec<_> = moov.traks().collect();
        assert_eq!(traks.len(), 2);

        let trak1 = traks[0].as_ref().unwrap();
        assert_eq!(trak1.tkhd().unwrap().track_id, 1);

        let trak2 = traks[1].as_ref().unwrap();
        assert_eq!(trak2.tkhd().unwrap().track_id, 2);
    }

    #[test]
    fn parse_moov_missing_mvhd() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_box(b"trak", &make_trak_payload(1)));

        let moov = MoovBoxView::decode(&payload).unwrap();
        let result = moov.mvhd();
        assert!(result.is_err());
    }

    #[test]
    fn parse_moov_no_traks() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_box(b"mvhd", &make_mvhd_payload()));

        let moov = MoovBoxView::decode(&payload).unwrap();
        let traks: Vec<_> = moov.traks().collect();
        assert_eq!(traks.len(), 0);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn parse_moov_owned() {
        let payload = make_moov_payload(2);
        let moov = MoovBox::decode(&payload).unwrap();

        assert_eq!(moov.mvhd.timescale, 1000);
        assert_eq!(moov.traks.len(), 2);
        assert_eq!(moov.traks[0].tkhd.track_id, 1);
        assert_eq!(moov.traks[1].tkhd.track_id, 2);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn parse_moov_owned_missing_mvhd() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_box(b"trak", &make_trak_payload(1)));

        let result = MoovBox::decode(&payload);
        assert!(result.is_err());
    }
}
