use crate::cursor::ReadCursor;

use crate::error::*;

use crate::descriptor::DescriptorView;
use crate::descriptor::EsDescriptorView;
use crate::descriptor::Tag;

use super::FullBoxFlags;

/// A reference to an ESDS box's contents.
#[derive(Debug)]
pub struct EsdsBoxView<'a> {
    /// The version of this ESDS box.
    pub version: u8,
    /// The flags of this ESDS box.
    pub flags: EsdsFlags,
    /// The ES Descriptor contained in this ESDS box.
    pub esd: EsDescriptorView<'a>,
}

impl<'a> EsdsBoxView<'a> {
    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<Self> {
        let version = cur.read_u8()?;
        let flags = EsdsFlags::from_bytes(cur.read_array()?);

        let es_descr = DescriptorView::parse_in(cur)?;
        let esd = if es_descr.tag == Tag::ES_DESCR_TAG {
            EsDescriptorView::parse(es_descr.instance)?
        } else {
            return Err(Error::at(
                ErrorKind::Other {
                    description: "Expected ES Descriptor",
                },
                cur.position() as u64,
            ));
        };

        Ok(EsdsBoxView {
            version,
            flags,
            esd,
        })
    }

    /// Parses an `EsdsBoxView` from the given payload.
    pub fn parse(payload: &'a [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(payload);
        let this = EsdsBoxView::parse_in(&mut cur)?;

        Ok(this)
    }
}

/// Specification type for ESDS Box ('esds')
pub struct EsdsSpec;

/// The flags for the ESDS Box ('esds')
pub type EsdsFlags = FullBoxFlags<EsdsSpec>;

#[cfg(feature = "alloc")]
pub use owned::EsdsBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::cursor::WriteCursor;
    use crate::descriptor::EsDescriptor;
    use crate::lib::Vec;

    use crate::BoxType;

    use super::*;

    /// An owned ESDS box.
    #[derive(Debug, Clone)]
    pub struct EsdsBox {
        /// The version of this ESDS box.
        pub version: u8,
        /// The flags of this ESDS box.
        pub flags: EsdsFlags,
        /// The ES Descriptor contained in this ESDS box.
        pub esd: EsDescriptor,
        /// Raw descriptor data for round-trip writing.
        descriptor_data: Vec<u8>,
    }

    impl EsdsBox {
        /// Creates an owned ESDS box from a view.
        pub fn from_view(view: &EsdsBoxView) -> Result<Self> {
            Ok(Self {
                version: view.version,
                flags: view.flags,
                esd: EsDescriptor::from_view(&view.esd)?,
                descriptor_data: Vec::new(), // Will be populated by from_view_with_data
            })
        }

        /// Creates an owned ESDS box from a view, preserving the raw descriptor data.
        pub fn from_view_with_data(view: &EsdsBoxView, payload: &[u8]) -> Result<Self> {
            // payload contains: version (1) + flags (3) + descriptor data
            let descriptor_data = if payload.len() > 4 {
                payload[4..].to_vec()
            } else {
                Vec::new()
            };

            Ok(Self {
                version: view.version,
                flags: view.flags,
                esd: EsDescriptor::from_view(&view.esd)?,
                descriptor_data,
            })
        }

        /// Parses an `EsdsBox` from the given payload.
        pub fn parse(payload: &[u8]) -> Result<Self> {
            let view = EsdsBoxView::parse(payload)?;
            Self::from_view_with_data(&view, payload)
        }

        /// Returns the size of the `EsdsBox` data.
        pub fn size(&self) -> usize {
            1  // version
            + 3  // flags
            + self.descriptor_data.len() // descriptor data
        }

        pub(crate) fn write_in(&self, cur: &mut WriteCursor<'_>) -> Result<()> {
            // Write version (1 byte)
            cur.write_u8(self.version)?;

            // Write flags (3 bytes)
            cur.write_slice(&self.flags.to_bytes())?;

            // Write descriptor data as-is
            cur.write_slice(&self.descriptor_data)?;

            if !cur.is_empty() {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxSize {
                        reason: "Buffer larger than expected",
                        got: cur.remaining() as u64,
                    },
                    BoxType::ESDS,
                ));
            }

            Ok(())
        }

        /// Writes this `EsdsBox` into the given payload.
        pub fn write(&self, payload: &mut [u8]) -> Result<()> {
            let mut cursor = WriteCursor::new(payload);
            self.write_in(&mut cursor)
        }
    }

    impl TryFrom<EsdsBoxView<'_>> for EsdsBox {
        type Error = Error;

        fn try_from(value: EsdsBoxView<'_>) -> Result<Self> {
            EsdsBox::from_view(&value)
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "alloc")]
    use super::*;

    #[cfg(feature = "alloc")]
    #[test]
    fn esds_box_round_trip() {
        // Sample ESDS payload with version, flags, and ES Descriptor
        let original_payload: [u8; 31] = [
            0x00, // version
            0x00, 0x00, 0x00, // flags
            // ES Descriptor (tag=0x03)
            0x03, // ES_DESCR_TAG
            0x19, // size = 25
            0x00, 0x00, // ES_ID
            0x00, // flags byte (no dependencies, no URL, no OCR)
            // DecoderConfigDescriptor (tag=0x04)
            0x04, // DECODER_CONFIG_DESCR_TAG
            0x11, // size = 17
            0x67, // objectTypeIndication (MPEG-4 Audio)
            0x15, // streamType (AudioStream) | reserved
            0x00, 0x01, 0x2c, // bufferSizeDB
            0x00, 0x01, 0xa2, 0xf0, // maxBitrate
            0x00, 0x00, 0x90, 0xd0, // avgBitrate
            // DecoderSpecificInfo (tag=0x05)
            0x05, // DECODER_SPECIFIC_INFO_TAG
            0x02, // size = 2
            0x13, 0x90, // AAC specific config
            // SLConfigDescriptor (tag=0x06)
            0x06, // SL_CONFIG_DESCR_TAG
            0x01, // size = 1
            0x02, // predefined = 2
        ];

        // Parse
        let esds = EsdsBox::parse(&original_payload).unwrap();

        // Write
        let mut buf = vec![0u8; esds.size()];
        esds.write(&mut buf).unwrap();

        // Verify round-trip
        assert_eq!(buf, original_payload);

        // Parse again and verify
        let reparsed = EsdsBox::parse(&buf).unwrap();
        assert_eq!(reparsed.version, esds.version);
        assert_eq!(reparsed.flags.get(), esds.flags.get());
        assert_eq!(reparsed.esd.es_id, esds.esd.es_id);
        assert_eq!(
            reparsed
                .esd
                .decoder_config_descriptor
                .object_type_indication,
            esds.esd.decoder_config_descriptor.object_type_indication
        );
    }
}
