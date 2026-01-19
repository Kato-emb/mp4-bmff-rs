use crate::cursor::ReadCursor;

use crate::BoxIter;
use crate::BoxType;
use crate::BoxView;
use crate::error::*;

use crate::boxes::Co64BoxView;
use crate::boxes::CslgBox;
use crate::boxes::CttsBoxView;
use crate::boxes::StcoBoxView;
use crate::boxes::StscBoxView;
use crate::boxes::StsdBoxView;
use crate::boxes::StssBoxView;
use crate::boxes::StszBoxView;
use crate::boxes::SttsBoxView;

/// An enum representing either a `stco` or `co64` chunk offsets box.
#[derive(Debug)]
pub enum ChunkOffsetsView<'a> {
    /// A 32-bit Chunk Offset Box (`stco`).
    Stco(StcoBoxView<'a>),
    /// A 64-bit Chunk Offset Box (`co64`).
    Co64(Co64BoxView<'a>),
}

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
            if child.header.boxtype() == BoxType::STSD {
                let stsd = StsdBoxView::parse(child.payload)?;
                return Ok(stsd);
            }
        }

        Err(Error::new(ErrorKind::BoxMissing {
            required: BoxType::STSD,
        }))
    }

    /// Returns the Time-to-Sample Box (`stts`) if present.
    pub fn stts(&self) -> Result<SttsBoxView<'a>> {
        for child in self.children() {
            let child = child?;
            if child.header.boxtype() == BoxType::STTS {
                let stts = SttsBoxView::parse(child.payload)?;
                return Ok(stts);
            }
        }

        Err(Error::new(ErrorKind::BoxMissing {
            required: BoxType::STTS,
        }))
    }

    /// Returns the Sample-to-Chunk Box (`stsc`) if present.
    pub fn stsc(&self) -> Result<StscBoxView<'a>> {
        for child in self.children() {
            let child = child?;
            if child.header.boxtype() == BoxType::STSC {
                let stsc = StscBoxView::parse(child.payload)?;
                return Ok(stsc);
            }
        }

        Err(Error::new(ErrorKind::BoxMissing {
            required: BoxType::STSC,
        }))
    }

    /// Returns the Composition Time to Sample Box (`ctts`) if present.
    pub fn ctts(&self) -> Result<Option<CttsBoxView<'a>>> {
        for child in self.children() {
            let child = child?;
            if child.header.boxtype() == BoxType::CTTS {
                let ctts = CttsBoxView::parse(child.payload)?;
                return Ok(Some(ctts));
            }
        }

        Ok(None)
    }

    /// Returns the Composition to Decode Timeline Mapping Box (`cslg`) if present.
    pub fn cslg(&self) -> Result<Option<CslgBox>> {
        for child in self.children() {
            let child = child?;
            if child.header.boxtype() == BoxType::CSLG {
                let cslg = CslgBox::parse(child.payload)?;
                return Ok(Some(cslg));
            }
        }

        Ok(None)
    }

    /// Returns the Sample Size Box (`stsz`) if present.
    pub fn stsz(&self) -> Result<Option<StszBoxView<'a>>> {
        for child in self.children() {
            let child = child?;
            if child.header.boxtype() == BoxType::STSZ {
                let stsz = StszBoxView::parse(child.payload)?;
                return Ok(Some(stsz));
            }
        }

        Ok(None)
    }

    /// Returns the Sync Sample Box (`stss`) if present.
    pub fn stss(&self) -> Result<Option<StssBoxView<'a>>> {
        for child in self.children() {
            let child = child?;
            if child.header.boxtype() == BoxType::STSS {
                let stss = StssBoxView::parse(child.payload)?;
                return Ok(Some(stss));
            }
        }

        Ok(None)
    }

    /// Returns the Chunk Offset Box (`stco`) if present.
    pub fn stco(&self) -> Result<Option<StcoBoxView<'a>>> {
        for child in self.children() {
            let child = child?;
            if child.header.boxtype() == BoxType::STCO {
                let stco = StcoBoxView::parse(child.payload)?;
                return Ok(Some(stco));
            }
        }

        Ok(None)
    }

    /// Returns the 64-bit Chunk Offset Box (`co64`) if present.
    pub fn co64(&self) -> Result<Option<Co64BoxView<'a>>> {
        for child in self.children() {
            let child = child?;
            if child.header.boxtype() == BoxType::CO64 {
                let co64 = Co64BoxView::parse(child.payload)?;
                return Ok(Some(co64));
            }
        }

        Ok(None)
    }

    /// Returns the Chunk Offsets Box (`stco` or `co64`) if present.
    pub fn chunk_offsets(&self) -> Result<ChunkOffsetsView<'a>> {
        let mut chunk_offsets = None;

        for child in self.children() {
            let child = child?;
            match child.header.boxtype() {
                BoxType::STCO if chunk_offsets.is_none() => {
                    let stco = StcoBoxView::parse(child.payload)?;
                    chunk_offsets = Some(ChunkOffsetsView::Stco(stco));
                }
                BoxType::CO64 if chunk_offsets.is_none() => {
                    let co64 = Co64BoxView::parse(child.payload)?;
                    chunk_offsets = Some(ChunkOffsetsView::Co64(co64));
                }
                BoxType::STCO | BoxType::CO64 => {
                    return Err(Error::new(ErrorKind::InvalidBoxField {
                        field: "Chunk offsets",
                        reason: "multiple chunk offset boxes found",
                    })
                    .with_box_type(BoxType::STBL));
                }
                _ => continue,
            }
        }

        Err(Error::new(ErrorKind::InvalidBoxField {
            field: "Chunk offsets",
            reason: "no chunk offset box found",
        })
        .with_box_type(BoxType::STBL))
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<StblBoxView<'a>> {
        let payload = cur.take(cur.remaining())?;
        Ok(StblBoxView { payload })
    }

    /// Parses a `StblBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<StblBoxView<'a>> {
        let mut cursor = ReadCursor::new(payload);
        let this = StblBoxView::parse_in(&mut cursor)?;

        Ok(this)
    }
}

impl<'a> TryFrom<&BoxView<'a>> for StblBoxView<'a> {
    type Error = Error;

    fn try_from(value: &BoxView<'a>) -> Result<Self> {
        if value.header.boxtype() != BoxType::STBL {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::STBL,
                found: value.header.boxtype(),
            }));
        }

        StblBoxView::parse(value.payload)
    }
}

#[cfg(feature = "alloc")]
pub use owned::StblBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::boxes::Co64Box;
    use crate::boxes::CttsBox;
    use crate::boxes::StcoBox;
    use crate::boxes::StscBox;
    use crate::boxes::StsdBox;
    use crate::boxes::StssBox;
    use crate::boxes::StszBox;
    use crate::boxes::SttsBox;

    use super::*;

    #[derive(Debug, Clone)]
    pub enum ChunkOffsets {
        Stco(StcoBox),
        Co64(Co64Box),
    }

    /// An owned Sample Table Box (`stbl`).
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
        /// The Chunk Offset Box (`stco` or `co64`).
        pub chunk_offsets: ChunkOffsets,
    }

    impl StblBox {
        /// Constructs a `StblBox` from a `StblBoxView`.
        pub fn from_view(view: &StblBoxView<'_>) -> Result<StblBox> {
            let mut stsd = None;
            let mut stts = None;
            let mut ctts = None;
            let mut cslg = None;
            let mut stsc = None;
            let mut stsz = None;
            let mut stss = None;
            let mut chunk_offsets = None;

            for child in view.children() {
                let child = child?;

                match child.header.boxtype() {
                    BoxType::STSD if stsd.is_none() => {
                        let stsd_view = StsdBoxView::parse(child.payload)?;
                        stsd = Some(StsdBox::from_view(&stsd_view)?);
                    }
                    BoxType::STTS if stts.is_none() => {
                        let stts_view = SttsBoxView::parse(child.payload)?;
                        stts = Some(SttsBox::from_view(&stts_view)?);
                    }
                    BoxType::CTTS if ctts.is_none() => {
                        let ctts_view = CttsBoxView::parse(child.payload)?;
                        ctts = Some(CttsBox::from_view(&ctts_view)?);
                    }
                    BoxType::CSLG if cslg.is_none() => {
                        cslg = Some(CslgBox::parse(child.payload)?);
                    }
                    BoxType::STSC if stsc.is_none() => {
                        let stsc_view = StscBoxView::parse(child.payload)?;
                        stsc = Some(StscBox::from_view(&stsc_view)?);
                    }
                    BoxType::STSZ if stsz.is_none() => {
                        let stsz_view = StszBoxView::parse(child.payload)?;
                        stsz = Some(StszBox::from_view(&stsz_view)?);
                    }
                    BoxType::STSS if stss.is_none() => {
                        let stss_view = StssBoxView::parse(child.payload)?;
                        stss = Some(StssBox::from_view(&stss_view)?);
                    }
                    BoxType::STCO if chunk_offsets.is_none() => {
                        let stco_view = StcoBoxView::parse(child.payload)?;
                        chunk_offsets = Some(ChunkOffsets::Stco(StcoBox::from_view(&stco_view)?));
                    }
                    BoxType::CO64 if chunk_offsets.is_none() => {
                        let co64_view = Co64BoxView::parse(child.payload)?;
                        chunk_offsets = Some(ChunkOffsets::Co64(Co64Box::from_view(&co64_view)?));
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
                chunk_offsets: chunk_offsets.ok_or(Error::in_box(
                    ErrorKind::InvalidBoxField {
                        field: "Chunk offsets",
                        reason: "no chunk offset box found",
                    },
                    BoxType::STBL,
                ))?,
            })
        }

        /// Parses a `StblBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<StblBox> {
            let view = StblBoxView::parse(payload)?;
            StblBox::from_view(&view)
        }
    }

    impl TryFrom<&StblBoxView<'_>> for StblBox {
        type Error = Error;

        fn try_from(value: &StblBoxView<'_>) -> Result<Self> {
            StblBox::from_view(value)
        }
    }

    impl TryFrom<&BoxView<'_>> for StblBox {
        type Error = Error;

        fn try_from(value: &BoxView<'_>) -> Result<Self> {
            if value.header.boxtype() != BoxType::STBL {
                return Err(Error::new(ErrorKind::MismatchedBoxType {
                    expected: BoxType::STBL,
                    found: value.header.boxtype(),
                }));
            }

            let view = StblBoxView::parse(value.payload)?;
            StblBox::from_view(&view)
        }
    }
}
