use crate::BoxCodec;
use crate::BoxDecode;
use crate::BoxEncode;
use crate::BoxType;
use crate::error::*;

use crate::cursor::ReadCursor;
use crate::cursor::WriteCursor;

define_box_flags!(
    /// Flags for the Movie Extends Header Box (`mehd`).
    MehdFlags {}
);

/// Movie Extends Header Box (`mehd`).
#[derive(Debug, Clone, Copy)]
pub struct MehdBox {
    /// The version of the box.
    pub version: u8,
    /// The flags of the box.
    pub flags: MehdFlags,
    /// The fragment duration.
    pub fragment_duration: u64,
}

impl BoxCodec for MehdBox {
    fn boxtype(&self) -> BoxType {
        BoxType::MEHD
    }
}

impl BoxDecode<'_> for MehdBox {
    fn decode(bytes: &'_ [u8]) -> Result<Self> {
        let mut cur = ReadCursor::new(bytes);

        let version = cur.read_u8()?;
        let flags = MehdFlags::from_be_bytes(cur.read_array::<3>()?);

        let fragment_duration: u64;
        match version {
            0 => fragment_duration = cur.read_u32_be()? as u64,
            1 => fragment_duration = cur.read_u64_be()?,
            _ => {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxVersion {
                        reason: "0 or 1 in this specification",
                        got: version,
                    },
                    BoxType::MEHD,
                ));
            }
        }

        Ok(MehdBox {
            version,
            flags,
            fragment_duration,
        })
    }
}

impl BoxEncode for MehdBox {
    fn encoded_len(&self) -> usize {
        1 // version
            + 3 // flags
            + match self.version {
                0 => 4, // fragment_duration (32 bits)
                1 => 8, // fragment_duration (64 bits)
                _ => 0,
            }
    }

    fn encode_into(&self, bytes: &mut [u8]) -> Result<usize> {
        let mut cur = WriteCursor::new(bytes);

        cur.write_u8(self.version)?;
        cur.write_array(&self.flags.to_be_bytes())?;

        match self.version {
            0 => cur.write_u32_be(self.fragment_duration as u32)?,
            1 => cur.write_u64_be(self.fragment_duration)?,
            _ => {
                return Err(Error::in_box(
                    ErrorKind::InvalidBoxVersion {
                        reason: "0 or 1 in this specification",
                        got: self.version,
                    },
                    BoxType::MEHD,
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
            0x00, 0x01, 0x51, 0x80, // fragment_duration = 86400 (1 day in seconds)
        ]
    }

    fn raw_data_v1() -> [u8; 12] {
        [
            0x01, // version = 1
            0x00, 0x00, 0x00, // flags = 0
            0x00, 0x00, 0x00, 0x01, // fragment_duration (high 32 bits)
            0x00, 0x00, 0x00, 0x00, // fragment_duration (low 32 bits) = 0x100000000
        ]
    }

    #[test]
    fn test_mehd_box_decode_v0() {
        let data = raw_data_v0();
        let mehd = MehdBox::decode(&data).unwrap();

        assert_eq!(mehd.version, 0);
        assert_eq!(mehd.flags.bits(), 0);
        assert_eq!(mehd.fragment_duration, 86400);
    }

    #[test]
    fn test_mehd_box_decode_v1() {
        let data = raw_data_v1();
        let mehd = MehdBox::decode(&data).unwrap();

        assert_eq!(mehd.version, 1);
        assert_eq!(mehd.flags.bits(), 0);
        assert_eq!(mehd.fragment_duration, 0x100000000);
    }

    #[test]
    fn test_mehd_box_decode_invalid_version() {
        let data: [u8; 8] = [
            0x02, // version = 2 (invalid)
            0x00, 0x00, 0x00, // flags
            0x00, 0x00, 0x00, 0x00, // fragment_duration
        ];

        let result = MehdBox::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_mehd_box_decode_truncated() {
        let data: [u8; 4] = [0x00, 0x00, 0x00, 0x00];
        let result = MehdBox::decode(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_mehd_box_round_trip_v0() {
        let original = raw_data_v0();
        let mehd = MehdBox::decode(&original).unwrap();

        let mut encoded = [0u8; 8];
        let len = mehd.encode_into(&mut encoded).unwrap();

        assert_eq!(len, original.len());
        assert_eq!(&encoded[..], &original[..]);
    }

    #[test]
    fn test_mehd_box_round_trip_v1() {
        let original = raw_data_v1();
        let mehd = MehdBox::decode(&original).unwrap();

        let mut encoded = [0u8; 12];
        let len = mehd.encode_into(&mut encoded).unwrap();

        assert_eq!(len, original.len());
        assert_eq!(&encoded[..], &original[..]);
    }

    #[test]
    fn test_mehd_box_encode_invalid_version() {
        let mehd = MehdBox {
            version: 2,
            flags: MehdFlags::empty(),
            fragment_duration: 0,
        };

        let mut encoded = [0u8; 12];
        let result = mehd.encode_into(&mut encoded);
        assert!(result.is_err());
    }
}
