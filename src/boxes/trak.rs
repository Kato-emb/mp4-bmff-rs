use crate::cursor::ReadCursor;

use crate::BoxIter;
use crate::BoxType;
use crate::BoxView;
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
            if child.header.boxtype() == BoxType::TKHD {
                let tkhd = TkhdBox::parse(child.payload)?;
                return Ok(tkhd);
            }
        }

        Err(Error::new(ErrorKind::BoxMissing {
            required: BoxType::TKHD,
        }))
    }

    /// Returns the Media Box (`mdia`).
    pub fn mdia(&self) -> Result<MdiaBoxView<'a>> {
        for child in self.children() {
            let child = child?;
            if child.header.boxtype() == BoxType::MDIA {
                let mdia = MdiaBoxView::parse(child.payload)?;
                return Ok(mdia);
            }
        }

        Err(Error::new(ErrorKind::BoxMissing {
            required: BoxType::MDIA,
        }))
    }

    /// Returns the Track Reference Box (`tref`) if present.
    pub fn tref(&self) -> Result<Option<TrefBoxView<'a>>> {
        for child in self.children() {
            let child = child?;
            if child.header.boxtype() == BoxType::TREF {
                let tref = TrefBoxView::parse(child.payload)?;
                return Ok(Some(tref));
            }
        }

        Ok(None)
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<TrakBoxView<'a>> {
        let payload = cur.take(cur.remaining())?;
        Ok(TrakBoxView { payload })
    }

    /// Parses a `TrakBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<TrakBoxView<'a>> {
        let mut cursor = ReadCursor::new(payload);
        TrakBoxView::parse_in(&mut cursor)
    }
}

impl<'a> TryFrom<&BoxView<'a>> for TrakBoxView<'a> {
    type Error = Error;

    fn try_from(value: &BoxView<'a>) -> Result<Self> {
        if value.header.boxtype() != BoxType::TRAK {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::TRAK,
                found: value.header.boxtype(),
            }));
        }

        TrakBoxView::parse(value.payload)
    }
}

#[cfg(feature = "alloc")]
pub use owned::TrakBox;

#[cfg(feature = "alloc")]
mod owned {
    use super::*;

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

    impl TrakBox {
        /// Constructs a `TrakBox` from a `TrakBoxView`.
        pub fn from_view(view: &TrakBoxView<'_>) -> Result<TrakBox> {
            let mut tkhd = None;
            let mut tref = None;
            let mut mdia = None;

            for child in view.children() {
                let child = child?;

                match child.header.boxtype() {
                    BoxType::TKHD if tkhd.is_none() => {
                        tkhd = Some(TkhdBox::parse(child.payload)?);
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
                        let tref_view = TrefBoxView::parse(child.payload)?;
                        tref = Some(TrefBox::from_view(&tref_view)?);
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
                        let mdia_view = MdiaBoxView::parse(child.payload)?;
                        mdia = Some(MdiaBox::from_view(&mdia_view)?);
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

        /// Parses a `TrakBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<TrakBox> {
            let view = TrakBoxView::parse(payload)?;
            TrakBox::from_view(&view)
        }
    }

    impl TryFrom<&TrakBoxView<'_>> for TrakBox {
        type Error = Error;

        fn try_from(value: &TrakBoxView<'_>) -> Result<Self> {
            TrakBox::from_view(value)
        }
    }

    impl TryFrom<&BoxView<'_>> for TrakBox {
        type Error = Error;

        fn try_from(value: &BoxView<'_>) -> Result<Self> {
            if value.header.boxtype() != BoxType::TRAK {
                return Err(Error::new(ErrorKind::MismatchedBoxType {
                    expected: BoxType::TRAK,
                    found: value.header.boxtype(),
                }));
            }

            let view = TrakBoxView::parse(value.payload)?;
            TrakBox::from_view(&view)
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
        let trak = TrakBoxView::parse(&payload).unwrap();

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

        let trak = TrakBoxView::parse(&payload).unwrap();
        let result = trak.tkhd();
        assert!(result.is_err());
    }

    #[test]
    fn parse_trak_missing_mdia() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_box(b"tkhd", &make_tkhd_payload()));

        let trak = TrakBoxView::parse(&payload).unwrap();
        let result = trak.mdia();
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn parse_trak_owned() {
        let payload = make_trak_payload();
        let trak = TrakBox::parse(&payload).unwrap();

        assert_eq!(trak.tkhd.track_id, 1);
        assert_eq!(trak.mdia.mdhd.timescale, 1000);
    }
}
