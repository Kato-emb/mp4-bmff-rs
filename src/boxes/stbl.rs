use crate::cursor::ReadCursor;

use crate::BoxFrame;
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
                let stsd = StsdBoxView::parse(child.payload())?;
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
                let stts = SttsBoxView::parse(child.payload())?;
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
                let stsc = StscBoxView::parse(child.payload())?;
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
                let ctts = CttsBoxView::parse(child.payload())?;
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
                let cslg = CslgBox::parse(child.payload())?;
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
                let stsz = StszBoxView::parse(child.payload())?;
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
                let stss = StssBoxView::parse(child.payload())?;
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
                let stco = StcoBoxView::parse(child.payload())?;
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
                let co64 = Co64BoxView::parse(child.payload())?;
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
                    let stco = StcoBoxView::parse(child.payload())?;
                    chunk_offsets = Some(ChunkOffsetsView::Stco(stco));
                }
                BoxType::CO64 if chunk_offsets.is_none() => {
                    let co64 = Co64BoxView::parse(child.payload())?;
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

impl<'a> TryFrom<BoxFrame<'a>> for StblBoxView<'a> {
    type Error = Error;

    fn try_from(value: BoxFrame<'a>) -> Result<Self> {
        if value.boxtype() != BoxType::STBL {
            return Err(Error::new(ErrorKind::MismatchedBoxType {
                expected: BoxType::STBL,
                found: value.boxtype(),
            }));
        }

        StblBoxView::parse(value.payload())
    }
}

#[cfg(feature = "alloc")]
pub use owned::StblBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::cursor::WriteCursor;

    use crate::BoxFrameMut;
    use crate::base::frame::write_box_in;

    use super::*;
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

        /// Returns the size of the payload in bytes.
        pub fn size(&self) -> usize {
            match self {
                ChunkOffsets::Stco(stco) => stco.size(),
                ChunkOffsets::Co64(co64) => co64.size(),
            }
        }

        /// Returns the total frame size (including box header).
        pub fn frame_size(&self) -> usize {
            BoxFrameMut::required_len(self.boxtype(), self.size())
        }

        fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
            let boxtype = self.boxtype();
            let payload_len = self.size();
            write_box_in(cur, boxtype, payload_len, |p| match self {
                ChunkOffsets::Stco(stco) => stco.write(p),
                ChunkOffsets::Co64(co64) => co64.write(p),
            })?;

            Ok(())
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

                match child.boxtype() {
                    BoxType::STSD if stsd.is_none() => {
                        let stsd_view = StsdBoxView::parse(child.payload())?;
                        stsd = Some(StsdBox::from_view(&stsd_view)?);
                    }
                    BoxType::STTS if stts.is_none() => {
                        let stts_view = SttsBoxView::parse(child.payload())?;
                        stts = Some(SttsBox::from_view(&stts_view)?);
                    }
                    BoxType::CTTS if ctts.is_none() => {
                        let ctts_view = CttsBoxView::parse(child.payload())?;
                        ctts = Some(CttsBox::from_view(&ctts_view)?);
                    }
                    BoxType::CSLG if cslg.is_none() => {
                        cslg = Some(CslgBox::parse(child.payload())?);
                    }
                    BoxType::STSC if stsc.is_none() => {
                        let stsc_view = StscBoxView::parse(child.payload())?;
                        stsc = Some(StscBox::from_view(&stsc_view)?);
                    }
                    BoxType::STSZ if stsz.is_none() => {
                        let stsz_view = StszBoxView::parse(child.payload())?;
                        stsz = Some(StszBox::from_view(&stsz_view)?);
                    }
                    BoxType::STSS if stss.is_none() => {
                        let stss_view = StssBoxView::parse(child.payload())?;
                        stss = Some(StssBox::from_view(&stss_view)?);
                    }
                    BoxType::STCO if chunk_offsets.is_none() => {
                        let stco_view = StcoBoxView::parse(child.payload())?;
                        chunk_offsets = Some(ChunkOffsets::Stco(StcoBox::from_view(&stco_view)?));
                    }
                    BoxType::CO64 if chunk_offsets.is_none() => {
                        let co64_view = Co64BoxView::parse(child.payload())?;
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

        /// Returns the size of the payload in bytes.
        pub fn size(&self) -> usize {
            let mut size = 0;

            // stsd (required)
            size += BoxFrameMut::required_len(BoxType::STSD, self.stsd.size());

            // stts (required)
            size += BoxFrameMut::required_len(BoxType::STTS, self.stts.size());

            // ctts (optional)
            if let Some(ref ctts) = self.ctts {
                size += BoxFrameMut::required_len(BoxType::CTTS, ctts.size());
            }

            // cslg (optional)
            if let Some(ref cslg) = self.cslg {
                size += BoxFrameMut::required_len(BoxType::CSLG, cslg.size());
            }

            // stsc (required)
            size += BoxFrameMut::required_len(BoxType::STSC, self.stsc.size());

            // stsz (optional)
            if let Some(ref stsz) = self.stsz {
                size += BoxFrameMut::required_len(BoxType::STSZ, stsz.size());
            }

            // stss (optional)
            if let Some(ref stss) = self.stss {
                size += BoxFrameMut::required_len(BoxType::STSS, stss.size());
            }

            // chunk_offsets (required)
            size += self.chunk_offsets.frame_size();

            size
        }

        pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
            // stsd
            write_box_in(cur, BoxType::STSD, self.stsd.size(), |p| self.stsd.write(p))?;

            // stts
            write_box_in(cur, BoxType::STTS, self.stts.size(), |p| self.stts.write(p))?;

            // ctts (optional)
            if let Some(ref ctts) = self.ctts {
                write_box_in(cur, BoxType::CTTS, ctts.size(), |p| ctts.write(p))?;
            }

            // cslg (optional)
            if let Some(ref cslg) = self.cslg {
                write_box_in(cur, BoxType::CSLG, cslg.size(), |p| cslg.write(p))?;
            }

            // stsc
            write_box_in(cur, BoxType::STSC, self.stsc.size(), |p| self.stsc.write(p))?;

            // stsz (optional)
            if let Some(ref stsz) = self.stsz {
                write_box_in(cur, BoxType::STSZ, stsz.size(), |p| stsz.write(p))?;
            }

            // stss (optional)
            if let Some(ref stss) = self.stss {
                write_box_in(cur, BoxType::STSS, stss.size(), |p| stss.write(p))?;
            }

            // chunk_offsets
            self.chunk_offsets.write_in(cur)?;

            if !cur.is_empty() {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxSize {
                        reason: "Buffer larger than expected",
                        got: cur.remaining() as u64,
                    },
                    BoxType::STBL,
                ));
            }

            Ok(())
        }

        /// Writes this `StblBox` into the given payload.
        pub fn write(&self, payload: &mut [u8]) -> Result<()> {
            let mut cursor = WriteCursor::new(payload);
            self.write_in(&mut cursor)
        }
    }

    impl TryFrom<&StblBoxView<'_>> for StblBox {
        type Error = Error;

        fn try_from(value: &StblBoxView<'_>) -> Result<Self> {
            StblBox::from_view(value)
        }
    }

    impl TryFrom<BoxFrame<'_>> for StblBox {
        type Error = Error;

        fn try_from(value: BoxFrame<'_>) -> Result<Self> {
            if value.boxtype() != BoxType::STBL {
                return Err(Error::new(ErrorKind::MismatchedBoxType {
                    expected: BoxType::STBL,
                    found: value.boxtype(),
                }));
            }

            let view = StblBoxView::parse(value.payload())?;
            StblBox::from_view(&view)
        }
    }
}
