//! Track Extends Defaults Box (`trex`) implementation.
//!
//! The Track Extends Box provides default values for sample properties
//! in movie fragments. These defaults reduce redundancy in fragment headers
//! by allowing fragments to inherit values rather than specifying them
//! repeatedly.
//!
//! One `trex` box is required for each track that may be extended by
//! movie fragments. This box resides within the Movie Extends Box (`mvex`).

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxEncode;
use crate::BoxType;
use crate::error::*;

use super::common::SampleFlags;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

define_box_flags!(
    /// Flags for the Track Extends Defaults Box (`trex`).
    ///
    /// Reserved (should be 0).
    TrexFlags {}
);

/// Track Extends Defaults Box (`trex`).
///
/// Provides default sample properties for a track's movie fragments.
/// Values here are used when not overridden by track fragment headers
/// or track run boxes.
///
/// # Structure
///
/// - `version`: Box version (should be 0).
/// - `flags`: Reserved (should be 0).
/// - `track_id`: Identifies which track these defaults apply to.
/// - `default_sample_description_index`: Default sample description.
/// - `default_sample_duration`: Default sample duration in timescale units.
/// - `default_sample_size`: Default sample size in bytes.
/// - `default_sample_flags`: Default sample flags (sync, depends_on, etc.).
///
/// # Example
///
/// ```
/// use mp4_bmff::BoxDecode;
/// use mp4_bmff::boxes::bmff::TrexBox;
///
/// let data: [u8; 24] = [
///     0x00,                   // version = 0
///     0x00, 0x00, 0x00,       // flags
///     0x00, 0x00, 0x00, 0x01, // track_id = 1
///     0x00, 0x00, 0x00, 0x01, // default_sample_description_index = 1
///     0x00, 0x00, 0x03, 0xE8, // default_sample_duration = 1000
///     0x00, 0x00, 0x00, 0x00, // default_sample_size = 0
///     0x00, 0x01, 0x00, 0x00, // default_sample_flags
/// ];
///
/// let trex = TrexBox::decode(&data).unwrap();
/// assert_eq!(trex.track_id, 1);
/// assert_eq!(trex.default_sample_duration, 1000);
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct TrexBox {
    /// Box version (should be 0).
    pub version: u8,
    /// Reserved flags (should be 0).
    pub flags: TrexFlags,
    /// Track ID this box applies to.
    pub track_id: u32,
    /// Default index into sample description table.
    pub default_sample_description_index: u32,
    /// Default sample duration in timescale units.
    pub default_sample_duration: u32,
    /// Default sample size in bytes.
    pub default_sample_size: u32,
    /// Default sample flags (sync, dependency info, etc.).
    pub default_sample_flags: SampleFlags,
}

impl BoxCodec for TrexBox {
    fn boxtype(&self) -> BoxType {
        BoxType::TREX
    }
}

impl BoxDecode<'_> for TrexBox {
    fn decode(bytes: &'_ [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = TrexFlags::from_be_bytes(cur.read_array::<3>()?);

        let track_id = cur.read_u32_be()?;
        let default_sample_description_index = cur.read_u32_be()?;
        let default_sample_duration = cur.read_u32_be()?;
        let default_sample_size = cur.read_u32_be()?;
        let default_sample_flags = SampleFlags::from_raw(cur.read_u32_be()?);

        Ok(TrexBox {
            version,
            flags,
            track_id,
            default_sample_description_index,
            default_sample_duration,
            default_sample_size,
            default_sample_flags,
        })
    }
}

impl BoxEncode for TrexBox {
    #[inline]
    fn encoded_len(&self) -> usize {
        1 // version
            + 3 // flags
            + 4 // track_id
            + 4 // default_sample_description_index
            + 4 // default_sample_duration
            + 4 // default_sample_size
            + 4 // default_sample_flags
    }

    fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
        let mut cur = WriteCursor::new(bytes);

        cur.write_u8(self.version)?;
        cur.write_array(&self.flags.to_be_bytes())?;
        cur.write_u32_be(self.track_id)?;
        cur.write_u32_be(self.default_sample_description_index)?;
        cur.write_u32_be(self.default_sample_duration)?;
        cur.write_u32_be(self.default_sample_size)?;
        cur.write_u32_be(self.default_sample_flags.to_raw())?;

        Ok(cur.position())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_data() -> [u8; 24] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x01, // track_id = 1
            0x00, 0x00, 0x00, 0x01, // default_sample_description_index = 1
            0x00, 0x00, 0x03, 0xE8, // default_sample_duration = 1000
            0x00, 0x00, 0x00, 0x00, // default_sample_size = 0
            0x00, 0x01, 0x00, 0x00, // default_sample_flags = 0x00010000
        ]
    }

    #[test]
    fn test_trex_box_decode() {
        let data = raw_data();
        let trex = TrexBox::decode(&data).unwrap();

        assert_eq!(trex.version, 0);
        assert_eq!(trex.flags.bits(), 0);
        assert_eq!(trex.track_id, 1);
        assert_eq!(trex.default_sample_description_index, 1);
        assert_eq!(trex.default_sample_duration, 1000);
        assert_eq!(trex.default_sample_size, 0);
        assert_eq!(trex.default_sample_flags.to_raw(), 0x00010000);
    }

    #[test]
    fn test_trex_box_decode_truncated() {
        let data: [u8; 12] = [0x00; 12];
        let result = TrexBox::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_trex_box_round_trip() {
        let original = raw_data();
        let trex = TrexBox::decode(&original).unwrap();

        let mut encoded = [0u8; 24];
        let len = trex.encode_into(&mut encoded).unwrap();

        assert_eq!(len, original.len());
        assert_eq!(&encoded[..], &original[..]);
    }
}
