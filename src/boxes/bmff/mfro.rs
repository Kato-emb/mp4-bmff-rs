use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxEncode;
use crate::BoxType;
use crate::error::*;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

define_box_flags!(
    /// Flags for the Movie Fragment Random Access Offset Box (`mfro`).
    MfroFlags {}
);

/// Movie Fragment Random Access Offset Box (`mfro`).
#[derive(Debug, Clone, Copy)]
pub struct MfroBox {
    /// The version of the box.
    pub version: u8,
    /// The flags of the box.
    pub flags: MfroFlags,
    /// The size of an integer gives the number of bytes of the enclosing `mfra` box.
    pub size: u32,
}

impl BoxCodec for MfroBox {
    fn boxtype(&self) -> BoxType {
        BoxType::MFRO
    }
}

impl BoxDecode<'_> for MfroBox {
    fn decode(bytes: &'_ [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = MfroFlags::from_be_bytes(cur.read_array::<3>()?);

        let size = cur.read_u32_be()?;

        Ok(MfroBox {
            version,
            flags,
            size,
        })
    }
}

impl BoxEncode for MfroBox {
    #[inline]
    fn encoded_len(&self) -> usize {
        1 // version
            + 3 // flags
            + 4 // size
    }

    fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
        let mut cur = WriteCursor::new(bytes);

        cur.write_u8(self.version)?;
        cur.write_array(&self.flags.to_be_bytes())?;

        cur.write_u32_be(self.size)?;

        Ok(cur.position())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_data() -> [u8; 8] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x10, 0x00, // size = 4096
        ]
    }

    #[test]
    fn test_mfro_box_decode() {
        let data = raw_data();
        let mfro = MfroBox::decode(&data).unwrap();

        assert_eq!(mfro.version, 0);
        assert_eq!(mfro.flags.bits(), 0);
        assert_eq!(mfro.size, 4096);
    }

    #[test]
    fn test_mfro_box_decode_truncated() {
        let data: [u8; 4] = [0x00; 4];
        let result = MfroBox::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_mfro_box_round_trip() {
        let original = raw_data();
        let mfro = MfroBox::decode(&original).unwrap();

        let mut encoded = [0u8; 8];
        let len = mfro.encode_into(&mut encoded).unwrap();

        assert_eq!(len, original.len());
        assert_eq!(&encoded[..], &original[..]);
    }
}
