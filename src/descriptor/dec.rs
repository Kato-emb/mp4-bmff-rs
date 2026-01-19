use crate::cursor::ReadCursor;

use crate::error::*;

use super::DescriptorView;
use super::DescrptorIter;
use super::ProfileLevelIndicationIndexDescriptor;
use super::Tag;

/// Object Type Indication (ISO/IEC 14496-3 Table 5 - ObjectTypeIndication Values)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObjectTypeIndication(u8);

impl ObjectTypeIndication {
    /// Object Type Indication for MPEG-4 Audio
    pub const MPEG4_AUDIO: ObjectTypeIndication = ObjectTypeIndication(0x40);
    /// Object Type Indication for MPEG-4 Visual
    pub const MPEG4_VISUAL: ObjectTypeIndication = ObjectTypeIndication(0x20);
    /// Object Type Indication for AVC Video
    pub const AVC_VIDEO: ObjectTypeIndication = ObjectTypeIndication(0x21);
}

/// Stream Type (ISO/IEC 14496-1 Table 6 - StreamType Values)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamType(u8);

impl StreamType {
    /// Stream Type for Audio Stream
    pub const AUDIO_STREAM: StreamType = StreamType(0x05);
    /// Stream Type for Visual Stream
    pub const VISUAL_STREAM: StreamType = StreamType(0x04);
    /// Stream Type for Text Stream
    pub const TEXT_STREAM: StreamType = StreamType(0x06);
}

/// Decoder Config Descriptor
#[derive(Debug, Clone, Copy)]
pub struct DecoderConfigDescriptorView<'a> {
    /// Object Type Indication
    pub object_type_indication: ObjectTypeIndication,
    /// Stream Type
    pub stream_type: StreamType,
    /// Used for upstream information
    pub up_stream: bool,
    /// Decoding buffer for this elementary stream in byte
    pub buffer_size_db: [u8; 3],
    /// Maximum bitrate in bits per second
    pub max_bitrate: u32,
    /// Average bitrate in bits per second
    pub avg_bitrate: u32,
    dec_specific_info: Option<&'a [u8]>,
    profile_level_indication_index_descriptor: &'a [u8],
}

impl<'a> DecoderConfigDescriptorView<'a> {
    /// Parses DecoderConfigDescriptor from a DescriptorView (ISO/IEC 14496-1 Section 7.2.6.7)
    pub fn dec_specific_info(&self) -> Option<Result<DescriptorView<'a>>> {
        self.dec_specific_info.map(|data| {
            let mut cur = ReadCursor::new(data);
            match DescriptorView::parse_in(&mut cur) {
                Ok(descr) => {
                    if descr.tag != Tag::DECODER_SPECIFIC_INFO_TAG {
                        Err(Error::at(
                            ErrorKind::Other {
                                description: "Expected Decoder Specific Info descriptor",
                            },
                            0,
                        ))
                    } else {
                        Ok(descr)
                    }
                }
                Err(e) => Err(e),
            }
        })
    }

    /// Returns an iterator over Profile Level Indication Index Descriptors
    pub fn profile_level_indication_index_descriptors(
        &self,
    ) -> impl Iterator<Item = Result<ProfileLevelIndicationIndexDescriptor>> {
        DescrptorIter::new(self.profile_level_indication_index_descriptor).map(|res| {
            res.and_then(|descr| {
                if descr.tag != Tag::PROFILE_LEVEL_INDICATION_INDEX_DESCR_TAG {
                    Err(Error::new(ErrorKind::Other {
                        description: "Expected Profile Level Indication Index descriptor",
                    }))
                } else {
                    Ok(ProfileLevelIndicationIndexDescriptor::parse(
                        descr.instance,
                    )?)
                }
            })
        })
    }

    pub(crate) fn parse_in(cur: &mut ReadCursor<'a>) -> Result<Self> {
        let object_type_indication = ObjectTypeIndication(cur.read_u8()?);

        let stream_type_byte = cur.read_u8()?;
        let stream_type = StreamType(stream_type_byte >> 2);
        let up_stream = (stream_type_byte & 0b00000010) != 0;
        // Reserved bit must be 1

        let buffer_size_db = cur.read_array::<3>()?;

        let max_bitrate = cur.read_u32_be()?;
        let avg_bitrate = cur.read_u32_be()?;

        let start_pos = cur.position();
        let dec_specific_info =
            if DescriptorView::parse_in(cur)?.tag == Tag::DECODER_CONFIG_DESCR_TAG {
                let end_pos = cur.position();
                Some(&cur.inner()[start_pos..end_pos])
            } else {
                cur.set_position(start_pos);
                None
            };

        let profile_level_indication_index_descriptor = &cur.inner()[cur.position() as usize..];

        Ok(DecoderConfigDescriptorView {
            object_type_indication,
            stream_type,
            up_stream,
            buffer_size_db,
            max_bitrate,
            avg_bitrate,
            dec_specific_info,
            profile_level_indication_index_descriptor,
        })
    }

    /// Parses DecoderConfigDescriptor from a byte slice
    pub fn parse(instance: &'a [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(instance);
        Self::parse_in(&mut cur)
    }
}

#[cfg(feature = "alloc")]
pub use owned::DecoderConfigDescriptor;

#[cfg(feature = "alloc")]
mod owned {
    use super::*;
    use crate::descriptor::DescriptorOwned;

    /// Owned Decoder Config Descriptor
    #[derive(Debug, Clone)]
    pub struct DecoderConfigDescriptor {
        /// Object Type Indication
        pub object_type_indication: ObjectTypeIndication,
        /// Stream Type
        pub stream_type: StreamType,
        /// Used for upstream information
        pub up_stream: bool,
        /// Decoding buffer for this elementary stream in byte
        pub buffer_size_db: [u8; 3],
        /// Maximum bitrate in bits per second
        pub max_bitrate: u32,
        /// Average bitrate in bits per second
        pub avg_bitrate: u32,
        /// Decoder Specific Info descriptor
        pub dec_specific_info: Option<DescriptorOwned>,
        /// Profile Level Indication Index Descriptors
        pub profile_level_indication_index_descriptors: Vec<ProfileLevelIndicationIndexDescriptor>,
    }

    impl DecoderConfigDescriptor {
        /// Creates an owned DecoderConfigDescriptor from a view
        pub fn from_view(view: &DecoderConfigDescriptorView<'_>) -> Result<Self> {
            let dec_specific_info = match view.dec_specific_info() {
                Some(res) => Some(DescriptorOwned::from_view(&res?)),
                None => None,
            };

            let mut profile_level_indication_index_descriptors = Vec::new();
            for res in view.profile_level_indication_index_descriptors() {
                profile_level_indication_index_descriptors.push(res?);
            }

            Ok(DecoderConfigDescriptor {
                object_type_indication: view.object_type_indication,
                stream_type: view.stream_type,
                up_stream: view.up_stream,
                buffer_size_db: view.buffer_size_db,
                max_bitrate: view.max_bitrate,
                avg_bitrate: view.avg_bitrate,
                dec_specific_info,
                profile_level_indication_index_descriptors,
            })
        }

        /// Parses DecoderConfigDescriptor from a byte slice
        pub fn parse(instance: &[u8]) -> Result<Self> {
            let view = DecoderConfigDescriptorView::parse(instance)?;
            Self::from_view(&view)
        }
    }

    impl TryFrom<&DecoderConfigDescriptorView<'_>> for DecoderConfigDescriptor {
        type Error = Error;

        fn try_from(value: &DecoderConfigDescriptorView<'_>) -> Result<Self> {
            Self::from_view(value)
        }
    }
}
