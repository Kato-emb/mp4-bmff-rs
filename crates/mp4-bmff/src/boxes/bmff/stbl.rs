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
use super::SdtpBoxView;
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

#[cfg(feature = "alloc")]
mod owned {
    use super::*;
    use crate::BoxEncode;

    use crate::codec::boxed_len;
    use crate::codec::write_box_in;
    use crate::cursor::WriteCursor;

    use crate::boxes::bmff::Co64Box;
    use crate::boxes::bmff::CttsBox;
    use crate::boxes::bmff::SdtpBox;
    use crate::boxes::bmff::StcoBox;
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
        Stsz(StszBox),
        Stz2(Stz2Box),
    }

    /// An owned reference to a chunk offset box, which can be either `stco` or `co64`.
    #[derive(Debug, Clone)]
    pub enum ChunkOffset {
        Stco(StcoBox),
        Co64(Co64Box),
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
    /// - `ctts`, `cslg`, `stss`, `stsh`, `sdtp`, `stdp`.
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
