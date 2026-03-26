use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxEncode;
use crate::BoxType;
use crate::error::*;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

define_box_flags!(
    /// Flags for the Hint Media Header Box (`hmhd`).
    ///
    /// The default value is 0.
    HmhdFlags {}
);

/// Hint Media Header Box (`hmhd`).
#[derive(Debug, Clone, Copy, Default)]
pub struct HmhdBox {
    /// Box version.
    pub version: u8,
    /// Box flags.
    pub flags: HmhdFlags,
    /// Maximum bitrate in bits per second.
    pub max_pdu_size: u16,
    /// Average bitrate in bits per second.
    pub avg_pdu_size: u16,
    /// Maximum number of bytes in any network packet.
    pub max_bit_rate: u32,
    /// Average number of bytes in network packets.
    pub avg_bit_rate: u32,
}

impl HmhdBox {
    const RESERVED: usize = 4; // reserved (4 bytes)
}

impl BoxCodec for HmhdBox {
    fn boxtype(&self) -> BoxType {
        BoxType::HMHD
    }
}

impl BoxDecode<'_> for HmhdBox {
    fn decode(bytes: &'_ [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = HmhdFlags::from_be_bytes(cur.read_array::<3>()?);

        let max_pdu_size = cur.read_u16_be()?;
        let avg_pdu_size = cur.read_u16_be()?;
        let max_bit_rate = cur.read_u32_be()?;
        let avg_bit_rate = cur.read_u32_be()?;

        cur.advance(Self::RESERVED)?; // skip reserved

        Ok(HmhdBox {
            version,
            flags,
            max_pdu_size,
            avg_pdu_size,
            max_bit_rate,
            avg_bit_rate,
        })
    }
}

impl BoxEncode for HmhdBox {
    #[inline]
    fn encoded_len(&self) -> usize {
        4 // version (1) + flags (3)
        + 2 // max_pdu_size
        + 2 // avg_pdu_size
        + 4 // max_bit_rate
        + 4 // avg_bit_rate
        + Self::RESERVED // reserved (4 bytes)
    }

    fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
        let mut cur = WriteCursor::new(bytes);

        cur.write_u8(self.version)?;
        cur.write_array(&self.flags.to_be_bytes())?;

        cur.write_u16_be(self.max_pdu_size)?;
        cur.write_u16_be(self.avg_pdu_size)?;
        cur.write_u32_be(self.max_bit_rate)?;
        cur.write_u32_be(self.avg_bit_rate)?;

        cur.reserve_zeros(Self::RESERVED)?; // write reserved as zeros

        Ok(cur.position())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_data() -> [u8; 20] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags (3 bytes)
            0x00, 0x10, // max_pdu_size = 16 (2 bytes)
            0x00, 0x08, // avg_pdu_size = 8 (2 bytes)
            0x00, 0x01, 0x00, 0x00, // max_bit_rate = 65536 (4 bytes)
            0x00, 0x00, 0x80, 0x00, // avg_bit_rate = 32768 (4 bytes)
            0x00, 0x00, 0x00, 0x00, // reserved (4 bytes)
        ]
    }

    #[test]
    fn test_hmhd_box_decode() {
        let data = raw_data();
        let hmhd = HmhdBox::decode(&data).unwrap();

        assert_eq!(hmhd.version, 0);
        assert_eq!(hmhd.flags.bits(), 0);
        assert_eq!(hmhd.max_pdu_size, 16);
        assert_eq!(hmhd.avg_pdu_size, 8);
        assert_eq!(hmhd.max_bit_rate, 65536);
        assert_eq!(hmhd.avg_bit_rate, 32768);
    }

    #[test]
    fn test_hmhd_box_truncated() {
        let data: [u8; 10] = [0x00; 10]; // too short

        let result = HmhdBox::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_hmhd_box_round_trip() {
        let original = raw_data();
        let hmhd = HmhdBox::decode(&original).unwrap();

        let mut encoded = vec![0u8; hmhd.encoded_len()];
        hmhd.encode_into(&mut encoded).unwrap();

        assert_eq!(&encoded[..], &original[..]);
    }
}
