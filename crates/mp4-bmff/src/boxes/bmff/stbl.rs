//! Sample Table Box (`stbl`) implementation.
//!
//! The Sample Table Box contains all the time and data indexing of the media
//! samples in a track. Using the tables here, it is possible to locate samples
//! in time, determine their type, size, container, and offset into that container.

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::BoxIter;

use super::Co64BoxView;
use super::CslgBox;
use super::CttsBoxView;
use super::SbgpBoxView;
use super::SdtpBoxView;
use super::SgpdBoxView;
use super::StcoBoxView;
use super::StdpBoxView;
use super::StscBoxView;
use super::StsdBoxView;
use super::StshBoxView;
use super::StssBoxView;
use super::StszBoxView;
use super::SttsBoxView;
use super::Stz2BoxView;

/// A reference to a Sample Table Box (`stbl`).
///
/// The Sample Table Box is the most important container in the media information
/// structure. It contains all data needed to locate and decode samples. The sample
/// table must contain a sample description, time-to-sample mapping, sample sizes,
/// sample-to-chunk mapping, and chunk offsets.
///
/// # Structure
///
/// Required child boxes:
/// - `stsd`: Sample Description Box - describes sample formats.
/// - `stts`: Decoding Time to Sample Box - sample timing.
/// - `stsz`: Sample Size Box - individual sample sizes.
/// - `stsc`: Sample to Chunk Box - sample-to-chunk mapping.
/// - `stco`/`co64`: Chunk Offset Box - chunk file positions.
///
/// Optional child boxes:
/// - `ctts`: Composition Time to Sample Box - composition time offsets.
/// - `cslg`: Composition to Decode Box - timing relationships.
/// - `stss`: Sync Sample Box - identifies keyframes.
/// - `stsh`: Shadow Sync Box - alternative sync points.
/// - `sdtp`: Sample Dependency Type Box - sample dependencies.
/// - `stdp`: Degradation Priority Box - sample priorities.
/// - `sbgp`: Sample to Group Box - sample grouping information (zero or more, one per grouping type).
/// - `sgpd`: Sample Group Description Box - group descriptions (zero or more, one per grouping type).
#[derive(Debug)]
pub struct StblBoxView<'a> {
    content: &'a [u8],
}

impl<'a> StblBoxView<'a> {
    /// Returns an iterator over the child boxes of this `stbl` box.
    pub fn boxes(&self) -> BoxIter<'a> {
        BoxIter::new(self.content)
    }

    /// Returns the Sample Description Box (`stsd`) contained in this `stbl` box.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is malformed or too short.
    pub fn stsd(&self) -> Result<StsdBoxView<'a>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::STSD {
                let stsd = StsdBoxView::decode(b.into_payload())?;
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

    /// Returns the Degradation Priority Box (`stdp`) contained in this `stbl` box, if any.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is malformed or too short.
    pub fn stdp(&self) -> Result<Option<StdpBoxView<'a>>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::STDP {
                let stdp = StdpBoxView::decode(b.into_payload())?;
                return Ok(Some(stdp));
            }
        }

        Ok(None)
    }

    /// Returns the Decoding Time to Sample Box (`stts`) contained in this `stbl` box.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is malformed or too short.
    pub fn stts(&self) -> Result<SttsBoxView<'a>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::STTS {
                let stts = SttsBoxView::decode(b.into_payload())?;
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

    /// Returns the Composition Time to Sample Box (`ctts`) contained in this `stbl` box, if any.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is malformed or too short.
    pub fn ctts(&self) -> Result<Option<CttsBoxView<'a>>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::CTTS {
                let ctts = CttsBoxView::decode(b.into_payload())?;
                return Ok(Some(ctts));
            }
        }

        Ok(None)
    }

    /// Returns the Composition to Decode Box (`cslg`) contained in this `stbl` box, if any.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is malformed or too short.
    pub fn cslg(&self) -> Result<Option<CslgBox>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::CSLG {
                let cslg = CslgBox::decode(b.into_payload())?;
                return Ok(Some(cslg));
            }
        }

        Ok(None)
    }

    /// Returns the Sync Sample Box (`stss`) contained in this `stbl` box, if any.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is malformed or too short.
    pub fn stss(&self) -> Result<Option<StssBoxView<'a>>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::STSS {
                let stss = StssBoxView::decode(b.into_payload())?;
                return Ok(Some(stss));
            }
        }

        Ok(None)
    }

    /// Returns the Shadow Sync Box (`stsh`) contained in this `stbl` box, if any.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is malformed or too short.
    pub fn stsh(&self) -> Result<Option<StshBoxView<'a>>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::STSH {
                let stsh = StshBoxView::decode(b.into_payload())?;
                return Ok(Some(stsh));
            }
        }

        Ok(None)
    }

    /// Returns the Sample Degradation Priority Box (`sdtp`) contained in this `stbl` box, if any.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is malformed or too short.
    pub fn sdtp(&self) -> Result<Option<SdtpBoxView<'a>>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::SDTP {
                let sdtp = SdtpBoxView::decode(b.into_payload())?;
                return Ok(Some(sdtp));
            }
        }

        Ok(None)
    }

    /// Returns the Sample Size Box (`stsz`) contained in this `stbl` box.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is malformed or too short.
    pub fn stsz(&self) -> Result<Option<StszBoxView<'a>>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::STSZ {
                let stsz = StszBoxView::decode(b.into_payload())?;
                return Ok(Some(stsz));
            }
        }

        Ok(None)
    }

    /// Returns the Compact Sample Size Box (`stz2`) contained in this `stbl` box, if any.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is malformed or too short.
    pub fn stz2(&self) -> Result<Option<Stz2BoxView<'a>>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::STZ2 {
                let stz2 = Stz2BoxView::decode(b.into_payload())?;
                return Ok(Some(stz2));
            }
        }

        Ok(None)
    }

    /// Returns the Sample to Chunk Box (`stsc`) contained in this `stbl` box.
    ///
    /// # Errors
    ///
    /// This method will return an error if the `stsc` box is missing or if there are multiple `stsc` boxes, as only one is allowed by the specification.
    pub fn stsc(&self) -> Result<StscBoxView<'a>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::STSC {
                let stsc = StscBoxView::decode(b.into_payload())?;
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

    /// Returns the Chunk Offset Box (`stco`) contained in this `stbl` box.
    ///
    /// # Errors
    ///
    /// This method will return an error if the `stco` box is missing or if there are multiple `stco` boxes, as only one is allowed by the specification.
    /// If the file uses `co64` instead of `stco`, this method will return `Ok(None)`.
    pub fn stco(&self) -> Result<Option<StcoBoxView<'a>>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::STCO {
                let stco = StcoBoxView::decode(b.into_payload())?;
                return Ok(Some(stco));
            }
        }

        Ok(None)
    }

    /// Returns the Chunk Large Offset Box (`co64`) contained in this `stbl` box, if any.
    ///
    /// # Errors
    ///
    /// This method will return an error if there are multiple `co64` boxes, as only one is allowed by the specification.
    /// If the file uses `stco` instead of `co64`, this method will return `Ok(None)`.
    pub fn co64(&self) -> Result<Option<Co64BoxView<'a>>> {
        for b in self.boxes() {
            let b = b?;
            if b.boxtype() == BoxType::CO64 {
                let co64 = Co64BoxView::decode(b.into_payload())?;
                return Ok(Some(co64));
            }
        }

        Ok(None)
    }

    /// Returns an iterator over the Sample to Group Boxes (`sbgp`) contained in this sample table.
    ///
    /// There may be zero or more `sbgp` boxes, each with a different `grouping_type`.
    pub fn sbgps(&self) -> impl Iterator<Item = Result<SbgpBoxView<'a>>> + 'a {
        self.boxes().filter_map(|result| match result {
            Ok(rawbox) if rawbox.boxtype() == BoxType::SBGP => {
                Some(SbgpBoxView::decode(rawbox.into_payload()))
            }
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
    }

    /// Returns an iterator over the Sample Group Description Boxes (`sgpd`) contained in this sample table.
    ///
    /// There may be zero or more `sgpd` boxes, each with a different `grouping_type`.
    pub fn sgpds(&self) -> impl Iterator<Item = Result<SgpdBoxView<'a>>> + 'a {
        self.boxes().filter_map(|result| match result {
            Ok(rawbox) if rawbox.boxtype() == BoxType::SGPD => {
                Some(SgpdBoxView::decode(rawbox.into_payload()))
            }
            Ok(_) => None,
            Err(e) => Some(Err(e)),
        })
    }

    /// Returns a reference to the sample size information for this sample table, which may be either a `stsz` or `stz2` box.
    pub fn sample_size(&self) -> Result<SampleSizeView<'a>> {
        if let Some(stsz) = self.stsz()? {
            Ok(SampleSizeView::Stsz(stsz))
        } else if let Some(stz2) = self.stz2()? {
            Ok(SampleSizeView::Stz2(stz2))
        } else {
            Err(Error::in_box(
                ErrorKind::BoxMissing {
                    required: BoxType::STSZ, // stsz or stz2
                },
                BoxType::STBL,
            ))
        }
    }

    /// Returns a reference to the chunk offset information for this sample table, which may be either a `stco` or `co64` box.
    pub fn chunk_offset(&self) -> Result<ChunkOffsetView<'a>> {
        if let Some(stco) = self.stco()? {
            Ok(ChunkOffsetView::Stco(stco))
        } else if let Some(co64) = self.co64()? {
            Ok(ChunkOffsetView::Co64(co64))
        } else {
            Err(Error::in_box(
                ErrorKind::BoxMissing {
                    required: BoxType::STCO, // stco or co64
                },
                BoxType::STBL,
            ))
        }
    }
}

impl BoxCodec for StblBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::STBL
    }
}

impl<'a> BoxDecode<'a> for StblBoxView<'a> {
    fn decode(bytes: &'a [u8]) -> Result<Self> {
        Ok(StblBoxView { content: bytes })
    }
}

/// A reference to a sample size box, which can be either `stsz` or `stz2`.
#[derive(Debug)]
pub enum SampleSizeView<'a> {
    /// Sample Size Box (`stsz`) - stores sizes for each sample, or a default size if all samples are the same size.
    Stsz(StszBoxView<'a>),
    /// Compact Sample Size Box (`stz2`) - stores sizes for each sample using a compact representation.
    Stz2(Stz2BoxView<'a>),
}

impl SampleSizeView<'_> {
    /// Returns the total number of samples described by this `SampleSizeView`, which is determined by the `sample_count` field in the underlying `stsz` or `stz2` box.
    pub fn sample_count(&self) -> u32 {
        match self {
            SampleSizeView::Stsz(stsz) => stsz.sample_count,
            SampleSizeView::Stz2(stz2) => stz2.sample_count,
        }
    }
}

/// A reference to a chunk offset box, which can be either `stco` or `co64`.
#[derive(Debug)]
pub enum ChunkOffsetView<'a> {
    /// Chunk Offset Box (`stco`) - stores 32-bit file offsets for each chunk.
    Stco(StcoBoxView<'a>),
    /// Chunk Large Offset Box (`co64`) - stores 64-bit file offsets for each chunk, used for files larger than 4GB.
    Co64(Co64BoxView<'a>),
}

impl ChunkOffsetView<'_> {
    /// Returns the number of chunk offsets stored in this `ChunkOffsetView`, which is determined by the `entry_count` field in the underlying `stco` or `co64` box.
    pub fn entry_count(&self) -> u32 {
        match self {
            ChunkOffsetView::Stco(stco) => stco.entry_count,
            ChunkOffsetView::Co64(co64) => co64.entry_count,
        }
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use alloc::vec::Vec;

    use super::*;
    use crate::BoxEncode;

    use crate::codec::boxed_len;
    use crate::codec::write_box_in;
    use crate::cursor::WriteCursor;

    use crate::boxes::bmff::Co64Box;
    use crate::boxes::bmff::Co64Entry;
    use crate::boxes::bmff::CttsBox;
    use crate::boxes::bmff::SbgpBox;
    use crate::boxes::bmff::SdtpBox;
    use crate::boxes::bmff::SgpdBox;
    use crate::boxes::bmff::StcoBox;
    use crate::boxes::bmff::StcoEntry;
    use crate::boxes::bmff::StdpBox;
    use crate::boxes::bmff::StscBox;
    use crate::boxes::bmff::StsdBox;
    use crate::boxes::bmff::StshBox;
    use crate::boxes::bmff::StssBox;
    use crate::boxes::bmff::StszBox;
    use crate::boxes::bmff::SttsBox;
    use crate::boxes::bmff::Stz2Box;

    /// An owned reference to a sample size box, which can be either `stsz` or `stz2`.
    #[derive(Debug, Clone)]
    pub enum SampleSize {
        /// Sample Size Box (`stsz`) - stores sizes for each sample, or a default size if all samples are the same size.
        Stsz(StszBox),
        /// Compact Sample Size Box (`stz2`) - stores sizes for each sample using a compact representation.
        Stz2(Stz2Box),
    }

    impl SampleSize {
        /// Returns the total number of samples described by this `SampleSize`.
        pub fn sample_count(&self) -> u32 {
            match self {
                SampleSize::Stsz(stsz) => stsz.sample_count,
                SampleSize::Stz2(stz2) => stz2.entries.len() as u32,
            }
        }

        /// Returns the size of the sample at the given index, or `None` if the index is out of bounds. For `stsz`, if `sample_size` is non-zero, that value is returned for all samples; otherwise, the size is looked up in the `entries` vector. For `stz2`, the size is looked up in the `entries` vector and promoted to `u32`.
        pub fn get(&self, index: usize) -> Option<u32> {
            match self {
                SampleSize::Stsz(stsz) if stsz.sample_size != 0 => Some(stsz.sample_size),
                SampleSize::Stsz(stsz) => stsz.entries.get(index).map(|e| e.entry_size),
                SampleSize::Stz2(stz2) => stz2.entries.get(index).map(|e| u32::from(e.entry_size)),
            }
        }
    }

    impl From<StszBox> for SampleSize {
        fn from(stsz: StszBox) -> Self {
            SampleSize::Stsz(stsz)
        }
    }

    impl From<Stz2Box> for SampleSize {
        fn from(stz2: Stz2Box) -> Self {
            SampleSize::Stz2(stz2)
        }
    }

    /// An owned reference to a chunk offset box, which can be either `stco` or `co64`.
    #[derive(Debug, Clone)]
    pub enum ChunkOffset {
        /// Chunk Offset Box (`stco`) - stores 32-bit file offsets for each chunk.
        Stco(StcoBox),
        /// Chunk Large Offset Box (`co64`) - stores 64-bit file offsets for each chunk, used for files larger than 4GB.
        Co64(Co64Box),
    }

    impl ChunkOffset {
        /// Returns the number of chunk offsets stored in this `ChunkOffset`, which is the number of entries in the underlying `stco` or `co64` box.
        pub fn entries_count(&self) -> usize {
            match self {
                ChunkOffset::Stco(stco) => stco.entries.len(),
                ChunkOffset::Co64(co64) => co64.entries.len(),
            }
        }

        /// Returns an iterator over the chunk offsets stored in this `ChunkOffset`, yielding each offset as a `u64`. For `stco`, the 32-bit offsets are promoted to `u64` when returned.
        pub fn iter(&self) -> impl Iterator<Item = u64> + '_ {
            let (stco, co64) = match self {
                ChunkOffset::Stco(s) => (Some(s.entries.iter()), None),
                ChunkOffset::Co64(c) => (None, Some(c.entries.iter())),
            };
            stco.into_iter()
                .flatten()
                .map(|e| u64::from(e.chunk_offset))
                .chain(co64.into_iter().flatten().map(|e| e.chunk_offset))
        }

        /// Adds a chunk offset. Starts as `stco` (32-bit) and automatically
        /// promotes to `co64` (64-bit) when an offset exceeds `u32::MAX`.
        pub fn push(&mut self, offset: u64) {
            match self {
                ChunkOffset::Stco(stco) => {
                    if offset > u32::MAX as u64 {
                        // Promote stco → co64
                        let mut co64_entries: Vec<Co64Entry> = stco
                            .entries
                            .iter()
                            .map(|e| Co64Entry {
                                chunk_offset: u64::from(e.chunk_offset),
                            })
                            .collect();
                        co64_entries.push(Co64Entry {
                            chunk_offset: offset,
                        });
                        *self = ChunkOffset::Co64(Co64Box {
                            entries: co64_entries,
                            ..Default::default()
                        });
                    } else {
                        stco.entries.push(StcoEntry {
                            chunk_offset: offset as u32,
                        });
                    }
                }
                ChunkOffset::Co64(co64) => {
                    co64.entries.push(Co64Entry {
                        chunk_offset: offset,
                    });
                }
            }
        }

        /// Creates a `ChunkOffset` from a slice of chunk offsets. If any offset exceeds the maximum value for a 32-bit unsigned integer, a `co64` box will be created; otherwise, an `stco` box will be used.
        pub fn from_offsets(offsets: &[u64]) -> Self {
            if offsets.iter().any(|&offset| offset > u32::MAX as u64) {
                let co64_entries = offsets
                    .iter()
                    .map(|&offset| Co64Entry {
                        chunk_offset: offset,
                    })
                    .collect();
                ChunkOffset::Co64(Co64Box {
                    entries: co64_entries,
                    ..Default::default()
                })
            } else {
                let stco_entries = offsets
                    .iter()
                    .map(|&offset| StcoEntry {
                        chunk_offset: offset as u32,
                    })
                    .collect();
                ChunkOffset::Stco(StcoBox {
                    entries: stco_entries,
                    ..Default::default()
                })
            }
        }
    }

    impl Default for ChunkOffset {
        fn default() -> Self {
            ChunkOffset::Stco(StcoBox::default())
        }
    }

    impl From<StcoBox> for ChunkOffset {
        fn from(stco: StcoBox) -> Self {
            ChunkOffset::Stco(stco)
        }
    }

    impl From<Co64Box> for ChunkOffset {
        fn from(co64: Co64Box) -> Self {
            ChunkOffset::Co64(co64)
        }
    }

    impl From<&[u64]> for ChunkOffset {
        fn from(offsets: &[u64]) -> Self {
            ChunkOffset::from_offsets(offsets)
        }
    }

    /// An owned Sample Table Box (`stbl`).
    ///
    /// This is the owned variant of [`StblBoxView`] that stores child boxes
    /// in heap-allocated structures.
    ///
    /// # Structure
    ///
    /// Required child boxes:
    /// - `stsd`: Sample descriptions (formats, codecs).
    /// - `stts`: Decoding time-to-sample mapping.
    /// - `stsz` or `stz2`: Sample sizes.
    /// - `stsc`: Sample-to-chunk grouping.
    /// - `stco` or `co64`: Chunk file offsets.
    ///
    /// Optional child boxes:
    /// - `ctts`, `cslg`, `stss`, `stsh`, `sdtp`, `stdp`, `sbgp`, `sgpd`.
    #[derive(Debug, Clone)]
    pub struct StblBox {
        /// Sample Description Box - describes formats for the samples.
        pub stsd: StsdBox,
        /// Decoding Time to Sample Box - maps samples to decoding time.
        pub stts: SttsBox,
        /// Sample Size Box - size of each sample.
        pub sample_size: SampleSize,
        /// Sample to Chunk Box - maps samples to chunks.
        pub stsc: StscBox,
        /// Chunk Offset Box - file offset of each chunk.
        pub chunk_offset: ChunkOffset,
        /// Degradation Priority Box (optional) - sample quality priorities.
        pub stdp: Option<StdpBox>,
        /// Composition Time to Sample Box (optional) - composition time offsets.
        pub ctts: Option<CttsBox>,
        /// Composition to Decode Box (optional) - timing relationships.
        pub cslg: Option<CslgBox>,
        /// Sync Sample Box (optional) - identifies random access points.
        pub stss: Option<StssBox>,
        /// Shadow Sync Box (optional) - alternative sync points.
        pub stsh: Option<StshBox>,
        /// Sample Dependency Type Box (optional) - inter-sample dependencies.
        pub sdtp: Option<SdtpBox>,
        /// Sample to Group Boxes - sample grouping information (zero or more, one per grouping type).
        pub sbgps: Vec<SbgpBox>,
        /// Sample Group Description Boxes - group descriptions (zero or more, one per grouping type).
        pub sgpds: Vec<SgpdBox>,
    }

    impl TryFrom<&StblBoxView<'_>> for StblBox {
        type Error = Error;

        fn try_from(view: &StblBoxView<'_>) -> Result<Self> {
            let mut stsd = None;
            let mut stdp = None;
            let mut stts = None;
            let mut ctts = None;
            let mut cslg = None;
            let mut stss = None;
            let mut stsh = None;
            let mut sdtp = None;
            let mut sbgps = Vec::new();
            let mut sgpds = Vec::new();
            let mut sample_size = None;
            let mut stsc = None;
            let mut chunk_offset = None;

            for result in view.boxes() {
                let rawbox = result?;

                match rawbox.boxtype() {
                    BoxType::STSD => {
                        if stsd.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::STSD,
                                },
                                BoxType::STBL,
                            ));
                        }
                        let stsd_box = StsdBox::decode(rawbox.payload())?;
                        stsd = Some(stsd_box);
                    }
                    BoxType::STDP => {
                        if stdp.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::STDP,
                                },
                                BoxType::STBL,
                            ));
                        }
                        let stdp_box = StdpBox::decode(rawbox.payload())?;
                        stdp = Some(stdp_box);
                    }
                    BoxType::STTS => {
                        if stts.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::STTS,
                                },
                                BoxType::STBL,
                            ));
                        }
                        let stts_box = SttsBox::decode(rawbox.payload())?;
                        stts = Some(stts_box);
                    }
                    BoxType::CTTS => {
                        if ctts.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::CTTS,
                                },
                                BoxType::STBL,
                            ));
                        }
                        let ctts_box = CttsBox::decode(rawbox.payload())?;
                        ctts = Some(ctts_box);
                    }
                    BoxType::CSLG => {
                        if cslg.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::CSLG,
                                },
                                BoxType::STBL,
                            ));
                        }
                        let cslg_box = CslgBox::decode(rawbox.payload())?;
                        cslg = Some(cslg_box);
                    }
                    BoxType::STSS => {
                        if stss.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::STSS,
                                },
                                BoxType::STBL,
                            ));
                        }
                        let stss_box = StssBox::decode(rawbox.payload())?;
                        stss = Some(stss_box);
                    }
                    BoxType::STSH => {
                        if stsh.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::STSH,
                                },
                                BoxType::STBL,
                            ));
                        }
                        let stsh_box = StshBox::decode(rawbox.payload())?;
                        stsh = Some(stsh_box);
                    }
                    BoxType::SDTP => {
                        if sdtp.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::SDTP,
                                },
                                BoxType::STBL,
                            ));
                        }
                        let sdtp_box = SdtpBox::decode(rawbox.payload())?;
                        sdtp = Some(sdtp_box);
                    }
                    BoxType::SBGP => {
                        let sbgp_box = SbgpBox::decode(rawbox.payload())?;
                        sbgps.push(sbgp_box);
                    }
                    BoxType::SGPD => {
                        let sgpd_box = SgpdBox::decode(rawbox.payload())?;
                        sgpds.push(sgpd_box);
                    }
                    BoxType::STSZ => {
                        if sample_size.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::STSZ,
                                },
                                BoxType::STBL,
                            ));
                        }
                        let stsz_box = StszBox::decode(rawbox.payload())?;
                        sample_size = Some(SampleSize::Stsz(stsz_box));
                    }
                    BoxType::STZ2 => {
                        if sample_size.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::STZ2,
                                },
                                BoxType::STBL,
                            ));
                        }
                        let stz2_box = Stz2Box::decode(rawbox.payload())?;
                        sample_size = Some(SampleSize::Stz2(stz2_box));
                    }
                    BoxType::STSC => {
                        if stsc.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::STSC,
                                },
                                BoxType::STBL,
                            ));
                        }
                        let stsc_box = StscBox::decode(rawbox.payload())?;
                        stsc = Some(stsc_box);
                    }
                    BoxType::STCO => {
                        if chunk_offset.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::STCO,
                                },
                                BoxType::STBL,
                            ));
                        }
                        let stco_box = StcoBox::decode(rawbox.payload())?;
                        chunk_offset = Some(ChunkOffset::Stco(stco_box));
                    }
                    BoxType::CO64 => {
                        if chunk_offset.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::CO64,
                                },
                                BoxType::STBL,
                            ));
                        }
                        let co64_box = Co64Box::decode(rawbox.payload())?;
                        chunk_offset = Some(ChunkOffset::Co64(co64_box));
                    }
                    _ => continue,
                }
            }

            Ok(StblBox {
                stsd: stsd.ok_or_else(|| {
                    Error::in_box(
                        ErrorKind::BoxMissing {
                            required: BoxType::STSD,
                        },
                        BoxType::STBL,
                    )
                })?,
                stdp,
                stts: stts.ok_or_else(|| {
                    Error::in_box(
                        ErrorKind::BoxMissing {
                            required: BoxType::STTS,
                        },
                        BoxType::STBL,
                    )
                })?,
                ctts,
                cslg,
                stss,
                stsh,
                sdtp,
                sbgps,
                sgpds,
                sample_size: sample_size.ok_or_else(|| {
                    Error::in_box(
                        ErrorKind::BoxMissing {
                            required: BoxType::STSZ,
                        },
                        BoxType::STBL,
                    )
                })?,
                stsc: stsc.ok_or_else(|| {
                    Error::in_box(
                        ErrorKind::BoxMissing {
                            required: BoxType::STSC,
                        },
                        BoxType::STBL,
                    )
                })?,
                chunk_offset: chunk_offset.ok_or_else(|| {
                    Error::in_box(
                        ErrorKind::BoxMissing {
                            required: BoxType::STCO,
                        },
                        BoxType::STBL,
                    )
                })?,
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
            len += boxed_len(&self.stsd);
            if let Some(stdp) = &self.stdp {
                len += boxed_len(stdp);
            }
            len += boxed_len(&self.stts);
            if let Some(ctts) = &self.ctts {
                len += boxed_len(ctts);
            }
            if let Some(cslg) = &self.cslg {
                len += boxed_len(cslg);
            }
            if let Some(stss) = &self.stss {
                len += boxed_len(stss);
            }
            if let Some(stsh) = &self.stsh {
                len += boxed_len(stsh);
            }
            if let Some(sdtp) = &self.sdtp {
                len += boxed_len(sdtp);
            }
            len += self.sbgps.iter().map(boxed_len).sum::<usize>();
            len += self.sgpds.iter().map(boxed_len).sum::<usize>();
            match &self.sample_size {
                SampleSize::Stsz(stsz) => len += boxed_len(stsz),
                SampleSize::Stz2(stz2) => len += boxed_len(stz2),
            }

            len += boxed_len(&self.stsc);
            match &self.chunk_offset {
                ChunkOffset::Stco(stco) => len += boxed_len(stco),
                ChunkOffset::Co64(co64) => len += boxed_len(co64),
            }

            len
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            write_box_in(&mut cur, &self.stsd)?;

            if let Some(stdp) = &self.stdp {
                write_box_in(&mut cur, stdp)?;
            }

            write_box_in(&mut cur, &self.stts)?;

            if let Some(ctts) = &self.ctts {
                write_box_in(&mut cur, ctts)?;
            }

            if let Some(cslg) = &self.cslg {
                write_box_in(&mut cur, cslg)?;
            }

            if let Some(stss) = &self.stss {
                write_box_in(&mut cur, stss)?;
            }

            if let Some(stsh) = &self.stsh {
                write_box_in(&mut cur, stsh)?;
            }

            if let Some(sdtp) = &self.sdtp {
                write_box_in(&mut cur, sdtp)?;
            }

            for sbgp in &self.sbgps {
                write_box_in(&mut cur, sbgp)?;
            }
            for sgpd in &self.sgpds {
                write_box_in(&mut cur, sgpd)?;
            }

            match &self.sample_size {
                SampleSize::Stsz(stsz) => write_box_in(&mut cur, stsz)?,
                SampleSize::Stz2(stz2) => write_box_in(&mut cur, stz2)?,
            }
            write_box_in(&mut cur, &self.stsc)?;

            match &self.chunk_offset {
                ChunkOffset::Stco(stco) => write_box_in(&mut cur, stco)?,
                ChunkOffset::Co64(co64) => write_box_in(&mut cur, co64)?,
            }

            Ok(cur.position())
        }
    }
}

#[cfg(feature = "alloc")]
pub use owned::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_data_empty() -> [u8; 0] {
        []
    }

    // Minimal stbl with required child boxes: stsd, stts, stsz, stsc, stco
    fn raw_data_minimal() -> Vec<u8> {
        let mut data = Vec::new();

        // stsd box (minimal)
        data.extend_from_slice(&[
            0x00, 0x00, 0x00, 0x10, // size = 16
            b's', b't', b's', b'd', // type = "stsd"
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x00, // entry_count = 0
        ]);

        // stts box (minimal)
        data.extend_from_slice(&[
            0x00, 0x00, 0x00, 0x10, // size = 16
            b's', b't', b't', b's', // type = "stts"
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x00, // entry_count = 0
        ]);

        // stsz box (minimal)
        data.extend_from_slice(&[
            0x00, 0x00, 0x00, 0x14, // size = 20
            b's', b't', b's', b'z', // type = "stsz"
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x00, // sample_size = 0
            0x00, 0x00, 0x00, 0x00, // sample_count = 0
        ]);

        // stsc box (minimal)
        data.extend_from_slice(&[
            0x00, 0x00, 0x00, 0x10, // size = 16
            b's', b't', b's', b'c', // type = "stsc"
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x00, // entry_count = 0
        ]);

        // stco box (minimal)
        data.extend_from_slice(&[
            0x00, 0x00, 0x00, 0x10, // size = 16
            b's', b't', b'c', b'o', // type = "stco"
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x00, // entry_count = 0
        ]);

        data
    }

    #[test]
    fn test_stbl_box_view_decode_empty() {
        let data = raw_data_empty();
        let stbl = StblBoxView::decode(&data).unwrap();

        assert_eq!(stbl.boxes().count(), 0);
    }

    #[test]
    fn test_stbl_box_view_missing_required_stsd() {
        let data = raw_data_empty();
        let stbl = StblBoxView::decode(&data).unwrap();

        let result = stbl.stsd();
        assert!(result.is_err());
    }

    #[test]
    fn test_stbl_box_view_decode_minimal() {
        let data = raw_data_minimal();
        let stbl = StblBoxView::decode(&data).unwrap();

        assert!(stbl.stsd().is_ok());
        assert!(stbl.stts().is_ok());
        assert!(stbl.stsz().is_ok());
        assert!(stbl.stsc().is_ok());
        assert!(stbl.stco().is_ok());

        // Optional boxes should return Ok(None)
        assert!(stbl.ctts().unwrap().is_none());
        assert!(stbl.cslg().unwrap().is_none());
        assert!(stbl.stss().unwrap().is_none());
        assert!(stbl.stsh().unwrap().is_none());
        assert!(stbl.sdtp().unwrap().is_none());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_stbl_box_try_from() {
        let data = raw_data_minimal();
        let view = StblBoxView::decode(&data).unwrap();
        let owned = StblBox::try_from(&view).unwrap();

        assert_eq!(owned.stsd.entries.len(), 0);
        assert_eq!(owned.stts.entries.len(), 0);
        match owned.sample_size {
            SampleSize::Stsz(ref stsz) => assert_eq!(stsz.entries.len(), 0),
            SampleSize::Stz2(ref stz2) => assert_eq!(stz2.entries.len(), 0),
        }
        assert_eq!(owned.stsc.entries.len(), 0);
        match owned.chunk_offset {
            ChunkOffset::Stco(ref stco) => assert_eq!(stco.entries.len(), 0),
            ChunkOffset::Co64(ref co64) => assert_eq!(co64.entries.len(), 0),
        }
    }
}
