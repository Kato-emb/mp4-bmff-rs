use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxEncode;
use crate::BoxType;
use crate::error::*;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

define_box_flags!(
    /// Flags for Null Media Header Box (`nmhd`).
    NmhdFlags {}
);

/// A Null Media Header Box (`nmhd`).
#[derive(Debug, Clone, Copy)]
pub struct NmhdBox {
    /// Box version (should be 0).
    pub version: u8,
    /// Box flags (should be 0).
    pub flags: NmhdFlags,
}

impl BoxCodec for NmhdBox {
    fn boxtype(&self) -> BoxType {
        BoxType::NMHD
    }
}

impl BoxDecode<'_> for NmhdBox {
    fn decode(bytes: &[u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = NmhdFlags::from_be_bytes(cur.read_array::<3>()?);

        Ok(NmhdBox { version, flags })
    }
}

impl BoxEncode for NmhdBox {
    #[inline]
    fn encoded_len(&self) -> usize {
        4 // version(1) + flags(3)
    }

    fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
        let mut cur = WriteCursor::new(bytes);

        cur.write_u8(self.version)?;
        cur.write_array(&self.flags.to_be_bytes())?;

        Ok(cur.position())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_data() -> [u8; 4] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags (3 bytes)
        ]
    }

    #[test]
    fn test_nmhd_box_decode() {
        let data = raw_data();
        let nmhd = NmhdBox::decode(&data).unwrap();

        assert_eq!(nmhd.version, 0);
        assert_eq!(nmhd.flags.bits(), 0);
    }

    #[test]
    fn test_nmhd_box_truncated() {
        let data: [u8; 2] = [0x00, 0x00]; // too short

        let result = NmhdBox::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_nmhd_box_round_trip() {
        let original = raw_data();
        let nmhd = NmhdBox::decode(&original).unwrap();

        let mut encoded = [0u8; 4];
        nmhd.encode_into(&mut encoded).unwrap();

        assert_eq!(encoded, original);
    }
}
