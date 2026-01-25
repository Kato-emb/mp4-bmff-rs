use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::BoxIter;

use crate::boxes::HdlrBoxView;
use crate::boxes::MdhdBox;
use crate::boxes::MinfBoxView;

/// A reference to a Media Box (`mdia`).
///
/// This box contains all the objects that declare information about the media data
/// within a track.
#[derive(Debug)]
pub struct MdiaBoxView<'a> {
    payload: &'a [u8],
}

impl<'a> MdiaBoxView<'a> {
    /// Returns an iterator over the child boxes of this `MdiaBoxView`.
    pub fn children(&self) -> BoxIter<'a> {
        BoxIter::new(self.payload)
    }

    /// Returns the Media Header Box (`mdhd`).
    pub fn mdhd(&self) -> Result<MdhdBox> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::MDHD {
                let mdhd = MdhdBox::decode(child.payload())?;
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

    /// Returns the Handler Reference Box (`hdlr`).
    pub fn hdlr(&self) -> Result<HdlrBoxView<'a>> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::HDLR {
                let hdlr = HdlrBoxView::decode(child.into_payload())?;
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

    /// Returns the Media Information Box (`minf`).
    pub fn minf(&self) -> Result<MinfBoxView<'a>> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::MINF {
                let minf = MinfBoxView::decode(child.into_payload())?;
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
}

impl BoxCodec for MdiaBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::MDIA
    }
}

impl<'de> BoxDecode<'de> for MdiaBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        Ok(MdiaBoxView { payload: bytes })
    }
}

impl<'a> TryFrom<&'a [u8]> for MdiaBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        MdiaBoxView::decode(value)
    }
}

#[cfg(feature = "alloc")]
pub use owned::MdiaBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::cursor::WriteCursor;

    use super::*;
    use crate::codec::boxed_len;
    use crate::codec::write_box_in;

    use crate::BoxCodec;
    use crate::BoxDecode;
    use crate::BoxEncode;
    use crate::boxes::HdlrBox;
    use crate::boxes::MinfBox;

    /// An owned Media Box (`mdia`).
    #[derive(Debug, Clone)]
    pub struct MdiaBox {
        /// The Media Header Box (`mdhd`).
        pub mdhd: MdhdBox,
        /// The Handler Reference Box (`hdlr`).
        pub hdlr: HdlrBox,
        /// The Media Information Box (`minf`).
        pub minf: MinfBox,
    }

    impl TryFrom<&MdiaBoxView<'_>> for MdiaBox {
        type Error = Error;

        fn try_from(view: &MdiaBoxView<'_>) -> Result<Self> {
            let mut mdhd = None;
            let mut hdlr = None;
            let mut minf = None;

            for child in view.children() {
                let child = child?;

                match child.boxtype() {
                    BoxType::MDHD if mdhd.is_none() => {
                        mdhd = Some(MdhdBox::decode(child.payload())?);
                    }
                    BoxType::MDHD => {
                        return Err(Error::in_box(
                            ErrorKind::InvalidBoxField {
                                field: "Media Header Box",
                                reason: "multiple mdhd boxes found",
                            },
                            BoxType::MDIA,
                        ));
                    }
                    BoxType::HDLR if hdlr.is_none() => {
                        let hdlr_view = HdlrBoxView::decode(child.payload())?;
                        hdlr = Some(HdlrBox::from(&hdlr_view));
                    }
                    BoxType::HDLR => {
                        return Err(Error::in_box(
                            ErrorKind::InvalidBoxField {
                                field: "Handler Reference Box",
                                reason: "multiple hdlr boxes found",
                            },
                            BoxType::MDIA,
                        ));
                    }
                    BoxType::MINF if minf.is_none() => {
                        let minf_view = MinfBoxView::decode(child.payload())?;
                        minf = Some(MinfBox::try_from(&minf_view)?);
                    }
                    BoxType::MINF => {
                        return Err(Error::in_box(
                            ErrorKind::InvalidBoxField {
                                field: "Media Information Box",
                                reason: "multiple minf boxes found",
                            },
                            BoxType::MDIA,
                        ));
                    }
                    _ => continue,
                }
            }

            Ok(MdiaBox {
                mdhd: mdhd.ok_or(Error::in_box(
                    ErrorKind::BoxMissing {
                        required: BoxType::MDHD,
                    },
                    BoxType::MDIA,
                ))?,
                hdlr: hdlr.ok_or(Error::in_box(
                    ErrorKind::BoxMissing {
                        required: BoxType::HDLR,
                    },
                    BoxType::MDIA,
                ))?,
                minf: minf.ok_or(Error::in_box(
                    ErrorKind::BoxMissing {
                        required: BoxType::MINF,
                    },
                    BoxType::MDIA,
                ))?,
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
        #[inline]
        fn encoded_len(&self) -> usize {
            boxed_len(&self.mdhd) // mdhd
            + boxed_len(&self.hdlr) // hdlr
            + boxed_len(&self.minf) // minf
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            write_box_in(&mut cur, &self.mdhd)?;
            write_box_in(&mut cur, &self.hdlr)?;
            write_box_in(&mut cur, &self.minf)?;

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

    fn make_mdhd_payload() -> Vec<u8> {
        let mut data = Vec::new();
        // FullBoxHeader: version=0, flags=0
        data.push(0);
        data.extend_from_slice(&[0, 0, 0]);
        // creation_time, modification_time, timescale, duration
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&1000u32.to_be_bytes());
        data.extend_from_slice(&0u32.to_be_bytes());
        // language
        data.extend_from_slice(&LanguageCode::UNDETERMINED.to_packed().to_be_bytes());
        // pre_defined
        data.extend_from_slice(&[0, 0]);
        data
    }

    fn make_hdlr_payload(handler_type: &[u8; 4]) -> Vec<u8> {
        let mut data = Vec::new();
        // FullBoxHeader: version=0, flags=0
        data.push(0);
        data.extend_from_slice(&[0, 0, 0]);
        // pre_defined
        data.extend_from_slice(&[0, 0, 0, 0]);
        // handler_type
        data.extend_from_slice(handler_type);
        // reserved
        data.extend_from_slice(&[0u8; 12]);
        // name (null-terminated)
        data.push(0);
        data
    }

    fn make_minf_payload() -> Vec<u8> {
        let mut data = Vec::new();

        // vmhd
        let vmhd_payload = make_vmhd_payload();
        data.extend_from_slice(&make_box(b"vmhd", &vmhd_payload));

        // dinf
        let dinf_payload = make_dinf_payload();
        data.extend_from_slice(&make_box(b"dinf", &dinf_payload));

        // stbl
        let stbl_payload = make_stbl_payload();
        data.extend_from_slice(&make_box(b"stbl", &stbl_payload));

        data
    }

    fn make_vmhd_payload() -> Vec<u8> {
        let mut data = Vec::new();
        data.push(0);
        data.extend_from_slice(&[0, 0, 1]); // flags = 1
        data.extend_from_slice(&[0, 0]); // graphicsmode
        data.extend_from_slice(&[0, 0, 0, 0, 0, 0]); // opcolor
        data
    }

    fn make_dinf_payload() -> Vec<u8> {
        let dref_payload = make_dref_payload();
        make_box(b"dref", &dref_payload)
    }

    fn make_dref_payload() -> Vec<u8> {
        let mut data = Vec::new();
        data.push(0);
        data.extend_from_slice(&[0, 0, 0]);
        data.extend_from_slice(&1u32.to_be_bytes());
        let url_payload = make_url_payload();
        data.extend_from_slice(&make_box(b"url ", &url_payload));
        data
    }

    fn make_url_payload() -> Vec<u8> {
        let mut data = Vec::new();
        data.push(0);
        data.extend_from_slice(&[0, 0, 1]); // self-contained
        data
    }

    fn make_stbl_payload() -> Vec<u8> {
        let mut data = Vec::new();

        // stsd
        let mut stsd = Vec::new();
        stsd.push(0);
        stsd.extend_from_slice(&[0, 0, 0]);
        stsd.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&make_box(b"stsd", &stsd));

        // stts
        let mut stts = Vec::new();
        stts.push(0);
        stts.extend_from_slice(&[0, 0, 0]);
        stts.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&make_box(b"stts", &stts));

        // stsc
        let mut stsc = Vec::new();
        stsc.push(0);
        stsc.extend_from_slice(&[0, 0, 0]);
        stsc.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&make_box(b"stsc", &stsc));

        // stco
        let mut stco = Vec::new();
        stco.push(0);
        stco.extend_from_slice(&[0, 0, 0]);
        stco.extend_from_slice(&0u32.to_be_bytes());
        data.extend_from_slice(&make_box(b"stco", &stco));

        data
    }

    fn make_mdia_payload() -> Vec<u8> {
        let mut data = Vec::new();

        // mdhd
        let mdhd_payload = make_mdhd_payload();
        data.extend_from_slice(&make_box(b"mdhd", &mdhd_payload));

        // hdlr
        let hdlr_payload = make_hdlr_payload(b"vide");
        data.extend_from_slice(&make_box(b"hdlr", &hdlr_payload));

        // minf
        let minf_payload = make_minf_payload();
        data.extend_from_slice(&make_box(b"minf", &minf_payload));

        data
    }

    #[test]
    fn parse_mdia_view() {
        let payload = make_mdia_payload();
        let mdia = MdiaBoxView::decode(&payload).unwrap();

        let mdhd = mdia.mdhd().unwrap();
        assert_eq!(mdhd.timescale, 1000);

        let hdlr = mdia.hdlr().unwrap();
        assert_eq!(hdlr.handler_type, crate::types::FourCC::new(*b"vide"));

        assert!(mdia.minf().is_ok());
    }

    #[test]
    fn parse_mdia_missing_mdhd() {
        let mut payload = Vec::new();

        // hdlr only
        let hdlr_payload = make_hdlr_payload(b"vide");
        payload.extend_from_slice(&make_box(b"hdlr", &hdlr_payload));

        // minf
        let minf_payload = make_minf_payload();
        payload.extend_from_slice(&make_box(b"minf", &minf_payload));

        let mdia = MdiaBoxView::decode(&payload).unwrap();
        let result = mdia.mdhd();
        assert!(result.is_err());
    }

    #[test]
    fn parse_mdia_missing_hdlr() {
        let mut payload = Vec::new();

        // mdhd only
        let mdhd_payload = make_mdhd_payload();
        payload.extend_from_slice(&make_box(b"mdhd", &mdhd_payload));

        // minf
        let minf_payload = make_minf_payload();
        payload.extend_from_slice(&make_box(b"minf", &minf_payload));

        let mdia = MdiaBoxView::decode(&payload).unwrap();
        let result = mdia.hdlr();
        assert!(result.is_err());
    }

    #[test]
    fn parse_mdia_missing_minf() {
        let mut payload = Vec::new();

        // mdhd
        let mdhd_payload = make_mdhd_payload();
        payload.extend_from_slice(&make_box(b"mdhd", &mdhd_payload));

        // hdlr
        let hdlr_payload = make_hdlr_payload(b"vide");
        payload.extend_from_slice(&make_box(b"hdlr", &hdlr_payload));

        let mdia = MdiaBoxView::decode(&payload).unwrap();
        let result = mdia.minf();
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn parse_mdia_owned() {
        let payload = make_mdia_payload();
        let mdia = MdiaBox::decode(&payload).unwrap();

        assert_eq!(mdia.mdhd.timescale, 1000);
        assert_eq!(mdia.hdlr.handler_type, crate::types::FourCC::new(*b"vide"));
    }
}
