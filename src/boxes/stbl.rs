use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::BoxIter;

use crate::boxes::Co64BoxView;
use crate::boxes::CslgBox;
use crate::boxes::CttsBoxView;
use crate::boxes::StcoBoxView;
use crate::boxes::StscBoxView;
use crate::boxes::StsdBoxView;
use crate::boxes::StssBoxView;
use crate::boxes::StszBoxView;
use crate::boxes::SttsBoxView;

/// A reference to a Sample Table Box (`stbl`).
#[derive(Debug)]
pub struct StblBoxView<'a> {
    payload: &'a [u8],
}

impl<'a> StblBoxView<'a> {
    /// Returns an iterator over the child boxes of this `StblBoxView`.
    pub fn children(&self) -> BoxIter<'a> {
        BoxIter::new(self.payload)
    }

    /// Returns the Sample Description Box (`stsd`) if present.
    pub fn stsd(&self) -> Result<StsdBoxView<'a>> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::STSD {
                let stsd = StsdBoxView::decode(child.into_payload())?;
                return Ok(stsd);
            }
        }

        Err(Error::in_box(
            ErrorKind::BoxMissing {
                required: BoxType::STSD,
            },
            BoxType::STBL,
        ))
    }

    /// Returns the Time-to-Sample Box (`stts`) if present.
    pub fn stts(&self) -> Result<SttsBoxView<'a>> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::STTS {
                let stts = SttsBoxView::decode(child.into_payload())?;
                return Ok(stts);
            }
        }

        Err(Error::in_box(
            ErrorKind::BoxMissing {
                required: BoxType::STTS,
            },
            BoxType::STBL,
        ))
    }

    /// Returns the Sample-to-Chunk Box (`stsc`) if present.
    pub fn stsc(&self) -> Result<StscBoxView<'a>> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::STSC {
                let stsc = StscBoxView::decode(child.into_payload())?;
                return Ok(stsc);
            }
        }

        Err(Error::in_box(
            ErrorKind::BoxMissing {
                required: BoxType::STSC,
            },
            BoxType::STBL,
        ))
    }

    /// Returns the Composition Time to Sample Box (`ctts`) if present.
    pub fn ctts(&self) -> Result<Option<CttsBoxView<'a>>> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::CTTS {
                let ctts = CttsBoxView::decode(child.into_payload())?;
                return Ok(Some(ctts));
            }
        }

        Ok(None)
    }

    /// Returns the Composition to Decode Timeline Mapping Box (`cslg`) if present.
    pub fn cslg(&self) -> Result<Option<CslgBox>> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::CSLG {
                let cslg = CslgBox::decode(child.into_payload())?;
                return Ok(Some(cslg));
            }
        }

        Ok(None)
    }

    /// Returns the Sample Size Box (`stsz`) if present.
    pub fn stsz(&self) -> Result<Option<StszBoxView<'a>>> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::STSZ {
                let stsz = StszBoxView::decode(child.into_payload())?;
                return Ok(Some(stsz));
            }
        }

        Ok(None)
    }

    /// Returns the Sync Sample Box (`stss`) if present.
    pub fn stss(&self) -> Result<Option<StssBoxView<'a>>> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::STSS {
                let stss = StssBoxView::decode(child.into_payload())?;
                return Ok(Some(stss));
            }
        }

        Ok(None)
    }

    /// Returns the Chunk Offset Box (`stco`) if present.
    pub fn stco(&self) -> Result<Option<StcoBoxView<'a>>> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::STCO {
                let stco = StcoBoxView::decode(child.into_payload())?;
                return Ok(Some(stco));
            }
        }

        Ok(None)
    }

    /// Returns the 64-bit Chunk Offset Box (`co64`) if present.
    pub fn co64(&self) -> Result<Option<Co64BoxView<'a>>> {
        for child in self.children() {
            let child = child?;
            if child.boxtype() == BoxType::CO64 {
                let co64 = Co64BoxView::decode(child.into_payload())?;
                return Ok(Some(co64));
            }
        }

        Ok(None)
    }
}

impl BoxCodec for StblBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::STBL
    }
}

impl<'de> BoxDecode<'de> for StblBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        Ok(StblBoxView { payload: bytes })
    }
}

impl<'a> TryFrom<&'a [u8]> for StblBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        StblBoxView::decode(value)
    }
}

#[cfg(feature = "alloc")]
pub use owned::StblBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::cursor::WriteCursor;

    use crate::codec::boxed_len;
    use crate::codec::write_box_in;

    use super::*;
    use crate::BoxCodec;
    use crate::BoxDecode;
    use crate::BoxEncode;
    use crate::boxes::Co64Box;
    use crate::boxes::CttsBox;
    use crate::boxes::StcoBox;
    use crate::boxes::StscBox;
    use crate::boxes::StsdBox;
    use crate::boxes::StssBox;
    use crate::boxes::StszBox;
    use crate::boxes::SttsBox;

    /// An owned Sample Table Box (`stbl`).
    #[derive(Debug, Clone)]
    pub struct StblBox {
        /// The Sample Description Box (`stsd`).
        pub stsd: StsdBox,
        /// The Time-to-Sample Box (`stts`).
        pub stts: SttsBox,
        /// The Composition Time to Sample Box (`ctts`), if present.
        pub ctts: Option<CttsBox>,
        /// The Composition to Decode Timeline Mapping Box (`cslg`), if present.
        pub cslg: Option<CslgBox>,
        /// The Sample-to-Chunk Box (`stsc`).
        pub stsc: StscBox,
        /// The Sample Size Box (`stsz`), if present.
        pub stsz: Option<StszBox>,
        /// The Sync Sample Box (`stss`), if present.
        pub stss: Option<StssBox>,
        /// The Chunk Offsets Box (`stco`), use 32-bit offsets.
        pub stco: Option<StcoBox>,
        /// The Chunk Offsets Box (`co64`), use 64-bit offsets.
        pub co64: Option<Co64Box>,
    }

    impl TryFrom<&StblBoxView<'_>> for StblBox {
        type Error = Error;

        fn try_from(view: &StblBoxView<'_>) -> Result<Self> {
            let mut stsd = None;
            let mut stts = None;
            let mut ctts = None;
            let mut cslg = None;
            let mut stsc = None;
            let mut stsz = None;
            let mut stss = None;
            let mut stco = None;
            let mut co64 = None;

            for child in view.children() {
                let child = child?;

                match child.boxtype() {
                    BoxType::STSD if stsd.is_none() => {
                        let stsd_view = StsdBoxView::decode(child.payload())?;
                        stsd = Some(StsdBox::try_from(&stsd_view)?);
                    }
                    BoxType::STTS if stts.is_none() => {
                        let stts_view = SttsBoxView::decode(child.payload())?;
                        stts = Some(SttsBox::from(&stts_view));
                    }
                    BoxType::CTTS if ctts.is_none() => {
                        let ctts_view = CttsBoxView::decode(child.payload())?;
                        ctts = Some(CttsBox::from(&ctts_view));
                    }
                    BoxType::CSLG if cslg.is_none() => {
                        cslg = Some(CslgBox::decode(child.payload())?);
                    }
                    BoxType::STSC if stsc.is_none() => {
                        let stsc_view = StscBoxView::decode(child.payload())?;
                        stsc = Some(StscBox::from(&stsc_view));
                    }
                    BoxType::STSZ if stsz.is_none() => {
                        let stsz_view = StszBoxView::decode(child.payload())?;
                        stsz = Some(StszBox::from(&stsz_view));
                    }
                    BoxType::STSS if stss.is_none() => {
                        let stss_view = StssBoxView::decode(child.payload())?;
                        stss = Some(StssBox::from(&stss_view));
                    }
                    BoxType::STCO if stco.is_none() => {
                        let stco_view = StcoBoxView::decode(child.payload())?;
                        stco = Some(StcoBox::from(&stco_view));
                    }
                    BoxType::CO64 if co64.is_none() => {
                        let co64_view = Co64BoxView::decode(child.payload())?;
                        co64 = Some(Co64Box::from(&co64_view));
                    }
                    BoxType::STSD
                    | BoxType::STTS
                    | BoxType::CTTS
                    | BoxType::CSLG
                    | BoxType::STSC
                    | BoxType::STSZ
                    | BoxType::STSS
                    | BoxType::STCO
                    | BoxType::CO64 => {
                        return Err(Error::in_box(
                            ErrorKind::InvalidBoxField {
                                field: "Sample Table Box",
                                reason: "multiple boxes of the same type found",
                            },
                            BoxType::STBL,
                        ));
                    }
                    _ => continue,
                }
            }

            Ok(StblBox {
                stsd: stsd.ok_or(Error::in_box(
                    ErrorKind::BoxMissing {
                        required: BoxType::STSD,
                    },
                    BoxType::STBL,
                ))?,
                stts: stts.ok_or(Error::in_box(
                    ErrorKind::BoxMissing {
                        required: BoxType::STTS,
                    },
                    BoxType::STBL,
                ))?,
                ctts,
                cslg,
                stsc: stsc.ok_or(Error::in_box(
                    ErrorKind::BoxMissing {
                        required: BoxType::STSC,
                    },
                    BoxType::STBL,
                ))?,
                stsz,
                stss,
                stco,
                co64,
            })
        }
    }

    impl BoxCodec for StblBox {
        fn boxtype(&self) -> BoxType {
            BoxType::STBL
        }
    }

    impl BoxDecode<'_> for StblBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = StblBoxView::decode(bytes)?;
            StblBox::try_from(&view)
        }
    }

    impl BoxEncode for StblBox {
        #[inline]
        fn encoded_len(&self) -> usize {
            let mut len = 0;

            // stsd
            len += boxed_len(&self.stsd);

            // stts
            len += boxed_len(&self.stts);

            // ctts (optional)
            if let Some(ref ctts) = self.ctts {
                len += boxed_len(ctts);
            }

            // cslg (optional)
            if let Some(ref cslg) = self.cslg {
                len += boxed_len(cslg);
            }

            // stsc
            len += boxed_len(&self.stsc);

            // stsz (optional)
            if let Some(ref stsz) = self.stsz {
                len += boxed_len(stsz);
            }

            // stss (optional)
            if let Some(ref stss) = self.stss {
                len += boxed_len(stss);
            }

            // stco (opional)
            if let Some(ref stco) = self.stco {
                len += boxed_len(stco);
            }

            // co64 (optional)
            if let Some(ref co64) = self.co64 {
                len += boxed_len(co64);
            }

            len
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            // stsd
            write_box_in(&mut cur, &self.stsd)?;

            // stts
            write_box_in(&mut cur, &self.stts)?;

            // ctts (optional)
            if let Some(ref ctts) = self.ctts {
                write_box_in(&mut cur, ctts)?;
            }

            // cslg (optional)
            if let Some(ref cslg) = self.cslg {
                write_box_in(&mut cur, cslg)?;
            }

            // stsc
            write_box_in(&mut cur, &self.stsc)?;

            // stsz (optional)
            if let Some(ref stsz) = self.stsz {
                write_box_in(&mut cur, stsz)?;
            }

            // stss (optional)
            if let Some(ref stss) = self.stss {
                write_box_in(&mut cur, stss)?;
            }

            if self.stco.is_some() && self.co64.is_some() {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxField {
                        field: "Sample Table Box",
                        reason: "both stco and co64 boxes are present",
                    },
                    BoxType::STBL,
                ));
            } else if self.stco.is_none() && self.co64.is_none() {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxField {
                        field: "Sample Table Box",
                        reason: "neither stco nor co64 box is present",
                    },
                    BoxType::STBL,
                ));
            }

            // stco (opional)
            if let Some(ref stco) = self.stco {
                write_box_in(&mut cur, stco)?;
            }

            // co64 (optional)
            if let Some(ref co64) = self.co64 {
                write_box_in(&mut cur, co64)?;
            }

            Ok(cur.position())
        }
    }
}

#[cfg(all(test, feature = "alloc"))]
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

    fn make_fullbox_payload(version: u8, flags: u32) -> Vec<u8> {
        let mut data = Vec::new();
        data.push(version);
        data.extend_from_slice(&flags.to_be_bytes()[1..4]);
        data
    }

    fn make_stsd_payload() -> Vec<u8> {
        let mut data = make_fullbox_payload(0, 0);
        data.extend_from_slice(&0u32.to_be_bytes()); // entry_count
        data
    }

    fn make_stts_payload() -> Vec<u8> {
        let mut data = make_fullbox_payload(0, 0);
        data.extend_from_slice(&0u32.to_be_bytes()); // entry_count
        data
    }

    fn make_stsc_payload() -> Vec<u8> {
        let mut data = make_fullbox_payload(0, 0);
        data.extend_from_slice(&0u32.to_be_bytes()); // entry_count
        data
    }

    fn make_stco_payload() -> Vec<u8> {
        let mut data = make_fullbox_payload(0, 0);
        data.extend_from_slice(&0u32.to_be_bytes()); // entry_count
        data
    }

    fn make_co64_payload() -> Vec<u8> {
        let mut data = make_fullbox_payload(0, 0);
        data.extend_from_slice(&0u32.to_be_bytes()); // entry_count
        data
    }

    fn make_stbl_with_required_boxes() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&make_box(b"stsd", &make_stsd_payload()));
        data.extend_from_slice(&make_box(b"stts", &make_stts_payload()));
        data.extend_from_slice(&make_box(b"stsc", &make_stsc_payload()));
        data
    }

    #[test]
    fn decode_missing_stsd_returns_error() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_box(b"stts", &make_stts_payload()));
        payload.extend_from_slice(&make_box(b"stsc", &make_stsc_payload()));

        let view = StblBoxView::decode(&payload).unwrap();
        let result = StblBox::try_from(&view);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err.kind(), ErrorKind::BoxMissing { required } if required == BoxType::STSD)
        );
    }

    #[test]
    fn decode_missing_stts_returns_error() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_box(b"stsd", &make_stsd_payload()));
        payload.extend_from_slice(&make_box(b"stsc", &make_stsc_payload()));

        let view = StblBoxView::decode(&payload).unwrap();
        let result = StblBox::try_from(&view);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err.kind(), ErrorKind::BoxMissing { required } if required == BoxType::STTS)
        );
    }

    #[test]
    fn decode_missing_stsc_returns_error() {
        let mut payload = Vec::new();
        payload.extend_from_slice(&make_box(b"stsd", &make_stsd_payload()));
        payload.extend_from_slice(&make_box(b"stts", &make_stts_payload()));

        let view = StblBoxView::decode(&payload).unwrap();
        let result = StblBox::try_from(&view);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err.kind(), ErrorKind::BoxMissing { required } if required == BoxType::STSC)
        );
    }

    #[test]
    fn decode_duplicate_box_returns_error() {
        let mut payload = make_stbl_with_required_boxes();
        payload.extend_from_slice(&make_box(b"stts", &make_stts_payload()));

        let view = StblBoxView::decode(&payload).unwrap();
        let result = StblBox::try_from(&view);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err.kind(), ErrorKind::InvalidBoxField { .. }));
    }

    #[test]
    fn encode_without_chunk_offset_returns_error() {
        use crate::BoxEncode;
        use crate::boxes::{StscBox, StsdBox, SttsBox};

        let stbl = StblBox {
            stsd: StsdBox {
                version: 0,
                flags: Default::default(),
                entry_count: 0,
                entries: Vec::new(),
            },
            stts: SttsBox::default(),
            ctts: None,
            cslg: None,
            stsc: StscBox::default(),
            stsz: None,
            stss: None,
            stco: None,
            co64: None,
        };

        let mut buf = vec![0u8; stbl.encoded_len()];
        let result = stbl.encode_into(&mut buf);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err.kind(), ErrorKind::InvalidBoxField { reason, .. } if reason.contains("neither"))
        );
    }

    #[test]
    fn encode_with_both_stco_and_co64_returns_error() {
        use crate::BoxEncode;
        use crate::boxes::{Co64Box, StcoBox, StscBox, StsdBox, SttsBox};

        let stbl = StblBox {
            stsd: StsdBox {
                version: 0,
                flags: Default::default(),
                entry_count: 0,
                entries: Vec::new(),
            },
            stts: SttsBox::default(),
            ctts: None,
            cslg: None,
            stsc: StscBox::default(),
            stsz: None,
            stss: None,
            stco: Some(StcoBox::default()),
            co64: Some(Co64Box::default()),
        };

        let mut buf = vec![0u8; stbl.encoded_len()];
        let result = stbl.encode_into(&mut buf);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err.kind(), ErrorKind::InvalidBoxField { reason, .. } if reason.contains("both"))
        );
    }

    #[test]
    fn round_trip_with_stco() {
        use crate::BoxEncode;

        let mut payload = make_stbl_with_required_boxes();
        payload.extend_from_slice(&make_box(b"stco", &make_stco_payload()));

        let view = StblBoxView::decode(&payload).unwrap();
        let original = StblBox::try_from(&view).unwrap();

        let mut encoded = vec![0u8; original.encoded_len()];
        original.encode_into(&mut encoded).unwrap();

        let decoded = StblBox::decode(&encoded).unwrap();

        assert!(original.stco.is_some() && decoded.stco.is_some());
        assert!(original.co64.is_none() && decoded.co64.is_none());
    }

    #[test]
    fn round_trip_with_co64() {
        use crate::BoxEncode;

        let mut payload = make_stbl_with_required_boxes();
        payload.extend_from_slice(&make_box(b"co64", &make_co64_payload()));

        let view = StblBoxView::decode(&payload).unwrap();
        let original = StblBox::try_from(&view).unwrap();

        let mut encoded = vec![0u8; original.encoded_len()];
        original.encode_into(&mut encoded).unwrap();

        let decoded = StblBox::decode(&encoded).unwrap();

        assert!(original.stco.is_none() && decoded.stco.is_none());
        assert!(original.co64.is_some() && decoded.co64.is_some());
    }
}
