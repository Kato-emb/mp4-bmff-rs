//! Sound Media Header Box (`smhd`) implementation.
//!
//! The Sound Media Header Box contains general presentation information
//! independent of the audio's coding. It is used for audio tracks to
//! specify stereo balance.

use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxEncode;
use crate::BoxType;
use crate::error::*;
use crate::types::I8F8;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

define_box_flags!(
    /// Flags for the Sound Media Header Box (`smhd`).
    ///
    /// Reserved (should be 0).
    SmhdFlags {}
);

/// Sound Media Header Box (`smhd`).
///
/// Contains information about audio presentation, specifically the stereo
/// balance. Present in audio tracks within the Media Information Box.
///
/// # Structure
///
/// - `version`: Box version (should be 0).
/// - `flags`: Reserved (should be 0).
/// - `balance`: Stereo balance as signed 8.8 fixed-point (0 = center, negative = left, positive = right).
#[derive(Debug, Clone, Copy, Default)]
pub struct SmhdBox {
    /// Box version.
    pub version: u8,
    /// Box flags.
    pub flags: SmhdFlags,
    /// Stereo balance as signed 8.8 fixed-point (0.0 = center, negative = left, positive = right).
    pub balance: I8F8,
}

const RESERVED: usize = 2;

impl BoxCodec for SmhdBox {
    fn boxtype(&self) -> BoxType {
        BoxType::SMHD
    }
}

impl BoxDecode<'_> for SmhdBox {
    fn decode(bytes: &[u8]) -> Result<Self> {
        let mut cursor = ReadCursor::new(bytes);

        let version = cursor.read_u8()?;
        let flags = SmhdFlags::from_be_bytes(cursor.read_array::<3>()?);

        let balance = I8F8::from_raw(cursor.read_i16_be()?);
        cursor.advance(RESERVED)?;

        Ok(SmhdBox {
            version,
            flags,
            balance,
        })
    }
}

impl BoxEncode for SmhdBox {
    #[inline]
    fn encoded_len(&self) -> usize {
        4 // version + flags
        + 2 // balance
        + RESERVED // reserved
    }

    fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
        let mut cursor = WriteCursor::new(bytes);

        cursor.write_u8(self.version)?;
        cursor.write_array(&self.flags.to_be_bytes())?;

        cursor.write_i16_be(self.balance.to_raw())?;

        cursor.reserve_zeros(RESERVED)?;

        Ok(cursor.position())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_data() -> [u8; 8] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0xFF, 0x00, // balance = -256 (left pan)
            0x00, 0x00, // reserved
        ]
    }

    #[test]
    fn test_smhd_box_decode() {
        let data = raw_data();
        let smhd = SmhdBox::decode(&data).unwrap();

        assert_eq!(smhd.version, 0);
        assert_eq!(smhd.flags.bits(), 0);
        assert_eq!(smhd.balance, I8F8::from_raw(-256)); // -1.0 (left pan)
    }

    #[test]
    fn test_smhd_box_decode_truncated() {
        let data: [u8; 4] = [0x00; 4];
        let result = SmhdBox::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_smhd_box_round_trip() {
        let original = raw_data();
        let smhd = SmhdBox::decode(&original).unwrap();

        let mut encoded = [0u8; 8];
        let len = smhd.encode_into(&mut encoded).unwrap();

        assert_eq!(len, original.len());
        assert_eq!(&encoded[..], &original[..]);
    }
}
