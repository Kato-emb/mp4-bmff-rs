use crate::cursor::ReadCursor;

use crate::RawBoxRef;
use crate::BoxIter;
use crate::BoxType;
use crate::error::*;

use crate::boxes::DinfBoxView;
use crate::boxes::HmhdBox;
use crate::boxes::NmhdBox;
use crate::boxes::SmhdBox;
use crate::boxes::StblBoxView;
use crate::boxes::VmhdBox;

/// An enum representing the media header box within a `minf`.
///
/// Exactly one of these must be present in a `minf` box, depending on the track type.
#[derive(Debug, Clone, Copy)]
pub enum MediaHeader {
    /// Video Media Header Box (`vmhd`) - for video tracks.
    Vmhd(VmhdBox),
    /// Sound Media Header Box (`smhd`) - for audio tracks.
    Smhd(SmhdBox),
    /// Hint Media Header Box (`hmhd`) - for hint tracks.
    Hmhd(HmhdBox),
    /// Null Media Header Box (`nmhd`) - for other tracks (e.g., metadata).
    Nmhd(NmhdBox),
}

impl MediaHeader {
    /// Returns the box type for this media header.
    pub fn boxtype(&self) -> BoxType {
        match self {
            MediaHeader::Vmhd(_) => BoxType::VMHD,
            MediaHeader::Smhd(_) => BoxType::SMHD,
            MediaHeader::Hmhd(_) => BoxType::HMHD,
            MediaHeader::Nmhd(_) => BoxType::NMHD,
        }
    }

    /// Returns the size of the payload in bytes.
    pub fn size(&self) -> usize {
        match self {
            MediaHeader::Vmhd(vmhd) => vmhd.size(),
            MediaHeader::Smhd(smhd) => smhd.size(),
            MediaHeader::Hmhd(hmhd) => hmhd.size(),
            MediaHeader::Nmhd(nmhd) => nmhd.size(),
        }
    }
}

/// A reference to a Media Information Box (`minf`).
#[derive(Debug)]
pub struct MinfBoxView<'a> {
    payload: &'a [u8],
}

impl<'a> MinfBoxView<'a> {
    /// Returns an iterator over the child boxes of this `MinfBoxView`.
    pub fn children(&self) -> BoxIter<'a> {
        BoxIter::new(self.payload)
    }

    /// Returns the media header box (`vmhd`, `smhd`, `hmhd`, or `nmhd`).
    ///
    /// Exactly one of these must be present in a valid `minf` box.
    pub fn media_header(&self) -> Result<MediaHeader> {
        let mut media_header = None;

        for child in self.children() {
            let child = child?;
            match child.boxtype() {
                BoxType::VMHD if media_header.is_none() => {
                    let vmhd = VmhdBox::parse(child.payload())?;
                    media_header = Some(MediaHeader::Vmhd(vmhd));
                }
                BoxType::SMHD if media_header.is_none() => {
                    let smhd = SmhdBox::parse(child.payload())?;
                    media_header = Some(MediaHeader::Smhd(smhd));
                }
                BoxType::HMHD if media_header.is_none() => {
                    let hmhd = HmhdBox::parse(child.payload())?;
                    media_header = Some(MediaHeader::Hmhd(hmhd));
                }
                BoxType::NMHD if media_header.is_none() => {
                    let nmhd = NmhdBox::parse(child.payload())?;
                    media_header = Some(MediaHeader::Nmhd(nmhd));
                }
                BoxType::VMHD | BoxType::SMHD | BoxType::HMHD | BoxType::NMHD => {
                    return Err(Error::new(ErrorKind::InvalidBoxField {
                        field: "Media header",
                        reason: "multiple media header boxes found",
                    })
                    .with_box_type(BoxType::MINF));
                }
                _ => continue,
            }
        }

        media_header.ok_or_else(|| {
            Error::new(ErrorKind::InvalidBoxField {
                field: "Media header",
                reason: "no media header box found (vmhd, smhd, hmhd, or nmhd required)",
            })
            .with_box_type(BoxType::MINF)
        })
    }

    /// Returns the Video Media Header Box (`vmhd`) if present.
    pub fn vmhd(&self) -> Result<Option<VmhdBox>> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::VMHD {
                let vmhd = VmhdBox::parse(child.payload())?;
                return Ok(Some(vmhd));
            }
        }

        Ok(None)
    }

    /// Returns the Sound Media Header Box (`smhd`) if present.
    pub fn smhd(&self) -> Result<Option<SmhdBox>> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::SMHD {
                let smhd = SmhdBox::parse(child.payload())?;
                return Ok(Some(smhd));
            }
        }

        Ok(None)
    }

    /// Returns the Hint Media Header Box (`hmhd`) if present.
    pub fn hmhd(&self) -> Result<Option<HmhdBox>> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::HMHD {
                let hmhd = HmhdBox::parse(child.payload())?;
                return Ok(Some(hmhd));
            }
        }

        Ok(None)
    }

    /// Returns the Null Media Header Box (`nmhd`) if present.
    pub fn nmhd(&self) -> Result<Option<NmhdBox>> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::NMHD {
                let nmhd = NmhdBox::parse(child.payload())?;
                return Ok(Some(nmhd));
            }
        }

        Ok(None)
    }

    /// Returns the Data Information Box (`dinf`).
    pub fn dinf(&self) -> Result<DinfBoxView<'a>> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::DINF {
                let dinf = DinfBoxView::parse(child.into_payload())?;
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

    /// Returns the Sample Table Box (`stbl`).
    pub fn stbl(&self) -> Result<StblBoxView<'a>> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::STBL {
                let stbl = StblBoxView::parse(child.into_payload())?;
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

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<MinfBoxView<'a>> {
        let payload = cur.take(cur.remaining())?;
        Ok(MinfBoxView { payload })
    }

    /// Parses a `MinfBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<MinfBoxView<'a>> {
        let mut cursor = ReadCursor::new(payload);
        let this = MinfBoxView::parse_in(&mut cursor)?;

        Ok(this)
    }
}

impl<'a> TryFrom<RawBoxRef<'a>> for MinfBoxView<'a> {
    type Error = Error;

    fn try_from(value: RawBoxRef<'a>) -> Result<Self> {
        if value.boxtype() != BoxType::MINF {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::MINF,
                found: value.boxtype(),
            }));
        }

        MinfBoxView::parse(value.payload())
    }
}

#[cfg(feature = "alloc")]
pub use owned::MinfBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::cursor::WriteCursor;

    use crate::BoxFrameMut;
    use crate::base::frame::write_box_in;

    use super::*;
    use crate::boxes::DinfBox;
    use crate::boxes::StblBox;

    /// An owned Media Information Box (`minf`).
    pub struct MinfBox {
        /// The media information header box (`vmhd`, `smhd`, `hmhd`, or `nmhd`).
        pub media_header: MediaHeader,
        /// The Data Information Box (`dinf`).
        pub dinf: DinfBox,
        /// The Sample Table Box (`stbl`).
        pub stbl: StblBox,
    }

    impl MinfBox {
        /// Constructs a `MinfBox` from a `MinfBoxView`.
        pub fn from_view(view: &MinfBoxView<'_>) -> Result<MinfBox> {
            let mut media_header = None;
            let mut dinf = None;
            let mut stbl = None;

            for child in view.children() {
                let child = child?;

                match child.boxtype() {
                    BoxType::VMHD if media_header.is_none() => {
                        let vmhd = VmhdBox::parse(child.payload())?;
                        media_header = Some(MediaHeader::Vmhd(vmhd));
                    }
                    BoxType::SMHD if media_header.is_none() => {
                        let smhd = SmhdBox::parse(child.payload())?;
                        media_header = Some(MediaHeader::Smhd(smhd));
                    }
                    BoxType::HMHD if media_header.is_none() => {
                        let hmhd = HmhdBox::parse(child.payload())?;
                        media_header = Some(MediaHeader::Hmhd(hmhd));
                    }
                    BoxType::NMHD if media_header.is_none() => {
                        let nmhd = NmhdBox::parse(child.payload())?;
                        media_header = Some(MediaHeader::Nmhd(nmhd));
                    }
                    BoxType::VMHD | BoxType::SMHD | BoxType::HMHD | BoxType::NMHD => {
                        return Err(Error::in_box(
                            ErrorKind::InvalidBoxField {
                                field: "Media header",
                                reason: "multiple media header boxes found",
                            },
                            BoxType::MINF,
                        ));
                    }
                    BoxType::DINF if dinf.is_none() => {
                        let dinf_view = DinfBoxView::parse(child.payload())?;
                        dinf = Some(DinfBox::from_view(&dinf_view)?);
                    }
                    BoxType::DINF => {
                        return Err(Error::in_box(
                            ErrorKind::InvalidBoxField {
                                field: "Data Information Box",
                                reason: "multiple dinf boxes found",
                            },
                            BoxType::MINF,
                        ));
                    }
                    BoxType::STBL if stbl.is_none() => {
                        let stbl_view = StblBoxView::parse(child.payload())?;
                        stbl = Some(StblBox::from_view(&stbl_view)?);
                    }
                    BoxType::STBL => {
                        return Err(Error::in_box(
                            ErrorKind::InvalidBoxField {
                                field: "Sample Table Box",
                                reason: "multiple stbl boxes found",
                            },
                            BoxType::MINF,
                        ));
                    }
                    _ => continue,
                }
            }

            Ok(MinfBox {
                media_header: media_header.ok_or(Error::in_box(
                    ErrorKind::InvalidBoxField {
                        field: "Media header",
                        reason: "no media header box found (vmhd, smhd, hmhd, or nmhd required)",
                    },
                    BoxType::MINF,
                ))?,
                dinf: dinf.ok_or(Error::in_box(
                    ErrorKind::BoxMissing {
                        required: BoxType::DINF,
                    },
                    BoxType::MINF,
                ))?,
                stbl: stbl.ok_or(Error::in_box(
                    ErrorKind::BoxMissing {
                        required: BoxType::STBL,
                    },
                    BoxType::MINF,
                ))?,
            })
        }

        /// Parses a `MinfBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<MinfBox> {
            let view = MinfBoxView::parse(payload)?;
            MinfBox::from_view(&view)
        }

        /// Returns the size of the payload in bytes.
        pub fn size(&self) -> usize {
            let mut size = 0;

            // media header (vmhd, smhd, hmhd, or nmhd)
            size +=
                BoxFrameMut::required_len(self.media_header.boxtype(), self.media_header.size());

            // dinf
            size += BoxFrameMut::required_len(BoxType::DINF, self.dinf.size());

            // stbl
            size += BoxFrameMut::required_len(BoxType::STBL, self.stbl.size());

            size
        }

        pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
            // media header
            let mhdr_boxtype = self.media_header.boxtype();
            let mhdr_size = self.media_header.size();
            write_box_in(cur, mhdr_boxtype, mhdr_size, |p| match &self.media_header {
                MediaHeader::Vmhd(vmhd) => vmhd.write(p),
                MediaHeader::Smhd(smhd) => smhd.write(p),
                MediaHeader::Hmhd(hmhd) => hmhd.write(p),
                MediaHeader::Nmhd(nmhd) => nmhd.write(p),
            })?;

            // dinf
            write_box_in(cur, BoxType::DINF, self.dinf.size(), |p| self.dinf.write(p))?;

            // stbl
            write_box_in(cur, BoxType::STBL, self.stbl.size(), |p| self.stbl.write(p))?;

            if !cur.is_empty() {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxSize {
                        reason: "Buffer larger than expected",
                        got: cur.remaining() as u64,
                    },
                    BoxType::MINF,
                ));
            }

            Ok(())
        }

        /// Writes this `MinfBox` into the given payload.
        pub fn write(&self, payload: &mut [u8]) -> Result<()> {
            let mut cursor = WriteCursor::new(payload);
            self.write_in(&mut cursor)
        }
    }

    impl TryFrom<&MinfBoxView<'_>> for MinfBox {
        type Error = Error;

        fn try_from(value: &MinfBoxView<'_>) -> Result<Self> {
            MinfBox::from_view(value)
        }
    }

    impl TryFrom<RawBoxRef<'_>> for MinfBox {
        type Error = Error;

        fn try_from(value: RawBoxRef<'_>) -> Result<Self> {
            if value.boxtype() != BoxType::MINF {
                return Err(Error::new(ErrorKind::MismatchedBoxType {
                    expected: BoxType::MINF,
                    found: value.boxtype(),
                }));
            }

            let view = MinfBoxView::parse(value.payload())?;
            MinfBox::from_view(&view)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_box(boxtype: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let size = (8 + payload.len()) as u32;
        let mut data = Vec::new();
        data.extend_from_slice(&size.to_be_bytes());
        data.extend_from_slice(boxtype);
        data.extend_from_slice(payload);
        data
    }

    fn make_vmhd_payload() -> Vec<u8> {
        let mut data = Vec::new();
        // FullBoxHeader: version (1 byte) + flags (3 bytes)
        data.push(0);
        data.extend_from_slice(&[0, 0, 1]); // flags = 1 (default for vmhd)
        // graphicsmode (2 bytes)
        data.extend_from_slice(&[0, 0]);
        // opcolor (3 x 2 bytes)
        data.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
        data
    }

    fn make_smhd_payload() -> Vec<u8> {
        let mut data = Vec::new();
        // FullBoxHeader: version (1 byte) + flags (3 bytes)
        data.push(0);
        data.extend_from_slice(&[0, 0, 0]);
        // balance (2 bytes)
        data.extend_from_slice(&[0, 0]);
        // reserved (2 bytes)
        data.extend_from_slice(&[0, 0]);
        data
    }

    fn make_nmhd_payload() -> Vec<u8> {
        let mut data = Vec::new();
        // FullBoxHeader: version (1 byte) + flags (3 bytes)
        data.push(0);
        data.extend_from_slice(&[0, 0, 0]);
        data
    }

    fn make_dinf_payload() -> Vec<u8> {
        // dinf contains dref
        let dref_payload = make_dref_payload();
        make_box(b"dref", &dref_payload)
    }

    fn make_dref_payload() -> Vec<u8> {
        let mut data = Vec::new();
        // FullBoxHeader: version (1 byte) + flags (3 bytes)
        data.push(0);
        data.extend_from_slice(&[0, 0, 0]);
        // entry_count (4 bytes)
        data.extend_from_slice(&1u32.to_be_bytes());
        // url entry
        let url_payload = make_url_payload();
        data.extend_from_slice(&make_box(b"url ", &url_payload));
        data
    }

    fn make_url_payload() -> Vec<u8> {
        let mut data = Vec::new();
        // FullBoxHeader: version (1 byte) + flags (3 bytes)
        // flags = 1 means self-contained (no location string)
        data.push(0);
        data.extend_from_slice(&[0, 0, 1]);
        data
    }

    fn make_stbl_payload() -> Vec<u8> {
        let mut data = Vec::new();

        // stsd
        let stsd_payload = make_stsd_payload();
        data.extend_from_slice(&make_box(b"stsd", &stsd_payload));

        // stts
        let stts_payload = make_stts_payload();
        data.extend_from_slice(&make_box(b"stts", &stts_payload));

        // stsc
        let stsc_payload = make_stsc_payload();
        data.extend_from_slice(&make_box(b"stsc", &stsc_payload));

        // stco
        let stco_payload = make_stco_payload();
        data.extend_from_slice(&make_box(b"stco", &stco_payload));

        data
    }

    fn make_stsd_payload() -> Vec<u8> {
        let mut data = Vec::new();
        // FullBoxHeader: version (1 byte) + flags (3 bytes)
        data.push(0);
        data.extend_from_slice(&[0, 0, 0]);
        // entry_count (4 bytes)
        data.extend_from_slice(&0u32.to_be_bytes());
        data
    }

    fn make_stts_payload() -> Vec<u8> {
        let mut data = Vec::new();
        // FullBoxHeader: version (1 byte) + flags (3 bytes)
        data.push(0);
        data.extend_from_slice(&[0, 0, 0]);
        // entry_count (4 bytes)
        data.extend_from_slice(&0u32.to_be_bytes());
        data
    }

    fn make_stsc_payload() -> Vec<u8> {
        let mut data = Vec::new();
        // FullBoxHeader: version (1 byte) + flags (3 bytes)
        data.push(0);
        data.extend_from_slice(&[0, 0, 0]);
        // entry_count (4 bytes)
        data.extend_from_slice(&0u32.to_be_bytes());
        data
    }

    fn make_stco_payload() -> Vec<u8> {
        let mut data = Vec::new();
        // FullBoxHeader: version (1 byte) + flags (3 bytes)
        data.push(0);
        data.extend_from_slice(&[0, 0, 0]);
        // entry_count (4 bytes)
        data.extend_from_slice(&0u32.to_be_bytes());
        data
    }

    fn make_minf_payload_with_vmhd() -> Vec<u8> {
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

    fn make_minf_payload_with_smhd() -> Vec<u8> {
        let mut data = Vec::new();

        // smhd
        let smhd_payload = make_smhd_payload();
        data.extend_from_slice(&make_box(b"smhd", &smhd_payload));

        // dinf
        let dinf_payload = make_dinf_payload();
        data.extend_from_slice(&make_box(b"dinf", &dinf_payload));

        // stbl
        let stbl_payload = make_stbl_payload();
        data.extend_from_slice(&make_box(b"stbl", &stbl_payload));

        data
    }

    #[test]
    fn parse_minf_with_vmhd() {
        let payload = make_minf_payload_with_vmhd();
        let minf = MinfBoxView::parse(&payload).unwrap();

        let media_header = minf.media_header().unwrap();
        assert!(matches!(media_header, MediaHeader::Vmhd(_)));

        assert!(minf.vmhd().unwrap().is_some());
        assert!(minf.smhd().unwrap().is_none());
        assert!(minf.dinf().is_ok());
        assert!(minf.stbl().is_ok());
    }

    #[test]
    fn parse_minf_with_smhd() {
        let payload = make_minf_payload_with_smhd();
        let minf = MinfBoxView::parse(&payload).unwrap();

        let media_header = minf.media_header().unwrap();
        assert!(matches!(media_header, MediaHeader::Smhd(_)));

        assert!(minf.vmhd().unwrap().is_none());
        assert!(minf.smhd().unwrap().is_some());
    }

    #[test]
    fn parse_minf_with_nmhd() {
        let mut payload = Vec::new();

        // nmhd
        let nmhd_payload = make_nmhd_payload();
        payload.extend_from_slice(&make_box(b"nmhd", &nmhd_payload));

        // dinf
        let dinf_payload = make_dinf_payload();
        payload.extend_from_slice(&make_box(b"dinf", &dinf_payload));

        // stbl
        let stbl_payload = make_stbl_payload();
        payload.extend_from_slice(&make_box(b"stbl", &stbl_payload));

        let minf = MinfBoxView::parse(&payload).unwrap();

        let media_header = minf.media_header().unwrap();
        assert!(matches!(media_header, MediaHeader::Nmhd(_)));
    }

    #[test]
    fn parse_minf_missing_media_header() {
        let mut payload = Vec::new();

        // dinf only, no media header
        let dinf_payload = make_dinf_payload();
        payload.extend_from_slice(&make_box(b"dinf", &dinf_payload));

        // stbl
        let stbl_payload = make_stbl_payload();
        payload.extend_from_slice(&make_box(b"stbl", &stbl_payload));

        let minf = MinfBoxView::parse(&payload).unwrap();
        let result = minf.media_header();
        assert!(result.is_err());
    }

    #[test]
    fn parse_minf_multiple_media_headers() {
        let mut payload = Vec::new();

        // vmhd
        let vmhd_payload = make_vmhd_payload();
        payload.extend_from_slice(&make_box(b"vmhd", &vmhd_payload));

        // smhd (duplicate media header - should error)
        let smhd_payload = make_smhd_payload();
        payload.extend_from_slice(&make_box(b"smhd", &smhd_payload));

        // dinf
        let dinf_payload = make_dinf_payload();
        payload.extend_from_slice(&make_box(b"dinf", &dinf_payload));

        // stbl
        let stbl_payload = make_stbl_payload();
        payload.extend_from_slice(&make_box(b"stbl", &stbl_payload));

        let minf = MinfBoxView::parse(&payload).unwrap();
        let result = minf.media_header();
        assert!(result.is_err());
    }

    #[test]
    fn parse_minf_missing_dinf() {
        let mut payload = Vec::new();

        // vmhd
        let vmhd_payload = make_vmhd_payload();
        payload.extend_from_slice(&make_box(b"vmhd", &vmhd_payload));

        // stbl only, no dinf
        let stbl_payload = make_stbl_payload();
        payload.extend_from_slice(&make_box(b"stbl", &stbl_payload));

        let minf = MinfBoxView::parse(&payload).unwrap();
        let result = minf.dinf();
        assert!(result.is_err());
    }

    #[test]
    fn parse_minf_missing_stbl() {
        let mut payload = Vec::new();

        // vmhd
        let vmhd_payload = make_vmhd_payload();
        payload.extend_from_slice(&make_box(b"vmhd", &vmhd_payload));

        // dinf only, no stbl
        let dinf_payload = make_dinf_payload();
        payload.extend_from_slice(&make_box(b"dinf", &dinf_payload));

        let minf = MinfBoxView::parse(&payload).unwrap();
        let result = minf.stbl();
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn parse_minf_owned() {
        let payload = make_minf_payload_with_vmhd();
        let minf = MinfBox::parse(&payload).unwrap();

        assert!(matches!(minf.media_header, MediaHeader::Vmhd(_)));
    }
}
