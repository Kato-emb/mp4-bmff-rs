use crate::cursor::ReadCursor;

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
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

/// Specification type for ESDS Box ('esds')
pub struct EsdsSpec;

/// The flags for the ESDS Box ('esds')
pub type EsdsFlags = FullBoxFlags<EsdsSpec>;

impl BoxCodec for EsdsBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::ESDS
    }
}

impl<'de> BoxDecode<'de> for EsdsBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = EsdsFlags::from_bytes(cur.read_array()?);

        let es_descr = DescriptorView::parse_in(&mut cur)?;
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
}

impl<'a> TryFrom<&'a [u8]> for EsdsBoxView<'a> {
    type Error = Error;

    fn try_from(value: &'a [u8]) -> Result<Self> {
        EsdsBoxView::decode(value)
    }
}

#[cfg(feature = "alloc")]
pub use owned::EsdsBox;

#[cfg(feature = "alloc")]
mod owned {
    use crate::BoxEncode;
    use crate::cursor::WriteCursor;
    use crate::descriptor::EsDescriptor;

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
    }

    impl TryFrom<&EsdsBoxView<'_>> for EsdsBox {
        type Error = Error;

        fn try_from(view: &EsdsBoxView<'_>) -> Result<Self> {
            let esd = EsDescriptor::from_view(&view.esd)?;
            Ok(EsdsBox {
                version: view.version,
                flags: view.flags,
                esd,
            })
        }
    }

    impl BoxCodec for EsdsBox {
        fn boxtype(&self) -> BoxType {
            BoxType::ESDS
        }
    }

    impl BoxDecode<'_> for EsdsBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = EsdsBoxView::decode(bytes)?;
            EsdsBox::try_from(&view)
        }
    }

    impl BoxEncode for EsdsBox {
        fn encode(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            // Write version (1 byte)
            cur.write_u8(self.version)?;

            // Write flags (3 bytes)
            cur.write_slice(&self.flags.to_bytes())?;

            // Write ES Descriptor
            self.esd.write_in(&mut cur)?;

            Ok(cur.position())
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

        use crate::BoxEncode;
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
        let esds = EsdsBox::decode(&original_payload).unwrap();

        // Write
        let mut buf = vec![0u8; 256];
        let written = esds.encode(&mut buf).unwrap();

        // Parse again and verify values match
        let reparsed = EsdsBox::decode(&buf[..written]).unwrap();
        assert_eq!(reparsed.version, esds.version);
        assert_eq!(reparsed.flags.get(), esds.flags.get());
        assert_eq!(reparsed.esd.es_id, esds.esd.es_id);
    }
}
