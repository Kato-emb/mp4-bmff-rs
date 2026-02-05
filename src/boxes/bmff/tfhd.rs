//! Track Fragment Header Box (`tfhd`) implementation.
//!
//! The Track Fragment Header Box identifies the track this fragment belongs to
//! and provides default values for sample properties. These defaults can be
//! overridden by individual Track Run (`trun`) boxes.
//!
//! This box is required within every Track Fragment Box (`traf`).

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxEncode;
use crate::BoxType;
use crate::error::*;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

define_box_flags!(
    /// Flags for the Track Fragment Header Box (`tfhd`).
    ///
    /// Control which optional fields are present in the box.
    TfhdFlags {
        /// Base data offset field is present (8 bytes).
        BASE_DATA_OFFSET_PRESENT = 0x000001,
        /// Sample description index field is present (4 bytes).
        SAMPLE_DESCRIPTION_INDEX_PRESENT = 0x000002,
        /// Default sample duration field is present (4 bytes).
        DEFAULT_SAMPLE_DURATION_PRESENT = 0x000008,
        /// Default sample size field is present (4 bytes).
        DEFAULT_SAMPLE_SIZE_PRESENT = 0x000010,
        /// Default sample flags field is present (4 bytes).
        DEFAULT_SAMPLE_FLAGS_PRESENT = 0x000020,
        /// This fragment has zero duration (empty edit).
        DURATION_IS_EMPTY = 0x010000,
        /// Base offset is the start of the enclosing `moof` box.
        DEFAULT_BASE_IS_MOOF = 0x020000,
    }
);

/// Track Fragment Header Box (`tfhd`).
///
/// Identifies the track and provides default sample properties for this
/// fragment. Fields are present based on flag bits.
///
/// # Structure
///
/// - `version`: Box version (should be 0).
/// - `flags`: Indicate which optional fields are present.
/// - `track_id`: Identifies which track this fragment belongs to.
/// - `base_data_offset`: Byte offset of media data (when flag set).
/// - `sample_description_index`: Index into sample description table.
/// - `default_sample_duration`: Default duration for samples in this fragment.
/// - `default_sample_size`: Default size for samples in this fragment.
/// - `default_sample_flags`: Default flags for samples in this fragment.
#[derive(Debug, Clone, Copy)]
pub struct TfhdBox {
    /// Box version (should be 0).
    pub version: u8,
    /// Flags indicating which optional fields are present.
    pub flags: TfhdFlags,
    /// Track ID this fragment belongs to.
    pub track_id: u32,
    /// Base offset for media data in the associated `mdat` box.
    pub base_data_offset: Option<u64>,
    /// Index into sample description table for this fragment.
    pub sample_description_index: Option<u32>,
    /// Default sample duration in timescale units.
    pub default_sample_duration: Option<u32>,
    /// Default sample size in bytes.
    pub default_sample_size: Option<u32>,
    /// Default sample flags (sync, dependency info, etc.).
    pub default_sample_flags: Option<u32>,
}

impl BoxCodec for TfhdBox {
    fn boxtype(&self) -> BoxType {
        BoxType::TFHD
    }
}

impl BoxDecode<'_> for TfhdBox {
    fn decode(bytes: &'_ [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = TfhdFlags::from_be_bytes(cur.read_array::<3>()?);

        let track_id = cur.read_u32_be()?;

        let base_data_offset = if flags.contains(TfhdFlags::BASE_DATA_OFFSET_PRESENT) {
            Some(cur.read_u64_be()?)
        } else {
            None
        };

        let sample_description_index =
            if flags.contains(TfhdFlags::SAMPLE_DESCRIPTION_INDEX_PRESENT) {
                Some(cur.read_u32_be()?)
            } else {
                None
            };

        let default_sample_duration = if flags.contains(TfhdFlags::DEFAULT_SAMPLE_DURATION_PRESENT)
        {
            Some(cur.read_u32_be()?)
        } else {
            None
        };

        let default_sample_size = if flags.contains(TfhdFlags::DEFAULT_SAMPLE_SIZE_PRESENT) {
            Some(cur.read_u32_be()?)
        } else {
            None
        };

        let default_sample_flags = if flags.contains(TfhdFlags::DEFAULT_SAMPLE_FLAGS_PRESENT) {
            Some(cur.read_u32_be()?)
        } else {
            None
        };

        Ok(TfhdBox {
            version,
            flags,
            track_id,
            base_data_offset,
            sample_description_index,
            default_sample_duration,
            default_sample_size,
            default_sample_flags,
        })
    }
}

impl BoxEncode for TfhdBox {
    #[inline]
    fn encoded_len(&self) -> usize {
        1 // version
            + 3 // flags
            + 4 // track_id
            + if self.flags.contains(TfhdFlags::BASE_DATA_OFFSET_PRESENT) {
                8 // base_data_offset
            } else {
                0
            }
            + if self.flags.contains(TfhdFlags::SAMPLE_DESCRIPTION_INDEX_PRESENT) {
                4 // sample_description_index
            } else {
                0
            }
            + if self.flags.contains(TfhdFlags::DEFAULT_SAMPLE_DURATION_PRESENT) {
                4 // default_sample_duration
            } else {
                0
            }
            + if self.flags.contains(TfhdFlags::DEFAULT_SAMPLE_SIZE_PRESENT) {
                4 // default_sample_size
            } else {
                0
            }
            + if self.flags.contains(TfhdFlags::DEFAULT_SAMPLE_FLAGS_PRESENT) {
                4 // default_sample_flags
            } else {
                0
            }
    }

    fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
        let mut cur = WriteCursor::new(bytes);

        cur.write_u8(self.version)?;
        cur.write_array(&self.flags.to_be_bytes())?;
        cur.write_u32_be(self.track_id)?;

        if let Some(base_data_offset) = self.base_data_offset {
            cur.write_u64_be(base_data_offset)?;
        }

        if let Some(sample_description_index) = self.sample_description_index {
            cur.write_u32_be(sample_description_index)?;
        }

        if let Some(default_sample_duration) = self.default_sample_duration {
            cur.write_u32_be(default_sample_duration)?;
        }

        if let Some(default_sample_size) = self.default_sample_size {
            cur.write_u32_be(default_sample_size)?;
        }

        if let Some(default_sample_flags) = self.default_sample_flags {
            cur.write_u32_be(default_sample_flags)?;
        }

        Ok(cur.position())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_data_minimal() -> [u8; 8] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x01, // track_id = 1
        ]
    }

    fn raw_data_all_optional() -> [u8; 32] {
        [
            0x00, // version = 0
            0x00, 0x00,
            0x3B, // flags = BASE_DATA_OFFSET | SAMPLE_DESC_INDEX | DURATION | SIZE | FLAGS
            0x00, 0x00, 0x00, 0x01, // track_id = 1
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, // base_data_offset = 4096
            0x00, 0x00, 0x00, 0x01, // sample_description_index = 1
            0x00, 0x00, 0x03, 0xE8, // default_sample_duration = 1000
            0x00, 0x00, 0x04, 0x00, // default_sample_size = 1024
            0x00, 0x01, 0x00, 0x00, // default_sample_flags = 0x00010000
        ]
    }

    #[test]
    fn test_tfhd_box_decode_minimal() {
        let data = raw_data_minimal();
        let tfhd = TfhdBox::decode(&data).unwrap();

        assert_eq!(tfhd.version, 0);
        assert_eq!(tfhd.flags.bits(), 0);
        assert_eq!(tfhd.track_id, 1);
        assert!(tfhd.base_data_offset.is_none());
        assert!(tfhd.sample_description_index.is_none());
        assert!(tfhd.default_sample_duration.is_none());
        assert!(tfhd.default_sample_size.is_none());
        assert!(tfhd.default_sample_flags.is_none());
    }

    #[test]
    fn test_tfhd_box_decode_all_optional() {
        let data = raw_data_all_optional();
        let tfhd = TfhdBox::decode(&data).unwrap();

        assert_eq!(tfhd.version, 0);
        assert_eq!(tfhd.track_id, 1);
        assert_eq!(tfhd.base_data_offset, Some(4096));
        assert_eq!(tfhd.sample_description_index, Some(1));
        assert_eq!(tfhd.default_sample_duration, Some(1000));
        assert_eq!(tfhd.default_sample_size, Some(1024));
        assert_eq!(tfhd.default_sample_flags, Some(0x00010000));
    }

    #[test]
    fn test_tfhd_box_decode_truncated() {
        let data: [u8; 4] = [0x00; 4];
        let result = TfhdBox::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_tfhd_box_round_trip_minimal() {
        let original = raw_data_minimal();
        let tfhd = TfhdBox::decode(&original).unwrap();

        let mut encoded = [0u8; 8];
        let len = tfhd.encode_into(&mut encoded).unwrap();

        assert_eq!(len, original.len());
        assert_eq!(&encoded[..], &original[..]);
    }

    #[test]
    fn test_tfhd_box_round_trip_all_optional() {
        let original = raw_data_all_optional();
        let tfhd = TfhdBox::decode(&original).unwrap();

        let mut encoded = [0u8; 32];
        let len = tfhd.encode_into(&mut encoded).unwrap();

        assert_eq!(len, original.len());
        assert_eq!(&encoded[..], &original[..]);
    }
}
