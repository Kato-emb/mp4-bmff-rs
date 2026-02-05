//!/ AVC (H.264) codec related structures and functions

use crate::error::*;

use crate::cursor::ReadCursor;

use super::iter::ParameterSetsIter;

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
    pub fn sps(&self) -> ParameterSetsIter<'a> {
        ParameterSetsIter {
            data: self.sps,
            remaining: self.num_of_sps as usize,
        }
    }

    /// Returns an iterator over the picture parameter sets
    pub fn pps(&self) -> ParameterSetsIter<'a> {
        ParameterSetsIter {
            data: self.pps,
            remaining: self.num_of_pps as usize,
        }
    }

    /// Returns an iterator over the sequence parameter set extensions
    pub fn sps_ext(&self) -> Option<ParameterSetsIter<'a>> {
        self.sps_ext
            .zip(self.num_of_sps_ext)
            .map(|(data, num)| ParameterSetsIter {
                data,
                remaining: num as usize,
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

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use super::*;

    use crate::cursor::WriteCursor;

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

    impl AVCDecoderConfigurationRecordView<'_> {
        /// Converts to an owned AVCDecoderConfigurationRecord
        pub fn to_owned(&self) -> AVCDecoderConfigurationRecord {
            AVCDecoderConfigurationRecord::from(self)
        }
    }

    impl AVCDecoderConfigurationRecord {
        /// Returns the size in bytes when encoded to a byte slice.
        pub fn encoded_len(&self) -> usize {
            // Base fields: 6 bytes
            // configuration_version (1) + avc_profile_indication (1) + profile_compatibility (1)
            // + avc_level_indication (1) + length_size_minus_one (1) + num_of_sps (1)
            let mut size = 6;

            // SPS: 2 bytes (length) + data for each
            for sps in &self.sps {
                size += 2 + sps.len();
            }

            // num_of_pps: 1 byte
            size += 1;

            // PPS: 2 bytes (length) + data for each
            for pps in &self.pps {
                size += 2 + pps.len();
            }

            // Optional fields (when profile is 100, 110, 122, or 144)
            if self.chroma_format.is_some() {
                // chroma_format (1) + bit_depth_luma_minus8 (1) + bit_depth_chroma_minus8 (1) + num_of_sps_ext (1)
                size += 4;

                // SPS ext: 2 bytes (length) + data for each
                if let Some(ref sps_ext) = self.sps_ext {
                    for ext in sps_ext {
                        size += 2 + ext.len();
                    }
                }
            }

            size
        }

        /// Writes the AVCDecoderConfigurationRecord to the given byte slice.
        pub fn write(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.configuration_version)?;
            cur.write_u8(self.avc_profile_indication)?;
            cur.write_u8(self.profile_compatibility)?;
            cur.write_u8(self.avc_level_indication)?;

            // reserved (6 bits) + lengthSizeMinusOne (2 bits)
            cur.write_u8(0xFC | (self.length_size_minus_one & 0x03))?;

            // reserved (3 bits) + numOfSequenceParameterSets (5 bits)
            cur.write_u8(0xE0 | ((self.sps.len() as u8) & 0x1F))?;
            for sps in &self.sps {
                cur.write_u16_be(sps.len() as u16)?;
                cur.write_slice(sps)?;
            }

            // numOfPictureParameterSets
            cur.write_u8(self.pps.len() as u8)?;
            for pps in &self.pps {
                cur.write_u16_be(pps.len() as u16)?;
                cur.write_slice(pps)?;
            }

            if let Some(chroma_format) = self.chroma_format {
                // reserved (6 bits) + chromaFormat (2 bits)
                cur.write_u8(0xFC | (chroma_format & 0x03))?;
                // reserved (5 bits) + bitDepthLumaMinus8 (3 bits)
                cur.write_u8(0xF8 | (self.bit_depth_luma_minus8.unwrap_or(0) & 0x07))?;
                // reserved (5 bits) + bitDepthChromaMinus8 (3 bits)
                cur.write_u8(0xF8 | (self.bit_depth_chroma_minus8.unwrap_or(0) & 0x07))?;

                if let Some(ref sps_ext) = self.sps_ext {
                    // numOfSequenceParameterSetExt
                    cur.write_u8(sps_ext.len() as u8)?;
                    for ext in sps_ext {
                        cur.write_u16_be(ext.len() as u16)?;
                        cur.write_slice(ext)?;
                    }
                } else {
                    // numOfSequenceParameterSetExt = 0
                    cur.write_u8(0)?;
                }
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

    // Sample AVCDecoderConfigurationRecord (baseline profile)
    fn sample_baseline_config() -> [u8; 23] {
        [
            0x01, // configuration_version
            0x42, // avc_profile_indication (Baseline)
            0xC0, // profile_compatibility
            0x1E, // avc_level_indication (Level 3.0)
            0xFF, // reserved (6 bits) + length_size_minus_one (2 bits) = 3
            0xE1, // reserved (3 bits) + num_of_sps (5 bits) = 1
            // SPS: length=8
            0x00, 0x08, 0x67, 0x42, 0xC0, 0x1E, 0xD9, 0x00, 0x68, 0x24,
            0x01, // num_of_pps = 1
            // PPS: length=4
            0x00, 0x04, 0x68, 0xCE, 0x3C, 0x80,
        ]
    }

    // Sample AVCDecoderConfigurationRecord (high profile with extended fields)
    fn sample_high_config() -> [u8; 27] {
        [
            0x01, // configuration_version
            0x64, // avc_profile_indication (High = 100)
            0x00, // profile_compatibility
            0x28, // avc_level_indication (Level 4.0)
            0xFF, // reserved + length_size_minus_one = 3
            0xE1, // reserved + num_of_sps = 1
            // SPS: length=8
            0x00, 0x08, 0x67, 0x64, 0x00, 0x28, 0xAC, 0xD9, 0x40, 0x78,
            0x01, // num_of_pps = 1
            // PPS: length=4
            0x00, 0x04, 0x68, 0xEE, 0x3C, 0x80,
            // Extended fields (High profile)
            0xFD, // reserved + chroma_format = 1
            0xF8, // reserved + bit_depth_luma_minus8 = 0
            0xF8, // reserved + bit_depth_chroma_minus8 = 0
            0x00, // num_of_sps_ext = 0
        ]
    }

    #[test]
    fn test_parse_baseline_config() {
        let data = sample_baseline_config();
        let view = AVCDecoderConfigurationRecordView::parse(&data).unwrap();

        assert_eq!(view.configuration_version, 1);
        assert_eq!(view.avc_profile_indication, 0x42);
        assert_eq!(view.profile_compatibility, 0xC0);
        assert_eq!(view.avc_level_indication, 0x1E);
        assert_eq!(view.length_size_minus_one, 3);
        assert_eq!(view.num_of_sps, 1);
        assert_eq!(view.num_of_pps, 1);

        // No extended fields for baseline profile
        assert!(view.chroma_format.is_none());
        assert!(view.bit_depth_luma_minus8.is_none());
        assert!(view.bit_depth_chroma_minus8.is_none());
        assert!(view.num_of_sps_ext.is_none());
    }

    #[test]
    fn test_parse_high_config() {
        let data = sample_high_config();
        let view = AVCDecoderConfigurationRecordView::parse(&data).unwrap();

        assert_eq!(view.configuration_version, 1);
        assert_eq!(view.avc_profile_indication, 0x64); // High profile = 100
        assert_eq!(view.profile_compatibility, 0x00);
        assert_eq!(view.avc_level_indication, 0x28); // Level 4.0
        assert_eq!(view.length_size_minus_one, 3);
        assert_eq!(view.num_of_sps, 1);
        assert_eq!(view.num_of_pps, 1);

        // Extended fields for high profile
        assert_eq!(view.chroma_format, Some(1));
        assert_eq!(view.bit_depth_luma_minus8, Some(0));
        assert_eq!(view.bit_depth_chroma_minus8, Some(0));
        assert_eq!(view.num_of_sps_ext, Some(0));
    }

    #[test]
    fn test_sps_iterator() {
        let data = sample_baseline_config();
        let view = AVCDecoderConfigurationRecordView::parse(&data).unwrap();

        let sps_list: Vec<&[u8]> = view.sps().collect();
        assert_eq!(sps_list.len(), 1);
        assert_eq!(sps_list[0], &[0x67, 0x42, 0xC0, 0x1E, 0xD9, 0x00, 0x68, 0x24]);
    }

    #[test]
    fn test_pps_iterator() {
        let data = sample_baseline_config();
        let view = AVCDecoderConfigurationRecordView::parse(&data).unwrap();

        let pps_list: Vec<&[u8]> = view.pps().collect();
        assert_eq!(pps_list.len(), 1);
        assert_eq!(pps_list[0], &[0x68, 0xCE, 0x3C, 0x80]);
    }

    #[test]
    fn test_sps_ext_none_for_baseline() {
        let data = sample_baseline_config();
        let view = AVCDecoderConfigurationRecordView::parse(&data).unwrap();

        assert!(view.sps_ext().is_none());
    }

    #[test]
    fn test_sps_ext_empty_for_high() {
        let data = sample_high_config();
        let view = AVCDecoderConfigurationRecordView::parse(&data).unwrap();

        let sps_ext_iter = view.sps_ext();
        assert!(sps_ext_iter.is_some());
        assert_eq!(sps_ext_iter.unwrap().count(), 0);
    }

    #[test]
    fn test_parse_truncated() {
        let data = [0x01, 0x42, 0xC0]; // Only 3 bytes
        let result = AVCDecoderConfigurationRecordView::parse(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_sps_pps() {
        // Config with 2 SPS and 2 PPS
        let data = [
            0x01, // configuration_version
            0x42, // avc_profile_indication
            0xC0, // profile_compatibility
            0x1E, // avc_level_indication
            0xFF, // reserved + length_size_minus_one
            0xE2, // reserved + num_of_sps = 2
            // SPS 1: length=3
            0x00, 0x03, 0x67, 0x42, 0xC0,
            // SPS 2: length=2
            0x00, 0x02, 0x67, 0x64,
            0x02, // num_of_pps = 2
            // PPS 1: length=2
            0x00, 0x02, 0x68, 0xCE,
            // PPS 2: length=3
            0x00, 0x03, 0x68, 0xEE, 0x3C,
        ];

        let view = AVCDecoderConfigurationRecordView::parse(&data).unwrap();

        assert_eq!(view.num_of_sps, 2);
        assert_eq!(view.num_of_pps, 2);

        let sps_list: Vec<&[u8]> = view.sps().collect();
        assert_eq!(sps_list.len(), 2);
        assert_eq!(sps_list[0], &[0x67, 0x42, 0xC0]);
        assert_eq!(sps_list[1], &[0x67, 0x64]);

        let pps_list: Vec<&[u8]> = view.pps().collect();
        assert_eq!(pps_list.len(), 2);
        assert_eq!(pps_list[0], &[0x68, 0xCE]);
        assert_eq!(pps_list[1], &[0x68, 0xEE, 0x3C]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_to_owned() {
        let data = sample_baseline_config();
        let view = AVCDecoderConfigurationRecordView::parse(&data).unwrap();
        let owned = view.to_owned();

        assert_eq!(owned.configuration_version, view.configuration_version);
        assert_eq!(owned.avc_profile_indication, view.avc_profile_indication);
        assert_eq!(owned.profile_compatibility, view.profile_compatibility);
        assert_eq!(owned.avc_level_indication, view.avc_level_indication);
        assert_eq!(owned.length_size_minus_one, view.length_size_minus_one);
        assert_eq!(owned.sps.len(), view.num_of_sps as usize);
        assert_eq!(owned.pps.len(), view.num_of_pps as usize);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_encoded_len_baseline() {
        let data = sample_baseline_config();
        let view = AVCDecoderConfigurationRecordView::parse(&data).unwrap();
        let owned = view.to_owned();

        assert_eq!(owned.encoded_len(), data.len());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_encoded_len_high() {
        let data = sample_high_config();
        let view = AVCDecoderConfigurationRecordView::parse(&data).unwrap();
        let owned = view.to_owned();

        assert_eq!(owned.encoded_len(), data.len());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_write_baseline() {
        let data = sample_baseline_config();
        let view = AVCDecoderConfigurationRecordView::parse(&data).unwrap();
        let owned = view.to_owned();

        let mut buffer = vec![0u8; owned.encoded_len()];
        let written = owned.write(&mut buffer).unwrap();

        assert_eq!(written, data.len());
        assert_eq!(&buffer[..], &data[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_write_high() {
        let data = sample_high_config();
        let view = AVCDecoderConfigurationRecordView::parse(&data).unwrap();
        let owned = view.to_owned();

        let mut buffer = vec![0u8; owned.encoded_len()];
        let written = owned.write(&mut buffer).unwrap();

        assert_eq!(written, data.len());
        assert_eq!(&buffer[..], &data[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_roundtrip() {
        let original = sample_baseline_config();
        let view = AVCDecoderConfigurationRecordView::parse(&original).unwrap();
        let owned = view.to_owned();

        let mut buffer = vec![0u8; owned.encoded_len()];
        owned.write(&mut buffer).unwrap();

        let reparsed = AVCDecoderConfigurationRecordView::parse(&buffer).unwrap();

        assert_eq!(reparsed.configuration_version, view.configuration_version);
        assert_eq!(reparsed.avc_profile_indication, view.avc_profile_indication);
        assert_eq!(reparsed.avc_level_indication, view.avc_level_indication);
        assert_eq!(reparsed.num_of_sps, view.num_of_sps);
        assert_eq!(reparsed.num_of_pps, view.num_of_pps);
    }
}
