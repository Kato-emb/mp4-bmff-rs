use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::BoxIter;

use crate::boxes::DinfBoxView;
use crate::boxes::HmhdBox;
use crate::boxes::NmhdBox;
use crate::boxes::SmhdBox;
use crate::boxes::StblBoxView;
use crate::boxes::VmhdBox;

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

    /// Returns the Video Media Header Box (`vmhd`) if present.
    pub fn vmhd(&self) -> Result<Option<VmhdBox>> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::VMHD {
                let vmhd = VmhdBox::decode(child.payload())?;
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
                let smhd = SmhdBox::decode(child.payload())?;
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
                let hmhd = HmhdBox::decode(child.payload())?;
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
                let nmhd = NmhdBox::decode(child.payload())?;
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
                let dinf = DinfBoxView::decode(child.into_payload())?;
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
                let stbl = StblBoxView::decode(child.into_payload())?;
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
}

impl BoxCodec for MinfBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::MINF
    }
}

impl<'de> BoxDecode<'de> for MinfBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        Ok(MinfBoxView { payload: bytes })
    }
}

impl<'a> TryFrom<&'a [u8]> for MinfBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        MinfBoxView::decode(value)
    }
}

#[cfg(feature = "alloc")]
pub use owned::MinfBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::codec::boxed_len;
    use crate::codec::write_box_in;
    use crate::cursor::WriteCursor;

    use super::*;
    use crate::BoxEncode;
    use crate::boxes::DinfBox;
    use crate::boxes::StblBox;

    /// An owned Media Information Box (`minf`).
    #[derive(Debug, Clone)]
    pub struct MinfBox {
        /// The Video Media Header Box (`vmhd`), if present.
        pub vmhd: Option<VmhdBox>,
        /// The Sound Media Header Box (`smhd`), if present.
        pub smhd: Option<SmhdBox>,
        /// The Hint Media Header Box (`hmhd`), if present.
        pub hmhd: Option<HmhdBox>,
        /// The Null Media Header Box (`nmhd`), if present.
        pub nmhd: Option<NmhdBox>,
        /// The Data Information Box (`dinf`).
        pub dinf: DinfBox,
        /// The Sample Table Box (`stbl`).
        pub stbl: StblBox,
    }

    impl MinfBox {
        /// Validates the `MinfBox` structure.
        pub fn validate(&self) -> Result<()> {
            if self.vmhd.is_some() as u8
                + self.smhd.is_some() as u8
                + self.hmhd.is_some() as u8
                + self.nmhd.is_some() as u8
                != 1
            {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxField {
                        field: "Media header",
                        reason: "exactly one media header box must be present",
                    },
                    BoxType::MINF,
                ));
            }

            Ok(())
        }
    }

    impl TryFrom<&MinfBoxView<'_>> for MinfBox {
        type Error = Error;

        fn try_from(view: &MinfBoxView<'_>) -> Result<Self> {
            let mut vmhd = None;
            let mut smhd = None;
            let mut hmhd = None;
            let mut nmhd = None;
            let mut dinf = None;
            let mut stbl = None;

            for child in view.children() {
                let child = child?;

                match child.boxtype() {
                    BoxType::VMHD if vmhd.is_none() => {
                        let vmhd_owned = VmhdBox::decode(child.payload())?;
                        vmhd = Some(vmhd_owned);
                    }
                    BoxType::SMHD if smhd.is_none() => {
                        let smhd_owned = SmhdBox::decode(child.payload())?;
                        smhd = Some(smhd_owned);
                    }
                    BoxType::HMHD if hmhd.is_none() => {
                        let hmhd_owned = HmhdBox::decode(child.payload())?;
                        hmhd = Some(hmhd_owned);
                    }
                    BoxType::NMHD if nmhd.is_none() => {
                        let nmhd_owned = NmhdBox::decode(child.payload())?;
                        nmhd = Some(nmhd_owned);
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
                        let dinf_view = DinfBoxView::decode(child.payload())?;
                        dinf = Some(DinfBox::try_from(&dinf_view)?);
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
                        let stbl_view = StblBoxView::decode(child.payload())?;
                        stbl = Some(StblBox::try_from(&stbl_view)?);
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
                vmhd,
                smhd,
                hmhd,
                nmhd,
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
            let mut len = 0;

            if let Some(vmhd) = &self.vmhd {
                len += boxed_len(vmhd);
            }

            if let Some(smhd) = &self.smhd {
                len += boxed_len(smhd);
            }

            if let Some(hmhd) = &self.hmhd {
                len += boxed_len(hmhd);
            }

            if let Some(nmhd) = &self.nmhd {
                len += boxed_len(nmhd);
            }

            len += boxed_len(&self.dinf);
            len += boxed_len(&self.stbl);

            len
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            self.validate()?;

            let mut cur = WriteCursor::new(bytes);

            // dinf
            write_box_in(&mut cur, &self.dinf)?;
            // stbl
            write_box_in(&mut cur, &self.stbl)?;

            Ok(cur.position())
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
        let minf = MinfBoxView::decode(&payload).unwrap();

        assert!(minf.vmhd().unwrap().is_some());
        assert!(minf.smhd().unwrap().is_none());
        assert!(minf.dinf().is_ok());
        assert!(minf.stbl().is_ok());
    }

    #[test]
    fn parse_minf_with_smhd() {
        let payload = make_minf_payload_with_smhd();
        let minf = MinfBoxView::decode(&payload).unwrap();

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

        let minf = MinfBoxView::decode(&payload).unwrap();

        assert!(minf.nmhd().unwrap().is_some());
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

        let minf = MinfBoxView::decode(&payload).unwrap();
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

        let minf = MinfBoxView::decode(&payload).unwrap();
        let result = minf.stbl();
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn parse_minf_owned() {
        let payload = make_minf_payload_with_vmhd();
        let minf = MinfBox::decode(&payload).unwrap();

        minf.validate().unwrap();
        assert!(minf.vmhd.is_some());
        assert!(minf.smhd.is_none());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn parse_minf_owned_invalid_multiple_headers() {
        let mut payload = Vec::new();

        // vmhd
        let vmhd_payload = make_vmhd_payload();
        payload.extend_from_slice(&make_box(b"vmhd", &vmhd_payload));

        // smhd
        let smhd_payload = make_smhd_payload();
        payload.extend_from_slice(&make_box(b"smhd", &smhd_payload));

        // dinf
        let dinf_payload = make_dinf_payload();
        payload.extend_from_slice(&make_box(b"dinf", &dinf_payload));

        // stbl
        let stbl_payload = make_stbl_payload();
        payload.extend_from_slice(&make_box(b"stbl", &stbl_payload));

        let minf = MinfBox::decode(&payload).unwrap();
        let result = minf.validate();
        assert!(result.is_err());
    }
}
