use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxEncode;
use crate::BoxType;
use crate::error::*;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

define_box_flags!(
    /// Flags for the Track Extends Defaults Box (`trex`).
    TrexFlags {}
);

/// Track Extends Defaults Box (`trex`).
#[derive(Debug, Clone, Copy)]
pub struct TrexBox {
    /// The version of the box.
    pub version: u8,
    /// The flags of the box.
    pub flags: TrexFlags,
    /// The track ID.
    pub track_id: u32,
    /// The default sample description index.
    pub default_sample_description_index: u32,
    /// The default sample duration.
    pub default_sample_duration: u32,
    /// The default sample size.
    pub default_sample_size: u32,
    /// The default sample flags.
    pub default_sample_flags: u32,
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
        let default_sample_flags = cur.read_u32_be()?;

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
        cur.write_u32_be(self.default_sample_flags)?;

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
        assert_eq!(trex.default_sample_flags, 0x00010000);
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
