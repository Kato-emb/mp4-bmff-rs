//! Elementary Stream Descriptor (ES_Descriptor) structures.
//!
//! This module provides types for the Elementary Stream Descriptor,
//! the top-level descriptor in an `esds` box that identifies an
//! elementary stream and its configuration.

use crate::error::*;

use crate::cursor::ReadCursor;

use super::RawDescriptorRef;
use super::Tag;
use super::iter::DescriptorIter;

use super::DecoderConfigDescriptorView;

/// Zero-copy view of an Elementary Stream Descriptor.
///
/// The ES_Descriptor is the primary descriptor in an `esds` box, containing
/// stream identification and linking to decoder configuration. It can
/// optionally reference other streams for dependencies or synchronization.
///
/// # Structure (ISO/IEC 14496-1)
///
/// - `es_id`: Unique identifier for this elementary stream
/// - `stream_dependence_flag`: Whether this stream depends on another
/// - `url_flag`: Whether a URL is provided
/// - `ocr_stream_flag`: Whether OCR (clock reference) stream is specified
/// - `stream_priority`: Priority for resource allocation (0-31)
/// - Optional: `depends_on_es_id`, `url_string`, `ocr_es_id`
/// - Child descriptors: DecoderConfigDescriptor, SLConfigDescriptor
#[derive(Debug)]
pub struct EsDescriptorView<'a> {
    /// Unique identifier for this elementary stream.
    pub es_id: u16,
    /// Whether this stream depends on another stream.
    pub stream_dependence_flag: bool,
    /// Whether a URL is provided for remote stream access.
    pub url_flag: bool,
    /// Whether an OCR (Object Clock Reference) stream is specified.
    pub ocr_stream_flag: bool,
    /// Stream priority for resource allocation (0-31, higher = more important).
    pub stream_priority: u8,
    /// ES ID of the stream this depends on (if `stream_dependence_flag` is set).
    pub depends_on_es_id: Option<u16>,
    /// URL for remote stream access (if `url_flag` is set).
    pub url_string: Option<&'a str>,
    /// ES ID of the OCR stream (if `ocr_stream_flag` is set).
    pub ocr_es_id: Option<u16>,
    /// Raw bytes containing child descriptors.
    descs: &'a [u8],
}

impl<'a> EsDescriptorView<'a> {
    /// Returns an iterator over the descriptors contained in this ES Descriptor.
    pub fn descriptors(&self) -> DescriptorIter<'a> {
        DescriptorIter::new(self.descs)
    }

    /// Returns the Decoder Config Descriptor contained in this ES Descriptor.
    ///
    /// # Errors
    ///
    /// Returns an error if the Decoder Config Descriptor is not found or
    /// contains invalid data.
    pub fn dec_config_descr(&self) -> Result<DecoderConfigDescriptorView<'a>> {
        for result in self.descriptors() {
            let descr = result?;
            if descr.tag() == Tag::DECODER_CONFIG_DESCR_TAG {
                let dec_config_descr = DecoderConfigDescriptorView::parse(descr.into_instance())?;
                return Ok(dec_config_descr);
            }
        }

        Err(Error::new(ErrorKind::Other {
            description: "Decoder Config Descriptor not found in ES Descriptor",
        }))
    }

    /// Returns the SL Config Descriptor contained in this ES Descriptor.
    ///
    /// # Errors
    ///
    /// Returns an error if the SL Config Descriptor is not found or
    /// contains invalid data.
    pub fn sl_config_descr(&self) -> Result<RawDescriptorRef<'a>> {
        for result in self.descriptors() {
            let descr = result?;
            if descr.tag() == Tag::SL_CONFIG_DESCR_TAG {
                return Ok(descr);
            }
        }

        Err(Error::new(ErrorKind::Other {
            description: "SL Config Descriptor not found in ES Descriptor",
        }))
    }

    /// Parses an ES Descriptor from the given byte slice.
    ///
    /// # Errors
    ///
    /// Returns an error if the data is too short or contains invalid values.
    pub fn parse(instance: &'a [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(instance);

        let es_id = cur.read_u16_be()?;

        let flags = cur.read_u8()?;
        let stream_dependence_flag = (flags & 0x80) != 0;
        let url_flag = (flags & 0x40) != 0;
        let ocr_stream_flag = (flags & 0x20) != 0;
        let stream_priority = flags & 0x1F;

        let depends_on_es_id = if stream_dependence_flag {
            Some(cur.read_u16_be()?)
        } else {
            None
        };

        let url_string = if url_flag {
            let url_length = usize::from(cur.read_u8()?);
            let url_bytes = cur.take(url_length)?;
            Some(core::str::from_utf8(url_bytes).map_err(|_| {
                Error::new(ErrorKind::Other {
                    description: "Invalid UTF-8 in URL string",
                })
            })?)
        } else {
            None
        };

        let ocr_es_id = if ocr_stream_flag {
            Some(cur.read_u16_be()?)
        } else {
            None
        };

        let descs = cur.take(cur.remaining())?;

        Ok(EsDescriptorView {
            es_id,
            stream_dependence_flag,
            url_flag,
            ocr_stream_flag,
            stream_priority,
            depends_on_es_id,
            url_string,
            ocr_es_id,
            descs,
        })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use alloc::string::{String, ToString};
    use alloc::vec::Vec;

    use super::*;

    use crate::cursor::WriteCursor;

    use crate::formats::mpeg4::systems::descriptor::RawDescriptorOwned;
    use crate::formats::mpeg4::systems::descriptor::SizeOfInstance;
    use crate::formats::mpeg4::systems::descriptor::dec::DecoderConfigDescriptor;

    /// Owned Elementary Stream Descriptor with heap-allocated data.
    ///
    /// This is the owned version of [`EsDescriptorView`], suitable for
    /// modification and storage independent of the source buffer.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use mp4_bmff::formats::mpeg4::systems::descriptor::EsDescriptor;
    ///
    /// // Parse ES_Descriptor from the instance data of a RawDescriptor
    /// // (In practice, this comes from an esds box)
    /// # let es_instance_data: &[u8] = &[];
    /// let es_descr = EsDescriptor::parse(es_instance_data).unwrap();
    /// println!("ES ID: {}", es_descr.es_id);
    /// println!("Codec: {:?}", es_descr.dec_config_descr.object_type_indication);
    /// ```
    #[derive(Debug, Clone)]
    pub struct EsDescriptor {
        /// Unique identifier for this elementary stream.
        pub es_id: u16,
        /// Whether this stream depends on another stream.
        pub stream_dependence_flag: bool,
        /// Whether a URL is provided for remote stream access.
        pub url_flag: bool,
        /// Whether an OCR (Object Clock Reference) stream is specified.
        pub ocr_stream_flag: bool,
        /// Stream priority for resource allocation (0-31).
        pub stream_priority: u8,
        /// ES ID of the stream this depends on.
        pub depends_on_es_id: Option<u16>,
        /// URL for remote stream access.
        pub url_string: Option<String>,
        /// ES ID of the OCR stream.
        pub ocr_es_id: Option<u16>,
        /// Decoder configuration descriptor.
        pub dec_config_descr: DecoderConfigDescriptor,
        /// Sync Layer configuration descriptor.
        pub sl_config_descr: RawDescriptorOwned,
        /// Additional extension descriptors.
        pub descriptors: Vec<RawDescriptorOwned>,
    }

    impl TryFrom<&EsDescriptorView<'_>> for EsDescriptor {
        type Error = Error;

        fn try_from(value: &EsDescriptorView<'_>) -> Result<Self> {
            let mut dec_config_descr = None;
            let mut sl_config_descr = None;
            let mut descriptors = Vec::new();

            for result in value.descriptors() {
                let descr = result?;

                match descr.tag() {
                    Tag::DECODER_CONFIG_DESCR_TAG => {
                        dec_config_descr = Some(DecoderConfigDescriptor::try_from(
                            &DecoderConfigDescriptorView::parse(descr.into_instance())?,
                        )?);
                    }
                    Tag::SL_CONFIG_DESCR_TAG => {
                        sl_config_descr = Some(descr.to_owned());
                    }
                    _ => {
                        descriptors.push(descr.to_owned());
                    }
                }
            }

            let dec_config_descr = dec_config_descr.ok_or_else(|| {
                Error::new(ErrorKind::Other {
                    description: "Decoder Config Descriptor not found in ES Descriptor",
                })
            })?;

            let sl_config_descr = sl_config_descr.ok_or_else(|| {
                Error::new(ErrorKind::Other {
                    description: "SL Config Descriptor not found in ES Descriptor",
                })
            })?;

            Ok(EsDescriptor {
                es_id: value.es_id,
                stream_dependence_flag: value.stream_dependence_flag,
                url_flag: value.url_flag,
                ocr_stream_flag: value.ocr_stream_flag,
                stream_priority: value.stream_priority,
                depends_on_es_id: value.depends_on_es_id,
                url_string: value.url_string.map(|s| s.to_string()),
                ocr_es_id: value.ocr_es_id,
                dec_config_descr,
                sl_config_descr,
                descriptors,
            })
        }
    }

    impl EsDescriptor {
        /// Returns the length of the EsDescriptor when encoded.
        ///
        /// # Panics
        ///
        /// Panics if the Decoder Config Descriptor length exceeds the maximum
        /// representable size for MPEG-4 Systems descriptors (`0x0FFFFFFF`).
        #[allow(clippy::cast_possible_truncation)]
        pub fn encoded_len(&self) -> usize {
            let mut len = 3; // es_id(2) + flags(1)

            if self.stream_dependence_flag {
                len += 2; // depends_on_es_id(2)
            }

            if let Some(url_string) = &self.url_string {
                len += 1 + url_string.len(); // url_length(1) + url_string
            }

            if self.ocr_stream_flag {
                len += 2; // ocr_es_id(2)
            }

            let size_of_instance =
                SizeOfInstance::from_u32(self.dec_config_descr.encoded_len() as u32)
                    .expect("Decoder Config Descriptor length too large");
            let dec_len = size_of_instance.to_bytes().1;

            len += 1 + dec_len + self.dec_config_descr.encoded_len();
            len += self.sl_config_descr.len();

            for descr in &self.descriptors {
                len += descr.len();
            }

            len
        }

        /// Parses EsDescriptor from a byte slice.
        ///
        /// # Errors
        ///
        /// Returns an error if the data is too short or contains invalid values.
        pub fn parse(instance: &[u8]) -> Result<Self> {
            let view = EsDescriptorView::parse(instance)?;
            Self::try_from(&view)
        }

        /// Writes the EsDescriptor into the given byte slice.
        ///
        /// # Errors
        ///
        /// Returns an error if the buffer is too small to hold the encoded data.
        ///
        /// # Panics
        ///
        /// Panics if the Decoder Config Descriptor length exceeds the maximum
        /// representable size for MPEG-4 Systems descriptors (`0x0FFFFFFF`).
        #[allow(clippy::cast_possible_truncation)]
        pub fn write(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u16_be(self.es_id)?;

            let mut flags = 0u8;
            if self.stream_dependence_flag {
                flags |= 0x80;
            }
            if self.url_flag {
                flags |= 0x40;
            }
            if self.ocr_stream_flag {
                flags |= 0x20;
            }
            flags |= self.stream_priority & 0x1F;

            cur.write_u8(flags)?;

            if let Some(depends_on_es_id) = self.depends_on_es_id {
                cur.write_u16_be(depends_on_es_id)?;
            }

            if let Some(url_string) = &self.url_string {
                let url_bytes = url_string.as_bytes();
                cur.write_u8(url_bytes.len() as u8)?;
                cur.write_slice(url_bytes)?;
            }

            if let Some(ocr_es_id) = self.ocr_es_id {
                cur.write_u16_be(ocr_es_id)?;
            }

            // Write DecoderConfigDescriptor as RawDescriptor format (tag + size + content)
            cur.write_u8(Tag::DECODER_CONFIG_DESCR_TAG.0)?;
            let dec_size = SizeOfInstance::from_u32(self.dec_config_descr.encoded_len() as u32)
                .expect("Decoder Config Descriptor length too large");
            let (size_bytes, size_len) = dec_size.to_bytes();
            cur.write_slice(&size_bytes[..size_len])?;
            let buf = cur.take_mut(self.dec_config_descr.encoded_len())?;
            self.dec_config_descr.write(buf)?;

            let buf = cur.take_mut(self.sl_config_descr.len())?;
            self.sl_config_descr.write(buf)?;

            for descr in &self.descriptors {
                let buf = cur.take_mut(descr.len())?;
                descr.write(buf)?;
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

    // Helper function to create a minimal DecoderConfigDescriptor bytes
    fn minimal_dec_config_descr() -> Vec<u8> {
        vec![
            0x40, // object_type_indication = MPEG4_AUDIO
            0x15, // stream_type = AUDIO_STREAM
            0x00, 0x00, 0x10, // buffer_size_db
            0x00, 0x01, 0x00, 0x00, // max_bitrate
            0x00, 0x00, 0x80, 0x00, // avg_bitrate
        ]
    }

    // Helper function to create a minimal SL Config Descriptor bytes
    fn minimal_sl_config_descr() -> Vec<u8> {
        vec![0x06, 0x01, 0x02] // tag + size + predefined
    }

    // Helper to wrap bytes as a descriptor
    fn wrap_as_descriptor(tag: u8, data: &[u8]) -> Vec<u8> {
        let mut result = vec![tag, data.len() as u8];
        result.extend_from_slice(data);
        result
    }

    #[test]
    fn test_es_descriptor_view_parse_minimal() {
        // ES_ID(2) + flags(1) + DecoderConfigDescr + SLConfigDescr
        let dec_config = minimal_dec_config_descr();
        let dec_config_wrapped = wrap_as_descriptor(0x04, &dec_config);
        let sl_config = minimal_sl_config_descr();

        let mut data = vec![
            0x00, 0x01, // es_id = 1
            0x00, // flags: no dependencies, no url, no ocr
        ];
        data.extend_from_slice(&dec_config_wrapped);
        data.extend_from_slice(&sl_config);

        let view = EsDescriptorView::parse(&data).unwrap();

        assert_eq!(view.es_id, 1);
        assert!(!view.stream_dependence_flag);
        assert!(!view.url_flag);
        assert!(!view.ocr_stream_flag);
        assert_eq!(view.stream_priority, 0);
        assert!(view.depends_on_es_id.is_none());
        assert!(view.url_string.is_none());
        assert!(view.ocr_es_id.is_none());
    }

    #[test]
    fn test_es_descriptor_view_parse_with_stream_priority() {
        let dec_config = minimal_dec_config_descr();
        let dec_config_wrapped = wrap_as_descriptor(0x04, &dec_config);
        let sl_config = minimal_sl_config_descr();

        let mut data = vec![
            0x00, 0x01, // es_id = 1
            0x0F, // flags: stream_priority = 15
        ];
        data.extend_from_slice(&dec_config_wrapped);
        data.extend_from_slice(&sl_config);

        let view = EsDescriptorView::parse(&data).unwrap();

        assert_eq!(view.stream_priority, 15);
    }

    #[test]
    fn test_es_descriptor_view_parse_with_depends_on() {
        let dec_config = minimal_dec_config_descr();
        let dec_config_wrapped = wrap_as_descriptor(0x04, &dec_config);
        let sl_config = minimal_sl_config_descr();

        let mut data = vec![
            0x00, 0x01, // es_id = 1
            0x80, // flags: stream_dependence_flag = 1
            0x00, 0x02, // depends_on_es_id = 2
        ];
        data.extend_from_slice(&dec_config_wrapped);
        data.extend_from_slice(&sl_config);

        let view = EsDescriptorView::parse(&data).unwrap();

        assert!(view.stream_dependence_flag);
        assert_eq!(view.depends_on_es_id, Some(2));
    }

    #[test]
    fn test_es_descriptor_view_parse_with_url() {
        let dec_config = minimal_dec_config_descr();
        let dec_config_wrapped = wrap_as_descriptor(0x04, &dec_config);
        let sl_config = minimal_sl_config_descr();

        let url = "http://example.com";
        let mut data = vec![
            0x00,
            0x01,            // es_id = 1
            0x40,            // flags: url_flag = 1
            url.len() as u8, // url length
        ];
        data.extend_from_slice(url.as_bytes());
        data.extend_from_slice(&dec_config_wrapped);
        data.extend_from_slice(&sl_config);

        let view = EsDescriptorView::parse(&data).unwrap();

        assert!(view.url_flag);
        assert_eq!(view.url_string, Some("http://example.com"));
    }

    #[test]
    fn test_es_descriptor_view_parse_with_ocr() {
        let dec_config = minimal_dec_config_descr();
        let dec_config_wrapped = wrap_as_descriptor(0x04, &dec_config);
        let sl_config = minimal_sl_config_descr();

        let mut data = vec![
            0x00, 0x01, // es_id = 1
            0x20, // flags: ocr_stream_flag = 1
            0x00, 0x03, // ocr_es_id = 3
        ];
        data.extend_from_slice(&dec_config_wrapped);
        data.extend_from_slice(&sl_config);

        let view = EsDescriptorView::parse(&data).unwrap();

        assert!(view.ocr_stream_flag);
        assert_eq!(view.ocr_es_id, Some(3));
    }

    #[test]
    fn test_es_descriptor_view_dec_config_descr() {
        let dec_config = minimal_dec_config_descr();
        let dec_config_wrapped = wrap_as_descriptor(0x04, &dec_config);
        let sl_config = minimal_sl_config_descr();

        let mut data = vec![0x00, 0x01, 0x00];
        data.extend_from_slice(&dec_config_wrapped);
        data.extend_from_slice(&sl_config);

        let view = EsDescriptorView::parse(&data).unwrap();
        let dec_config_view = view.dec_config_descr().unwrap();

        assert_eq!(
            dec_config_view.object_type_indication,
            super::super::ObjectTypeIndication::MPEG4_AUDIO
        );
    }

    #[test]
    fn test_es_descriptor_view_sl_config_descr() {
        let dec_config = minimal_dec_config_descr();
        let dec_config_wrapped = wrap_as_descriptor(0x04, &dec_config);
        let sl_config = minimal_sl_config_descr();

        let mut data = vec![0x00, 0x01, 0x00];
        data.extend_from_slice(&dec_config_wrapped);
        data.extend_from_slice(&sl_config);

        let view = EsDescriptorView::parse(&data).unwrap();
        let sl_descr = view.sl_config_descr().unwrap();

        assert_eq!(sl_descr.tag(), Tag::SL_CONFIG_DESCR_TAG);
        assert_eq!(sl_descr.instance(), &[0x02]);
    }

    #[test]
    fn test_es_descriptor_view_missing_dec_config() {
        let sl_config = minimal_sl_config_descr();

        let mut data = vec![0x00, 0x01, 0x00];
        data.extend_from_slice(&sl_config);

        let view = EsDescriptorView::parse(&data).unwrap();
        let result = view.dec_config_descr();

        assert!(result.is_err());
    }

    #[test]
    fn test_es_descriptor_view_missing_sl_config() {
        let dec_config = minimal_dec_config_descr();
        let dec_config_wrapped = wrap_as_descriptor(0x04, &dec_config);

        let mut data = vec![0x00, 0x01, 0x00];
        data.extend_from_slice(&dec_config_wrapped);

        let view = EsDescriptorView::parse(&data).unwrap();
        let result = view.sl_config_descr();

        assert!(result.is_err());
    }

    #[test]
    fn test_es_descriptor_view_parse_too_short() {
        let data = [0x00, 0x01]; // Only es_id, missing flags

        let result = EsDescriptorView::parse(&data);
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_es_descriptor_parse() {
        let dec_config = minimal_dec_config_descr();
        let dec_config_wrapped = wrap_as_descriptor(0x04, &dec_config);
        let sl_config = minimal_sl_config_descr();

        let mut data = vec![0x00, 0x01, 0x00];
        data.extend_from_slice(&dec_config_wrapped);
        data.extend_from_slice(&sl_config);

        let desc = EsDescriptor::parse(&data).unwrap();

        assert_eq!(desc.es_id, 1);
        assert!(!desc.stream_dependence_flag);
        assert!(!desc.url_flag);
        assert!(!desc.ocr_stream_flag);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_es_descriptor_write() {
        let dec_config = minimal_dec_config_descr();
        let dec_config_wrapped = wrap_as_descriptor(0x04, &dec_config);
        let sl_config = minimal_sl_config_descr();

        let mut original_data = vec![0x00, 0x01, 0x00];
        original_data.extend_from_slice(&dec_config_wrapped);
        original_data.extend_from_slice(&sl_config);

        let desc = EsDescriptor::parse(&original_data).unwrap();

        // Calculate expected size and create buffer
        let expected_size = 3 + desc.dec_config_descr.encoded_len() + desc.sl_config_descr.len();
        let mut buffer = vec![0u8; expected_size + 10];

        let written = desc.write(&mut buffer).unwrap();

        // Verify basic structure
        assert_eq!(buffer[0], 0x00); // es_id high byte
        assert_eq!(buffer[1], 0x01); // es_id low byte
        assert_eq!(buffer[2], 0x00); // flags
        assert!(written > 0);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_es_descriptor_with_all_optional_fields() {
        let dec_config = minimal_dec_config_descr();
        let dec_config_wrapped = wrap_as_descriptor(0x04, &dec_config);
        let sl_config = minimal_sl_config_descr();

        let url = "test";
        let mut data = vec![
            0x00,
            0x05, // es_id = 5
            0xE5, // flags: all flags set + priority = 5
            0x00,
            0x02, // depends_on_es_id = 2
            url.len() as u8,
        ];
        data.extend_from_slice(url.as_bytes());
        data.extend_from_slice(&[0x00, 0x03]); // ocr_es_id = 3
        data.extend_from_slice(&dec_config_wrapped);
        data.extend_from_slice(&sl_config);

        let desc = EsDescriptor::parse(&data).unwrap();

        assert_eq!(desc.es_id, 5);
        assert!(desc.stream_dependence_flag);
        assert!(desc.url_flag);
        assert!(desc.ocr_stream_flag);
        assert_eq!(desc.stream_priority, 5);
        assert_eq!(desc.depends_on_es_id, Some(2));
        assert_eq!(desc.url_string.as_deref(), Some("test"));
        assert_eq!(desc.ocr_es_id, Some(3));
    }
}
