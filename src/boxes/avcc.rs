//! ISO/IEC 14496-15 (AVC/HEVC file format) boxes.

use crate::BoxType;

use crate::BoxCodec;
use crate::BoxDecode;
use crate::error::*;
use crate::iter::BoxIter;

use crate::boxes::VisualSampleEntry;

use crate::cursor::ReadCursor;

/// AVC Configuration Box (`avcC`)
#[derive(Debug)]
pub struct AvcCBoxView<'a> {
    /// Configuration Version
    pub configuration_version: u8,
    /// AVC Profile Indication
    pub avc_profile_indication: u8,
    /// Profile Compatibility
    pub profile_compatibility: u8,
    /// AVC Level Indication
    pub avc_level_indication: u8,
    /// Length Size Minus One
    pub length_size_minus_one: u8,
    /// Number of SPS NAL units
    pub nb_sps_nalus: u8,
    sps: &'a [u8],
    /// Number of PPS NAL units
    pub nb_pps_nalus: u8,
    pps: &'a [u8],
    /// Chroma Format IDC (ISO/IEC 14496-10)
    pub chroma_format_idc: Option<u8>,
    /// Bit Depth Luma Minus8
    pub bit_depth_luma_minus8: Option<u8>,
    /// Bit Depth Chroma Minus8
    pub bit_depth_chroma_minus8: Option<u8>,
    /// SPS extension NAL units (ISO/IEC 14496-10)
    sps_ext: Option<&'a [u8]>,
}

impl<'a> AvcCBoxView<'a> {
    /// Returns an iterator over the SPS NAL units
    pub fn sps(&self) -> NalUnitIter<'_> {
        NalUnitIter { data: self.sps }
    }

    /// Returns an iterator over the PPS NAL units
    pub fn pps(&self) -> NalUnitIter<'_> {
        NalUnitIter { data: self.pps }
    }

    /// Returns an iterator over the SPS extension NAL units, if present
    pub fn sps_ext(&self) -> Option<NalUnitIter<'_>> {
        self.sps_ext.map(|data| NalUnitIter { data })
    }
}

impl BoxCodec for AvcCBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::AVCC
    }
}

impl<'de> BoxDecode<'de> for AvcCBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let configuration_version = cur.read_u8()?;
        let avc_profile_indication = cur.read_u8()?;
        let avc_profile_compatibility = cur.read_u8()?;
        let avc_level_indication = cur.read_u8()?;
        // reserved (6 bits) | lengthSizeMinusOne (2 bits)
        let length_size_minus_one = cur.read_u8()? & 0x03;

        // SPS NAL units
        // reserved (3 bits) | numOfSequenceParameterSets (5 bits)
        let nb_sps_nalus = cur.read_u8()? & 0x1f;
        let sps_start = cur.position();
        for _ in 0..nb_sps_nalus {
            let sps_length = cur.read_u16_be()? as usize;
            // bit(8) * sps_length
            cur.advance(sps_length)?;
        }
        let sps = &cur.inner()[sps_start..cur.position()];

        // PPS NAL units
        let nb_pps_nalus = cur.read_u8()?;
        let pps_start = cur.position();
        for _ in 0..nb_pps_nalus {
            let pps_length = cur.read_u16_be()? as usize;
            // bit(8) * pps_length
            cur.advance(pps_length)?;
        }
        let pps = &cur.inner()[pps_start..cur.position()];

        // Chroma Format IDC and Bit Depths (ISO/IEC 14496-15)
        let (chroma_format_idc, bit_depth_luma_minus8, bit_depth_chroma_minus8, sps_ext) =
            if avc_profile_indication == 100
                || avc_profile_indication == 110
                || avc_profile_indication == 122
                || avc_profile_indication == 244
            {
                // reserved (6 bits) | chroma_format_idc (2 bits)
                let chroma_format_idc = cur.read_u8()? & 0x03;
                // reserved (5 bits) | bit_depth_luma_minus8 (3 bits)
                let bit_depth_luma_minus8 = cur.read_u8()? & 0x07;
                // reserved (5 bits) | bit_depth_chroma_minus8 (3 bits)
                let bit_depth_chroma_minus8 = cur.read_u8()? & 0x07;

                // SPS extension NAL units
                let sps_ext = if !cur.is_empty() {
                    let nb_sps_ext = cur.read_u8()?;
                    let sps_ext_start = cur.position();
                    for _ in 0..nb_sps_ext {
                        let len = cur.read_u16_be()? as usize;
                        cur.advance(len)?;
                    }
                    Some(&cur.inner()[sps_ext_start..cur.position()])
                } else {
                    None
                };

                (
                    Some(chroma_format_idc),
                    Some(bit_depth_luma_minus8),
                    Some(bit_depth_chroma_minus8),
                    sps_ext,
                )
            } else {
                (None, None, None, None)
            };

        Ok(AvcCBoxView {
            configuration_version,
            avc_profile_indication,
            profile_compatibility: avc_profile_compatibility,
            avc_level_indication,
            length_size_minus_one,
            nb_sps_nalus,
            sps,
            nb_pps_nalus,
            pps,
            chroma_format_idc,
            bit_depth_luma_minus8,
            bit_depth_chroma_minus8,
            sps_ext,
        })
    }
}

impl<'a> TryFrom<&'a [u8]> for AvcCBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        AvcCBoxView::decode(value)
    }
}

/// An iterator over NAL units in an AVC configuration box.
pub struct NalUnitIter<'a> {
    data: &'a [u8],
}

impl<'a> Iterator for NalUnitIter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.len() < 2 {
            return None;
        }

        let len = u16::from_be_bytes([self.data[0], self.data[1]]) as usize;
        if self.data.len() < 2 + len {
            return None;
        }

        let nalu = &self.data[2..2 + len];
        self.data = &self.data[2 + len..];
        Some(nalu)
    }
}

/// AVC Sample Entry View (`avcX`)
#[derive(Debug)]
pub struct AvcSampleEntryView<'a, C> {
    base: VisualSampleEntry,
    extensions: &'a [u8],
    _marker: core::marker::PhantomData<&'a C>,
}

/// Specification type for AVC Sample Entry Box ('avc1')
#[derive(Debug, Clone)]
pub struct Avc1Spec;
/// Type alias for AVC1 Sample Entry Box View
pub type Avc1BoxView<'a> = AvcSampleEntryView<'a, Avc1Spec>;

/// Specification type for AVC Sample Entry Box ('avc3')
#[derive(Debug, Clone)]
pub struct Avc3Spec;
/// Type alias for AVC3 Sample Entry Box View
pub type Avc3BoxView<'a> = AvcSampleEntryView<'a, Avc3Spec>;

impl<'a, C> AvcSampleEntryView<'a, C> {
    /// Returns the base Visual Sample Entry.
    pub fn visual_sample_entry(&self) -> &VisualSampleEntry {
        &self.base
    }

    /// Returns an iterator over the extension boxes.
    pub fn extensions(&self) -> BoxIter<'a> {
        BoxIter::new(self.extensions)
    }

    /// Returns the AVC Configuration Box (`avcC`) if present.
    pub fn avcc(&self) -> Result<AvcCBoxView<'a>> {
        for child in self.extensions() {
            match child {
                Ok(c) if c.boxtype() == BoxType::AVCC => {
                    return AvcCBoxView::decode(c.into_payload());
                }
                Ok(_) => continue,
                Err(e) => return Err(e),
            }
        }

        Err(Error::in_box(
            ErrorKind::BoxMissing {
                required: BoxType::AVCC,
            },
            BoxType::AVC1,
        ))
    }
}

impl BoxCodec for Avc1BoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::AVC1
    }
}

impl BoxCodec for Avc3BoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::AVC3
    }
}

impl<'de, C> BoxDecode<'de> for AvcSampleEntryView<'de, C> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let base = VisualSampleEntry::parse_in(&mut cur)?;
        let extensions = cur.take(cur.remaining())?;

        Ok(AvcSampleEntryView {
            base,
            extensions,
            _marker: core::marker::PhantomData,
        })
    }
}

impl<'a, C> TryFrom<&'a [u8]> for AvcSampleEntryView<'a, C> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        AvcSampleEntryView::decode(value)
    }
}

#[cfg(feature = "alloc")]
pub use owned::{
    Avc1Box, //
    Avc3Box,
    AvcCBox,
};

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::{
        String, //
        Vec,
        format,
    };

    use super::*;

    use crate::codec::BoxEncode;
    use crate::codec::boxed_len;
    use crate::codec::write_box_in;

    use crate::cursor::WriteCursor;

    /// An owned AVC Configuration Box (`avcC`)
    #[derive(Debug, Clone)]
    pub struct AvcCBox {
        /// Configuration Version
        pub configuration_version: u8,
        /// AVC Profile Indication
        pub avc_profile_indication: u8,
        /// Profile Compatibility
        pub profile_compatibility: u8,
        /// AVC Level Indication
        pub avc_level_indication: u8,
        /// Length Size Minus One
        pub length_size_minus_one: u8,
        /// Number of SPS NAL units
        pub sps: Vec<Vec<u8>>,
        /// Number of PPS NAL units
        pub pps: Vec<Vec<u8>>,
        /// Chroma Format IDC (ISO/IEC 14496-10)
        pub chroma_format_idc: Option<u8>,
        /// Bit Depth Luma Minus8
        pub bit_depth_luma_minus8: Option<u8>,
        /// Bit Depth Chroma Minus8
        pub bit_depth_chroma_minus8: Option<u8>,
        /// SPS extension NAL units (ISO/IEC 14496-10)
        pub sps_ext: Option<Vec<Vec<u8>>>,
    }

    impl From<&AvcCBoxView<'_>> for AvcCBox {
        fn from(view: &AvcCBoxView<'_>) -> Self {
            let sps = view.sps().map(|nalu| nalu.to_vec()).collect();
            let pps = view.pps().map(|nalu| nalu.to_vec()).collect();
            let sps_ext = view
                .sps_ext()
                .map(|iter| iter.map(|nalu| nalu.to_vec()).collect());

            AvcCBox {
                configuration_version: view.configuration_version,
                avc_profile_indication: view.avc_profile_indication,
                profile_compatibility: view.profile_compatibility,
                avc_level_indication: view.avc_level_indication,
                length_size_minus_one: view.length_size_minus_one,
                sps,
                pps,
                chroma_format_idc: view.chroma_format_idc,
                bit_depth_luma_minus8: view.bit_depth_luma_minus8,
                bit_depth_chroma_minus8: view.bit_depth_chroma_minus8,
                sps_ext,
            }
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
        #[inline]
        fn encoded_len(&self) -> usize {
            let mut len = 5; // fixed fields
            len += 1; // numOfSequenceParameterSets

            // SPS NAL units
            for sps in &self.sps {
                len += 2; // length field
                len += sps.len();
            }

            len += 1; // numOfPictureParameterSets

            // PPS NAL units
            for pps in &self.pps {
                len += 2; // length field
                len += pps.len();
            }

            // chroma_format_idc
            if self.chroma_format_idc.is_some() {
                len += 1;
            }

            // bit_depth_luma_minus8
            if self.bit_depth_luma_minus8.is_some() {
                len += 1;
            }

            // bit_depth_chroma_minus8
            if self.bit_depth_chroma_minus8.is_some() {
                len += 1;
            }

            // SPS extension NAL units
            if let Some(sps_ext) = &self.sps_ext {
                len += 1; // numOfSequenceParameterSetExt
                for ext in sps_ext {
                    len += 2; // length field
                    len += ext.len();
                }
            }

            len
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.configuration_version)?;
            cur.write_u8(self.avc_profile_indication)?;
            cur.write_u8(self.profile_compatibility)?;
            cur.write_u8(self.avc_level_indication)?;

            // reserved (6 bits) | lengthSizeMinusOne (2 bits)
            cur.write_u8(0xFC | (self.length_size_minus_one & 0x03))?;

            // reserved (3 bits) | numOfSequenceParameterSets (5 bits)
            cur.write_u8(0xE0 | (self.sps.len() as u8 & 0x1F))?;
            // SPS NAL units
            for sps in &self.sps {
                cur.write_u16_be(sps.len() as u16)?;
                cur.write_slice(sps)?;
            }

            // numOfPictureParameterSets
            cur.write_u8(self.pps.len() as u8)?;

            // PPS NAL units
            for pps in &self.pps {
                cur.write_u16_be(pps.len() as u16)?;
                cur.write_slice(pps)?;
            }

            // Chroma Format IDC and Bit Depths (ISO/IEC 14496-10)
            if let Some(chroma_format_idc) = self.chroma_format_idc {
                // reserved (6 bits) | chroma_format_idc (2 bits)
                cur.write_u8(0xFC | (chroma_format_idc & 0x03))?;
            }

            if let Some(bit_depth_luma_minus8) = self.bit_depth_luma_minus8 {
                // reserved (5 bits) | bit_depth_luma_minus8 (3 bits)
                cur.write_u8(0xF8 | (bit_depth_luma_minus8 & 0x07))?;
            }

            if let Some(bit_depth_chroma_minus8) = self.bit_depth_chroma_minus8 {
                // reserved (5 bits) | bit_depth_chroma_minus8 (3 bits)
                cur.write_u8(0xF8 | (bit_depth_chroma_minus8 & 0x07))?;
            }

            // SPS extension NAL units
            if let Some(sps_ext) = &self.sps_ext {
                // numOfSequenceParameterSetExtLength
                cur.write_u8(sps_ext.len() as u8)?;

                for ext in sps_ext {
                    cur.write_u16_be(ext.len() as u16)?;
                    cur.write_slice(ext)?;
                }
            }

            Ok(cur.position())
        }
    }

    /// An owned AVC Sample Entry Box (`avc1`)
    #[derive(Debug, Clone)]
    pub struct AvcSampleEntry<C> {
        /// The base Visual Sample Entry.
        pub base: VisualSampleEntry,
        // pub clap: Option<ClapBox>,
        // pub pasp: Option<PaspBox>,
        /// The AVC Configuration Box.
        pub avcc: AvcCBox,
        _marker: core::marker::PhantomData<C>,
    }

    impl<C> AvcSampleEntry<C> {
        pub fn new(base: VisualSampleEntry, avcc: AvcCBox) -> Self {
            Self {
                base,
                avcc,
                _marker: core::marker::PhantomData,
            }
        }
    }

    /// Type alias for owned AVC1 Sample Entry Box
    pub type Avc1Box = AvcSampleEntry<Avc1Spec>;
    /// Type alias for owned AVC3 Sample Entry Box
    pub type Avc3Box = AvcSampleEntry<Avc3Spec>;

    impl Avc1Box {
        /// Returns the codec string in the format "avc1.ppccll"
        pub fn codec(&self) -> String {
            format!(
                "avc1.{:02X}{:02X}{:02X}",
                self.avcc.avc_profile_indication,
                self.avcc.profile_compatibility,
                self.avcc.avc_level_indication
            )
        }
    }

    impl Avc3Box {
        /// Returns the codec string in the format "avc3.ppccll"
        pub fn codec(&self) -> String {
            format!(
                "avc3.{:02X}{:02X}{:02X}",
                self.avcc.avc_profile_indication,
                self.avcc.profile_compatibility,
                self.avcc.avc_level_indication
            )
        }
    }

    impl<C> TryFrom<&AvcSampleEntryView<'_, C>> for AvcSampleEntry<C> {
        type Error = Error;

        fn try_from(view: &AvcSampleEntryView<'_, C>) -> Result<Self> {
            let mut avcc = None;

            for extention in view.extensions() {
                match extention {
                    Ok(c) if c.boxtype() == BoxType::AVCC => {
                        let avcc_view = AvcCBoxView::decode(c.payload())?;
                        avcc = Some(AvcCBox::from(&avcc_view));
                    }
                    Ok(_) => continue,
                    Err(e) => return Err(e),
                }
            }

            Ok(AvcSampleEntry {
                base: view.base,
                avcc: avcc.ok_or(Error::in_box(
                    ErrorKind::BoxMissing {
                        required: BoxType::AVCC,
                    },
                    BoxType::AVC1,
                ))?,
                _marker: core::marker::PhantomData,
            })
        }
    }

    impl BoxCodec for Avc1Box {
        fn boxtype(&self) -> BoxType {
            BoxType::AVC1
        }
    }

    impl BoxCodec for Avc3Box {
        fn boxtype(&self) -> BoxType {
            BoxType::AVC3
        }
    }

    impl<C> BoxDecode<'_> for AvcSampleEntry<C> {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = AvcSampleEntryView::decode(bytes)?;
            AvcSampleEntry::try_from(&view)
        }
    }

    impl<C> BoxEncode for AvcSampleEntry<C> {
        #[inline]
        fn encoded_len(&self) -> usize {
            let base_len = VisualSampleEntry::size();
            let avcc_len = boxed_len(&self.avcc);

            base_len + avcc_len
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);
            self.base.write_in(&mut cur)?;

            write_box_in(&mut cur, &self.avcc)?;

            Ok(cur.position())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_nalu(data: &[u8]) -> Vec<u8> {
        let mut result = Vec::new();
        result.extend_from_slice(&(data.len() as u16).to_be_bytes());
        result.extend_from_slice(data);
        result
    }

    struct AvccExt {
        chroma_format_idc: u8,
        bit_depth_luma_minus8: u8,
        bit_depth_chroma_minus8: u8,
        sps_ext: Vec<Vec<u8>>,
    }

    fn make_avcc_payload(
        profile: u8,
        compatibility: u8,
        level: u8,
        sps_list: &[&[u8]],
        pps_list: &[&[u8]],
        ext: Option<AvccExt>,
    ) -> Vec<u8> {
        let mut data = vec![
            1,                                    // configurationVersion
            profile,                              // AVCProfileIndication
            compatibility,                        // profile_compatibility
            level,                                // AVCLevelIndication
            0xFF, // reserved (6 bits) | lengthSizeMinusOne (2 bits) = 3
            0xE0 | (sps_list.len() as u8 & 0x1F), // reserved (3 bits) | numOfSequenceParameterSets (5 bits)
        ];

        for sps in sps_list {
            data.extend_from_slice(&make_nalu(sps));
        }

        data.push(pps_list.len() as u8);

        for pps in pps_list {
            data.extend_from_slice(&make_nalu(pps));
        }

        // Extension fields for High profiles (100, 110, 122, 244)
        if let Some(ext) = ext {
            data.push(0xFC | (ext.chroma_format_idc & 0x03));
            data.push(0xF8 | (ext.bit_depth_luma_minus8 & 0x07));
            data.push(0xF8 | (ext.bit_depth_chroma_minus8 & 0x07));
            data.push(ext.sps_ext.len() as u8);
            for sps_ext in &ext.sps_ext {
                data.extend_from_slice(&make_nalu(sps_ext));
            }
        }

        data
    }

    fn make_box_header(size: u32, fourcc: &[u8; 4]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&size.to_be_bytes());
        data.extend_from_slice(fourcc);
        data
    }

    fn make_visual_sample_entry(width: u16, height: u16) -> Vec<u8> {
        let mut data = Vec::new();

        // SampleEntry base (8 bytes)
        data.extend_from_slice(&[0u8; 6]); // reserved
        data.extend_from_slice(&1u16.to_be_bytes()); // data_reference_index

        // VisualSampleEntry fields
        data.extend_from_slice(&[0u8; 2]); // pre_defined
        data.extend_from_slice(&[0u8; 2]); // reserved
        data.extend_from_slice(&[0u8; 12]); // pre_defined
        data.extend_from_slice(&width.to_be_bytes());
        data.extend_from_slice(&height.to_be_bytes());
        data.extend_from_slice(&0x00480000u32.to_be_bytes()); // horiz_resolution
        data.extend_from_slice(&0x00480000u32.to_be_bytes()); // vert_resolution
        data.extend_from_slice(&[0u8; 4]); // reserved
        data.extend_from_slice(&1u16.to_be_bytes()); // frame_count
        data.extend_from_slice(&[0u8; 32]); // compressorname
        data.extend_from_slice(&0x0018u16.to_be_bytes()); // depth
        data.extend_from_slice(&[0xFF, 0xFF]); // pre_defined

        data
    }

    #[test]
    fn avcc_parse_multiple_sps_pps() {
        let sps1 = b"sps_one";
        let sps2 = b"sps_two";
        let pps1 = b"pps_one";
        let pps2 = b"pps_two";

        // Use Baseline profile (66) which doesn't require extension fields
        let payload = make_avcc_payload(66, 0, 31, &[sps1, sps2], &[pps1, pps2], None);
        let avcc = AvcCBoxView::decode(&payload).unwrap();

        assert_eq!(avcc.avc_profile_indication, 66);
        assert_eq!(avcc.avc_level_indication, 31);

        let sps_list: Vec<_> = avcc.sps().collect();
        assert_eq!(sps_list, vec![&b"sps_one"[..], &b"sps_two"[..]]);

        let pps_list: Vec<_> = avcc.pps().collect();
        assert_eq!(pps_list, vec![&b"pps_one"[..], &b"pps_two"[..]]);

        assert!(avcc.chroma_format_idc.is_none());
        assert!(avcc.sps_ext.is_none());
    }

    #[test]
    fn avcc_parse_high_profile_with_extensions() {
        let sps = b"sps_data";
        let pps = b"pps_data";
        let sps_ext_data = b"sps_ext";

        let ext = AvccExt {
            chroma_format_idc: 1,
            bit_depth_luma_minus8: 0,
            bit_depth_chroma_minus8: 0,
            sps_ext: vec![sps_ext_data.to_vec()],
        };

        let payload = make_avcc_payload(100, 0, 40, &[sps], &[pps], Some(ext));
        let avcc = AvcCBoxView::decode(&payload).unwrap();

        assert_eq!(avcc.avc_profile_indication, 100);
        assert_eq!(avcc.chroma_format_idc, Some(1));
        assert_eq!(avcc.bit_depth_luma_minus8, Some(0));
        assert_eq!(avcc.bit_depth_chroma_minus8, Some(0));

        let sps_ext_list: Vec<_> = avcc.sps_ext().unwrap().collect();
        assert_eq!(sps_ext_list, vec![&b"sps_ext"[..]]);
    }

    #[test]
    fn avcc_parse_truncated() {
        let payload = [1, 100, 0, 31, 0xFF];
        assert!(AvcCBoxView::decode(&payload).is_err());
    }

    #[test]
    fn avc1_parse_with_avcc() {
        let sps = b"\x67\x64\x00\x1f";
        let pps = b"\x68\xeb\xe3\xcb";
        let ext = AvccExt {
            chroma_format_idc: 1,
            bit_depth_luma_minus8: 0,
            bit_depth_chroma_minus8: 0,
            sps_ext: vec![],
        };
        let avcc_payload = make_avcc_payload(100, 0, 31, &[sps], &[pps], Some(ext));

        let avcc_size = 8 + avcc_payload.len() as u32;
        let mut avcc_box = make_box_header(avcc_size, b"avcC");
        avcc_box.extend_from_slice(&avcc_payload);

        let mut payload = make_visual_sample_entry(1920, 1080);
        payload.extend_from_slice(&avcc_box);

        let avc1 = Avc1BoxView::decode(&payload).unwrap();

        assert_eq!(avc1.visual_sample_entry().width, 1920);
        assert_eq!(avc1.visual_sample_entry().height, 1080);

        let avcc = avc1.avcc().unwrap();
        assert_eq!(avcc.avc_profile_indication, 100);
    }

    #[test]
    fn avc1_missing_avcc() {
        let payload = make_visual_sample_entry(1920, 1080);
        let avc1 = Avc1BoxView::decode(&payload).unwrap();
        assert!(avc1.avcc().is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn avc1_box_codec_string() {
        let sps = b"sps";
        let pps = b"pps";
        let ext = AvccExt {
            chroma_format_idc: 1,
            bit_depth_luma_minus8: 0,
            bit_depth_chroma_minus8: 0,
            sps_ext: vec![],
        };
        let avcc_payload = make_avcc_payload(100, 0, 31, &[sps], &[pps], Some(ext));

        let avcc_size = 8 + avcc_payload.len() as u32;
        let mut avcc_box = make_box_header(avcc_size, b"avcC");
        avcc_box.extend_from_slice(&avcc_payload);

        let mut payload = make_visual_sample_entry(1920, 1080);
        payload.extend_from_slice(&avcc_box);

        let avc1 = Avc1Box::decode(&payload).unwrap();
        assert_eq!(avc1.codec(), "avc1.64001F");
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn avc1_box_round_trip() {
        use crate::codec::BoxEncode;

        let sps = b"\x67\x64\x00\x1f";
        let pps = b"\x68\xeb\xe3\xcb";
        let ext = AvccExt {
            chroma_format_idc: 1,
            bit_depth_luma_minus8: 0,
            bit_depth_chroma_minus8: 0,
            sps_ext: vec![],
        };
        let avcc_payload = make_avcc_payload(100, 0, 31, &[sps], &[pps], Some(ext));

        let avcc_size = 8 + avcc_payload.len() as u32;
        let mut avcc_box = make_box_header(avcc_size, b"avcC");
        avcc_box.extend_from_slice(&avcc_payload);

        let mut original_payload = make_visual_sample_entry(1920, 1080);
        original_payload.extend_from_slice(&avcc_box);

        // Parse
        let original = Avc1Box::decode(&original_payload).unwrap();

        // Write
        let mut buf = vec![0u8; original.encoded_len()];
        original.encode_into(&mut buf).unwrap();

        // Parse again
        let reparsed = Avc1Box::decode(&buf).unwrap();

        assert_eq!(reparsed.base.width, original.base.width);
        assert_eq!(reparsed.base.height, original.base.height);
        assert_eq!(
            reparsed.avcc.avc_profile_indication,
            original.avcc.avc_profile_indication
        );
        assert_eq!(reparsed.avcc.sps, original.avcc.sps);
        assert_eq!(reparsed.avcc.pps, original.avcc.pps);
        assert_eq!(
            reparsed.avcc.chroma_format_idc,
            original.avcc.chroma_format_idc
        );
    }
}
