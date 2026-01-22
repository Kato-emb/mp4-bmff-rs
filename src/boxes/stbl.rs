use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxIter;
use crate::BoxType;
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

    /// Returns the Chunk Offsets Box (`stco` or `co64`) if present.
    pub fn chunk_offsets(&self) -> Result<ChunkOffsetsView<'a>> {
        let mut chunk_offsets = None;

        for child in self.children() {
            let child = child?;
            match child.boxtype() {
                BoxType::STCO if chunk_offsets.is_none() => {
                    let stco = StcoBoxView::decode(child.into_payload())?;
                    chunk_offsets = Some(ChunkOffsetsView::Stco(stco));
                }
                BoxType::CO64 if chunk_offsets.is_none() => {
                    let co64 = Co64BoxView::decode(child.into_payload())?;
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

        chunk_offsets.ok_or_else(|| {
            Error::new(ErrorKind::InvalidBoxField {
                field: "Chunk offsets",
                reason: "no chunk offset box found",
            })
            .with_box_type(BoxType::STBL)
        })
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

    use crate::base::writer::write_box_in;

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

    #[derive(Debug, Clone)]
    pub enum ChunkOffsets {
        Stco(StcoBox),
        Co64(Co64Box),
    }

    impl ChunkOffsets {
        /// Returns the box type for this chunk offsets box.
        pub fn boxtype(&self) -> BoxType {
            match self {
                ChunkOffsets::Stco(_) => BoxType::STCO,
                ChunkOffsets::Co64(_) => BoxType::CO64,
            }
        }
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
            let mut chunk_offsets = None;

            for child in view.children() {
                let child = child?;

                match child.boxtype() {
                    BoxType::STSD if stsd.is_none() => {
                        let stsd_view = StsdBoxView::decode(child.payload())?;
                        stsd = Some(StsdBox::try_from(&stsd_view)?);
                    }
                    BoxType::STTS if stts.is_none() => {
                        let stts_view = SttsBoxView::decode(child.payload())?;
                        stts = Some(SttsBox::try_from(&stts_view)?);
                    }
                    BoxType::CTTS if ctts.is_none() => {
                        let ctts_view = CttsBoxView::decode(child.payload())?;
                        ctts = Some(CttsBox::try_from(&ctts_view)?);
                    }
                    BoxType::CSLG if cslg.is_none() => {
                        cslg = Some(CslgBox::decode(child.payload())?);
                    }
                    BoxType::STSC if stsc.is_none() => {
                        let stsc_view = StscBoxView::decode(child.payload())?;
                        stsc = Some(StscBox::try_from(&stsc_view)?);
                    }
                    BoxType::STSZ if stsz.is_none() => {
                        let stsz_view = StszBoxView::decode(child.payload())?;
                        stsz = Some(StszBox::try_from(&stsz_view)?);
                    }
                    BoxType::STSS if stss.is_none() => {
                        let stss_view = StssBoxView::decode(child.payload())?;
                        stss = Some(StssBox::try_from(&stss_view)?);
                    }
                    BoxType::STCO if chunk_offsets.is_none() => {
                        let stco_view = StcoBoxView::decode(child.payload())?;
                        chunk_offsets = Some(ChunkOffsets::Stco(StcoBox::try_from(&stco_view)?));
                    }
                    BoxType::CO64 if chunk_offsets.is_none() => {
                        let co64_view = Co64BoxView::decode(child.payload())?;
                        chunk_offsets = Some(ChunkOffsets::Co64(Co64Box::try_from(&co64_view)?));
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
        fn encode(&self, bytes: &mut [u8]) -> Result<usize> {
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

            // chunk_offsets
            match &self.chunk_offsets {
                ChunkOffsets::Stco(stco) => write_box_in(&mut cur, stco)?,
                ChunkOffsets::Co64(co64) => write_box_in(&mut cur, co64)?,
            }

            Ok(cur.position())
        }
    }
}
