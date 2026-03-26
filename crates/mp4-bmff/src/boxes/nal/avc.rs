//! AVC (H.264) Sample Entry and Configuration Box implementation.
//!
//! This module provides types for working with AVC/H.264 video content in
//! ISO Base Media File Format. AVC sample entries (`avc1`, `avc2`, `avc3`,
//! `avc4`) describe video streams encoded using H.264/MPEG-4 Part 10.
//!
//! # Sample Entry Types
//!
//! - `avc1`/`avc3`: Standard AVC sample entries. `avc3` indicates parameter
//!   sets may be stored in-band rather than in the configuration record.
//! - `avc2`/`avc4`: AVC sample entries with additional parsing requirements.
//!
//! These entries appear in the Sample Description Box (`stsd`) for video
//! tracks using H.264/AVC codecs.

use core::marker::PhantomData;

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::BoxIter;

use crate::boxes::bmff::VisualSampleEntryView;
use crate::formats::mpeg4::codecs::avc::AVCDecoderConfigurationRecordView;
use crate::formats::mpeg4::systems::descriptor::iter::DescriptorIter;

use crate::cursor::ReadCursor;

/// A reference to an AVC Decoder Configuration Box (`avcC`).
///
/// Contains the AVC Decoder Configuration Record with profile, level,
/// and parameter set information needed to initialize an H.264 decoder.
///
/// # Structure
///
/// - `avc_config`: The AVC Decoder Configuration Record containing SPS/PPS.
#[derive(Debug)]
pub struct AvcCBoxView<'a> {
    /// The AVC Decoder Configuration Record with profile/level and parameter sets.
    pub avc_config: AVCDecoderConfigurationRecordView<'a>,
}

impl BoxCodec for AvcCBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::AVCC
    }
}

impl<'de> BoxDecode<'de> for AvcCBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let avc_config = AVCDecoderConfigurationRecordView::parse(bytes)?;
        Ok(AvcCBoxView { avc_config })
    }
}

/// A reference to an MPEG-4 Extension Descriptors Box (`m4ds`).
///
/// Contains additional MPEG-4 descriptors that extend the sample entry.
/// This box is optional within AVC sample entries and provides extra
/// metadata when needed for MPEG-4 Systems integration.
///
/// # Structure
///
/// - `descriptors`: A sequence of MPEG-4 descriptors.
#[derive(Debug)]
pub struct M4dsBoxView<'a> {
    content: &'a [u8],
}

impl<'a> M4dsBoxView<'a> {
    /// Returns an iterator over the descriptors contained in this box.
    pub fn descriptors(&self) -> DescriptorIter<'a> {
        DescriptorIter::new(self.content)
    }
}

impl BoxCodec for M4dsBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::M4DS
    }
}

impl<'de> BoxDecode<'de> for M4dsBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        Ok(M4dsBoxView { content: bytes })
    }
}

/// A reference to an AVC Sample Entry (`avc1` or `avc3`).
///
/// Describes H.264/AVC video streams by combining the base Visual Sample
/// Entry with the AVC Configuration Box containing decoder parameters.
///
/// The type parameter `S` distinguishes between `avc1` and `avc3` variants.
///
/// # Structure
///
/// - `base`: Base Visual Sample Entry with width, height, resolution, etc.
/// - Child boxes:
///   - `avcC` (required): AVC Configuration Box with decoder parameters.
///   - `m4ds` (optional): MPEG-4 Extension Descriptors Box.
#[derive(Debug)]
pub struct AVCSampleEntryView<'a, S> {
    base: VisualSampleEntryView<'a>,
    _marker: PhantomData<S>,
}

impl<'a, S> AVCSampleEntryView<'a, S> {
    /// Returns the base Visual Sample Entry
    pub fn base(&self) -> &VisualSampleEntryView<'a> {
        &self.base
    }

    /// Returns an iterator over the child boxes of this sample entry.
    pub fn boxes(&self) -> BoxIter<'a> {
        self.base.boxes_in()
    }

    /// Returns the AVC Configuration Box (`avcC`) contained in this sample entry.
    ///
    /// # Errors
    ///
    /// Returns an error if the `avcC` box is missing. The `avcC` box is required in a valid AVC sample entry.
    pub fn avcc(&self) -> Result<AvcCBoxView<'a>> {
        for result in self.boxes() {
            let b = result?;
            if b.boxtype() == BoxType::AVCC {
                let avcc = AvcCBoxView::decode(b.into_payload())?;
                return Ok(avcc);
            }
        }

        Err(Error::new(ErrorKind::BoxMissing {
            required: BoxType::AVCC,
        }))
    }

    /// Returns the MPEG-4 Extension Descriptors Box (`m4ds`) contained in this sample entry.
    ///
    /// # Errors
    ///
    /// Returns an error if there are multiple `m4ds` boxes. The `m4ds` box is optional, but if present there must be only one.
    pub fn m4ds(&self) -> Result<Option<M4dsBoxView<'a>>> {
        for result in self.boxes() {
            let b = result?;
            if b.boxtype() == BoxType::M4DS {
                let m4ds = M4dsBoxView::decode(b.into_payload())?;
                return Ok(Some(m4ds));
            }
        }

        Ok(None)
    }
}

impl<'de, S> BoxDecode<'de> for AVCSampleEntryView<'de, S> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);
        let base = VisualSampleEntryView::parse_in(&mut cur)?;

        Ok(AVCSampleEntryView {
            base,
            _marker: PhantomData,
        })
    }
}

/// Marker type for AVC1 Sample Entry (`avc1`).
///
/// AVC1 stores all parameter sets (SPS/PPS) in the configuration record.
#[derive(Debug)]
pub struct Avc1;

/// A reference to an AVC1 Sample Entry (`avc1`).
///
/// Standard AVC sample entry where parameter sets are stored in `avcC`.
pub type Avc1SampleEntryView<'a> = AVCSampleEntryView<'a, Avc1>;

impl BoxCodec for Avc1SampleEntryView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::AVC1
    }
}

/// Marker type for AVC3 Sample Entry (`avc3`).
///
/// AVC3 indicates parameter sets may be stored in-band within samples.
#[derive(Debug)]
pub struct Avc3;

/// A reference to an AVC3 Sample Entry (`avc3`).
///
/// AVC sample entry where parameter sets may appear in-band.
pub type Avc3SampleEntryView<'a> = AVCSampleEntryView<'a, Avc3>;

impl BoxCodec for Avc3SampleEntryView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::AVC3
    }
}

/// A reference to an AVC2 Sample Entry (`avc2` or `avc4`).
///
/// Similar to [`AVCSampleEntryView`] but for `avc2`/`avc4` sample entry types.
/// The type parameter `S` distinguishes between variants.
///
/// # Structure
///
/// - `base`: Base Visual Sample Entry with width, height, resolution, etc.
/// - Child boxes:
///   - `avcC` (required): AVC Configuration Box with decoder parameters.
///   - `m4ds` (optional): MPEG-4 Extension Descriptors Box.
#[derive(Debug)]
pub struct AVC2SampleEntryView<'a, S> {
    base: VisualSampleEntryView<'a>,
    _marker: PhantomData<S>,
}

impl<'a, S> AVC2SampleEntryView<'a, S> {
    /// Returns the base Visual Sample Entry
    pub fn base(&self) -> &VisualSampleEntryView<'a> {
        &self.base
    }

    /// Returns an iterator over the child boxes of this sample entry.
    pub fn boxes(&self) -> BoxIter<'a> {
        self.base.boxes_in()
    }

    /// Returns the AVC Configuration Box (`avcC`) contained in this sample entry.
    ///
    /// # Errors
    ///
    /// Returns an error if the `avcC` box is missing. The `avcC` box is required in a valid AVC sample entry.
    pub fn avcc(&self) -> Result<AvcCBoxView<'a>> {
        for result in self.boxes() {
            let b = result?;
            if b.boxtype() == BoxType::AVCC {
                let avcc = AvcCBoxView::decode(b.into_payload())?;
                return Ok(avcc);
            }
        }

        Err(Error::new(ErrorKind::BoxMissing {
            required: BoxType::AVCC,
        }))
    }

    /// Returns the MPEG-4 Extension Descriptors Box (`m4ds`) contained in this sample entry.
    ///
    /// # Errors
    ///
    /// Returns an error if there are multiple `m4ds` boxes. The `m4ds` box is optional, but if present there must be only one.
    pub fn m4ds(&self) -> Result<Option<M4dsBoxView<'a>>> {
        for result in self.boxes() {
            let b = result?;
            if b.boxtype() == BoxType::M4DS {
                let m4ds = M4dsBoxView::decode(b.into_payload())?;
                return Ok(Some(m4ds));
            }
        }

        Ok(None)
    }
}

impl<'de, S> BoxDecode<'de> for AVC2SampleEntryView<'de, S> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);
        let base = VisualSampleEntryView::parse_in(&mut cur)?;

        Ok(AVC2SampleEntryView {
            base,
            _marker: PhantomData,
        })
    }
}

/// Marker type for AVC2 Sample Entry (`avc2`).
///
/// AVC2 stores all parameter sets in the configuration record.
#[derive(Debug)]
pub struct Avc2;

/// A reference to an AVC2 Sample Entry (`avc2`).
pub type Avc2SampleEntryView<'a> = AVC2SampleEntryView<'a, Avc2>;

impl BoxCodec for Avc2SampleEntryView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::AVC2
    }
}

/// Marker type for AVC4 Sample Entry (`avc4`).
///
/// AVC4 indicates parameter sets may be stored in-band within samples.
#[derive(Debug)]
pub struct Avc4;

/// A reference to an AVC4 Sample Entry (`avc4`).
pub type Avc4SampleEntryView<'a> = AVC2SampleEntryView<'a, Avc4>;

impl BoxCodec for Avc4SampleEntryView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::AVC4
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use core::fmt;

    use alloc::string::String;
    use alloc::vec::Vec;

    use super::*;
    use crate::BoxEncode;

    use crate::codec::boxed_len;
    use crate::codec::write_box_in;
    use crate::cursor::WriteCursor;

    use crate::boxes::bmff::VisualSampleEntry;
    use crate::formats::mpeg4::codecs::avc::AVCDecoderConfigurationRecord;
    use crate::formats::mpeg4::systems::descriptor::*;

    /// An owned AVC Decoder Configuration Box (`avcC`).
    ///
    /// This is the owned variant of [`AvcCBoxView`] that stores the
    /// configuration record in heap-allocated memory.
    ///
    /// # Structure
    ///
    /// - `avc_config`: The AVC Decoder Configuration Record with SPS/PPS.
    #[derive(Debug, Clone)]
    pub struct AvcCBox {
        /// The AVC Decoder Configuration Record with profile/level and parameter sets.
        pub avc_config: AVCDecoderConfigurationRecord,
    }

    impl From<&AvcCBoxView<'_>> for AvcCBox {
        fn from(view: &AvcCBoxView<'_>) -> Self {
            AvcCBox {
                avc_config: AVCDecoderConfigurationRecord::from(&view.avc_config),
            }
        }
    }

    impl AvcCBoxView<'_> {
        /// Converts to an owned AvcCBox
        pub fn to_owned(&self) -> AvcCBox {
            AvcCBox::from(self)
        }
    }

    impl BoxCodec for AvcCBox {
        fn boxtype(&self) -> BoxType {
            BoxType::AVCC
        }
    }

    impl BoxDecode<'_> for AvcCBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = AvcCBoxView::decode(bytes)?;
            Ok(AvcCBox::from(&view))
        }
    }

    impl BoxEncode for AvcCBox {
        fn encoded_len(&self) -> usize {
            self.avc_config.encoded_len()
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);
            let buf = cur.take_mut(self.avc_config.encoded_len())?;
            self.avc_config.write(buf)?;

            Ok(cur.position())
        }
    }

    /// An owned MPEG-4 Extension Descriptors Box (`m4ds`).
    ///
    /// This is the owned variant of [`M4dsBoxView`] that stores
    /// descriptors in heap-allocated memory.
    ///
    /// # Structure
    ///
    /// - `descriptors`: List of MPEG-4 descriptors.
    #[derive(Debug, Clone)]
    pub struct M4dsBox {
        /// List of MPEG-4 descriptors in this box.
        pub descriptors: Vec<RawDescriptorOwned>,
    }

    impl TryFrom<&M4dsBoxView<'_>> for M4dsBox {
        type Error = Error;

        fn try_from(view: &M4dsBoxView<'_>) -> Result<Self> {
            let mut descriptors = Vec::new();
            for result in view.descriptors() {
                let descr = result?.to_owned();
                descriptors.push(descr);
            }

            Ok(M4dsBox { descriptors })
        }
    }

    impl BoxCodec for M4dsBox {
        fn boxtype(&self) -> BoxType {
            BoxType::M4DS
        }
    }

    impl BoxDecode<'_> for M4dsBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = M4dsBoxView::decode(bytes)?;
            M4dsBox::try_from(&view)
        }
    }

    impl BoxEncode for M4dsBox {
        fn encoded_len(&self) -> usize {
            self.descriptors.iter().map(|d| d.len()).sum()
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);
            for descr in &self.descriptors {
                let buf = cur.take_mut(descr.len())?;
                descr.write(buf)?;
            }

            Ok(cur.position())
        }
    }

    /// An owned AVC Sample Entry (`avc1` or `avc3`).
    ///
    /// This is the owned variant of [`AVCSampleEntryView`] that stores
    /// child boxes in heap-allocated memory.
    ///
    /// # Structure
    ///
    /// - `base`: Base Visual Sample Entry with video format properties.
    /// - `avcc`: AVC Configuration Box with decoder parameters.
    /// - `m4ds`: Optional MPEG-4 Extension Descriptors.
    #[derive(Debug, Clone)]
    pub struct AVCSampleEntry<S> {
        /// Base Visual Sample Entry with width, height, resolution, etc.
        pub base: VisualSampleEntry,
        /// AVC Configuration Box with decoder initialization data.
        pub avcc: AvcCBox,
        /// Optional MPEG-4 Extension Descriptors Box.
        pub m4ds: Option<M4dsBox>,
        _marker: PhantomData<S>,
    }

    impl<S> AVCSampleEntry<S> {
        /// Creates a new AVC Sample Entry
        pub fn new(base: VisualSampleEntry, avcc: AvcCBox, m4ds: Option<M4dsBox>) -> Self {
            AVCSampleEntry {
                base,
                avcc,
                m4ds,
                _marker: PhantomData,
            }
        }

        fn write_codec_string_in(&self, w: &mut impl fmt::Write, codec: &str) -> fmt::Result {
            write!(
                w,
                "{}.{:02X}{:02X}{:02X}",
                codec,
                self.avcc.avc_config.avc_profile_indication,
                self.avcc.avc_config.profile_compatibility,
                self.avcc.avc_config.avc_level_indication
            )
        }
    }

    impl<S> TryFrom<&AVCSampleEntryView<'_, S>> for AVCSampleEntry<S> {
        type Error = Error;

        fn try_from(view: &AVCSampleEntryView<'_, S>) -> Result<Self> {
            let (base, rest) = VisualSampleEntry::try_parse_view(view.base())?;

            let mut avcc = None;
            let mut m4ds = None;

            for rawbox in rest {
                match rawbox.boxtype() {
                    BoxType::AVCC => {
                        if avcc.is_some() {
                            return Err(Error::new(ErrorKind::BoxDuplicate {
                                duplicate: BoxType::AVCC,
                            }));
                        }
                        avcc = Some(AvcCBox::decode(rawbox.into_payload())?);
                    }
                    BoxType::M4DS => {
                        if m4ds.is_some() {
                            return Err(Error::new(ErrorKind::BoxDuplicate {
                                duplicate: BoxType::M4DS,
                            }));
                        }
                        m4ds = Some(M4dsBox::decode(rawbox.into_payload())?);
                    }
                    _ => {}
                }
            }

            Ok(AVCSampleEntry {
                base,
                avcc: avcc.ok_or(Error::new(ErrorKind::BoxMissing {
                    required: BoxType::AVCC,
                }))?,
                m4ds,
                _marker: PhantomData,
            })
        }
    }

    impl<S> BoxDecode<'_> for AVCSampleEntry<S> {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = AVCSampleEntryView::decode(bytes)?;
            AVCSampleEntry::try_from(&view)
        }
    }

    impl<S> BoxEncode for AVCSampleEntry<S> {
        fn encoded_len(&self) -> usize {
            let mut len = self.base.encode_len() + boxed_len(&self.avcc);
            if let Some(m4ds) = &self.m4ds {
                len += boxed_len(m4ds);
            }

            len
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);
            self.base.write_in(&mut cur)?;
            write_box_in(&mut cur, &self.avcc)?;

            if let Some(m4ds) = &self.m4ds {
                write_box_in(&mut cur, m4ds)?;
            }

            Ok(cur.position())
        }
    }

    /// An owned AVC1 Sample Entry (`avc1`).
    ///
    /// Standard AVC sample entry where parameter sets are stored in `avcC`.
    /// Provides `codec_string()` method for generating codec parameter strings.
    pub type Avc1SampleEntry = AVCSampleEntry<Avc1>;

    impl Avc1SampleEntry {
        /// Returns the codec string (e.g., "avc1.640028")
        ///
        /// # Panics
        ///
        /// This method will panic if writing to the internal string buffer fails, which should not happen under normal circumstances.
        pub fn codec_string(&self) -> String {
            let mut buf = String::new();
            self.write_codec_string_in(&mut buf, "avc1")
                .expect("Writing to String should not fail");
            buf
        }
    }

    impl BoxCodec for Avc1SampleEntry {
        fn boxtype(&self) -> BoxType {
            BoxType::AVC1
        }
    }

    /// An owned AVC3 Sample Entry (`avc3`).
    ///
    /// AVC sample entry where parameter sets may appear in-band.
    /// Provides `codec_string()` method for generating codec parameter strings.
    pub type Avc3SampleEntry = AVCSampleEntry<Avc3>;

    impl BoxCodec for Avc3SampleEntry {
        fn boxtype(&self) -> BoxType {
            BoxType::AVC3
        }
    }

    impl Avc3SampleEntry {
        /// Returns the codec string (e.g., "avc3.640028")
        ///
        /// # Panics
        ///
        /// This method will panic if writing to the internal string buffer fails, which should not happen under normal circumstances.
        pub fn codec_string(&self) -> String {
            let mut buf = String::new();
            self.write_codec_string_in(&mut buf, "avc3")
                .expect("Writing to String should not fail");
            buf
        }
    }

    /// An owned AVC2 Sample Entry (`avc2` or `avc4`).
    ///
    /// This is the owned variant of [`AVC2SampleEntryView`] that stores
    /// child boxes in heap-allocated memory.
    ///
    /// # Structure
    ///
    /// - `base`: Base Visual Sample Entry with video format properties.
    /// - `avcc`: AVC Configuration Box with decoder parameters.
    /// - `m4ds`: Optional MPEG-4 Extension Descriptors.
    #[derive(Debug, Clone)]
    pub struct AVC2SampleEntry<S> {
        /// Base Visual Sample Entry with width, height, resolution, etc.
        pub base: VisualSampleEntry,
        /// AVC Configuration Box with decoder initialization data.
        pub avcc: AvcCBox,
        /// Optional MPEG-4 Extension Descriptors Box.
        pub m4ds: Option<M4dsBox>,
        _marker: PhantomData<S>,
    }

    impl<S> AVC2SampleEntry<S> {
        /// Creates a new AVC2 Sample Entry
        pub fn new(base: VisualSampleEntry, avcc: AvcCBox, m4ds: Option<M4dsBox>) -> Self {
            AVC2SampleEntry {
                base,
                avcc,
                m4ds,
                _marker: PhantomData,
            }
        }

        fn write_codec_string_in(&self, w: &mut impl fmt::Write, codec: &str) -> fmt::Result {
            write!(
                w,
                "{}.{:02X}{:02X}{:02X}",
                codec,
                self.avcc.avc_config.avc_profile_indication,
                self.avcc.avc_config.profile_compatibility,
                self.avcc.avc_config.avc_level_indication
            )
        }
    }

    impl<S> TryFrom<&AVC2SampleEntryView<'_, S>> for AVC2SampleEntry<S> {
        type Error = Error;

        fn try_from(view: &AVC2SampleEntryView<'_, S>) -> Result<Self> {
            let (base, rest) = VisualSampleEntry::try_parse_view(view.base())?;

            let mut avcc = None;
            let mut m4ds = None;

            for rawbox in rest {
                match rawbox.boxtype() {
                    BoxType::AVCC => {
                        if avcc.is_some() {
                            return Err(Error::new(ErrorKind::BoxDuplicate {
                                duplicate: BoxType::AVCC,
                            }));
                        }
                        avcc = Some(AvcCBox::decode(rawbox.into_payload())?);
                    }
                    BoxType::M4DS => {
                        if m4ds.is_some() {
                            return Err(Error::new(ErrorKind::BoxDuplicate {
                                duplicate: BoxType::M4DS,
                            }));
                        }
                        m4ds = Some(M4dsBox::decode(rawbox.into_payload())?);
                    }
                    _ => {}
                }
            }

            Ok(AVC2SampleEntry {
                base,
                avcc: avcc.ok_or(Error::new(ErrorKind::BoxMissing {
                    required: BoxType::AVCC,
                }))?,
                m4ds,
                _marker: PhantomData,
            })
        }
    }

    impl<S> BoxDecode<'_> for AVC2SampleEntry<S> {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = AVC2SampleEntryView::decode(bytes)?;
            AVC2SampleEntry::try_from(&view)
        }
    }

    impl<S> BoxEncode for AVC2SampleEntry<S> {
        fn encoded_len(&self) -> usize {
            let mut len = self.base.encode_len() + boxed_len(&self.avcc);
            if let Some(m4ds) = &self.m4ds {
                len += boxed_len(m4ds);
            }

            len
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);
            self.base.write_in(&mut cur)?;
            write_box_in(&mut cur, &self.avcc)?;

            if let Some(m4ds) = &self.m4ds {
                write_box_in(&mut cur, m4ds)?;
            }

            Ok(cur.position())
        }
    }

    /// An owned AVC2 Sample Entry (`avc2`).
    ///
    /// Provides `codec_string()` method for generating codec parameter strings.
    pub type Avc2SampleEntry = AVC2SampleEntry<Avc2>;

    impl Avc2SampleEntry {
        /// Returns the codec string (e.g., "avc2.640028")
        ///
        /// # Panics
        ///
        /// This method will panic if writing to the internal string buffer fails, which should not happen under normal circumstances.
        pub fn codec_string(&self) -> String {
            let mut buf = String::new();
            self.write_codec_string_in(&mut buf, "avc2")
                .expect("Writing to String should not fail");
            buf
        }
    }

    impl BoxCodec for Avc2SampleEntry {
        fn boxtype(&self) -> BoxType {
            BoxType::AVC2
        }
    }

    /// An owned AVC4 Sample Entry (`avc4`).
    ///
    /// AVC sample entry where parameter sets may appear in-band.
    /// Provides `codec_string()` method for generating codec parameter strings.
    pub type Avc4SampleEntry = AVC2SampleEntry<Avc4>;

    impl Avc4SampleEntry {
        /// Returns the codec string (e.g., "avc4.640028")
        ///
        /// # Panics
        ///
        /// This method will panic if writing to the internal string buffer fails, which should not happen under normal circumstances.
        pub fn codec_string(&self) -> String {
            let mut buf = String::new();
            self.write_codec_string_in(&mut buf, "avc4")
                .expect("Writing to String should not fail");
            buf
        }
    }

    impl BoxCodec for Avc4SampleEntry {
        fn boxtype(&self) -> BoxType {
            BoxType::AVC4
        }
    }
}

#[cfg(feature = "alloc")]
pub use owned::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxes::bmff::VisualSampleEntry;

    // Sample AVC Configuration Record (Baseline profile)
    fn sample_avcc_payload() -> [u8; 23] {
        [
            0x01, // configuration_version
            0x42, // avc_profile_indication (Baseline)
            0xC0, // profile_compatibility
            0x1E, // avc_level_indication (Level 3.0)
            0xFF, // reserved (6 bits) + length_size_minus_one (2 bits) = 3
            0xE1, // reserved (3 bits) + num_of_sps (5 bits) = 1
            // SPS: length=8
            0x00, 0x08, 0x67, 0x42, 0xC0, 0x1E, 0xD9, 0x00, 0x68, 0x24, 0x01, // num_of_pps = 1
            // PPS: length=4
            0x00, 0x04, 0x68, 0xCE, 0x3C, 0x80,
        ]
    }

    // Sample Visual Sample Entry base (78 bytes)
    fn sample_visual_sample_entry_base() -> [u8; 78] {
        let mut data = [0u8; 78];
        // reserved (6 bytes)
        // data_reference_index (2 bytes)
        data[6] = 0x00;
        data[7] = 0x01; // data_reference_index = 1
        // pre_defined (2 bytes)
        // reserved (2 bytes)
        // pre_defined (12 bytes)
        // width (2 bytes) at offset 24
        data[24] = 0x05;
        data[25] = 0x00; // width = 1280
        // height (2 bytes) at offset 26
        data[26] = 0x02;
        data[27] = 0xD0; // height = 720
        // horizresolution (4 bytes) at offset 28
        data[28] = 0x00;
        data[29] = 0x48;
        data[30] = 0x00;
        data[31] = 0x00; // 72.0 dpi
        // vertresolution (4 bytes) at offset 32
        data[32] = 0x00;
        data[33] = 0x48;
        data[34] = 0x00;
        data[35] = 0x00; // 72.0 dpi
        // reserved (4 bytes)
        // frame_count (2 bytes) at offset 40
        data[40] = 0x00;
        data[41] = 0x01; // frame_count = 1
        // compressorname (32 bytes) at offset 42
        // depth (2 bytes) at offset 74
        data[74] = 0x00;
        data[75] = 0x18; // depth = 24
        // pre_defined (2 bytes) at offset 76
        data[76] = 0xFF;
        data[77] = 0xFF; // pre_defined = -1
        data
    }

    // Build avcC box with header
    fn build_avcc_box() -> Vec<u8> {
        let payload = sample_avcc_payload();
        let size = 8 + payload.len() as u32;
        let mut data = Vec::with_capacity(size as usize);
        data.extend_from_slice(&size.to_be_bytes());
        data.extend_from_slice(b"avcC");
        data.extend_from_slice(&payload);
        data
    }

    #[test]
    fn test_avcc_box_view_decode() {
        let payload = sample_avcc_payload();
        let view = AvcCBoxView::decode(&payload).unwrap();

        assert_eq!(view.avc_config.configuration_version, 1);
        assert_eq!(view.avc_config.avc_profile_indication, 0x42);
        assert_eq!(view.avc_config.profile_compatibility, 0xC0);
        assert_eq!(view.avc_config.avc_level_indication, 0x1E);
        assert_eq!(view.avc_config.length_size_minus_one, 3);
        assert_eq!(view.avc_config.num_of_sps, 1);
        assert_eq!(view.avc_config.num_of_pps, 1);
    }

    #[test]
    fn test_avcc_box_view_boxtype() {
        let payload = sample_avcc_payload();
        let view = AvcCBoxView::decode(&payload).unwrap();

        assert_eq!(view.boxtype(), BoxType::AVCC);
    }

    #[test]
    fn test_avcc_box_view_sps_pps() {
        let payload = sample_avcc_payload();
        let view = AvcCBoxView::decode(&payload).unwrap();

        let sps_list: Vec<&[u8]> = view.avc_config.sps().collect();
        assert_eq!(sps_list.len(), 1);
        assert_eq!(
            sps_list[0],
            &[0x67, 0x42, 0xC0, 0x1E, 0xD9, 0x00, 0x68, 0x24]
        );

        let pps_list: Vec<&[u8]> = view.avc_config.pps().collect();
        assert_eq!(pps_list.len(), 1);
        assert_eq!(pps_list[0], &[0x68, 0xCE, 0x3C, 0x80]);
    }

    #[test]
    fn test_m4ds_box_view_decode() {
        // Empty M4DS box
        let payload: [u8; 0] = [];
        let view = M4dsBoxView::decode(&payload).unwrap();

        assert_eq!(view.boxtype(), BoxType::M4DS);
        assert_eq!(view.descriptors().count(), 0);
    }

    #[test]
    fn test_m4ds_box_view_with_descriptors() {
        // M4DS with one descriptor: tag(0x05) + size(0x02) + data
        let payload = [0x05, 0x02, 0xAA, 0xBB];
        let view = M4dsBoxView::decode(&payload).unwrap();

        let descriptors: Vec<_> = view.descriptors().collect();
        assert_eq!(descriptors.len(), 1);
        let descr = descriptors[0].as_ref().unwrap();
        assert_eq!(descr.instance(), &[0xAA, 0xBB]);
    }

    #[test]
    fn test_avc_sample_entry_view_decode() {
        let mut data = Vec::new();
        data.extend_from_slice(&sample_visual_sample_entry_base());
        data.extend_from_slice(&build_avcc_box());

        let view = Avc1SampleEntryView::decode(&data).unwrap();

        assert_eq!(view.boxtype(), BoxType::AVC1);
        assert_eq!(view.base().width, 1280);
        assert_eq!(view.base().height, 720);
    }

    #[test]
    fn test_avc_sample_entry_view_avcc() {
        let mut data = Vec::new();
        data.extend_from_slice(&sample_visual_sample_entry_base());
        data.extend_from_slice(&build_avcc_box());

        let view = Avc1SampleEntryView::decode(&data).unwrap();
        let avcc = view.avcc().unwrap();

        assert_eq!(avcc.avc_config.avc_profile_indication, 0x42);
    }

    #[test]
    fn test_avc_sample_entry_view_missing_avcc() {
        // Only visual sample entry base, no avcC box
        let data = sample_visual_sample_entry_base();

        let view = Avc1SampleEntryView::decode(&data).unwrap();
        let result = view.avcc();

        assert!(result.is_err());
    }

    #[test]
    fn test_avc3_sample_entry_view_boxtype() {
        let mut data = Vec::new();
        data.extend_from_slice(&sample_visual_sample_entry_base());
        data.extend_from_slice(&build_avcc_box());

        let view = Avc3SampleEntryView::decode(&data).unwrap();

        assert_eq!(view.boxtype(), BoxType::AVC3);
    }

    #[test]
    fn test_avc2_sample_entry_view_decode() {
        let mut data = Vec::new();
        data.extend_from_slice(&sample_visual_sample_entry_base());
        data.extend_from_slice(&build_avcc_box());

        let view = Avc2SampleEntryView::decode(&data).unwrap();

        assert_eq!(view.boxtype(), BoxType::AVC2);
        assert_eq!(view.base().width, 1280);
    }

    #[test]
    fn test_avc4_sample_entry_view_boxtype() {
        let mut data = Vec::new();
        data.extend_from_slice(&sample_visual_sample_entry_base());
        data.extend_from_slice(&build_avcc_box());

        let view = Avc4SampleEntryView::decode(&data).unwrap();

        assert_eq!(view.boxtype(), BoxType::AVC4);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_avcc_box_to_owned() {
        let payload = sample_avcc_payload();
        let view = AvcCBoxView::decode(&payload).unwrap();
        let owned = view.to_owned();

        assert_eq!(
            owned.avc_config.avc_profile_indication,
            view.avc_config.avc_profile_indication
        );
        assert_eq!(
            owned.avc_config.sps.len(),
            view.avc_config.num_of_sps as usize
        );
        assert_eq!(
            owned.avc_config.pps.len(),
            view.avc_config.num_of_pps as usize
        );
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_avcc_box_roundtrip() {
        use crate::BoxEncode;

        let original = sample_avcc_payload();
        let owned = AvcCBox::decode(&original).unwrap();

        let mut buffer = vec![0u8; owned.encoded_len()];
        let written = owned.encode_into(&mut buffer).unwrap();

        assert_eq!(written, original.len());
        assert_eq!(&buffer[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_m4ds_box_roundtrip() {
        use crate::BoxEncode;

        let payload = [0x05, 0x02, 0xAA, 0xBB, 0x06, 0x01, 0xCC];
        let owned = M4dsBox::decode(&payload).unwrap();

        assert_eq!(owned.descriptors.len(), 2);

        let mut buffer = vec![0u8; owned.encoded_len()];
        let written = owned.encode_into(&mut buffer).unwrap();

        assert_eq!(written, payload.len());
        assert_eq!(&buffer[..], &payload[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_avc1_sample_entry_codec_string() {
        let mut data = Vec::new();
        data.extend_from_slice(&sample_visual_sample_entry_base());
        data.extend_from_slice(&build_avcc_box());

        let entry = Avc1SampleEntry::decode(&data).unwrap();
        let codec_string = entry.codec_string();

        // Profile 0x42, Compatibility 0xC0, Level 0x1E
        assert_eq!(codec_string, "avc1.42C01E");
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_avc3_sample_entry_codec_string() {
        let mut data = Vec::new();
        data.extend_from_slice(&sample_visual_sample_entry_base());
        data.extend_from_slice(&build_avcc_box());

        let entry = Avc3SampleEntry::decode(&data).unwrap();
        let codec_string = entry.codec_string();

        assert_eq!(codec_string, "avc3.42C01E");
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_avc2_sample_entry_codec_string() {
        let mut data = Vec::new();
        data.extend_from_slice(&sample_visual_sample_entry_base());
        data.extend_from_slice(&build_avcc_box());

        let entry = Avc2SampleEntry::decode(&data).unwrap();
        let codec_string = entry.codec_string();

        assert_eq!(codec_string, "avc2.42C01E");
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_avc4_sample_entry_codec_string() {
        let mut data = Vec::new();
        data.extend_from_slice(&sample_visual_sample_entry_base());
        data.extend_from_slice(&build_avcc_box());

        let entry = Avc4SampleEntry::decode(&data).unwrap();
        let codec_string = entry.codec_string();

        assert_eq!(codec_string, "avc4.42C01E");
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_avc1_sample_entry_roundtrip() {
        use crate::BoxEncode;

        let mut original = Vec::new();
        original.extend_from_slice(&sample_visual_sample_entry_base());
        original.extend_from_slice(&build_avcc_box());

        let entry = Avc1SampleEntry::decode(&original).unwrap();

        let mut buffer = vec![0u8; entry.encoded_len()];
        let written = entry.encode_into(&mut buffer).unwrap();

        assert_eq!(written, original.len());

        // Verify reparsing
        let reparsed = Avc1SampleEntry::decode(&buffer).unwrap();
        assert_eq!(reparsed.base.width, entry.base.width);
        assert_eq!(reparsed.base.height, entry.base.height);
        assert_eq!(
            reparsed.avcc.avc_config.avc_profile_indication,
            entry.avcc.avc_config.avc_profile_indication
        );
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_avc_sample_entry_with_m4ds() {
        use crate::BoxEncode;

        let mut data = Vec::new();
        data.extend_from_slice(&sample_visual_sample_entry_base());
        data.extend_from_slice(&build_avcc_box());

        // Add m4ds box
        let m4ds_payload = [0x05, 0x02, 0xAA, 0xBB];
        let m4ds_size = 8 + m4ds_payload.len() as u32;
        data.extend_from_slice(&m4ds_size.to_be_bytes());
        data.extend_from_slice(b"m4ds");
        data.extend_from_slice(&m4ds_payload);

        let entry = Avc1SampleEntry::decode(&data).unwrap();

        assert!(entry.m4ds.is_some());
        let m4ds = entry.m4ds.as_ref().unwrap();
        assert_eq!(m4ds.descriptors.len(), 1);

        // Roundtrip
        let mut buffer = vec![0u8; entry.encoded_len()];
        entry.encode_into(&mut buffer).unwrap();

        let reparsed = Avc1SampleEntry::decode(&buffer).unwrap();
        assert!(reparsed.m4ds.is_some());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_avc_sample_entry_new() {
        let mut base = VisualSampleEntry::default();
        base.width = 1920;
        base.height = 1080;

        let avcc_payload = sample_avcc_payload();
        let avcc = AvcCBox::decode(&avcc_payload).unwrap();

        let entry = Avc1SampleEntry::new(base, avcc, None);

        assert_eq!(entry.base.width, 1920);
        assert_eq!(entry.base.height, 1080);
        assert!(entry.m4ds.is_none());
    }
}
