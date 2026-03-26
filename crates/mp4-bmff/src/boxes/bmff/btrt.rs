//! Bit Rate Box (`btrt`) implementation.
//!
//! The Bit Rate Box signals the bit rate information of a stream.
//! It may be present within any sample entry, and is explicitly
//! referenced by metadata sample entries (`metx`, `mett`, `urim`)
//! and NAL-based codec sample entries (`avc1`, `hvc1`).

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxEncode;
use crate::BoxType;
use crate::error::*;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

/// Bit Rate Box (`btrt`).
///
/// Signals the bit rate information of a stream contained in a sample entry.
/// This box provides the decoding buffer size and the maximum and average
/// bit rates, allowing players to allocate appropriate resources.
///
/// # Structure
///
/// - `buffer_size_db`: Size of the decoding buffer for the elementary stream in bytes.
/// - `max_bitrate`: Maximum rate in bits/second over any window of one second.
/// - `avg_bitrate`: Average rate in bits/second over the entire presentation.
#[derive(Debug, Clone, Copy)]
pub struct BtrtBox {
    /// Size of the decoding buffer for the elementary stream in bytes.
    pub buffer_size_db: u32,
    /// Maximum rate in bits/second over any window of one second.
    pub max_bitrate: u32,
    /// Average rate in bits/second over the entire presentation.
    pub avg_bitrate: u32,
}

impl BoxCodec for BtrtBox {
    fn boxtype(&self) -> BoxType {
        BoxType::BTRT
    }
}

impl<'de> BoxDecode<'de> for BtrtBox {
    fn decode(bytes: &'de [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let buffer_size_db = cur.read_u32_be()?;
        let max_bitrate = cur.read_u32_be()?;
        let avg_bitrate = cur.read_u32_be()?;

        Ok(BtrtBox {
            buffer_size_db,
            max_bitrate,
            avg_bitrate,
        })
    }
}

impl BoxEncode for BtrtBox {
    fn encoded_len(&self) -> usize {
        12
    }

    fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
        let mut cur = WriteCursor::new(bytes);

        cur.write_u32_be(self.buffer_size_db)?;
        cur.write_u32_be(self.max_bitrate)?;
        cur.write_u32_be(self.avg_bitrate)?;

        Ok(cur.position())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_data() -> [u8; 12] {
        [
            0x00, 0x01, 0x00, 0x00, // buffer_size_db = 65536
            0x00, 0x1E, 0x84, 0x80, // max_bitrate = 2000000
            0x00, 0x0F, 0x42, 0x40, // avg_bitrate = 1000000
        ]
    }

    #[test]
    fn test_btrt_box_decode() {
        let data = raw_data();
        let btrt = BtrtBox::decode(&data).unwrap();

        assert_eq!(btrt.buffer_size_db, 65536);
        assert_eq!(btrt.max_bitrate, 2_000_000);
        assert_eq!(btrt.avg_bitrate, 1_000_000);
    }

    #[test]
    fn test_btrt_box_decode_truncated() {
        let data: [u8; 8] = [0x00; 8];
        let result = BtrtBox::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_btrt_box_round_trip() {
        let original = raw_data();
        let btrt = BtrtBox::decode(&original).unwrap();

        let mut encoded = [0u8; 12];
        let len = btrt.encode_into(&mut encoded).unwrap();

        assert_eq!(len, original.len());
        assert_eq!(&encoded[..], &original[..]);
    }
}
