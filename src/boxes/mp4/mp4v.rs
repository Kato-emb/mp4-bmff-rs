use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;
use crate::iter::BoxIter;

use crate::cursor::ReadCursor;

use super::esds::EsdsBoxView;
use crate::boxes::sample_entry::VisualSampleEntry;

/// A reference to an MP4 Visual Sample Entry
#[derive(Debug)]
pub struct Mp4vSampleEntryView<'a> {
    base: VisualSampleEntry,
    content: &'a [u8],
}

impl<'a> Mp4vSampleEntryView<'a> {
    /// Returns the base Visual Sample Entry.
    pub fn base(&self) -> &VisualSampleEntry {
        &self.base
    }

    /// Returns an iterator over the child boxes of this Mp4v Sample Entry.
    pub fn boxes(&self) -> BoxIter<'a> {
        BoxIter::new(self.content)
    }

    /// Returns the ESDS box contained in this Mp4v Sample Entry.
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
            BoxType::MP4V,
        ))
    }
}

impl BoxCodec for Mp4vSampleEntryView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::MP4V
    }
}

impl<'de> BoxDecode<'de> for Mp4vSampleEntryView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let base = VisualSampleEntry::parse_in(&mut cur)?;
        let content = cur.take(cur.remaining())?;

        Ok(Mp4vSampleEntryView { base, content })
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

    /// An owned MP4V Sample Entry
    #[derive(Debug, Clone)]
    pub struct Mp4vSampleEntry {
        /// The base Visual Sample Entry
        pub base: VisualSampleEntry,
        /// The ESDS box contained in this Mp4v Sample Entry.
        pub esds: EsdsBox,
    }

    impl TryFrom<&Mp4vSampleEntryView<'_>> for Mp4vSampleEntry {
        type Error = Error;

        fn try_from(view: &Mp4vSampleEntryView<'_>) -> Result<Self> {
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
                                BoxType::MP4V,
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
                    BoxType::MP4V,
                )
            })?;

            Ok(Mp4vSampleEntry {
                base: view.base,
                esds,
            })
        }
    }

    impl BoxCodec for Mp4vSampleEntry {
        fn boxtype(&self) -> BoxType {
            BoxType::MP4V
        }
    }

    impl BoxDecode<'_> for Mp4vSampleEntry {
        fn decode(bytes: &'_ [u8]) -> Result<Self> {
            let view = Mp4vSampleEntryView::decode(bytes)?;
            Mp4vSampleEntry::try_from(&view)
        }
    }

    impl BoxEncode for Mp4vSampleEntry {
        fn encoded_len(&self) -> usize {
            VisualSampleEntry::size() + boxed_len(&self.esds)
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
            0x20, // objectTypeIndication (Visual ISO/IEC 14496-2)
            0x11, // streamType (VisualStream) | reserved
            0x00, 0x01, 0x2c, // bufferSizeDB
            0x00, 0x01, 0xa2, 0xf0, // maxBitrate
            0x00, 0x00, 0x90, 0xd0, // avgBitrate
            // DecoderSpecificInfo (tag=0x05)
            0x05, // DECODER_SPECIFIC_INFO_TAG
            0x02, // size = 2
            0x00, 0x00, // MPEG-4 Visual specific config
            // SLConfigDescriptor (tag=0x06)
            0x06, // SL_CONFIG_DESCR_TAG
            0x01, // size = 1
            0x02, // predefined = 2
        ]
    }

    /// Creates a VisualSampleEntry payload (86 bytes).
    fn visual_sample_entry_bytes() -> Vec<u8> {
        vec![
            // SampleEntry base (8 bytes)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // reserved
            0x00, 0x01, // data_reference_index = 1
            // VisualSampleEntry specific (78 bytes)
            0x00, 0x00, // pre_defined
            0x00, 0x00, // reserved
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, // pre_defined (12 bytes)
            0x02, 0x80, // width = 640
            0x01, 0xe0, // height = 480
            0x00, 0x48, 0x00, 0x00, // horizresolution (72 dpi)
            0x00, 0x48, 0x00, 0x00, // vertresolution (72 dpi)
            0x00, 0x00, 0x00, 0x00, // reserved
            0x00, 0x01, // frame_count = 1
            // compressorname (32 bytes): first byte is length
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x18, // depth = 24
            0xFF, 0xFF, // pre_defined = -1
        ]
    }

    /// Creates a complete Mp4vSampleEntry payload for testing.
    fn sample_mp4v_payload() -> Vec<u8> {
        let mut payload = visual_sample_entry_bytes();
        payload.extend(sample_esds_box());
        payload
    }

    #[test]
    fn mp4v_sample_entry_view_decode() {
        let payload = sample_mp4v_payload();
        let view = Mp4vSampleEntryView::decode(&payload).unwrap();

        let base = view.base();
        assert_eq!(base.sample_entry().data_reference_index, 1);
        assert_eq!(base.width, 640);
        assert_eq!(base.height, 480);
        assert_eq!(base.frame_count, 1);
        assert_eq!(base.depth, 24);
    }

    #[test]
    fn mp4v_sample_entry_view_boxtype() {
        let payload = sample_mp4v_payload();
        let view = Mp4vSampleEntryView::decode(&payload).unwrap();
        assert_eq!(view.boxtype(), BoxType::MP4V);
    }

    #[test]
    fn mp4v_sample_entry_view_boxes_iterator() {
        let payload = sample_mp4v_payload();
        let view = Mp4vSampleEntryView::decode(&payload).unwrap();

        let boxes: Vec<_> = view.boxes().collect();
        assert_eq!(boxes.len(), 1);
        let rawbox = boxes[0].as_ref().unwrap();
        assert_eq!(rawbox.boxtype(), BoxType::ESDS);
    }

    #[test]
    fn mp4v_sample_entry_view_esds() {
        let payload = sample_mp4v_payload();
        let view = Mp4vSampleEntryView::decode(&payload).unwrap();

        let esds = view.esds().unwrap();
        assert_eq!(esds.version, 0);
        assert_eq!(esds.esd.es_id, 0);
    }

    #[test]
    fn mp4v_sample_entry_view_esds_missing() {
        // Only VisualSampleEntry base, no ESDS box
        let payload = visual_sample_entry_bytes();
        let view = Mp4vSampleEntryView::decode(&payload).unwrap();

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
    fn mp4v_sample_entry_try_from_view() {
        let payload = sample_mp4v_payload();
        let view = Mp4vSampleEntryView::decode(&payload).unwrap();

        let owned = Mp4vSampleEntry::try_from(&view).unwrap();
        assert_eq!(owned.base.sample_entry().data_reference_index, 1);
        assert_eq!(owned.base.width, 640);
        assert_eq!(owned.base.height, 480);
        assert_eq!(owned.esds.version, 0);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn mp4v_sample_entry_decode() {
        let payload = sample_mp4v_payload();
        let owned = Mp4vSampleEntry::decode(&payload).unwrap();

        assert_eq!(owned.base.width, 640);
        assert_eq!(owned.base.height, 480);
        assert_eq!(owned.esds.version, 0);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn mp4v_sample_entry_boxtype() {
        let payload = sample_mp4v_payload();
        let owned = Mp4vSampleEntry::decode(&payload).unwrap();
        assert_eq!(owned.boxtype(), BoxType::MP4V);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn mp4v_sample_entry_encode_decode_roundtrip() {
        use crate::BoxEncode;

        let payload = sample_mp4v_payload();
        let original = Mp4vSampleEntry::decode(&payload).unwrap();

        let mut buf = vec![0u8; original.encoded_len()];
        let written = original.encode_into(&mut buf).unwrap();
        assert_eq!(written, original.encoded_len());

        let reparsed = Mp4vSampleEntry::decode(&buf).unwrap();
        assert_eq!(reparsed.base.width, original.base.width);
        assert_eq!(reparsed.base.height, original.base.height);
        assert_eq!(reparsed.base.frame_count, original.base.frame_count);
        assert_eq!(reparsed.esds.version, original.esds.version);
        assert_eq!(reparsed.esds.esd.es_id, original.esds.esd.es_id);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn mp4v_sample_entry_duplicate_esds_error() {
        // Create payload with two ESDS boxes
        let mut payload = visual_sample_entry_bytes();
        payload.extend(sample_esds_box());
        payload.extend(sample_esds_box()); // duplicate

        let view = Mp4vSampleEntryView::decode(&payload).unwrap();
        let err = Mp4vSampleEntry::try_from(&view).unwrap_err();

        match err.kind() {
            ErrorKind::BoxDuplicate { duplicate } => {
                assert_eq!(duplicate, BoxType::ESDS);
            }
            _ => panic!("Expected BoxDuplicate error"),
        }
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn mp4v_sample_entry_missing_esds_error() {
        let payload = visual_sample_entry_bytes();
        let err = Mp4vSampleEntry::decode(&payload).unwrap_err();

        match err.kind() {
            ErrorKind::BoxMissing { required } => {
                assert_eq!(required, BoxType::ESDS);
            }
            _ => panic!("Expected BoxMissing error"),
        }
    }
}
