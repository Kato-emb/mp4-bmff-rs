use crate::cursor::ReadCursor;

use crate::BoxIter;
use crate::BoxType;
use crate::error::*;

use crate::boxes::VisualSampleEntry;

/// AVC Configuration Box (`avcC`)
#[derive(Debug)]
pub struct AvcCBoxView<'a> {
    /// Configuration Version
    pub configuration_version: u8,
    /// AVC Profile Indication
    pub avc_profile_indication: u8,
    /// Profile Compatibility
    pub avc_profile_compatibility: u8,
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
    /// Extensions (optional)
    pub ext: Option<&'a [u8]>,
}

impl<'a> AvcCBoxView<'a> {
    /// Returns an iterator over the SPS NAL units
    pub fn sps(&self) -> impl Iterator<Item = &'a [u8]> {
        NalUnitIter { data: self.sps }
    }

    /// Returns an iterator over the PPS NAL units
    pub fn pps(&self) -> impl Iterator<Item = &'a [u8]> {
        NalUnitIter { data: self.pps }
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<AvcCBoxView<'a>> {
        let configuration_version = cur.read_u8()?;
        let avc_profile_indication = cur.read_u8()?;
        let avc_profile_compatibility = cur.read_u8()?;
        let avc_level_indication = cur.read_u8()?;
        let length_size_minus_one = cur.read_u8()? & 0x03;

        // SPS NAL units
        let nb_sps_nalus = cur.read_u8()? & 0x1f;
        let sps_start = cur.position();
        for _ in 0..nb_sps_nalus {
            let sps_length = cur.read_u16_be()? as usize;
            cur.advance(sps_length)?;
        }
        let sps = &cur.inner()[sps_start..cur.position()];

        // PPS NAL units
        let nb_pps_nalus = cur.read_u8()?;
        let pps_start = cur.position();
        for _ in 0..nb_pps_nalus {
            let pps_length = cur.read_u16_be()? as usize;
            cur.advance(pps_length)?;
        }
        let pps = &cur.inner()[pps_start..cur.position()];

        // Extensions (optional)
        let ext = if !cur.is_empty() {
            Some(cur.take(cur.remaining())?)
        } else {
            None
        };

        Ok(AvcCBoxView {
            configuration_version,
            avc_profile_indication,
            avc_profile_compatibility,
            avc_level_indication,
            length_size_minus_one,
            nb_sps_nalus,
            sps,
            nb_pps_nalus,
            pps,
            ext,
        })
    }

    /// Parses an `AvcCBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<AvcCBoxView<'a>> {
        let mut cursor = ReadCursor::new(payload);
        let this = AvcCBoxView::parse_in(&mut cursor)?;

        Ok(this)
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

/// AVC Sample Entry Box (`avc1`)
#[derive(Debug)]
pub struct Avc1BoxView<'a> {
    base: VisualSampleEntry,
    extensions: &'a [u8],
}

impl<'a> Avc1BoxView<'a> {
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
                Ok(c) if c.header.boxtype() == BoxType::AVCC => {
                    return AvcCBoxView::parse(c.payload);
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

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<Avc1BoxView<'a>> {
        let base = VisualSampleEntry::parse_in(cur)?;
        let extensions = cur.take(cur.remaining())?;

        Ok(Avc1BoxView { base, extensions })
    }

    /// Parses an `Avc1BoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<Avc1BoxView<'a>> {
        let mut cursor = ReadCursor::new(payload);
        let this = Avc1BoxView::parse_in(&mut cursor)?;

        Ok(this)
    }
}

#[cfg(feature = "alloc")]
pub use owned::{
    Avc1Box, //
    AvcCBox,
};

#[cfg(feature = "alloc")]
mod owned {
    use super::*;

    /// An owned AVC Configuration Box (`avcC`)
    #[derive(Debug, Clone)]
    pub struct AvcCBox {
        /// Configuration Version
        pub configuration_version: u8,
        /// AVC Profile Indication
        pub avc_profile_indication: u8,
        /// Profile Compatibility
        pub avc_profile_compatibility: u8,
        /// AVC Level Indication
        pub avc_level_indication: u8,
        /// Length Size Minus One
        pub length_size_minus_one: u8,
        /// Number of SPS NAL units
        pub sps: Vec<Vec<u8>>,
        /// Number of PPS NAL units
        pub pps: Vec<Vec<u8>>,
        /// Extensions (optional)
        pub ext: Option<Vec<u8>>,
    }

    impl AvcCBox {
        /// Creates an `AvcCBox` from an `AvcCBoxView`.
        pub fn from_view(view: &AvcCBoxView<'_>) -> Self {
            let sps = view.sps().map(|nalu| nalu.to_vec()).collect();
            let pps = view.pps().map(|nalu| nalu.to_vec()).collect();
            let ext = view.ext.map(|e| e.to_vec());

            AvcCBox {
                configuration_version: view.configuration_version,
                avc_profile_indication: view.avc_profile_indication,
                avc_profile_compatibility: view.avc_profile_compatibility,
                avc_level_indication: view.avc_level_indication,
                length_size_minus_one: view.length_size_minus_one,
                sps,
                pps,
                ext,
            }
        }
    }

    impl From<AvcCBoxView<'_>> for AvcCBox {
        fn from(view: AvcCBoxView) -> Self {
            Self::from_view(&view)
        }
    }

    /// An owned AVC Sample Entry Box (`avc1`)
    #[derive(Debug, Clone)]
    pub struct Avc1Box {
        /// The base Visual Sample Entry.
        pub base: VisualSampleEntry,
        // pub clap: Option<ClapBox>,
        // pub pasp: Option<PaspBox>,
        /// The AVC Configuration Box.
        pub avcc: AvcCBox,
    }

    impl Avc1Box {
        /// Returns the codec string in the format "avc1.ppccll"
        pub fn codec(&self) -> String {
            format!(
                "avc1.{:02X}{:02X}{:02X}",
                self.avcc.avc_profile_indication,
                self.avcc.avc_profile_compatibility,
                self.avcc.avc_level_indication
            )
        }

        /// Creates an `Avc1Box` from an `Avc1BoxView`.
        pub fn from_view(view: &Avc1BoxView<'_>) -> Result<Self> {
            let mut avcc = None;

            for extention in view.extensions() {
                match extention {
                    Ok(c) if c.header.boxtype() == BoxType::AVCC => {
                        let avcc_view = AvcCBoxView::parse(c.payload)?;
                        avcc = Some(AvcCBox::from_view(&avcc_view));
                    }
                    Ok(_) => continue,
                    Err(e) => return Err(e),
                }
            }

            Ok(Avc1Box {
                base: view.base,
                avcc: avcc.ok_or(Error::in_box(
                    ErrorKind::BoxMissing {
                        required: BoxType::AVCC,
                    },
                    BoxType::AVC1,
                ))?,
            })
        }

        /// Parses an `Avc1Box` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<Self> {
            let avc1_view = Avc1BoxView::parse(payload)?;
            Self::from_view(&avc1_view)
        }
    }

    impl TryFrom<&Avc1BoxView<'_>> for Avc1Box {
        type Error = Error;

        fn try_from(view: &Avc1BoxView) -> Result<Self> {
            Self::from_view(view)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: Create NAL unit data [length (2 bytes)][data (length bytes)]
    fn make_nalu(data: &[u8]) -> Vec<u8> {
        let mut result = Vec::new();
        result.extend_from_slice(&(data.len() as u16).to_be_bytes());
        result.extend_from_slice(data);
        result
    }

    // Helper: Create AvcC payload
    fn make_avcc_payload(
        profile: u8,
        compatibility: u8,
        level: u8,
        length_size_minus_one: u8,
        sps_list: &[&[u8]],
        pps_list: &[&[u8]],
    ) -> Vec<u8> {
        let mut data = vec![
            1,                                     // configurationVersion
            profile,                               // AVCProfileIndication
            compatibility,                         // profile_compatibility
            level,                                 // AVCLevelIndication
            0xFC | (length_size_minus_one & 0x03), // reserved (6 bits) | lengthSizeMinusOne (2 bits)
            0xE0 | (sps_list.len() as u8 & 0x1F), // reserved (3 bits) | numOfSequenceParameterSets (5 bits)
        ];

        // SPS NAL units
        for sps in sps_list {
            data.extend_from_slice(&make_nalu(sps));
        }

        // numOfPictureParameterSets
        data.push(pps_list.len() as u8);

        // PPS NAL units
        for pps in pps_list {
            data.extend_from_slice(&make_nalu(pps));
        }

        data
    }

    // ==================== NalUnitIter tests ====================

    #[test]
    fn nal_unit_iter_empty() {
        let iter = NalUnitIter { data: &[] };
        let nalus: Vec<_> = iter.collect();
        assert!(nalus.is_empty());
    }

    #[test]
    fn nal_unit_iter_single() {
        let nalu_data = b"test_nalu";
        let data = make_nalu(nalu_data);

        let iter = NalUnitIter { data: &data };
        let nalus: Vec<_> = iter.collect();

        assert_eq!(nalus.len(), 1);
        assert_eq!(nalus[0], nalu_data);
    }

    #[test]
    fn nal_unit_iter_multiple() {
        let nalu1 = b"first";
        let nalu2 = b"second";
        let nalu3 = b"third";

        let mut data = Vec::new();
        data.extend_from_slice(&make_nalu(nalu1));
        data.extend_from_slice(&make_nalu(nalu2));
        data.extend_from_slice(&make_nalu(nalu3));

        let iter = NalUnitIter { data: &data };
        let nalus: Vec<_> = iter.collect();

        assert_eq!(nalus.len(), 3);
        assert_eq!(nalus[0], b"first");
        assert_eq!(nalus[1], b"second");
        assert_eq!(nalus[2], b"third");
    }

    #[test]
    fn nal_unit_iter_insufficient_length_field() {
        // Only 1 byte, need 2 for length field
        let data = [0x00];
        let iter = NalUnitIter { data: &data };
        let nalus: Vec<_> = iter.collect();
        assert!(nalus.is_empty());
    }

    #[test]
    fn nal_unit_iter_insufficient_data() {
        // Length says 10 bytes, but only 5 available
        let mut data = Vec::new();
        data.extend_from_slice(&10u16.to_be_bytes());
        data.extend_from_slice(b"12345");

        let iter = NalUnitIter { data: &data };
        let nalus: Vec<_> = iter.collect();
        assert!(nalus.is_empty());
    }

    // ==================== AvcCBoxView tests ====================

    #[test]
    fn avcc_parse_basic() {
        let sps = b"\x67\x64\x00\x1f"; // Example SPS NAL unit
        let pps = b"\x68\xeb\xe3\xcb"; // Example PPS NAL unit

        let payload = make_avcc_payload(100, 0, 31, 3, &[sps], &[pps]);

        let avcc = AvcCBoxView::parse(&payload).unwrap();

        assert_eq!(avcc.configuration_version, 1);
        assert_eq!(avcc.avc_profile_indication, 100);
        assert_eq!(avcc.avc_profile_compatibility, 0);
        assert_eq!(avcc.avc_level_indication, 31);
        assert_eq!(avcc.length_size_minus_one, 3);
        assert_eq!(avcc.nb_sps_nalus, 1);
        assert_eq!(avcc.nb_pps_nalus, 1);
    }

    #[test]
    fn avcc_sps_pps_iteration() {
        let sps1 = b"sps_one";
        let sps2 = b"sps_two";
        let pps1 = b"pps_one";
        let pps2 = b"pps_two";
        let pps3 = b"pps_three";

        let payload = make_avcc_payload(66, 192, 30, 3, &[sps1, sps2], &[pps1, pps2, pps3]);

        let avcc = AvcCBoxView::parse(&payload).unwrap();

        assert_eq!(avcc.nb_sps_nalus, 2);
        assert_eq!(avcc.nb_pps_nalus, 3);

        let sps_list: Vec<_> = avcc.sps().collect();
        assert_eq!(sps_list.len(), 2);
        assert_eq!(sps_list[0], b"sps_one");
        assert_eq!(sps_list[1], b"sps_two");

        let pps_list: Vec<_> = avcc.pps().collect();
        assert_eq!(pps_list.len(), 3);
        assert_eq!(pps_list[0], b"pps_one");
        assert_eq!(pps_list[1], b"pps_two");
        assert_eq!(pps_list[2], b"pps_three");
    }

    #[test]
    fn avcc_no_sps_pps() {
        let payload = make_avcc_payload(77, 64, 40, 3, &[], &[]);

        let avcc = AvcCBoxView::parse(&payload).unwrap();

        assert_eq!(avcc.nb_sps_nalus, 0);
        assert_eq!(avcc.nb_pps_nalus, 0);
        assert_eq!(avcc.sps().count(), 0);
        assert_eq!(avcc.pps().count(), 0);
    }

    #[test]
    fn avcc_with_extensions() {
        let sps = b"sps";
        let pps = b"pps";

        let mut payload = make_avcc_payload(100, 0, 31, 3, &[sps], &[pps]);
        // Add some extension data
        payload.extend_from_slice(b"extension_data");

        let avcc = AvcCBoxView::parse(&payload).unwrap();

        assert!(avcc.ext.is_some());
        assert_eq!(avcc.ext.unwrap(), b"extension_data");
    }

    #[test]
    fn avcc_parse_truncated() {
        // Only header, no SPS/PPS data
        let payload = [1, 100, 0, 31, 0xFF];
        let result = AvcCBoxView::parse(&payload);
        assert!(result.is_err());
    }

    // ==================== Avc1BoxView tests ====================

    // Helper: Create box header [size (4 bytes)][type (4 bytes)]
    fn make_box_header(size: u32, fourcc: &[u8; 4]) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&size.to_be_bytes());
        data.extend_from_slice(fourcc);
        data
    }

    // Helper: Create VisualSampleEntry base data
    fn make_visual_sample_entry(width: u16, height: u16) -> Vec<u8> {
        let mut data = Vec::new();

        // SampleEntry base (8 bytes)
        data.extend_from_slice(&[0u8; 6]); // reserved
        data.extend_from_slice(&1u16.to_be_bytes()); // data_reference_index

        // VisualSampleEntry fields
        data.extend_from_slice(&[0u8; 2]); // pre_defined (2 bytes)
        data.extend_from_slice(&[0u8; 2]); // reserved (2 bytes)
        data.extend_from_slice(&[0u8; 12]); // pre_defined (3 * u32 = 12 bytes)
        data.extend_from_slice(&width.to_be_bytes()); // width
        data.extend_from_slice(&height.to_be_bytes()); // height
        data.extend_from_slice(&0x00480000u32.to_be_bytes()); // horiz_resolution (72.0)
        data.extend_from_slice(&0x00480000u32.to_be_bytes()); // vert_resolution (72.0)
        data.extend_from_slice(&[0u8; 4]); // reserved (4 bytes)
        data.extend_from_slice(&1u16.to_be_bytes()); // frame_count
        // compressorname: 32 bytes total (first byte is length, followed by 31 bytes of data)
        data.extend_from_slice(&[0u8; 32]); // compressorname (32 bytes)
        data.extend_from_slice(&0x0018u16.to_be_bytes()); // depth (24)
        data.extend_from_slice(&[0xFF, 0xFF]); // pre_defined (-1)

        data
    }

    #[test]
    fn avc1_parse_with_avcc() {
        let sps = b"\x67\x64\x00\x1f";
        let pps = b"\x68\xeb\xe3\xcb";
        let avcc_payload = make_avcc_payload(100, 0, 31, 3, &[sps], &[pps]);

        // Create avcC box
        let avcc_size = 8 + avcc_payload.len() as u32;
        let mut avcc_box = make_box_header(avcc_size, b"avcC");
        avcc_box.extend_from_slice(&avcc_payload);

        // Create avc1 payload
        let mut payload = make_visual_sample_entry(1920, 1080);
        payload.extend_from_slice(&avcc_box);

        let avc1 = Avc1BoxView::parse(&payload).unwrap();

        assert_eq!(avc1.visual_sample_entry().width, 1920);
        assert_eq!(avc1.visual_sample_entry().height, 1080);

        let avcc = avc1.avcc().unwrap();
        assert_eq!(avcc.avc_profile_indication, 100);
        assert_eq!(avcc.avc_level_indication, 31);
    }

    #[test]
    fn avc1_missing_avcc() {
        // avc1 without avcC box
        let payload = make_visual_sample_entry(1920, 1080);

        let avc1 = Avc1BoxView::parse(&payload).unwrap();
        let result = avc1.avcc();

        assert!(result.is_err());
    }

    // ==================== Owned type tests (alloc) ====================

    #[cfg(feature = "alloc")]
    mod alloc_tests {
        use super::*;

        #[test]
        fn avcc_box_from_view() {
            let sps1 = b"sps_data_1";
            let sps2 = b"sps_data_2";
            let pps = b"pps_data";

            let payload = make_avcc_payload(100, 0, 31, 3, &[sps1, sps2], &[pps]);
            let view = AvcCBoxView::parse(&payload).unwrap();
            let owned = AvcCBox::from_view(&view);

            assert_eq!(owned.avc_profile_indication, 100);
            assert_eq!(owned.avc_level_indication, 31);
            assert_eq!(owned.sps.len(), 2);
            assert_eq!(owned.sps[0], b"sps_data_1");
            assert_eq!(owned.sps[1], b"sps_data_2");
            assert_eq!(owned.pps.len(), 1);
            assert_eq!(owned.pps[0], b"pps_data");
        }

        #[test]
        fn avc1_box_from_view() {
            let sps = b"\x67\x64\x00\x1f";
            let pps = b"\x68\xeb\xe3\xcb";
            let avcc_payload = make_avcc_payload(100, 0, 31, 3, &[sps], &[pps]);

            let avcc_size = 8 + avcc_payload.len() as u32;
            let mut avcc_box = make_box_header(avcc_size, b"avcC");
            avcc_box.extend_from_slice(&avcc_payload);

            let mut payload = make_visual_sample_entry(1280, 720);
            payload.extend_from_slice(&avcc_box);

            let view = Avc1BoxView::parse(&payload).unwrap();
            let owned = Avc1Box::from_view(&view).unwrap();

            assert_eq!(owned.base.width, 1280);
            assert_eq!(owned.base.height, 720);
            assert_eq!(owned.avcc.avc_profile_indication, 100);
        }

        #[test]
        fn avc1_box_codec_string() {
            let sps = b"sps";
            let pps = b"pps";
            let avcc_payload = make_avcc_payload(100, 0, 31, 3, &[sps], &[pps]);

            let avcc_size = 8 + avcc_payload.len() as u32;
            let mut avcc_box = make_box_header(avcc_size, b"avcC");
            avcc_box.extend_from_slice(&avcc_payload);

            let mut payload = make_visual_sample_entry(1920, 1080);
            payload.extend_from_slice(&avcc_box);

            let view = Avc1BoxView::parse(&payload).unwrap();
            let owned = Avc1Box::from_view(&view).unwrap();

            // avc1.64001f (profile=100, compatibility=0, level=31)
            assert_eq!(owned.codec(), "avc1.64001F");
        }
    }
}
