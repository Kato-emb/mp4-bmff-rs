use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxEncode;
use crate::BoxType;
use crate::error::*;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

define_box_flags!(
    /// Flags for the Track Fragment Decode Time Box (`tfdt`).
    TfdtFlags {}
);

/// Track Fragment Decode Time Box (`tfdt`).
#[derive(Debug, Clone, Copy)]
pub struct TfdtBox {
    /// The version of the box.
    pub version: u8,
    /// The flags of the box.
    pub flags: TfdtFlags,
    /// The base media decode time.
    pub base_media_decode_time: u64,
}

impl BoxCodec for TfdtBox {
    fn boxtype(&self) -> BoxType {
        BoxType::TFDT
    }
}

impl BoxDecode<'_> for TfdtBox {
    fn decode(bytes: &'_ [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = TfdtFlags::from_be_bytes(cur.read_array::<3>()?);

        let base_media_decode_time = match version {
            0 => cur.read_u32_be()? as u64,
            1 => cur.read_u64_be()?,
            _ => {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxVersion {
                        reason: "0 or 1 in this specification",
                        got: version,
                    },
                    BoxType::TFDT,
                ));
            }
        };

        Ok(TfdtBox {
            version,
            flags,
            base_media_decode_time,
        })
    }
}

impl BoxEncode for TfdtBox {
    #[inline]
    fn encoded_len(&self) -> usize {
        1 // version
            + 3 // flags
            + match self.version {
                0 => 4, // base_media_decode_time (u32)
                1 => 8, // base_media_decode_time (u64)
                _ => 0, // invalid version, but encoded_len should not fail
            }
    }

    fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
        let mut cur = WriteCursor::new(bytes);

        cur.write_u8(self.version)?;
        cur.write_array(&self.flags.to_be_bytes())?;

        match self.version {
            0 => cur.write_u32_be(self.base_media_decode_time as u32)?,
            1 => cur.write_u64_be(self.base_media_decode_time)?,
            _ => {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxVersion {
                        reason: "0 or 1 in this specification",
                        got: self.version,
                    },
                    BoxType::TFDT,
                ));
            }
        }

        Ok(cur.position())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw_data_v0() -> [u8; 8] {
        [
            0x00, // version = 0
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x01, 0x51, 0x80, // base_media_decode_time = 86400
        ]
    }

    fn raw_data_v1() -> [u8; 12] {
        [
            0x01, // version = 1
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x01, // base_media_decode_time (high 32 bits)
            0x00, 0x00, 0x00, 0x00, // base_media_decode_time (low 32 bits) = 0x100000000
        ]
    }

    #[test]
    fn test_tfdt_box_decode_v0() {
        let data = raw_data_v0();
        let tfdt = TfdtBox::decode(&data).unwrap();

        assert_eq!(tfdt.version, 0);
        assert_eq!(tfdt.flags.bits(), 0);
        assert_eq!(tfdt.base_media_decode_time, 86400);
    }

    #[test]
    fn test_tfdt_box_decode_v1() {
        let data = raw_data_v1();
        let tfdt = TfdtBox::decode(&data).unwrap();

        assert_eq!(tfdt.version, 1);
        assert_eq!(tfdt.flags.bits(), 0);
        assert_eq!(tfdt.base_media_decode_time, 0x100000000);
    }

    #[test]
    fn test_tfdt_box_decode_invalid_version() {
        let data: [u8; 8] = [
            0x02, // version = 2 (invalid)
            0x00, 0x00, 0x00, // flags
            0x00, 0x00, 0x00, 0x00, // base_media_decode_time
        ];

        let result = TfdtBox::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_tfdt_box_decode_truncated() {
        let data: [u8; 4] = [0x00; 4];
        let result = TfdtBox::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_tfdt_box_round_trip_v0() {
        let original = raw_data_v0();
        let tfdt = TfdtBox::decode(&original).unwrap();

        let mut encoded = [0u8; 8];
        let len = tfdt.encode_into(&mut encoded).unwrap();

        assert_eq!(len, original.len());
        assert_eq!(&encoded[..], &original[..]);
    }

    #[test]
    fn test_tfdt_box_round_trip_v1() {
        let original = raw_data_v1();
        let tfdt = TfdtBox::decode(&original).unwrap();

        let mut encoded = [0u8; 12];
        let len = tfdt.encode_into(&mut encoded).unwrap();

        assert_eq!(len, original.len());
        assert_eq!(&encoded[..], &original[..]);
    }

    #[test]
    fn test_tfdt_box_encode_invalid_version() {
        let tfdt = TfdtBox {
            version: 2,
            flags: TfdtFlags::empty(),
            base_media_decode_time: 0,
        };

        let mut encoded = [0u8; 12];
        let result = tfdt.encode_into(&mut encoded);
        assert!(result.is_err());
    }
}
