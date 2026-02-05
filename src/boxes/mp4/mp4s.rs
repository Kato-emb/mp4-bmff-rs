use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::BoxIter;

use crate::cursor::ReadCursor;

use super::esds::EsdsBoxView;
use crate::boxes::sample_entry::SampleEntry;

/// A reference to an All other Mpeg stream Sample Entry
#[derive(Debug)]
pub struct MpegSampleEntryView<'a> {
    base: SampleEntry,
    content: &'a [u8],
}

impl<'a> MpegSampleEntryView<'a> {
    /// Returns the base Visual Sample Entry.
    pub fn base(&self) -> &SampleEntry {
        &self.base
    }

    /// Returns an iterator over the child boxes of this Mpeg Sample Entry.
    pub fn boxes(&self) -> BoxIter<'a> {
        BoxIter::new(self.content)
    }

    /// Returns the ESDS box contained in this Mpeg Sample Entry.
    pub fn esds(&self) -> Result<EsdsBoxView<'a>> {
        for result in self.boxes() {
            let rawbox = result?;
            if rawbox.boxtype() == BoxType::ESDS {
                return EsdsBoxView::decode(rawbox.into_payload());
            }
        }

        Err(Error::in_box(
            ErrorKind::BoxMissing {
                required: BoxType::ESDS,
            },
            BoxType::MP4S,
        ))
    }
}

impl BoxCodec for MpegSampleEntryView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::MP4S
    }
}

impl<'de> BoxDecode<'de> for MpegSampleEntryView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let base = SampleEntry::parse_in(&mut cur)?;
        let content = cur.take(cur.remaining())?;

        Ok(MpegSampleEntryView { base, content })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use super::*;
    use crate::BoxEncode;

    use crate::codec::boxed_len;
    use crate::codec::write_box_in;
    use crate::cursor::WriteCursor;

    use crate::boxes::mp4::EsdsBox;

    /// An owned All other Mpeg stream Sample Entry
    #[derive(Debug, Clone)]
    pub struct MpegSampleEntry {
        /// The base Sample Entry
        pub base: SampleEntry,
        /// The ESDS box contained in this Mpeg Sample Entry.
        pub esds: EsdsBox,
    }

    impl TryFrom<&MpegSampleEntryView<'_>> for MpegSampleEntry {
        type Error = Error;

        fn try_from(view: &MpegSampleEntryView<'_>) -> Result<Self> {
            let mut esds = None;

            for result in view.boxes() {
                let rawbox = result?;

                match rawbox.boxtype() {
                    BoxType::ESDS => {
                        if esds.is_some() {
                            return Err(Error::in_box(
                                ErrorKind::BoxDuplicate {
                                    duplicate: BoxType::ESDS,
                                },
                                BoxType::MP4S,
                            ));
                        }
                        esds = Some(EsdsBox::decode(rawbox.into_payload())?);
                    }
                    _ => continue,
                }
            }

            let esds = esds.ok_or_else(|| {
                Error::in_box(
                    ErrorKind::BoxMissing {
                        required: BoxType::ESDS,
                    },
                    BoxType::MP4S,
                )
            })?;

            Ok(MpegSampleEntry {
                base: view.base,
                esds,
            })
        }
    }

    impl BoxCodec for MpegSampleEntry {
        fn boxtype(&self) -> BoxType {
            BoxType::MP4S
        }
    }

    impl BoxDecode<'_> for MpegSampleEntry {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = MpegSampleEntryView::decode(bytes)?;
            MpegSampleEntry::try_from(&view)
        }
    }

    impl BoxEncode for MpegSampleEntry {
        fn encoded_len(&self) -> usize {
            SampleEntry::size() + boxed_len(&self.esds)
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            self.base.write_in(&mut cur)?;
            write_box_in(&mut cur, &self.esds)?;

            Ok(cur.position())
        }
    }
}

#[cfg(feature = "alloc")]
pub use owned::*;

#[cfg(test)]
mod tests {
    use super::*;

    /// Creates a sample ESDS box payload for testing.
    fn sample_esds_box() -> Vec<u8> {
        vec![
            // Box header
            0x00, 0x00, 0x00, 0x27, // size = 39
            b'e', b's', b'd', b's', // type
            // ESDS box payload
            0x00, // version
            0x00, 0x00, 0x00, // flags
            // ES Descriptor (tag=0x03)
            0x03, // ES_DESCR_TAG
            0x19, // size = 25
            0x00, 0x00, // ES_ID
            0x00, // flags byte
            // DecoderConfigDescriptor (tag=0x04)
            0x04, // DECODER_CONFIG_DESCR_TAG
            0x11, // size = 17
            0x01, // objectTypeIndication (Systems ISO/IEC 14496-1)
            0x01, // streamType (ObjectDescriptorStream) | reserved
            0x00, 0x01, 0x2c, // bufferSizeDB
            0x00, 0x01, 0xa2, 0xf0, // maxBitrate
            0x00, 0x00, 0x90, 0xd0, // avgBitrate
            // DecoderSpecificInfo (tag=0x05)
            0x05, // DECODER_SPECIFIC_INFO_TAG
            0x02, // size = 2
            0x00, 0x00, // Systems specific config
            // SLConfigDescriptor (tag=0x06)
            0x06, // SL_CONFIG_DESCR_TAG
            0x01, // size = 1
            0x02, // predefined = 2
        ]
    }

    /// Creates a SampleEntry payload (8 bytes).
    fn sample_entry_bytes() -> Vec<u8> {
        vec![
            // SampleEntry base (8 bytes)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // reserved
            0x00, 0x01, // data_reference_index = 1
        ]
    }

    /// Creates a complete MpegSampleEntry payload for testing.
    fn sample_mp4s_payload() -> Vec<u8> {
        let mut payload = sample_entry_bytes();
        payload.extend(sample_esds_box());
        payload
    }

    #[test]
    fn mpeg_sample_entry_view_decode() {
        let payload = sample_mp4s_payload();
        let view = MpegSampleEntryView::decode(&payload).unwrap();

        let base = view.base();
        assert_eq!(base.data_reference_index, 1);
    }

    #[test]
    fn mpeg_sample_entry_view_boxtype() {
        let payload = sample_mp4s_payload();
        let view = MpegSampleEntryView::decode(&payload).unwrap();
        assert_eq!(view.boxtype(), BoxType::MP4S);
    }

    #[test]
    fn mpeg_sample_entry_view_boxes_iterator() {
        let payload = sample_mp4s_payload();
        let view = MpegSampleEntryView::decode(&payload).unwrap();

        let boxes: Vec<_> = view.boxes().collect();
        assert_eq!(boxes.len(), 1);
        let rawbox = boxes[0].as_ref().unwrap();
        assert_eq!(rawbox.boxtype(), BoxType::ESDS);
    }

    #[test]
    fn mpeg_sample_entry_view_esds() {
        let payload = sample_mp4s_payload();
        let view = MpegSampleEntryView::decode(&payload).unwrap();

        let esds = view.esds().unwrap();
        assert_eq!(esds.version, 0);
        assert_eq!(esds.esd.es_id, 0);
    }

    #[test]
    fn mpeg_sample_entry_view_esds_missing() {
        // Only SampleEntry base, no ESDS box
        let payload = sample_entry_bytes();
        let view = MpegSampleEntryView::decode(&payload).unwrap();

        let err = view.esds().unwrap_err();
        match err.kind() {
            ErrorKind::BoxMissing { required } => {
                assert_eq!(required, BoxType::ESDS);
            }
            _ => panic!("Expected BoxMissing error"),
        }
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn mpeg_sample_entry_try_from_view() {
        let payload = sample_mp4s_payload();
        let view = MpegSampleEntryView::decode(&payload).unwrap();

        let owned = MpegSampleEntry::try_from(&view).unwrap();
        assert_eq!(owned.base.data_reference_index, 1);
        assert_eq!(owned.esds.version, 0);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn mpeg_sample_entry_decode() {
        let payload = sample_mp4s_payload();
        let owned = MpegSampleEntry::decode(&payload).unwrap();

        assert_eq!(owned.base.data_reference_index, 1);
        assert_eq!(owned.esds.version, 0);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn mpeg_sample_entry_boxtype() {
        let payload = sample_mp4s_payload();
        let owned = MpegSampleEntry::decode(&payload).unwrap();
        assert_eq!(owned.boxtype(), BoxType::MP4S);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn mpeg_sample_entry_encode_decode_roundtrip() {
        use crate::BoxEncode;

        let payload = sample_mp4s_payload();
        let original = MpegSampleEntry::decode(&payload).unwrap();

        let mut buf = vec![0u8; original.encoded_len()];
        let written = original.encode_into(&mut buf).unwrap();
        assert_eq!(written, original.encoded_len());

        let reparsed = MpegSampleEntry::decode(&buf).unwrap();
        assert_eq!(
            reparsed.base.data_reference_index,
            original.base.data_reference_index
        );
        assert_eq!(reparsed.esds.version, original.esds.version);
        assert_eq!(reparsed.esds.esd.es_id, original.esds.esd.es_id);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn mpeg_sample_entry_duplicate_esds_error() {
        // Create payload with two ESDS boxes
        let mut payload = sample_entry_bytes();
        payload.extend(sample_esds_box());
        payload.extend(sample_esds_box()); // duplicate

        let view = MpegSampleEntryView::decode(&payload).unwrap();
        let err = MpegSampleEntry::try_from(&view).unwrap_err();

        match err.kind() {
            ErrorKind::BoxDuplicate { duplicate } => {
                assert_eq!(duplicate, BoxType::ESDS);
            }
            _ => panic!("Expected BoxDuplicate error"),
        }
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn mpeg_sample_entry_missing_esds_error() {
        let payload = sample_entry_bytes();
        let err = MpegSampleEntry::decode(&payload).unwrap_err();

        match err.kind() {
            ErrorKind::BoxMissing { required } => {
                assert_eq!(required, BoxType::ESDS);
            }
            _ => panic!("Expected BoxMissing error"),
        }
    }
}
