use crate::cursor::ReadCursor;

use crate::BoxFrame;
use crate::BoxIter;
use crate::BoxType;
use crate::error::*;

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
                let mdhd = MdhdBox::parse(child.payload())?;
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
                let hdlr = HdlrBoxView::parse(child.payload())?;
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
                let minf = MinfBoxView::parse(child.payload())?;
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

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<MdiaBoxView<'a>> {
        let payload = cur.take(cur.remaining())?;
        Ok(MdiaBoxView { payload })
    }

    /// Parses a `MdiaBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<MdiaBoxView<'a>> {
        let mut cursor = ReadCursor::new(payload);
        MdiaBoxView::parse_in(&mut cursor)
    }
}

impl<'a> TryFrom<BoxFrame<'a>> for MdiaBoxView<'a> {
    type Error = Error;

    fn try_from(value: BoxFrame<'a>) -> Result<Self> {
        if value.boxtype() != BoxType::MDIA {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::MDIA,
                found: value.boxtype(),
            }));
        }

        MdiaBoxView::parse(value.payload())
    }
}

#[cfg(feature = "alloc")]
pub use owned::MdiaBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::cursor::WriteCursor;

    use super::*;
    use crate::BoxFrameMut;
    use crate::base::frame::write_box_in;

    use crate::boxes::HdlrBox;
    use crate::boxes::MinfBox;

    /// An owned Media Box (`mdia`).
    pub struct MdiaBox {
        /// The Media Header Box (`mdhd`).
        pub mdhd: MdhdBox,
        /// The Handler Reference Box (`hdlr`).
        pub hdlr: HdlrBox,
        /// The Media Information Box (`minf`).
        pub minf: MinfBox,
    }

    impl MdiaBox {
        /// Constructs a `MdiaBox` from a `MdiaBoxView`.
        pub fn from_view(view: &MdiaBoxView<'_>) -> Result<MdiaBox> {
            let mut mdhd = None;
            let mut hdlr = None;
            let mut minf = None;

            for child in view.children() {
                let child = child?;

                match child.boxtype() {
                    BoxType::MDHD if mdhd.is_none() => {
                        mdhd = Some(MdhdBox::parse(child.payload())?);
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
                        let hdlr_view = HdlrBoxView::parse(child.payload())?;
                        hdlr = Some(HdlrBox::from_view(&hdlr_view));
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
                        let minf_view = MinfBoxView::parse(child.payload())?;
                        minf = Some(MinfBox::from_view(&minf_view)?);
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

        /// Parses a `MdiaBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<MdiaBox> {
            let view = MdiaBoxView::parse(payload)?;
            MdiaBox::from_view(&view)
        }

        /// Returns the size of the payload in bytes.
        pub fn size(&self) -> usize {
            let mut size = 0;
            size += BoxFrameMut::required_len(BoxType::MDHD, self.mdhd.size());
            size += BoxFrameMut::required_len(BoxType::HDLR, self.hdlr.size());
            size += BoxFrameMut::required_len(BoxType::MINF, self.minf.size());
            size
        }

        pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
            write_box_in(cur, BoxType::MDHD, self.mdhd.size(), |p| self.mdhd.write(p))?;
            write_box_in(cur, BoxType::HDLR, self.hdlr.size(), |p| self.hdlr.write(p))?;
            write_box_in(cur, BoxType::MINF, self.minf.size(), |p| self.minf.write(p))?;

            if !cur.is_empty() {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxSize {
                        reason: "Buffer larger than expected",
                        got: cur.remaining() as u64,
                    },
                    BoxType::MDIA,
                ));
            }

            Ok(())
        }

        /// Writes this `MdiaBox` into the given payload.
        pub fn write(&self, payload: &mut [u8]) -> Result<()> {
            let mut cursor = WriteCursor::new(payload);
            self.write_in(&mut cursor)
        }
    }

    impl TryFrom<&MdiaBoxView<'_>> for MdiaBox {
        type Error = Error;

        fn try_from(value: &MdiaBoxView<'_>) -> Result<Self> {
            MdiaBox::from_view(value)
        }
    }

    impl TryFrom<BoxFrame<'_>> for MdiaBox {
        type Error = Error;

        fn try_from(value: BoxFrame<'_>) -> Result<Self> {
            if value.boxtype() != BoxType::MDIA {
                return Err(Error::new(ErrorKind::MismatchedBoxType {
                    expected: BoxType::MDIA,
                    found: value.boxtype(),
                }));
            }

            let view = MdiaBoxView::parse(value.payload())?;
            MdiaBox::from_view(&view)
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
        let mdia = MdiaBoxView::parse(&payload).unwrap();

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

        let mdia = MdiaBoxView::parse(&payload).unwrap();
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

        let mdia = MdiaBoxView::parse(&payload).unwrap();
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

        let mdia = MdiaBoxView::parse(&payload).unwrap();
        let result = mdia.minf();
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn parse_mdia_owned() {
        let payload = make_mdia_payload();
        let mdia = MdiaBox::parse(&payload).unwrap();

        assert_eq!(mdia.mdhd.timescale, 1000);
        assert_eq!(mdia.hdlr.handler_type, crate::types::FourCC::new(*b"vide"));
    }
}
