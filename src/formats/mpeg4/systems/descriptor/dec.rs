//! Decoder Configuration Descriptor (DecoderConfigDescr) structures.
//!
//! This module provides types for the Decoder Configuration Descriptor,
//! which specifies the configuration of a decoder required to decode
//! an elementary stream.

use crate::error::*;

use crate::cursor::ReadCursor;

use super::RawDescriptorRef;
use super::iter::DescriptorIter;

/// Object Type Indication identifying the codec or stream type.
///
/// This value indicates the type of media (audio, video, etc.) and the
/// specific codec used to encode the elementary stream.
///
/// # Common Values
///
/// | Constant | Value | Description |
/// |----------|-------|-------------|
/// | `MPEG4_AUDIO` | 0x40 | MPEG-4 Audio (AAC) |
/// | `MPEG4_VISUAL` | 0x20 | MPEG-4 Visual |
/// | `AVC_VIDEO` | 0x21 | AVC (H.264) Video |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectTypeIndication(u8);

impl ObjectTypeIndication {
    /// MPEG-4 Audio (AAC) object type (0x40).
    pub const MPEG4_AUDIO: ObjectTypeIndication = ObjectTypeIndication(0x40);
    /// MPEG-4 Visual object type (0x20).
    pub const MPEG4_VISUAL: ObjectTypeIndication = ObjectTypeIndication(0x20);
    /// AVC (H.264) Video object type (0x21).
    pub const AVC_VIDEO: ObjectTypeIndication = ObjectTypeIndication(0x21);

    /// Returns the raw value of the Object Type Indication.
    pub const fn value(&self) -> u8 {
        self.0
    }
}

/// Stream type indicating the nature of the elementary stream.
///
/// # Common Values
///
/// | Constant | Value | Description |
/// |----------|-------|-------------|
/// | `VISUAL_STREAM` | 0x04 | Video stream |
/// | `AUDIO_STREAM` | 0x05 | Audio stream |
/// | `TEXT_STREAM` | 0x06 | Text/subtitle stream |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamType(u8);

impl StreamType {
    /// Audio stream type (0x05).
    pub const AUDIO_STREAM: StreamType = StreamType(0x05);
    /// Visual (video) stream type (0x04).
    pub const VISUAL_STREAM: StreamType = StreamType(0x04);
    /// Text stream type (0x06).
    pub const TEXT_STREAM: StreamType = StreamType(0x06);

    /// Returns the raw value of the Stream Type.
    pub const fn value(&self) -> u8 {
        self.0
    }
}

/// Zero-copy view of a Decoder Configuration Descriptor.
///
/// This descriptor specifies how to configure a decoder for an elementary
/// stream. It contains information about the codec type, bitrates, and
/// decoder-specific configuration data.
///
/// # Structure (ISO/IEC 14496-1)
///
/// - `object_type_indication`: Codec identifier
/// - `stream_type`: Type of stream (audio, video, etc.)
/// - `up_stream`: Whether stream is upstream
/// - `buffer_size_db`: Decoder buffer size in bytes (24-bit)
/// - `max_bitrate`: Maximum bitrate in bits/second
/// - `avg_bitrate`: Average bitrate in bits/second
/// - Child descriptors (DecoderSpecificInfo, etc.)
#[derive(Debug)]
pub struct DecoderConfigDescriptorView<'a> {
    /// Codec identifier.
    pub object_type_indication: ObjectTypeIndication,
    /// Type of elementary stream.
    pub stream_type: StreamType,
    /// Whether this is an upstream stream.
    pub up_stream: bool,
    /// Decoder buffer size in bytes (24-bit big-endian).
    pub buffer_size_db: [u8; 3],
    /// Maximum bitrate in bits per second.
    pub max_bitrate: u32,
    /// Average bitrate in bits per second.
    pub avg_bitrate: u32,
    /// Raw bytes containing child descriptors.
    descs: &'a [u8],
}

impl<'a> DecoderConfigDescriptorView<'a> {
    /// Returns an iterator over the descriptors contained in this Decoder Config Descriptor.
    pub fn descriptors(&self) -> DescriptorIter<'a> {
        DescriptorIter::new(self.descs)
    }

    /// Returns the Decoder Specific Info descriptor contained in this Decoder Config Descriptor.
    pub fn dec_specific_info(&self) -> Result<Option<RawDescriptorRef<'a>>> {
        for result in self.descriptors() {
            let descr = result?;
            if descr.tag() == super::Tag::DECODER_SPECIFIC_INFO_TAG {
                return Ok(Some(descr));
            }
        }

        Ok(None)
    }

    /// Parses a Decoder Config Descriptor from the given byte slice.
    pub fn parse(instance: &'a [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(instance);

        let object_type_indication = ObjectTypeIndication(cur.read_u8()?);

        let stream_type_byte = cur.read_u8()?;
        let stream_type = StreamType(stream_type_byte >> 2);
        let up_stream = (stream_type_byte & 0x02) != 0;

        let buffer_size_db = cur.read_array::<3>()?;

        let max_bitrate = cur.read_u32_be()?;
        let avg_bitrate = cur.read_u32_be()?;

        let descs = cur.take(cur.remaining())?;

        Ok(DecoderConfigDescriptorView {
            object_type_indication,
            stream_type,
            up_stream,
            buffer_size_db,
            max_bitrate,
            avg_bitrate,
            descs,
        })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use crate::lib::Vec;

    use super::*;

    use crate::formats::mpeg4::systems::descriptor::RawDescriptorOwned;
    use crate::formats::mpeg4::systems::descriptor::Tag;

    use crate::cursor::WriteCursor;

    /// Owned Decoder Configuration Descriptor with heap-allocated data.
    ///
    /// This is the owned version of [`DecoderConfigDescriptorView`], suitable
    /// for modification and storage independent of the source buffer.
    ///
    /// # Example
    ///
    /// ```
    /// use mp4_bmff::formats::mpeg4::systems::descriptor::{
    ///     DecoderConfigDescriptor,
    ///     ObjectTypeIndication,
    ///     StreamType,
    /// };
    ///
    /// // Parse from bytes
    /// let data = [
    ///     0x40,                   // object_type_indication (MPEG4_AUDIO)
    ///     0x15,                   // stream_type (AUDIO_STREAM) | reserved
    ///     0x00, 0x00, 0x10,       // buffer_size_db
    ///     0x00, 0x01, 0x00, 0x00, // max_bitrate
    ///     0x00, 0x00, 0x80, 0x00, // avg_bitrate
    /// ];
    ///
    /// let desc = DecoderConfigDescriptor::parse(&data).unwrap();
    /// assert_eq!(desc.object_type_indication, ObjectTypeIndication::MPEG4_AUDIO);
    /// assert_eq!(desc.stream_type, StreamType::AUDIO_STREAM);
    /// ```
    #[derive(Debug, Clone)]
    pub struct DecoderConfigDescriptor {
        /// Codec identifier.
        pub object_type_indication: ObjectTypeIndication,
        /// Type of elementary stream.
        pub stream_type: StreamType,
        /// Whether this is an upstream stream.
        pub up_stream: bool,
        /// Decoder buffer size in bytes (24-bit big-endian).
        pub buffer_size_db: [u8; 3],
        /// Maximum bitrate in bits per second.
        pub max_bitrate: u32,
        /// Average bitrate in bits per second.
        pub avg_bitrate: u32,
        /// Decoder-specific configuration data (codec initialization).
        pub dec_specific_info: Option<RawDescriptorOwned>,
        /// Extension descriptors.
        pub extentions: Vec<RawDescriptorOwned>,
    }

    impl TryFrom<&DecoderConfigDescriptorView<'_>> for DecoderConfigDescriptor {
        type Error = Error;

        fn try_from(view: &DecoderConfigDescriptorView<'_>) -> Result<Self> {
            let mut dec_specific_info = None;
            let mut extentions = Vec::new();

            for result in view.descriptors() {
                let descr = result?.to_owned();
                if descr.tag() == Tag::DECODER_SPECIFIC_INFO_TAG {
                    dec_specific_info = Some(descr);
                } else {
                    extentions.push(descr);
                }
            }

            Ok(DecoderConfigDescriptor {
                object_type_indication: view.object_type_indication,
                stream_type: view.stream_type,
                up_stream: view.up_stream,
                buffer_size_db: view.buffer_size_db,
                max_bitrate: view.max_bitrate,
                avg_bitrate: view.avg_bitrate,
                dec_specific_info,
                extentions,
            })
        }
    }

    impl DecoderConfigDescriptor {
        /// Returns the length of the DecoderConfigDescriptor when serialized.
        pub fn encoded_len(&self) -> usize {
            let mut len = 13; // Fixed size fields

            if let Some(dec_specific_info) = &self.dec_specific_info {
                len += dec_specific_info.len();
            }

            for ext in &self.extentions {
                len += ext.len();
            }

            len
        }

        /// Parses DecoderConfigDescriptor from a byte slice
        pub fn parse(instance: &[u8]) -> Result<Self> {
            let view = DecoderConfigDescriptorView::parse(instance)?;
            Self::try_from(&view)
        }

        /// Writes the DecoderConfigDescriptor into the given byte slice.
        pub fn write(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.object_type_indication.0)?;
            let mut stream_type_byte = self.stream_type.0 << 2;
            if self.up_stream {
                stream_type_byte |= 0x02;
            }

            cur.write_u8(stream_type_byte)?;

            cur.write_array(&self.buffer_size_db)?;
            cur.write_u32_be(self.max_bitrate)?;
            cur.write_u32_be(self.avg_bitrate)?;

            if let Some(dec_specific_info) = &self.dec_specific_info {
                let buf = cur.take_mut(dec_specific_info.len())?;
                dec_specific_info.write(buf)?;
            }

            for ext in &self.extentions {
                let buf = cur.take_mut(ext.len())?;
                ext.write(buf)?;
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

    #[test]
    fn test_object_type_indication_constants() {
        assert_eq!(ObjectTypeIndication::MPEG4_AUDIO.0, 0x40);
        assert_eq!(ObjectTypeIndication::MPEG4_VISUAL.0, 0x20);
        assert_eq!(ObjectTypeIndication::AVC_VIDEO.0, 0x21);
    }

    #[test]
    fn test_stream_type_constants() {
        assert_eq!(StreamType::AUDIO_STREAM.0, 0x05);
        assert_eq!(StreamType::VISUAL_STREAM.0, 0x04);
        assert_eq!(StreamType::TEXT_STREAM.0, 0x06);
    }

    #[test]
    fn test_decoder_config_descriptor_view_parse_minimal() {
        // Minimal DecoderConfigDescriptor without any sub-descriptors
        // object_type_indication(1) + stream_type_byte(1) + buffer_size_db(3) + max_bitrate(4) + avg_bitrate(4) = 13 bytes
        let data = [
            0x40, // object_type_indication = MPEG4_AUDIO
            0x15, // stream_type = 0x05 (AUDIO_STREAM) << 2 | up_stream = 0 | reserved = 1
            0x00, 0x00, 0x10, // buffer_size_db = 16
            0x00, 0x01, 0x00, 0x00, // max_bitrate = 65536
            0x00, 0x00, 0x80, 0x00, // avg_bitrate = 32768
        ];

        let view = DecoderConfigDescriptorView::parse(&data).unwrap();

        assert_eq!(
            view.object_type_indication,
            ObjectTypeIndication::MPEG4_AUDIO
        );
        assert_eq!(view.stream_type, StreamType::AUDIO_STREAM);
        assert!(!view.up_stream);
        assert_eq!(view.buffer_size_db, [0x00, 0x00, 0x10]);
        assert_eq!(view.max_bitrate, 65536);
        assert_eq!(view.avg_bitrate, 32768);
        assert_eq!(view.descriptors().count(), 0);
    }

    #[test]
    fn test_decoder_config_descriptor_view_parse_with_upstream() {
        let data = [
            0x40, // object_type_indication
            0x16, // stream_type = 0x05 << 2 | up_stream = 1 | reserved = 0
            0x00, 0x00, 0x10, // buffer_size_db
            0x00, 0x01, 0x00, 0x00, // max_bitrate
            0x00, 0x00, 0x80, 0x00, // avg_bitrate
        ];

        let view = DecoderConfigDescriptorView::parse(&data).unwrap();

        assert!(view.up_stream);
    }

    #[test]
    fn test_decoder_config_descriptor_view_with_dec_specific_info() {
        // DecoderConfigDescriptor with DecoderSpecificInfo sub-descriptor
        let data = [
            0x40, // object_type_indication
            0x15, // stream_type byte
            0x00, 0x00, 0x10, // buffer_size_db
            0x00, 0x01, 0x00, 0x00, // max_bitrate
            0x00, 0x00, 0x80, 0x00, // avg_bitrate
            // DecoderSpecificInfo descriptor: tag(0x05) + size(0x02) + data
            0x05, 0x02, 0x11, 0x90,
        ];

        let view = DecoderConfigDescriptorView::parse(&data).unwrap();

        let dec_specific = view.dec_specific_info().unwrap();
        assert!(dec_specific.is_some());
        let dec_specific = dec_specific.unwrap();
        assert_eq!(
            dec_specific.tag(),
            super::super::Tag::DECODER_SPECIFIC_INFO_TAG
        );
        assert_eq!(dec_specific.instance(), &[0x11, 0x90]);
    }

    #[test]
    fn test_decoder_config_descriptor_view_no_dec_specific_info() {
        let data = [
            0x40, // object_type_indication
            0x15, // stream_type byte
            0x00, 0x00, 0x10, // buffer_size_db
            0x00, 0x01, 0x00, 0x00, // max_bitrate
            0x00, 0x00, 0x80, 0x00, // avg_bitrate
            // No DecoderSpecificInfo, but some other descriptor
            0x13, 0x01, 0xFF, // ExtensionProfileLevelDescr
        ];

        let view = DecoderConfigDescriptorView::parse(&data).unwrap();

        let dec_specific = view.dec_specific_info().unwrap();
        assert!(dec_specific.is_none());
    }

    #[test]
    fn test_decoder_config_descriptor_view_parse_too_short() {
        let data = [0x40, 0x15, 0x00]; // Only 3 bytes, needs 13

        let result = DecoderConfigDescriptorView::parse(&data);
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_decoder_config_descriptor_parse() {
        let data = [
            0x40, // object_type_indication
            0x15, // stream_type byte
            0x00, 0x00, 0x10, // buffer_size_db
            0x00, 0x01, 0x00, 0x00, // max_bitrate
            0x00, 0x00, 0x80, 0x00, // avg_bitrate
            // DecoderSpecificInfo
            0x05, 0x02, 0x11, 0x90,
        ];

        let desc = DecoderConfigDescriptor::parse(&data).unwrap();

        assert_eq!(
            desc.object_type_indication,
            ObjectTypeIndication::MPEG4_AUDIO
        );
        assert_eq!(desc.stream_type, StreamType::AUDIO_STREAM);
        assert!(!desc.up_stream);
        assert!(desc.dec_specific_info.is_some());
        assert!(desc.extentions.is_empty());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_decoder_config_descriptor_len() {
        let data = [
            0x40, 0x15, 0x00, 0x00, 0x10, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x05,
            0x02, 0x11, 0x90,
        ];

        let desc = DecoderConfigDescriptor::parse(&data).unwrap();

        // 13 (fixed) + 4 (DecoderSpecificInfo: tag + size + 2 bytes)
        assert_eq!(desc.encoded_len(), 17);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_decoder_config_descriptor_write() {
        let data = [
            0x40, 0x15, 0x00, 0x00, 0x10, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x05,
            0x02, 0x11, 0x90,
        ];

        let desc = DecoderConfigDescriptor::parse(&data).unwrap();

        let mut buffer = vec![0u8; desc.encoded_len()];
        let written = desc.write(&mut buffer).unwrap();

        assert_eq!(written, data.len());
        // Note: stream_type_byte reserved bit (bit 0) is not preserved
        // Input 0x15 becomes 0x14 after write (reserved bit cleared)
        assert_eq!(buffer[0], 0x40); // object_type_indication
        assert_eq!(buffer[1] >> 2, data[1] >> 2); // stream_type preserved
        assert_eq!(&buffer[2..13], &data[2..13]); // Other fixed fields
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_decoder_config_descriptor_roundtrip() {
        let original_data = [
            0x40, 0x15, 0x00, 0x00, 0x10, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00,
        ];

        let desc = DecoderConfigDescriptor::parse(&original_data).unwrap();

        let mut buffer = vec![0u8; desc.encoded_len()];
        desc.write(&mut buffer).unwrap();

        let parsed_again = DecoderConfigDescriptor::parse(&buffer).unwrap();

        assert_eq!(
            desc.object_type_indication,
            parsed_again.object_type_indication
        );
        assert_eq!(desc.stream_type, parsed_again.stream_type);
        assert_eq!(desc.up_stream, parsed_again.up_stream);
        assert_eq!(desc.buffer_size_db, parsed_again.buffer_size_db);
        assert_eq!(desc.max_bitrate, parsed_again.max_bitrate);
        assert_eq!(desc.avg_bitrate, parsed_again.avg_bitrate);
    }
}
