//! Channel Layout Box (`chnl`) implementation.
//!
//! The Channel Layout Box provides information about the channel layout
//! of an audio stream, supporting both channel-structured and
//! object-structured configurations as defined in ISO/IEC 14496-12.

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxType;
use crate::error::*;

use crate::cursor::ReadCursor;

/// Speaker position information for a single channel in a channel-structured stream.
///
/// The `position` field indicates the speaker's location in the sound field, with predefined values from 0 to 125.
/// If `position` is 126, it indicates an explicit position defined by the `azimuth` and `elevation` fields. Values 127 and above are reserved.
#[derive(Debug, Clone, Copy)]
pub struct SpeakerPosition {
    /// Position code indicating the speaker's location in the sound field.
    /// Values 0-125 are predefined positions, while 126 indicates an explicit position defined by the `azimuth` and `elevation` fields.
    pub position: u8,
    /// Present only when position == 126 (explicit).
    /// Range: 0 to 360 degrees, where 0 is directly in front of the listener,
    /// positive is clockwise, and negative is counterclockwise.
    pub azimuth: Option<i16>,
    /// Present only when position == 126 (explicit).
    /// Range: -90 to +90 degrees, where 0 is the horizontal plane,
    /// positive is above, and negative is below.
    pub elevation: Option<i8>,
}

pub(crate) struct SpeakerPositionIter<'a> {
    cur: ReadCursor<'a>,
    remaining: usize,
}

impl<'a> SpeakerPositionIter<'a> {
    fn new(data: &'a [u8], channel_count: usize) -> Self {
        SpeakerPositionIter {
            cur: ReadCursor::new(data),
            remaining: channel_count,
        }
    }
}

impl Iterator for SpeakerPositionIter<'_> {
    type Item = Result<SpeakerPosition>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }

        self.remaining -= 1;

        let position = match self.cur.read_u8() {
            Ok(pos) => pos,
            Err(e) => return Some(Err(e.into())),
        };

        if position == 126 {
            // Explicit position with azimuth and elevation
            let azimuth = match self.cur.read_i16_be() {
                Ok(az) => az,
                Err(e) => return Some(Err(e.into())),
            };
            let elevation = match self.cur.read_i8() {
                Ok(el) => el,
                Err(e) => return Some(Err(e.into())),
            };
            Some(Ok(SpeakerPosition {
                position,
                azimuth: Some(azimuth),
                elevation: Some(elevation),
            }))
        } else {
            // Predefined position, no azimuth/elevation
            Some(Ok(SpeakerPosition {
                position,
                azimuth: None,
                elevation: None,
            }))
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for SpeakerPositionIter<'_> {}

/// Channel Layout information for a channel-structured stream, which can be either predefined or explicit.
///
/// - For predefined layouts, `defined_layout` indicates the layout type, and `omitted_channels_map` specifies which channels are omitted from the defined layout.
/// - For explicit layouts, the `data` field contains the speaker position information for each channel in the stream,
/// with one byte per channel indicating the speaker position code (0-125 for predefined positions, 126 for explicit positions with additional azimuth/elevation data, and 127+ reserved).
#[derive(Debug, Clone)]
pub enum ChannelLayout<T> {
    /// Explicit channel layout with custom speaker positions for each channel.
    Explicit(T),
    /// Predefined channel layout with a defined layout type and a bitmask of omitted channels.
    Predefined {
        /// Defined layout type, where 0 indicates an explicit layout and non-zero values indicate predefined layouts.
        defined_layout: u8,
        /// Bitmask indicating which channels are omitted from the defined layout.
        /// Each bit corresponds to a channel in the predefined layout, where 0 means the channel is included and 1 means it is omitted.
        omitted_channels_map: u64,
    },
}

impl<T: AsRef<[u8]>> ChannelLayout<T> {
    /// If this is an explicit channel layout, returns an iterator over the speaker positions for each channel.
    pub fn speaker_positions(
        &self,
        channel_count: u16,
    ) -> Option<impl Iterator<Item = Result<SpeakerPosition>> + ExactSizeIterator + '_> {
        match self {
            ChannelLayout::Explicit(data) => Some(SpeakerPositionIter::new(
                data.as_ref(),
                usize::from(channel_count),
            )),
            _ => None,
        }
    }
}

define_box_flags!(
    /// Flags for the Channel Layout Box (`chnl`).
    ///
    /// Reserved (should be 0).
    ChnlFlags {}
);

const CHANNEL_STRUCTURED: u8 = 1;
const OBJECT_STRUCTURED: u8 = 2;

/// A reference to a Channel Layout Box (`chnl`).
///
/// Provides information about the channel layout of an audio stream, which can be either channel-structured (with specific speaker positions) or object-structured (with a count of audio objects).
///
/// # Structure
/// - `version`: Box version (0 or 1).
/// - `flags`: Reserved (should be 0).
/// - `stream_structure`: Bit flags indicating the stream structure:
///   - Bit 0 (0x01): If set, the stream is channel-structured and contains channel layout information.
///   - Bit 1 (0x02): If set, the stream is object-structured and contains an object count.
/// - `channel_layout`: Channel layout information, present if `stream_structure & 0x01 != 0`. Can be either predefined or explicit.
/// - `object_count`: Object count, present if `stream_structure & 0x02 != 0`. Indicates the number of audio objects in an object-structured stream.
#[derive(Debug, Clone)]
pub struct ChnlBoxView<'a> {
    /// Box version (0 or 1).
    pub version: u8,
    /// Reserved (should be 0).
    pub flags: ChnlFlags,
    /// Stream structure flags:
    /// - Bit 0 (0x01): If set, the stream is channel-structured and contains channel layout information.
    /// - Bit 1 (0x02): If set, the stream is object-structured and contains an object count.
    pub stream_structure: u8,
    /// Channel layout information, present if `stream_structure & 0x01 != 0`. Can be either predefined or explicit.
    pub channel_layout: Option<ChannelLayout<&'a [u8]>>,
    /// Object count, present if `stream_structure & 0x02 != 0`. Indicates the number of audio objects in an object-structured stream.
    pub object_count: Option<u8>,
}

impl BoxCodec for ChnlBoxView<'_> {
    fn boxtype(&self) -> BoxType {
        BoxType::CHNL
    }
}

impl<'de> BoxDecode<'de> for ChnlBoxView<'de> {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = ChnlFlags::from_be_bytes(cur.read_array::<3>()?);

        let stream_structure = cur.read_u8()?;

        let has_channel_structure = (stream_structure & CHANNEL_STRUCTURED) != 0;
        let has_object_structure = (stream_structure & OBJECT_STRUCTURED) != 0;

        let channel_layout = if has_channel_structure {
            let defined_layout = cur.read_u8()?;
            if defined_layout == 0 {
                // If object_structured flag is also set, the last byte
                // is object_count and must not be included in speaker data.
                let remaining = cur.remaining();
                let data_len = if has_object_structure {
                    remaining.saturating_sub(1)
                } else {
                    remaining
                };
                let data = cur.take(data_len)?;
                Some(ChannelLayout::Explicit(data))
            } else {
                let omitted_channels_map = cur.read_u64_be()?;
                Some(ChannelLayout::Predefined {
                    defined_layout,
                    omitted_channels_map,
                })
            }
        } else {
            None
        };

        let object_count = if has_object_structure {
            Some(cur.read_u8()?)
        } else {
            None
        };

        Ok(ChnlBoxView {
            version,
            flags,
            stream_structure,
            channel_layout,
            object_count,
        })
    }
}

#[cfg(feature = "alloc")]
mod owned {
    use alloc::vec::Vec;

    use super::*;
    use crate::BoxEncode;

    use crate::cursor::WriteCursor;

    /// An owned Channel Layout Box (`chnl`).
    #[derive(Debug, Clone)]
    pub struct ChnlBox {
        /// Box version (should be 0).
        pub version: u8,
        /// Reserved (should be 0).
        pub flags: ChnlFlags,
        /// Stream structure flags:
        /// - Bit 0 (0x01): If set, the stream is channel-structured and contains channel layout information.
        /// - Bit 1 (0x02): If set, the stream is object-structured and contains an object count.
        pub stream_structure: u8,
        /// Channel layout information, present if `stream_structure & 0x01 != 0`. Can be either predefined or explicit.
        pub channel_layout: Option<ChannelLayout<Vec<u8>>>,
        /// Object count, present if `stream_structure & 0x02 != 0`. Indicates the number of audio objects in an object-structured stream.
        pub object_count: Option<u8>,
    }

    impl Default for ChnlBox {
        fn default() -> Self {
            ChnlBox {
                version: 0,
                flags: ChnlFlags::empty(),
                stream_structure: 0,
                channel_layout: None,
                object_count: None,
            }
        }
    }

    impl From<&ChnlBoxView<'_>> for ChnlBox {
        fn from(view: &ChnlBoxView<'_>) -> Self {
            let channel_layout = view.channel_layout.as_ref().map(|cl| match cl {
                ChannelLayout::Explicit(data) => ChannelLayout::Explicit(data.to_vec()),
                ChannelLayout::Predefined {
                    defined_layout,
                    omitted_channels_map,
                } => ChannelLayout::Predefined {
                    defined_layout: *defined_layout,
                    omitted_channels_map: *omitted_channels_map,
                },
            });

            ChnlBox {
                version: view.version,
                flags: view.flags,
                stream_structure: view.stream_structure,
                channel_layout,
                object_count: view.object_count,
            }
        }
    }

    impl ChnlBoxView<'_> {
        /// Converts this `ChnlBoxView` into an owned `ChnlBox`.
        pub fn to_owned(&self) -> ChnlBox {
            ChnlBox::from(self)
        }
    }

    impl BoxCodec for ChnlBox {
        fn boxtype(&self) -> BoxType {
            BoxType::CHNL
        }
    }

    impl BoxDecode<'_> for ChnlBox {
        fn decode(bytes: &[u8]) -> Result<Self> {
            let view = ChnlBoxView::decode(bytes)?;
            Ok(ChnlBox::from(&view))
        }
    }

    impl BoxEncode for ChnlBox {
        fn encoded_len(&self) -> usize {
            4 // version + flags
            + 1 // stream_structure
            + self
                .channel_layout
                .as_ref()
                .map(|cl| match cl {
                    ChannelLayout::Explicit(data) => 1 + data.len(), // defined_layout + data
                    ChannelLayout::Predefined { .. } => 1 + 8,       // defined_layout + omitted_channels_map
                })
                .unwrap_or(0)
            + self.object_count.map(|_| 1).unwrap_or(0) // object_count if present
        }

        fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
            let mut cur = WriteCursor::new(bytes);

            cur.write_u8(self.version)?;
            cur.write_array(&self.flags.to_be_bytes())?;

            cur.write_u8(self.stream_structure)?;

            if let Some(channel_layout) = &self.channel_layout {
                match channel_layout {
                    ChannelLayout::Explicit(data) => {
                        cur.write_u8(0)?; // defined_layout = 0 for explicit
                        cur.write_slice(data)?;
                    }
                    ChannelLayout::Predefined {
                        defined_layout,
                        omitted_channels_map,
                    } => {
                        cur.write_u8(*defined_layout)?;
                        cur.write_u64_be(*omitted_channels_map)?;
                    }
                }
            }

            if let Some(object_count) = self.object_count {
                cur.write_u8(object_count)?;
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

    fn raw_data_predefined() -> [u8; 14] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x01, // stream_structure = CHANNEL_STRUCTURED
            0x02, // defined_layout = 2 (predefined)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, // omitted_channels_map = 3
        ]
    }

    fn raw_data_explicit() -> [u8; 7] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x01, // stream_structure = CHANNEL_STRUCTURED
            0x00, // defined_layout = 0 (explicit)
            0x01, // speaker position data (1 byte)
        ]
    }

    #[test]
    fn test_chnl_box_view_decode_predefined() {
        let data = raw_data_predefined();
        let chnl = ChnlBoxView::decode(&data).unwrap();

        assert_eq!(chnl.version, 0);
        assert_eq!(chnl.flags.bits(), 0);
        assert_eq!(chnl.stream_structure, CHANNEL_STRUCTURED);
        match chnl.channel_layout {
            Some(ChannelLayout::Predefined {
                defined_layout,
                omitted_channels_map,
            }) => {
                assert_eq!(defined_layout, 2);
                assert_eq!(omitted_channels_map, 3);
            }
            _ => panic!("expected Predefined"),
        }
        assert!(chnl.object_count.is_none());
    }

    #[test]
    fn test_chnl_box_view_decode_explicit() {
        let data = raw_data_explicit();
        let chnl = ChnlBoxView::decode(&data).unwrap();

        assert_eq!(chnl.stream_structure, CHANNEL_STRUCTURED);
        match chnl.channel_layout {
            Some(ChannelLayout::Explicit(data)) => {
                assert_eq!(data, &[0x01]);
            }
            _ => panic!("expected Explicit"),
        }
    }

    #[test]
    fn test_chnl_box_view_decode_object_structured() {
        let data: [u8; 6] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x02, // stream_structure = OBJECT_STRUCTURED
            0x04, // object_count = 4
        ];
        let chnl = ChnlBoxView::decode(&data).unwrap();

        assert_eq!(chnl.stream_structure, OBJECT_STRUCTURED);
        assert!(chnl.channel_layout.is_none());
        assert_eq!(chnl.object_count, Some(4));
    }

    #[test]
    fn test_chnl_box_view_decode_truncated() {
        let data: [u8; 3] = [0x00; 3];
        let result = ChnlBoxView::decode(&data);
        assert!(result.is_err());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_chnl_box_round_trip_predefined() {
        use crate::BoxEncode;

        let original = raw_data_predefined();
        let chnl = ChnlBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; chnl.encoded_len()];
        chnl.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_chnl_box_round_trip_explicit() {
        use crate::BoxEncode;

        let original = raw_data_explicit();
        let chnl = ChnlBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; chnl.encoded_len()];
        chnl.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_chnl_box_to_owned() {
        let data = raw_data_predefined();
        let view = ChnlBoxView::decode(&data).unwrap();
        let owned = view.to_owned();

        assert_eq!(owned.version, view.version);
        assert_eq!(owned.flags.bits(), view.flags.bits());
        assert_eq!(owned.stream_structure, view.stream_structure);
        assert_eq!(owned.object_count, view.object_count);
    }

    #[test]
    fn test_chnl_box_view_decode_no_flags() {
        let data: [u8; 5] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, // stream_structure = 0 (no flags)
        ];
        let chnl = ChnlBoxView::decode(&data).unwrap();

        assert_eq!(chnl.stream_structure, 0);
        assert!(chnl.channel_layout.is_none());
        assert!(chnl.object_count.is_none());
    }

    #[test]
    fn test_chnl_box_view_decode_both_flags_predefined() {
        let data: [u8; 15] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x03, // stream_structure = CHANNEL_STRUCTURED | OBJECT_STRUCTURED
            0x02, // defined_layout = 2 (predefined)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, // omitted_channels_map = 3
            0x05, // object_count = 5
        ];
        let chnl = ChnlBoxView::decode(&data).unwrap();

        assert_eq!(
            chnl.stream_structure,
            CHANNEL_STRUCTURED | OBJECT_STRUCTURED
        );
        match chnl.channel_layout {
            Some(ChannelLayout::Predefined {
                defined_layout,
                omitted_channels_map,
            }) => {
                assert_eq!(defined_layout, 2);
                assert_eq!(omitted_channels_map, 3);
            }
            _ => panic!("expected Predefined"),
        }
        assert_eq!(chnl.object_count, Some(5));
    }

    #[test]
    fn test_chnl_box_view_decode_both_flags_explicit() {
        let data: [u8; 9] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x03, // stream_structure = CHANNEL_STRUCTURED | OBJECT_STRUCTURED
            0x00, // defined_layout = 0 (explicit)
            0x01, 0x02, // speaker positions (2 channels)
            0x03, // object_count = 3
        ];
        let chnl = ChnlBoxView::decode(&data).unwrap();

        assert_eq!(
            chnl.stream_structure,
            CHANNEL_STRUCTURED | OBJECT_STRUCTURED
        );
        match chnl.channel_layout {
            Some(ChannelLayout::Explicit(data)) => {
                assert_eq!(data, &[0x01, 0x02]); // object_count byte excluded
            }
            _ => panic!("expected Explicit"),
        }
        assert_eq!(chnl.object_count, Some(3));
    }

    #[test]
    fn test_speaker_positions_predefined() {
        let data = raw_data_explicit();
        let chnl = ChnlBoxView::decode(&data).unwrap();

        let layout = chnl.channel_layout.as_ref().unwrap();
        let positions: Vec<_> = layout
            .speaker_positions(1)
            .unwrap()
            .collect::<core::result::Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].position, 0x01);
        assert!(positions[0].azimuth.is_none());
        assert!(positions[0].elevation.is_none());
    }

    #[test]
    fn test_speaker_positions_explicit_position() {
        let data: [u8; 10] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x01, // stream_structure = CHANNEL_STRUCTURED
            0x00, // defined_layout = 0 (explicit)
            126,  // speaker_position = 126 (explicit)
            0x00, 0x5A, // azimuth = 90
            0x2D, // elevation = 45
        ];
        let chnl = ChnlBoxView::decode(&data).unwrap();

        let layout = chnl.channel_layout.as_ref().unwrap();
        let positions: Vec<_> = layout
            .speaker_positions(1)
            .unwrap()
            .collect::<core::result::Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].position, 126);
        assert_eq!(positions[0].azimuth, Some(90));
        assert_eq!(positions[0].elevation, Some(45));
    }

    #[test]
    fn test_speaker_positions_returns_none_for_predefined_layout() {
        let data = raw_data_predefined();
        let chnl = ChnlBoxView::decode(&data).unwrap();

        let layout = chnl.channel_layout.as_ref().unwrap();
        assert!(layout.speaker_positions(2).is_none());
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn test_chnl_box_round_trip_both_flags_explicit() {
        use crate::BoxEncode;

        let original: [u8; 9] = [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x03, // stream_structure = CHANNEL_STRUCTURED | OBJECT_STRUCTURED
            0x00, // defined_layout = 0 (explicit)
            0x01, 0x02, // speaker positions (2 channels)
            0x03, // object_count = 3
        ];
        let chnl = ChnlBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; chnl.encoded_len()];
        chnl.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }
}
