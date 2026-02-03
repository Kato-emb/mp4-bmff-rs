//! AVC (H.264) related boxes

use core::marker::PhantomData;

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::boxes::nal::NalUnitIter;
use crate::error::*;
use crate::iter::BoxIter;

use crate::boxes::sample_entry::VisualSampleEntry;

use crate::cursor::ReadCursor;

/// A reference to a AVC Decoder configuration record
#[derive(Debug)]
pub struct AVCDecoderConfigurationRecordView<'a> {
    /// Configuration version, should be 1
    pub configuration_version: u8,
    /// AVC profile indication
    pub avc_profile_indication: u8,
    /// Profile compatibility
    pub profile_compatibility: u8,
    /// AVC level indication
    pub avc_level_indication: u8,
    /// Length size minus one
    pub length_size_minus_one: u8,
    /// Number of sequence parameter sets
    pub num_of_sps: u8,
    /// Sequence parameter sets
    sps: &'a [u8],
    /// Number of picture parameter sets
    pub num_of_pps: u8,
    /// Picture parameter sets
    pps: &'a [u8],
    /// Chroma format (optional)
    pub chroma_format: Option<u8>,
    /// Bit depth luma minus 8 (optional)
    pub bit_depth_luma_minus8: Option<u8>,
    /// Bit depth chroma minus 8 (optional)
    pub bit_depth_chroma_minus8: Option<u8>,
    /// Number of sequence parameter set extensions (optional)
    pub num_of_sps_ext: Option<u8>,
    /// Sequence parameter set extensions (optional)
    sps_ext: Option<&'a [u8]>,
}

impl<'a> AVCDecoderConfigurationRecordView<'a> {
    /// Returns an iterator over the sequence parameter sets
    pub fn sps(&self) -> NalUnitIter<'a> {
        NalUnitIter {
            data: self.sps,
            remaining: self.num_of_sps as usize,
        }
    }

    /// Returns an iterator over the picture parameter sets
    pub fn pps(&self) -> NalUnitIter<'a> {
        NalUnitIter {
            data: self.pps,
            remaining: self.num_of_pps as usize,
        }
    }

    /// Returns an iterator over the sequence parameter set extensions, if present
    pub fn sps_ext(&self) -> Option<NalUnitIter<'a>> {
        self.sps_ext
            .zip(self.num_of_sps_ext)
            .map(|(sps_ext, n)| NalUnitIter {
                data: sps_ext,
                remaining: n as usize,
            })
    }

    /// Parses an AVCDecoderConfigurationRecord from the given byte slice.
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let configuration_version = cur.read_u8()?;
        let avc_profile_indication = cur.read_u8()?;
        let profile_compatibility = cur.read_u8()?;
        let avc_level_indication = cur.read_u8()?;

        // reserved (6 bits) + lengthSizeMinusOne (2 bits)
        let length_size_minus_one = cur.read_u8()? & 0x03;

        // reserved (3 bits) + numOfSequenceParameterSets (5 bits)
        let num_of_sps = cur.read_u8()? & 0x1F;
        let sps_start_position = cur.position();
        for _ in 0..num_of_sps {
            let sps_size = cur.read_u16_be()? as usize;
            cur.advance(sps_size)?;
        }
        let sps_end_position = cur.position();
        cur.set_position(sps_start_position);
        let sps = cur.take(sps_end_position - sps_start_position)?;

        // numOfPictureParameterSets
        let num_of_pps = cur.read_u8()?;
        let pps_start_position = cur.position();
        for _ in 0..num_of_pps {
            let pps_size = cur.read_u16_be()? as usize;
            cur.advance(pps_size)?;
        }
        let pps_end_position = cur.position();
        cur.set_position(pps_start_position);
        let pps = cur.take(pps_end_position - pps_start_position)?;

        let mut chroma_format = None;
        let mut bit_depth_luma_minus8 = None;
        let mut bit_depth_chroma_minus8 = None;
        let mut num_of_sps_ext = None;
        let mut sps_ext = None;

        if [100, 110, 122, 144].contains(&avc_profile_indication) {
            // reserved (6 bits) + chromaFormat (2 bits)
            chroma_format = Some(cur.read_u8()? & 0x03);
            // reserved (5 bits) + bitDepthLumaMinus8 (3 bits)
            bit_depth_luma_minus8 = Some(cur.read_u8()? & 0x07);
            // reserved (5 bits) + bitDepthChromaMinus8 (3 bits)
            bit_depth_chroma_minus8 = Some(cur.read_u8()? & 0x07);

            // numOfSequenceParameterSetExt
            num_of_sps_ext = Some(cur.read_u8()?);
            let sps_ext_start_position = cur.position();
            for _ in 0..num_of_sps_ext.unwrap() {
                let sps_ext_size = cur.read_u16_be()? as usize;
                cur.advance(sps_ext_size)?;
            }
            let sps_ext_end_position = cur.position();
            cur.set_position(sps_ext_start_position);
            sps_ext = Some(cur.take(sps_ext_end_position - sps_ext_start_position)?);
        }

        Ok(AVCDecoderConfigurationRecordView {
            configuration_version,
            avc_profile_indication,
            profile_compatibility,
            avc_level_indication,
            length_size_minus_one,
            num_of_sps,
            sps,
            num_of_pps,
            pps,
            chroma_format,
            bit_depth_luma_minus8,
            bit_depth_chroma_minus8,
            num_of_sps_ext,
            sps_ext,
        })
    }
}

/// A reference to an AVC Decoder Configuration Box
#[derive(Debug)]
pub struct AvcCBoxView<'a> {
    /// The AVC Decoder Configuration Record
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

/// A reference to an AVC Sample Entry
#[derive(Debug)]
pub struct AvcSampleEntryView<'a, S> {
    base: VisualSampleEntry,
    content: &'a [u8],
    _marker: PhantomData<S>,
}

impl<'a, S> AvcSampleEntryView<'a, S> {
    /// Returns the base Visual Sample Entry
    pub fn base(&self) -> &VisualSampleEntry {
        &self.base
    }

    /// Returns an iterator over the child boxes of this sample entry.
    pub fn boxes(&self) -> BoxIter<'a> {
        BoxIter::new(self.content)
    }

    /// Returns the AVC Configuration Box (`avcC`) contained in this sample entry.
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
}

impl<'de, S> BoxDecode<'de> for AvcSampleEntryView<'de, S> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);
        let base = VisualSampleEntry::parse_in(&mut cur)?;
        let content = cur.take(cur.remaining())?;

        Ok(AvcSampleEntryView {
            base,
            content,
            _marker: PhantomData,
        })
    }
}

/// A marker type for AVC1 Sample Entry
pub struct Avc1;
/// A reference to an AVC1 Sample Entry
pub type Avc1SampleEntryView<'a> = AvcSampleEntryView<'a, Avc1>;

impl BoxCodec for Avc1SampleEntryView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::AVC1
    }
}

/// A marker type for AVC3 Sample Entry
pub struct Avc3;
/// A reference to an AVC3 Sample Entry
pub type Avc3SampleEntryView<'a> = AvcSampleEntryView<'a, Avc3>;

impl BoxCodec for Avc3SampleEntryView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::AVC3
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use core::fmt;

    use crate::lib::String;

    use super::*;

    /// An owned AVC Decoder configuration record
    #[derive(Debug, Clone)]
    pub struct AVCDecoderConfigurationRecord {
        /// Configuration version, should be 1
        pub configuration_version: u8,
        /// AVC profile indication
        pub avc_profile_indication: u8,
        /// Profile compatibility
        pub profile_compatibility: u8,
        /// AVC level indication
        pub avc_level_indication: u8,
        /// Length size minus one
        pub length_size_minus_one: u8,
        /// Sequence parameter sets
        pub sps: Vec<Vec<u8>>,
        /// Picture parameter sets
        pub pps: Vec<Vec<u8>>,
        /// Chroma format (optional)
        pub chroma_format: Option<u8>,
        /// Bit depth luma minus 8 (optional)
        pub bit_depth_luma_minus8: Option<u8>,
        /// Bit depth chroma minus 8 (optional)
        pub bit_depth_chroma_minus8: Option<u8>,
        /// Sequence parameter set extensions (optional)
        pub sps_ext: Option<Vec<Vec<u8>>>,
    }

    impl From<&AVCDecoderConfigurationRecordView<'_>> for AVCDecoderConfigurationRecord {
        fn from(view: &AVCDecoderConfigurationRecordView<'_>) -> Self {
            let sps = view.sps().map(|nalu| nalu.to_vec()).collect();
            let pps = view.pps().map(|nalu| nalu.to_vec()).collect();
            let sps_ext = view
                .sps_ext()
                .map(|iter| iter.map(|nalu| nalu.to_vec()).collect());

            AVCDecoderConfigurationRecord {
                configuration_version: view.configuration_version,
                avc_profile_indication: view.avc_profile_indication,
                profile_compatibility: view.profile_compatibility,
                avc_level_indication: view.avc_level_indication,
                length_size_minus_one: view.length_size_minus_one,
                sps,
                pps,
                chroma_format: view.chroma_format,
                bit_depth_luma_minus8: view.bit_depth_luma_minus8,
                bit_depth_chroma_minus8: view.bit_depth_chroma_minus8,
                sps_ext,
            }
        }
    }

    /// An owned AVC Configuration Box
    #[derive(Debug, Clone)]
    pub struct AvcCBox {
        /// The AVC Decoder Configuration Record
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

    /// An owned AVC Sample Entry
    #[derive(Debug, Clone)]
    pub struct AvcSampleEntry<S> {
        /// The base Visual Sample Entry
        pub base: VisualSampleEntry,
        /// The AVC Configuration Box (`avcC`)
        pub avcc: AvcCBox,
        _marker: PhantomData<S>,
    }

    impl<S> TryFrom<&AvcSampleEntryView<'_, S>> for AvcSampleEntry<S> {
        type Error = Error;

        fn try_from(view: &AvcSampleEntryView<'_, S>) -> Result<Self> {
            let avcc = view.avcc()?;
            Ok(AvcSampleEntry {
                base: view.base,
                avcc: AvcCBox::from(&avcc),
                _marker: PhantomData,
            })
        }
    }

    impl<S> BoxDecode<'_> for AvcSampleEntry<S> {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = AvcSampleEntryView::decode(bytes)?;
            AvcSampleEntry::try_from(&view)
        }
    }

    /// An owned AVC1 Sample Entry
    pub type Avc1SampleEntry = AvcSampleEntry<Avc1>;

    impl Avc1SampleEntry {
        /// Returns the codec string (e.g., "avc1.640028")
        pub fn codec_string(&self) -> String {
            let mut buf = String::new();
            self.write_codec_string(&mut buf).unwrap();
            buf
        }

        /// Writes the codec string (e.g., "avc1.640028") to the given formatter.
        pub fn write_codec_string(&self, w: &mut impl fmt::Write) -> fmt::Result {
            write!(
                w,
                "avc1.{:02X}{:02X}{:02X}",
                self.avcc.avc_config.avc_profile_indication,
                self.avcc.avc_config.profile_compatibility,
                self.avcc.avc_config.avc_level_indication
            )
        }
    }

    impl BoxCodec for Avc1SampleEntry {
        fn boxtype(&self) -> BoxType {
            BoxType::AVC1
        }
    }

    /// An owned AVC3 Sample Entry
    pub type Avc3SampleEntry = AvcSampleEntry<Avc3>;

    impl BoxCodec for Avc3SampleEntry {
        fn boxtype(&self) -> BoxType {
            BoxType::AVC3
        }
    }
}

#[cfg(feature = "alloc")]
pub use owned::*;
