use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxIter;
use crate::BoxType;
use crate::error::*;

use crate::boxes::MdiaBoxView;
use crate::boxes::TkhdBox;
use crate::boxes::TrefBoxView;

/// A reference to a Track Box (`trak`).
#[derive(Debug)]
pub struct TrakBoxView<'a> {
    payload: &'a [u8],
}

impl<'a> TrakBoxView<'a> {
    /// Returns an iterator over the child boxes of this `TrakBoxView`.
    pub fn children(&self) -> BoxIter<'a> {
        BoxIter::new(self.payload)
    }

    /// Returns the Track Header Box (`tkhd`).
    pub fn tkhd(&self) -> Result<TkhdBox> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::TKHD {
                let tkhd = TkhdBox::decode(child.payload())?;
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

    /// Returns the Media Box (`mdia`).
    pub fn mdia(&self) -> Result<MdiaBoxView<'a>> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::MDIA {
                let mdia = MdiaBoxView::decode(child.into_payload())?;
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

    /// Returns the Track Reference Box (`tref`) if present.
    pub fn tref(&self) -> Result<Option<TrefBoxView<'a>>> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::TREF {
                let tref = TrefBoxView::decode(child.into_payload())?;
                return Ok(Some(tref));
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
        Ok(TrakBoxView { payload: bytes })
    }
}

impl<'a> TryFrom<&'a [u8]> for TrakBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        TrakBoxView::decode(value)
    }
}

#[cfg(feature = "alloc")]
pub use owned::TrakBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::cursor::WriteCursor;

    use crate::codec::write_box_in;

    use super::*;
    use crate::BoxCodec;
    use crate::BoxDecode;
    use crate::BoxEncode;
    use crate::boxes::MdiaBox;
    use crate::boxes::TrefBox;

    /// An owned Track Box (`trak`).
    pub struct TrakBox {
        /// The Track Header Box (`tkhd`).
        pub tkhd: TkhdBox,
        /// The Track Reference Box (`tref`), if present.
        pub tref: Option<TrefBox>,
        /// The Media Box (`mdia`).
        pub mdia: MdiaBox,
    }

    impl TryFrom<&TrakBoxView<'_>> for TrakBox {
        type Error = Error;

        fn try_from(view: &TrakBoxView<'_>) -> Result<Self> {
            let mut tkhd = None;
            let mut tref = None;
            let mut mdia = None;

            for child in view.children() {
                let child = child?;

                match child.boxtype() {
                    BoxType::TKHD if tkhd.is_none() => {
                        tkhd = Some(TkhdBox::decode(child.payload())?);
                    }
                    BoxType::TKHD => {
                        return Err(Error::in_box(
                            ErrorKind::InvalidBoxField {
                                field: "Track Header Box",
                                reason: "multiple tkhd boxes found",
                            },
                            BoxType::TRAK,
                        ));
                    }
                    BoxType::TREF if tref.is_none() => {
                        let tref_view = TrefBoxView::decode(child.payload())?;
                        tref = Some(TrefBox::try_from(&tref_view)?);
                    }
                    BoxType::TREF => {
                        return Err(Error::in_box(
                            ErrorKind::InvalidBoxField {
                                field: "Track Reference Box",
                                reason: "multiple tref boxes found",
                            },
                            BoxType::TRAK,
                        ));
                    }
                    BoxType::MDIA if mdia.is_none() => {
                        let mdia_view = MdiaBoxView::decode(child.payload())?;
                        mdia = Some(MdiaBox::try_from(&mdia_view)?);
                    }
                    BoxType::MDIA => {
                        return Err(Error::in_box(
                            ErrorKind::InvalidBoxField {
                                field: "Media Box",
                                reason: "multiple mdia boxes found",
                            },
                            BoxType::TRAK,
                        ));
                    }
                    _ => continue,
                }
            }

            Ok(TrakBox {
                tkhd: tkhd.ok_or(Error::in_box(
                    ErrorKind::BoxMissing {
                        required: BoxType::TKHD,
                    },
                    BoxType::TRAK,
                ))?,
                tref,
                mdia: mdia.ok_or(Error::in_box(
                    ErrorKind::BoxMissing {
                        required: BoxType::MDIA,
                    },
                    BoxType::TRAK,
                ))?,
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
        fn encode(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            write_box_in(&mut cur, &self.tkhd)?;

            if let Some(ref tref) = self.tref {
                write_box_in(&mut cur, tref)?;
            }

            write_box_in(&mut cur, &self.mdia)?;

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

    fn make_tkhd_payload() -> Vec<u8> {
        let mut data = Vec::new();
        // FullBoxHeader: version=0, flags=0x000003
        data.push(0);
        data.extend_from_slice(&[0, 0, 3]);
        // creation_time, modification_time (u32 each)
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());
        // track_id
        data.extend_from_slice(&1u32.to_be_bytes());
        // reserved
        data.extend_from_slice(&0u32.to_be_bytes());
        // duration
        data.extend_from_slice(&1000u32.to_be_bytes());
        // reserved (2 x u32)
        data.extend_from_slice(&[0u8; 8]);
        // layer, alternate_group
        data.extend_from_slice(&[0u8; 4]);
        // volume, reserved
        data.extend_from_slice(&[0x01, 0x00]); // volume = 1.0
        data.extend_from_slice(&[0u8; 2]);
        // matrix (9 x i32)
        data.extend_from_slice(&0x00010000i32.to_be_bytes());
        data.extend_from_slice(&0i32.to_be_bytes());
        data.extend_from_slice(&0i32.to_be_bytes());
        data.extend_from_slice(&0i32.to_be_bytes());
        data.extend_from_slice(&0x00010000i32.to_be_bytes());
        data.extend_from_slice(&0i32.to_be_bytes());
        data.extend_from_slice(&0i32.to_be_bytes());
        data.extend_from_slice(&0i32.to_be_bytes());
        data.extend_from_slice(&0x40000000i32.to_be_bytes());
        // width, height (u32 each, 16.16 fixed point)
        data.extend_from_slice(&(1920u32 << 16).to_be_bytes());
        data.extend_from_slice(&(1080u32 << 16).to_be_bytes());
        data
    }

    fn make_mdia_payload() -> Vec<u8> {
        let mut data = Vec::new();

        // mdhd
        data.extend_from_slice(&make_box(b"mdhd", &make_mdhd_payload()));
        // hdlr
        data.extend_from_slice(&make_box(b"hdlr", &make_hdlr_payload()));
        // minf
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

    fn make_trak_payload() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&make_box(b"tkhd", &make_tkhd_payload()));
        data.extend_from_slice(&make_box(b"mdia", &make_mdia_payload()));
        data
    }

    #[test]
    fn parse_trak_view() {
        let payload = make_trak_payload();
        let trak = TrakBoxView::decode(&payload).unwrap();

        let tkhd = trak.tkhd().unwrap();
        assert_eq!(tkhd.track_id, 1);

        let mdia = trak.mdia().unwrap();
        let mdhd = mdia.mdhd().unwrap();
        assert_eq!(mdhd.timescale, 1000);
    }

    #[test]
    fn parse_trak_missing_tkhd() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_box(b"mdia", &make_mdia_payload()));

        let trak = TrakBoxView::decode(&payload).unwrap();
        let result = trak.tkhd();
        assert!(result.is_err());
    }

    #[test]
    fn parse_trak_missing_mdia() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_box(b"tkhd", &make_tkhd_payload()));

        let trak = TrakBoxView::decode(&payload).unwrap();
        let result = trak.mdia();
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn parse_trak_owned() {
        let payload = make_trak_payload();
        let trak = TrakBox::decode(&payload).unwrap();

        assert_eq!(trak.tkhd.track_id, 1);
        assert_eq!(trak.mdia.mdhd.timescale, 1000);
    }
}
